//! Per-frame gameplay orchestration. The pure rules live in `rules`
//! (lane math, deaths, croc/dive cycles); match lifecycle lives in `flow`.

mod flow;
pub(crate) mod rules;

#[cfg(test)]
pub(crate) use flow::start_col;

use engine_core::prelude::*;

use crate::constants::*;
use crate::effects;
use crate::types::*;
use rules::*;

impl FroggerGame {
    pub(crate) fn update_gameplay(&mut self, ctx: &mut GameContext) {
        // F1 keeps the engine collider-overlay toggle wired for convention
        // parity (this game has no colliders, so it only affects the grid).
        if ctx.input.is_key_just_pressed(KeyCode::F1) {
            self.debug_colliders = !self.debug_colliders;
        }

        // Pause gate: only an active match is pausable. While paused the
        // whole update freezes — no lane movement, no timers; the overlay
        // draws in the UI pass and the grid re-emits without advancing.
        if self.state == GameState::Playing {
            let action = self.pause.update(ctx.players, ctx.input);
            ctx.time_scale = self.pause.time_scale();
            match action {
                PauseAction::Restart => { self.start_game(ctx); return; }
                PauseAction::QuitToTitle => { self.reset_to_title(ctx); return; }
                PauseAction::ExitGame => { ctx.exit_requested = true; return; }
                // Skip the rest of the frame so the resuming keypress can't
                // leak into gameplay; the world unfreezes next frame.
                PauseAction::Resumed => return,
                PauseAction::Idle => {}
            }
            if self.pause.is_active() {
                step_and_emit_grid(
                    self.grid.as_mut(), ctx.world, ctx.lines, 0.0, self.debug_colliders,
                );
                return;
            }
        }

        self.handle_state_input(ctx);
        // Lanes scroll during GameOver too, so traffic keeps flowing behind
        // the overlay; only frog logic is gated on an active match.
        if matches!(self.state, GameState::Playing | GameState::GameOver) {
            self.play_time += ctx.delta_time;
            self.advance_lanes(ctx.delta_time);
        }
        if self.state == GameState::Playing {
            self.step_frogs(ctx);
            self.step_respawns(ctx);
        }
        self.sync_sprites(ctx);

        step_and_emit_grid(
            self.grid.as_mut(), ctx.world, ctx.lines, ctx.delta_time, self.debug_colliders,
        );
    }

    /// Advance every obstacle along its lane torus.
    fn advance_lanes(&mut self, dt: f32) {
        for lane in &mut self.lanes {
            let v = lane_velocity(&lane.def, self.chaos_mode, self.round);
            for x in &mut lane.xs {
                *x = wrap_lane_x(*x + v * dt);
            }
        }
    }

    /// Obstacle centers of the lane covering `row`, if any.
    fn lane_for_row(&self, row: u32) -> Option<&LaneState> {
        self.lanes.iter().find(|l| l.def.row == row)
    }

    /// One frame for every live frog: timer, riding, hazards, then the hop.
    fn step_frogs(&mut self, ctx: &mut GameContext) {
        for index in 0..self.frogs.len() {
            if !self.frogs[index].active() {
                continue;
            }

            // Attempt timer.
            self.frogs[index].timer -= ctx.delta_time;
            if self.frogs[index].timer <= 0.0 {
                self.kill_frog(ctx, index, DeathCause::Timeout);
                continue;
            }

            // Riding / drowning / sweeping on water; traffic on roads.
            let (x, row) = (self.frogs[index].x, self.frogs[index].row);
            if is_water_row(row) {
                let lane = self.lane_for_row(row).expect("water rows all have lanes");
                match platform_under(
                    x, &lane.def, &lane.xs, self.chaos_mode, self.round, self.play_time,
                ) {
                    Some(v) => {
                        self.frogs[index].x += v * ctx.delta_time;
                        if swept_off(self.frogs[index].x) {
                            self.kill_frog(ctx, index, DeathCause::Swept);
                            continue;
                        }
                    }
                    None => {
                        self.kill_frog(ctx, index, DeathCause::Drown);
                        continue;
                    }
                }
            } else if is_road_row(row) {
                let lane = self.lane_for_row(row).expect("road rows all have lanes");
                if road_hit(x, &lane.def, &lane.xs) {
                    self.kill_frog(ctx, index, DeathCause::Car);
                    continue;
                }
            }

            self.try_hop(ctx, index);
        }
    }

