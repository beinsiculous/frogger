//! Match lifecycle: starting/ending a match, deaths, nest fills, the
//! round-clear beat, and pushing the chaos theme onto the scenery.

use engine_core::prelude::*;

use crate::achievements;
use crate::board::tile_center;
use crate::constants::*;
use crate::effects;
use crate::gameplay::rules::{attempt_timer, LANES};
use crate::spawning;
use crate::types::*;

impl FroggerGame {
    /// State changes on the game-over overlay: Action1 restarts, Menu bails
    /// to the title. During play, Menu is the pause gate's business.
    pub(crate) fn handle_state_input(&mut self, ctx: &mut GameContext) {
        let primary = ctx.players.just_activated_any(GameAction::Action1, ctx.input);
        let menu = ctx.players.just_activated_any(GameAction::Menu, ctx.input);
        if self.state == GameState::GameOver {
            if primary {
                self.start_game(ctx);
            } else if menu {
                self.reset_to_title(ctx);
            }
        }
    }

    /// Reset every counter, lay the board, and spawn the lanes, the nests, the
    /// bun and the chickens for the current mode. Called from mode select,
    /// restart, and pause-restart.
    pub(crate) fn start_game(&mut self, ctx: &mut GameContext) {
        self.clear_match_entities(ctx);

        self.score = 0;
        self.round = 1;
        self.play_time = 0.0;
        self.deaths_this_round = 0;
        self.fills_this_round = [0; 2];
        self.homes = [false; 5];

        let mode = self.mode;
        let chaos_mode = self.chaos_mode;
        let theme = ChaosTheme::for_mode(chaos_mode);

        self.board_maps = spawning::spawn_board(ctx.world, &self.sheets);
        self.lanes = LANES
            .iter()
            .map(|&def| {
                spawning::spawn_lane(ctx.world, &self.sheets, def, chaos_mode, self.play_time)
            })
            .collect();
        self.nests = spawning::spawn_nests(ctx.world, &self.sheets);
        // Only the Ridiculous family has a bun to guard a nest.
        self.bun = if chaos_mode.is_ridiculous() {
            Some(spawning::spawn_bun(
                ctx.world, &self.sheets, chaos_mode, self.round, self.play_time, &self.homes,
            ))
        } else {
            None
        };
        self.life_icons = spawning::spawn_life_icons(ctx.world, &self.sheets, mode);
        self.backdrop = Some(spawning::spawn_backdrop(ctx.world, &theme));

        let timer = attempt_timer(chaos_mode);
        self.chickens = (0..mode.player_count())
            .map(|index| {
                let mut chicken = ChickenState::new(start_col(mode, index), timer);
                chicken.entity = Some(spawning::spawn_player(ctx.world, &self.sheets, index));
                chicken
            })
            .collect();
        // The spawn frame draws before any sync runs, so the chickens are stood
        // on their start tiles here rather than a frame later at the origin.
        self.place_chickens_at_start(ctx, Revive::Waiting);

        self.apply_theme(ctx.world);
        self.state = GameState::Playing;
    }

    /// A chicken dies: its feather poof, the soup's splash under it when the
    /// soup is what took it, a ripple, a life lost, and its respawn countdown
    /// (or retirement on the last life).
    pub(crate) fn kill_chicken(&mut self, ctx: &mut GameContext, index: usize, cause: DeathCause) {
        let position = self.chicken_pos(index);
        // One effect per cause: a poof everywhere, and under it the soup's own
        // sauce when the chicken drowned or was swept off a raft.
        if cause.is_soup() {
            let theme = ChaosTheme::for_mode(self.chaos_mode);
            ctx.particles
                .spawn_burst(position, &effects::soup_splash(&theme, self.sheets.white));
        }
        let poof = spawning::spawn_oneshot(
            ctx.world,
            "Feather poof",
            &FEATHER_POOF,
            &self.sheets.feather_poof,
            position,
            POOF_CLIP,
        );
        self.transient_visuals.push(poof);
        self.ripple_grid(ctx.world, position, GRID_IMPULSE_DEATH);
        self.deaths_this_round += 1;

        let chicken = &mut self.chickens[index];
        chicken.lives = chicken.lives.saturating_sub(1);
        chicken.respawn_timer = RESPAWN_DELAY;
        if chicken.lives == 0 {
            chicken.retired = true;
        }
    }

