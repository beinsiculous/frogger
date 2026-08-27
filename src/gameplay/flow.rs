//! Match lifecycle: starting/ending a match, deaths, home fills, round
//! progression, scoring, and pushing the chaos theme onto the scenery.

use engine_core::prelude::*;

use crate::achievements;
use crate::board::{tile_center, tileset_pixels, TILESET_H, TILESET_W};
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

    /// Reset every counter, rebuild the themed board, spawn the lanes and
    /// frogs for the current mode. Called from mode select, restart, and
    /// pause-restart.
    pub(crate) fn start_game(&mut self, ctx: &mut GameContext) {
        self.clear_match_entities(ctx);

        self.score = 0;
        self.round = 1;
        self.play_time = 0.0;
        self.deaths_this_round = 0;
        self.fills_this_round = [0; 2];
        self.homes = [false; 5];

        self.rebuild_board(ctx);
        self.lanes = LANES
            .iter()
            .map(|&def| spawning::spawn_lane(ctx.world, self.tex_id, def))
            .collect();
        self.home_markers = spawning::spawn_home_markers(ctx.world, self.tex_id);
        self.croc_marker = self.chaos_mode.is_ridiculous()
            .then(|| spawning::spawn_croc(ctx.world, self.tex_id));

        let timer = attempt_timer(self.chaos_mode);
        let count = self.mode.player_count();
        self.frogs = (0..count)
            .map(|index| {
                let start_col = start_col(self.mode, index);
                let mut frog = FrogState::new(start_col, timer);
                frog.x = tile_center(start_col, 0).x;
                frog.entity = Some(spawning::spawn_frog(ctx.world, self.tex_id, index));
                frog
            })
            .collect();

        self.apply_theme(ctx.world);
        self.state = GameState::Playing;
    }

    /// Build (or rebuild) the chaos-themed tileset texture and the board
    /// entity. On texture failure the board falls back to the plain white
    /// texture — flat tiles instead of a crash.
    pub(crate) fn rebuild_board(&mut self, ctx: &mut GameContext) {
        let theme = ChaosTheme::for_mode(self.chaos_mode);
        let tileset = match ctx.assets.create_texture_from_rgba(
            TILESET_W, TILESET_H, &tileset_pixels(&theme),
        ) {
            Ok(handle) => {
                self.tileset_tex = Some(handle.id);
                handle.id
            }
            Err(e) => {
                eprintln!("frogger: tileset texture failed ({e}); using white fallback");
                self.tex_id
            }
        };
        self.board = Some(spawning::spawn_board(ctx.world, tileset));
    }

    /// A frog dies: burst, ripple, life lost, respawn countdown (or
    /// retirement on the last life).
    pub(crate) fn kill_frog(&mut self, ctx: &mut GameContext, index: usize, cause: DeathCause) {
        let pos = self.frog_pos(index);
        let theme = ChaosTheme::for_mode(self.chaos_mode);
        ctx.particles.spawn_burst(pos, &effects::death_burst(cause, &theme, self.tex_id));
        self.ripple_grid(pos, GRID_IMPULSE_DEATH);
        self.deaths_this_round += 1;

        let frog = &mut self.frogs[index];
        frog.lives = frog.lives.saturating_sub(1);
        frog.respawn_timer = RESPAWN_DELAY;
        if frog.lives == 0 {
            frog.retired = true;
        }
    }

    /// A frog reaches an open home slot: score, achievements, marker on,
    /// frog straight back to the start row.
    pub(crate) fn fill_home(&mut self, ctx: &mut GameContext, index: usize, slot: usize) {
        self.homes[slot] = true;
        self.total_homes += 1;
        self.fills_this_round[index.min(1)] += 1;

        let secs_left = self.frogs[index].timer.max(0.0);
        self.score += SCORE_HOME + SCORE_PER_SEC_LEFT * secs_left as u32;

        let pos = tile_center(HOME_COLS[slot], HOME_ROW);
        let theme = ChaosTheme::for_mode(self.chaos_mode);
        ctx.particles.spawn_burst(pos, &effects::home_fill_burst(&theme, self.tex_id));
        self.ripple_grid(pos, GRID_IMPULSE_HOME);

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
        let frog = &mut self.frogs[index];
        frog.x = tile_center(frog.start_col, 0).x;
        frog.row = START_ROW;
        frog.timer = attempt_timer(self.chaos_mode);
        frog.furthest_row = START_ROW;

        if self.homes.iter().all(|&h| h) {
            self.clear_round(ctx);
        }
    }

    /// All five homes filled: bonus, achievements, next round (faster lanes,
    /// homes reset, croc re-picked via the round number).
    fn clear_round(&mut self, ctx: &mut GameContext) {
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
            && self.fills_this_round.iter().all(|&f| f > 0)
        {
            ctx.achievements.unlock(achievements::COOP_ROUND);
        }

        self.round += 1;
        self.homes = [false; 5];
        self.deaths_this_round = 0;
        self.fills_this_round = [0; 2];
        for frog in &mut self.frogs {
            if frog.retired {
                continue;
            }
            frog.x = tile_center(frog.start_col, 0).x;
            frog.row = START_ROW;
            frog.timer = attempt_timer(self.chaos_mode);
            frog.furthest_row = START_ROW;
        }
    }

    /// Out of frogs. The lanes keep scrolling behind the overlay; the next
    /// start clears everything. The pooled score is submitted exactly once
    /// here — the only transition into GameOver — under the mode's
    /// high-score list (co-op shares one score, so one entry).
    pub(crate) fn finish_game(&mut self, ctx: &mut GameContext) {
        let mode = match self.mode {
            GameMode::SinglePlayer => "single",
            GameMode::TwoPlayerCoop => "coop",
        };
        ctx.scores.submit(mode, u64::from(self.score));
        self.state = GameState::GameOver;
    }

    pub(crate) fn reset_to_title(&mut self, ctx: &mut GameContext) {
        self.clear_match_entities(ctx);
        self.state = GameState::TitleScreen { selection: 0 };
    }

    /// Despawn everything a match owns: board, lanes (+ ghosts), frogs,
    /// markers, croc. The background and the grid stay.
    fn clear_match_entities(&mut self, ctx: &mut GameContext) {
        let mut doomed: Vec<EntityId> = Vec::new();
        doomed.extend(self.board.take());
        doomed.extend(self.croc_marker.take());
        doomed.append(&mut self.home_markers);
        for lane in self.lanes.drain(..) {
            for (main, ghost) in lane.sprites {
                doomed.push(main);
                doomed.push(ghost);
            }
        }
        for frog in &mut self.frogs {
            doomed.extend(frog.entity.take());
        }
        self.frogs.clear();
        for entity in doomed {
            ctx.world.remove_entity(&entity).ok();
        }
    }

    /// Push the current chaos mode's look onto the persistent scenery.
    pub(crate) fn apply_theme(&mut self, world: &mut World) {
        let theme = ChaosTheme::for_mode(self.chaos_mode);
        if let Some(bg) = self.background {
            if let Some(s) = world.get_mut::<Sprite>(bg) {
                s.color = theme.bg_color;
            }
        }
        self.grid = Some(default_playfield_grid(&theme));
    }
}

/// Which column a frog respawns at: solo center, co-op split left/right.
pub(crate) fn start_col(mode: GameMode, index: usize) -> u32 {
    match mode {
        GameMode::SinglePlayer => SOLO_START_COL,
        GameMode::TwoPlayerCoop => COOP_START_COLS[index.min(1)],
    }
}
