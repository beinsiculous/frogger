//! Per-frame gameplay orchestration. The pure rules live in `rules` (lane
//! math, deaths, the dive and duty cycles), the pose functions the rules drive
//! in `poses`, and match lifecycle in `flow`.

mod flow;
pub(crate) mod poses;
pub(crate) mod rules;

#[cfg(test)]
pub(crate) use flow::start_col;

use engine_core::prelude::*;

use crate::board;
use crate::constants::*;
use crate::effects;
use crate::spawning;
use crate::types::*;
use poses::*;
use rules::*;

/// Write the chicken's drawn pose — the one writer of it. The facing is the
/// source, and every path that moves a chicken goes through here.
///
/// Both calls are needed and neither is enough alone: `transition_to` is a
/// no-op on the state the machine is already in, so three fast hops up a road
/// would play one jump; and `play` alone is reverted the same frame, because
/// the machine re-selects its own state's clip, so a turn mid-jump would keep
/// drawing the old direction.
pub(crate) fn pose_chicken(world: &mut World, entity: EntityId, facing: Facing, jumping: bool) {
    let state = if jumping {
        facing.jump_state()
    } else {
        facing.idle_state()
    };
    if let Some(machine) = world.get_mut::<ClipStateMachine>(entity) {
        let _ = machine.transition_to(&state);
    }
    if let Some(animation) = world.get_mut::<SpriteAnimation>(entity) {
        let _ = animation.play(&state);
    }
}

impl FroggerGame {
    pub(crate) fn update_gameplay(&mut self, ctx: &mut GameContext) {
        // Pause gate: only an active match is pausable. While paused the
        // whole update freezes — no lane movement, no timers; the overlay
        // draws in the UI pass and the engine holds the backdrop still with
        // the rest of the world.
        if self.state == GameState::Playing {
            let action = self.pause.update(ctx.players, ctx.input, ctx.window_size);
            ctx.time_scale = self.pause.time_scale();
            match action {
                PauseAction::Restart => { self.start_game(ctx); return; }
                PauseAction::QuitToTitle => { self.reset_to_title(ctx); return; }
                PauseAction::ExitGame => { ctx.request_exit(); return; }
                // Skip the rest of the frame so the resuming keypress can't
                // leak into gameplay; the world unfreezes next frame.
                PauseAction::Resumed => return,
                PauseAction::Idle => {}
            }
            if self.pause.is_active() {
                return;
            }
        }

        self.handle_state_input(ctx);
        // Lanes scroll during GameOver too, so traffic keeps flowing behind
        // the overlay; only chicken logic is gated on an active match.
        if matches!(self.state, GameState::Playing | GameState::GameOver) {
            self.play_time += ctx.delta_time;
            self.advance_lanes(ctx.delta_time);
        }
        if self.state == GameState::Playing {
            // A round-clear beat is the one time the match is still `Playing`
            // while nothing is simulated: the traffic and the art run on, and
            // the round advances the frame the beat runs out.
            if self.round_clear_in.is_some() {
                self.step_round_clear(ctx);
            } else {
                self.step_chickens(ctx);
                // A fill inside `step_chickens` may have armed the beat this
                // very frame: a respawn must not run behind it, or a chicken
                // comes back to a board the beat already owns.
                if self.round_clear_in.is_none() {
                    self.step_respawns(ctx);
                }
            }
        }
        self.sync_sprites(ctx);

        board::animate(ctx.world, &self.board_maps, &self.sheets, self.play_time);
    }

    /// Count the round-clear beat down, and advance the round in the frame it
    /// runs out — the nests and the seated chickens reset with it.
    fn step_round_clear(&mut self, ctx: &mut GameContext) {
        let Some(remaining) = self.round_clear_in else {
            return;
        };
        let remaining = remaining - ctx.delta_time;
        if remaining > 0.0 {
            self.round_clear_in = Some(remaining);
            return;
        }
        self.round_clear_in = None;
        self.advance_round(ctx);
    }

    /// Advance every segment along its lane torus.
    fn advance_lanes(&mut self, delta_time: f32) {
        for lane in &mut self.lanes {
            let velocity = lane_velocity(&lane.def, self.chaos_mode, self.round);
            for x in &mut lane.xs {
                *x = wrap_lane_x(*x + velocity * delta_time);
            }
        }
    }

    /// Segment centers of the lane covering `row`, if any.
    fn lane_for_row(&self, row: u32) -> Option<&LaneState> {
        self.lanes.iter().find(|lane| lane.def.row == row)
    }