    /// A chicken reaches an open nest: score, achievements, the nest's arrival
    /// and the seated chicken, the celebration, and straight back to the start
    /// row. The fifth one hands the round its rewards and opens the beat.
    pub(crate) fn fill_home(&mut self, ctx: &mut GameContext, index: usize, slot: usize) {
        self.homes[slot] = true;
        self.total_homes += 1;
        self.fills_this_round[index.min(1)] += 1;

        let secs_left = self.chickens[index].timer.max(0.0);
        self.score += SCORE_HOME + SCORE_PER_SEC_LEFT * secs_left as u32;

        if let Some(nest) = self.nests[slot] {
            if let Some(machine) = ctx.world.get_mut::<ClipStateMachine>(nest) {
                let _ = machine.transition_to(NEST_ARRIVAL);
            }
        }
        self.seated[slot] = Some(spawning::spawn_seated(ctx.world, &self.sheets, index, slot));
        let burst = spawning::spawn_oneshot(
            ctx.world,
            "Arrival burst",
            &ARRIVAL_BURST,
            &self.sheets.arrival_burst,
            spawning::nest_center(slot),
            CELEBRATE_CLIP,
        );
        self.transient_visuals.push(burst);
        self.ripple_grid(ctx.world, tile_center(HOME_COLS[slot], HOME_ROW), GRID_IMPULSE_HOME);

        ctx.achievements.unlock(achievements::FIRST_HOME);
        if secs_left >= SPEEDY_SECS_LEFT {
            ctx.achievements.unlock(achievements::SPEEDY);
        }
        if self.total_homes >= HOMES_MILESTONE {
            ctx.achievements.unlock(achievements::HOMES_25);
        }
        if self.score >= SCORE_TIER {
            ctx.achievements.unlock(achievements::SCORE_5K);
        }

        // Straight back to the start; the attempt timer refills.
        let timer = attempt_timer(self.chaos_mode);
        self.place_chicken_at_start(ctx, index, timer, Revive::Waiting);

        if self.homes.iter().all(|&home| home) {
            self.award_round_clear(ctx);
        }
    }

    /// The fifth arrival pays the round's rewards and opens the beat.
    ///
    /// They are read here, while the fill that earned them is still the last
    /// thing that happened, rather than when the beat ends: a Restart or a
    /// quit inside the beat must not forfeit a deathless clear.
    fn award_round_clear(&mut self, ctx: &mut GameContext) {
        self.score += SCORE_ROUND_CLEAR;
        if self.score >= SCORE_TIER {
            ctx.achievements.unlock(achievements::SCORE_5K);
        }
        ctx.achievements.unlock(achievements::ROUND_CLEAR);
        if self.deaths_this_round == 0 {
            ctx.achievements.unlock(achievements::DEATHLESS_ROUND);
        }
        if self.chaos_mode == ChaosMode::Insiculous {
            ctx.achievements.unlock(achievements::INSICULOUS_CLEAR);
        }
        if self.mode == GameMode::TwoPlayerCoop
            && self.fills_this_round.iter().all(|&fills| fills > 0)
        {
            ctx.achievements.unlock(achievements::COOP_ROUND);
        }

        // The beat opens on a fresh board: every live chicken back on its
        // start tile, so none stands unsimulated on a raft that drifts on
        // without it while the beat runs. A partner still in its respawn
        // wait stays gone — its poof plays out through the beat, and the
        // round's end is what stands it back up.
        self.place_chickens_at_start(ctx, Revive::Never);
        self.round_clear_in = Some(ROUND_CLEAR_BEAT);
    }

    /// The beat's end: the round number, the nests, the per-round counters and
    /// the chickens' placement all move in this one frame, and the seats empty
    /// with them.
    pub(crate) fn advance_round(&mut self, ctx: &mut GameContext) {
        self.round += 1;
        self.homes = [false; 5];
        self.deaths_this_round = 0;
        self.fills_this_round = [0; 2];
        self.place_chickens_at_start(ctx, Revive::Waiting);

        for slot in 0..self.homes.len() {
            if let Some(seated) = self.seated[slot].take() {
                ctx.world.remove_entity(&seated).ok();
            }
            if let Some(nest) = self.nests[slot] {
                if let Some(machine) = ctx.world.get_mut::<ClipStateMachine>(nest) {
                    let _ = machine.transition_to(NEST_EMPTY);
                }
            }
        }
    }

    /// Put every live chicken back on its start tile, facing north, with a
    /// full attempt timer. A match's start and a round's clear do this with
    /// `Revive::Waiting`, so both players begin together; the beat's opening
    /// does it with `Revive::Never`, leaving a chicken still in its respawn
    /// wait where it died until the round ends.
    pub(crate) fn place_chickens_at_start(&mut self, ctx: &mut GameContext, revive: Revive) {
        let timer = attempt_timer(self.chaos_mode);
        for index in 0..self.chickens.len() {
            self.place_chicken_at_start(ctx, index, timer, revive);
        }
    }