    /// Read this frog's hop input (1P merges both player slots), apply at
    /// most one hop, then resolve what it landed on.
    fn try_hop(&mut self, ctx: &mut GameContext, index: usize) {
        let players: &[PlayerId] = match self.mode {
            GameMode::SinglePlayer => &[PlayerId::P1, PlayerId::P2],
            GameMode::TwoPlayerCoop => {
                if index == 0 { &[PlayerId::P1] } else { &[PlayerId::P2] }
            }
        };
        let just = |action: GameAction| {
            players.iter().any(|&p| ctx.players.just_activated(p, action, ctx.input))
        };
        let Some(hop) = resolve_hop(
            just(GameAction::MoveUp),
            just(GameAction::MoveDown),
            just(GameAction::MoveLeft),
            just(GameAction::MoveRight),
        ) else {
            return;
        };

        let frog = &mut self.frogs[index];
        let (new_x, new_row) = apply_hop(frog.x, frog.row, hop);
        frog.x = new_x;
        frog.row = new_row;
        // Land rows re-snap the frog to the grid; water keeps the drift.
        if !is_water_row(new_row) && new_row != HOME_ROW {
            frog.x = crate::board::tile_center(nearest_col(frog.x), 0).x;
        }
        // First time this attempt reaches a nearer-to-home row: +10 each.
        if new_row < frog.furthest_row {
            let rows_gained = frog.furthest_row - new_row;
            frog.furthest_row = new_row;
            self.score += SCORE_PER_ROW * rows_gained;
        }
        let pos = self.frog_pos(index);
        let theme = ChaosTheme::for_mode(self.chaos_mode);
        ctx.particles.spawn_burst(pos, &effects::hop_puff(&theme, self.tex_id));

        // Resolve the landing square immediately — no one-frame grace.
        match self.frogs[index].row {
            HOME_ROW => {
                match home_slot_at(
                    self.frogs[index].x, &self.homes,
                    self.chaos_mode, self.round, self.play_time,
                ) {
                    SlotOutcome::Fill(slot) => self.fill_home(ctx, index, slot),
                    SlotOutcome::Miss => self.kill_frog(ctx, index, DeathCause::HomeMiss),
                    SlotOutcome::Croc => self.kill_frog(ctx, index, DeathCause::Croc),
                }
            }
            row if is_road_row(row) => {
                let lane = self.lane_for_row(row).expect("road rows all have lanes");
                if road_hit(self.frogs[index].x, &lane.def, &lane.xs) {
                    self.kill_frog(ctx, index, DeathCause::Car);
                }
            }
            row if is_water_row(row) => {
                let lane = self.lane_for_row(row).expect("water rows all have lanes");
                if platform_under(
                    self.frogs[index].x, &lane.def, &lane.xs,
                    self.chaos_mode, self.round, self.play_time,
                ).is_none() {
                    self.kill_frog(ctx, index, DeathCause::Drown);
                }
            }
            _ => {}
        }
    }

    /// Tick down dead frogs; respawn or retire them, and end the match once
    /// every frog is retired.
    fn step_respawns(&mut self, ctx: &mut GameContext) {
        let timer = attempt_timer(self.chaos_mode);
        for frog in &mut self.frogs {
            if frog.retired || frog.respawn_timer <= 0.0 {
                continue;
            }
            frog.respawn_timer -= ctx.delta_time;
            if frog.respawn_timer <= 0.0 {
                frog.x = crate::board::tile_center(frog.start_col, 0).x;
                frog.row = START_ROW;
                frog.timer = timer;
                frog.furthest_row = START_ROW;
            }
        }
        if self.frogs.iter().all(|f| f.retired) && self.state == GameState::Playing {
            self.finish_game();
        }
    }

    /// World position of frog `index` right now.
    pub(crate) fn frog_pos(&self, index: usize) -> Vec2 {
        let frog = &self.frogs[index];
        Vec2::new(frog.x, crate::board::tile_center(0, frog.row).y)
    }

    /// Push every logical position into the sprite world: frogs, obstacles
    /// (+ seam ghosts), home markers, and the croc.
    fn sync_sprites(&mut self, ctx: &mut GameContext) {
        for index in 0..self.frogs.len() {
            let pos = self.frog_pos(index);
            let frog = &self.frogs[index];
            let Some(entity) = frog.entity else { continue };
            let visible = frog.active();
            if let Some(t) = ctx.world.get_mut::<Transform2D>(entity) {
                t.position = pos;
            }
            if let Some(s) = ctx.world.get_mut::<Sprite>(entity) {
                s.visible = visible;
            }
        }

        for lane in &self.lanes {
            let y = crate::board::tile_center(0, lane.def.row).y;
            let half = lane.def.half_len();
            for (i, &x) in lane.xs.iter().enumerate() {
                let (main, ghost) = lane.sprites[i];
                let submerged = lane.def.kind == LaneKind::Turtles
                    && turtles_submerged(self.chaos_mode, lane.def.row, i, self.play_time);
                if let Some(t) = ctx.world.get_mut::<Transform2D>(main) {
                    t.position = Vec2::new(x, y);
                }
                if let Some(s) = ctx.world.get_mut::<Sprite>(main) {
                    s.visible = !submerged;
                }
                // Ghost: shown one period away only while the obstacle
                // straddles the torus seam (its body pokes out both edges).
                let straddling = x.abs() + half > LANE_PERIOD / 2.0;
                let ghost_x = x - LANE_PERIOD * x.signum();
                if let Some(t) = ctx.world.get_mut::<Transform2D>(ghost) {
                    t.position = Vec2::new(ghost_x, y);
                }
                if let Some(s) = ctx.world.get_mut::<Sprite>(ghost) {
                    s.visible = straddling && !submerged;
                }
            }
        }

        for (i, &marker) in self.home_markers.iter().enumerate() {
            if let Some(s) = ctx.world.get_mut::<Sprite>(marker) {
                s.visible = self.homes[i];
            }
        }

        if let Some(croc) = self.croc_marker {
            let slot = croc_slot(self.chaos_mode, self.round);
            let visible = match slot {
                Some(slot) => !self.homes[slot] && croc_present(self.round, self.play_time),
                None => false,
            };
            if let (Some(slot), Some(t)) = (slot, ctx.world.get_mut::<Transform2D>(croc)) {
                t.position = crate::board::tile_center(HOME_COLS[slot], HOME_ROW);
            }
            if let Some(s) = ctx.world.get_mut::<Sprite>(croc) {
                s.visible = visible;
            }
        }
    }

    /// Push a radial shockwave into the deforming grid.
    pub(crate) fn ripple_grid(&mut self, position: Vec2, (strength, radius): (f32, f32)) {
        if let Some(grid) = self.grid.as_mut() {
            grid.apply_impulse(&GridImpulse::Radial { position, strength, radius, attractive: false });
        }
    }
}