    /// One frame for every live chicken: timer, riding, hazards, then the hop.
    fn step_chickens(&mut self, ctx: &mut GameContext) {
        for index in 0..self.chickens.len() {
            if !self.chickens[index].active() {
                continue;
            }

            // Attempt timer.
            self.chickens[index].timer -= ctx.delta_time;
            if self.chickens[index].timer <= 0.0 {
                self.kill_chicken(ctx, index, DeathCause::Timeout);
                continue;
            }

            // Riding / drowning / sweeping on soup; traffic on the belts.
            let (x, row) = (self.chickens[index].x, self.chickens[index].row);
            if is_water_row(row) {
                let lane = self.lane_for_row(row).expect("soup rows all have lanes");
                match platform_under(
                    x, &lane.def, &lane.xs, self.chaos_mode, self.round, self.play_time,
                ) {
                    Some(velocity) => {
                        self.chickens[index].x += velocity * ctx.delta_time;
                        if swept_off(self.chickens[index].x) {
                            self.kill_chicken(ctx, index, DeathCause::Swept);
                            continue;
                        }
                    }
                    None => {
                        self.kill_chicken(ctx, index, DeathCause::Drown);
                        continue;
                    }
                }
            } else if is_road_row(row) {
                let lane = self.lane_for_row(row).expect("belt rows all have lanes");
                if road_hit(x, &lane.def, &lane.xs) {
                    self.kill_chicken(ctx, index, DeathCause::Cart);
                    continue;
                }
            }

            self.try_hop(ctx, index);

            // A fill that armed the beat ends this frame's chicken logic at
            // once: the next player's Up must not hop it off the start tile
            // it was just placed on.
            if self.round_clear_in.is_some() {
                break;
            }
        }
    }

    /// Read this chicken's hop input (1P merges both player slots), apply at
    /// most one hop, then resolve what it landed on.
    fn try_hop(&mut self, ctx: &mut GameContext, index: usize) {
        let players: &[PlayerId] = match self.mode {
            GameMode::SinglePlayer => &[PlayerId::P1, PlayerId::P2],
            GameMode::TwoPlayerCoop => {
                if index == 0 { &[PlayerId::P1] } else { &[PlayerId::P2] }
            }
        };
        let just = |action: GameAction| {
            players.iter().any(|&player| ctx.players.just_activated(player, action, ctx.input))
        };
        let Some(hop) = resolve_hop(
            just(GameAction::MoveUp),
            just(GameAction::MoveDown),
            just(GameAction::MoveLeft),
            just(GameAction::MoveRight),
        ) else {
            return;
        };

        let chicken = &mut self.chickens[index];
        let (new_x, new_row) = apply_hop(chicken.x, chicken.row, hop);
        chicken.x = new_x;
        chicken.row = new_row;
        chicken.facing = hop.facing();
        // Land rows re-snap the chicken to the grid; soup keeps the drift.
        if !is_water_row(new_row) && new_row != HOME_ROW {
            chicken.x = crate::board::tile_center(nearest_col(chicken.x), 0).x;
        }
        // First time this attempt reaches a nearer-to-home row: +10 each.
        if new_row < chicken.furthest_row {
            let rows_gained = chicken.furthest_row - new_row;
            chicken.furthest_row = new_row;
            self.score += SCORE_PER_ROW * rows_gained;
        }
        // The hop is instant and the clip plays where the chicken stands: the
        // drawn pose snaps with the rules, so a chicken is never drawn between
        // tiles while the lanes already have it in one.
        if let Some(entity) = chicken.entity {
            pose_chicken(ctx.world, entity, hop.facing(), true);
        }

        let position = self.chicken_pos(index);
        let theme = ChaosTheme::for_mode(self.chaos_mode);
        ctx.particles.spawn_burst(position, &effects::hop_puff(&theme, self.sheets.white));

        // Resolve the landing square immediately — no one-frame grace.
        match self.chickens[index].row {
            HOME_ROW => {
                match home_slot_at(
                    self.chickens[index].x, &self.homes,
                    self.chaos_mode, self.round, self.play_time,
                ) {
                    SlotOutcome::Fill(slot) => self.fill_home(ctx, index, slot),
                    SlotOutcome::Miss => self.kill_chicken(ctx, index, DeathCause::HomeMiss),
                    SlotOutcome::Bun => self.kill_chicken(ctx, index, DeathCause::Bun),
                }
            }
            row if is_road_row(row) => {
                let lane = self.lane_for_row(row).expect("belt rows all have lanes");
                if road_hit(self.chickens[index].x, &lane.def, &lane.xs) {
                    self.kill_chicken(ctx, index, DeathCause::Cart);
                }
            }
            row if is_water_row(row) => {
                let lane = self.lane_for_row(row).expect("soup rows all have lanes");
                if platform_under(
                    self.chickens[index].x, &lane.def, &lane.xs,
                    self.chaos_mode, self.round, self.play_time,
                ).is_none() {
                    self.kill_chicken(ctx, index, DeathCause::Drown);
                }
            }
            _ => {}
        }
    }