    /// The same, for one chicken.
    fn place_chicken_at_start(&mut self, ctx: &mut GameContext, index: usize, timer: f32, revive: Revive) {
        if !self.stand_chicken_at_start(index, timer, revive) {
            return;
        }
        let (start_col, entity) = (self.chickens[index].start_col, self.chickens[index].entity);
        if let Some(entity) = entity {
            spawning::place_on_start_tile(ctx.world, entity, start_col);
        }
    }

    /// The logical half, which needs no context: stand chicken `index` back on
    /// its start tile alive — out of any respawn wait, with a full attempt
    /// timer, facing north. A retired chicken stays retired: it is out of the
    /// match, and its partner plays on without it.
    ///
    /// Returns whether the chicken was stood: a retired one never is, and one
    /// still in its respawn wait only when `revive` says so — the beat's
    /// opening leaves it gone, the round's end brings it back.
    pub(crate) fn stand_chicken_at_start(&mut self, index: usize, timer: f32, revive: Revive) -> bool {
        let chicken = &self.chickens[index];
        if chicken.retired || (chicken.respawn_timer > 0.0 && revive == Revive::Never) {
            return false;
        }
        let start_col = self.chickens[index].start_col;
        let chicken = &mut self.chickens[index];
        chicken.x = tile_center(start_col, 0).x;
        chicken.row = START_ROW;
        chicken.facing = Facing::START;
        chicken.timer = timer;
        chicken.furthest_row = START_ROW;
        chicken.respawn_timer = 0.0;
        true
    }

    /// Out of chickens. The lanes keep scrolling behind the overlay; the next
    /// start clears everything. The pooled score is submitted exactly once
    /// here — the only transition into GameOver — under the mode's high-score
    /// list (co-op shares one score, so one entry).
    pub(crate) fn finish_game(&mut self, ctx: &mut GameContext) {
        let mode = match self.mode {
            GameMode::SinglePlayer => "single",
            GameMode::TwoPlayerCoop => "coop",
        };
        let _ = ctx.scores.submit(mode, u64::from(self.score));
        self.state = GameState::GameOver;
    }

    pub(crate) fn reset_to_title(&mut self, ctx: &mut GameContext) {
        self.clear_match_entities(ctx);
        self.state = GameState::TitleScreen { selection: 0 };
    }

    /// Despawn everything a match owns, through the one field that holds each
    /// kind: the four maps, the five nests and their seats, the bun, the life
    /// icons, the backdrop, every detached one-shot, the lanes plus ghosts,
    /// and the chickens. The background and the sheets stay.
    ///
    /// The round-clear beat is one of those owners: a Restart inside a beat
    /// leaves nothing of it behind.
    fn clear_match_entities(&mut self, ctx: &mut GameContext) {
        let mut doomed: Vec<EntityId> = Vec::new();
        doomed.extend(self.board_maps.drain(..).map(|map| map.entity));
        doomed.extend(self.nests.iter_mut().filter_map(Option::take));
        doomed.extend(self.seated.iter_mut().filter_map(Option::take));
        doomed.extend(self.bun.take());
        doomed.append(&mut self.life_icons);
        doomed.append(&mut self.transient_visuals);
        doomed.extend(self.backdrop.take());
        for lane in self.lanes.drain(..) {
            for pairs in lane.sprites {
                for (main, ghost) in pairs {
                    doomed.push(main);
                    doomed.push(ghost);
                }
            }
        }
        for chicken in &mut self.chickens {
            doomed.extend(chicken.entity.take());
        }
        self.chickens.clear();
        self.round_clear_in = None;
        for entity in doomed {
            ctx.world.remove_entity(&entity).ok();
        }
    }

    /// Push the current chaos mode's look onto the scenery.
    ///
    /// The tiles are never tinted: the art is the art, and the theme reaches
    /// the band, the grid and the particles only.
    pub(crate) fn apply_theme(&mut self, world: &mut World) {
        let theme = ChaosTheme::for_mode(self.chaos_mode);
        if let Some(background) = self.background {
            if let Some(sprite) = world.get_mut::<Sprite>(background) {
                sprite.color = theme.bg_color;
            }
        }
        if let Some(backdrop) = self.backdrop {
            if let Some(grid) = world.get_mut::<GridBackdrop>(backdrop) {
                grid.color = spawning::backdrop_color(&theme);
            }
        }
    }
}

/// Which column a chicken respawns at: solo center, co-op split left/right.
pub(crate) fn start_col(mode: GameMode, index: usize) -> u32 {
    match mode {
        GameMode::SinglePlayer => SOLO_START_COL,
        GameMode::TwoPlayerCoop => COOP_START_COLS[index.min(1)],
    }
}