    /// Tick down dead chickens; respawn or retire them, and end the match once
    /// every one is retired.
    fn step_respawns(&mut self, ctx: &mut GameContext) {
        let timer = attempt_timer(self.chaos_mode);
        let mut returned: Vec<usize> = Vec::new();
        for (index, chicken) in self.chickens.iter_mut().enumerate() {
            if chicken.retired || chicken.respawn_timer <= 0.0 {
                continue;
            }
            chicken.respawn_timer -= ctx.delta_time;
            if chicken.respawn_timer <= 0.0 {
                chicken.x = crate::board::tile_center(chicken.start_col, 0).x;
                chicken.row = START_ROW;
                chicken.facing = Facing::START;
                chicken.timer = timer;
                chicken.furthest_row = START_ROW;
                returned.push(index);
            }
        }
        // Only the frame a chicken actually comes back re-enters its idle;
        // re-posing a settled chicken every frame would pin it to frame one.
        for index in returned {
            if let Some(entity) = self.chickens[index].entity {
                pose_chicken(ctx.world, entity, Facing::START, false);
            }
        }
        if self.chickens.iter().all(|chicken| chicken.retired) && self.state == GameState::Playing {
            self.finish_game(ctx);
        }
    }

    /// World position of chicken `index` right now.
    pub(crate) fn chicken_pos(&self, index: usize) -> Vec2 {
        let chicken = &self.chickens[index];
        Vec2::new(chicken.x, crate::board::tile_center(0, chicken.row).y)
    }

    /// Push every logical position into the sprite world: the chickens, the
    /// lane segments (+ seam ghosts), the nests' seats, the bun and the poses
    /// the rules own, and the life icons.
    fn sync_sprites(&mut self, ctx: &mut GameContext) {
        for index in 0..self.chickens.len() {
            let position = self.chicken_pos(index);
            let chicken = &self.chickens[index];
            let Some(entity) = chicken.entity else {
                continue;
            };
            let visible = chicken.active();
            if let Some(transform) = ctx.world.get_mut::<Transform2D>(entity) {
                transform.position = position;
            }
            if let Some(sprite) = ctx.world.get_mut::<Sprite>(entity) {
                sprite.visible = visible;
            }
        }

        for lane in &self.lanes {
            let y = crate::board::tile_center(0, lane.def.row).y;
            let half = lane.def.half_len();
            for (segment, pairs) in lane.sprites.iter().enumerate() {
                let center = lane.xs[segment];
                // The ghost shows one period away, and only while the segment
                // straddles the seam — its body pokes out both edges then.
                let straddling = center.abs() + half > LANE_PERIOD / 2.0;
                let ghost_center = center - LANE_PERIOD * center.signum();
                for (sprite, &(main, ghost)) in pairs.iter().enumerate() {
                    let offset = lane.def.sprite_offset(sprite);
                    if let Some(transform) = ctx.world.get_mut::<Transform2D>(main) {
                        transform.position = Vec2::new(center + offset, y);
                    }
                    if let Some(transform) = ctx.world.get_mut::<Transform2D>(ghost) {
                        transform.position = Vec2::new(ghost_center + offset, y);
                    }
                    if let Some(rendered) = ctx.world.get_mut::<Sprite>(ghost) {
                        rendered.visible = straddling;
                    }
                }
            }
        }
        for lane in &self.lanes {
            pose_lane_crackers(ctx.world, lane, self.chaos_mode, self.play_time);
        }

        self.sync_bun(ctx);
        self.sync_life_icons(ctx);
    }

    /// The bun surfaces inside the nest it guards, and draws nothing at all
    /// while its nest is filled or while its duty cycle has it away. Its
    /// position is written even when it is hidden, so it never pops.
    fn sync_bun(&mut self, ctx: &mut GameContext) {
        let Some(bun) = self.bun else {
            return;
        };
        let placement =
            spawning::bun_placement(self.chaos_mode, self.round, self.play_time, &self.homes);
        // The position is written whether or not the bun is drawn, so it
        // surfaces where it will be instead of sliding into place.
        if let Some(transform) = ctx.world.get_mut::<Transform2D>(bun) {
            transform.position = placement.position;
        }
        if let Some(sprite) = ctx.world.get_mut::<Sprite>(bun) {
            sprite.visible = placement.is_visible();
        }
        if let Some(pose) = placement.pose {
            write_pose(ctx.world, bun, pose);
        }
    }

    /// The life icons are the count now: one is shown per life left.
    fn sync_life_icons(&mut self, ctx: &mut GameContext) {
        let per_player = STARTING_LIVES as usize;
        for (index, &icon) in self.life_icons.iter().enumerate() {
            let player = index / per_player;
            let life = index % per_player;
            let lives = self.chickens.get(player).map_or(0, |chicken| chicken.lives) as usize;
            if let Some(sprite) = ctx.world.get_mut::<Sprite>(icon) {
                sprite.visible = life < lives;
            }
        }
    }

    /// Push a radial shockwave into the board's backdrop grid: the engine
    /// applies it to every backdrop on its next running frame.
    pub(crate) fn ripple_grid(&mut self, world: &mut World, position: Vec2, (strength, radius): (f32, f32)) {
        ripple(world, GridImpulse::Radial { position, strength, radius, attractive: false });
    }
}
