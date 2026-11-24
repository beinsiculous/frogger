//! Crosswalk Carnival: original G-major electro-swing with offbeat bass and chords.
//! The backing plays independently through menus and pause. The two chickens share
//! the galop hook on distinct short tuned hop voices and their own sources, including
//! clamped hop requests. Death uses a low chord; home and clear use bright rewards,
//! on the game's source. Pause navigation walks the playful jam counter-line.
//! GameMusic retains restart, pulse and telemetry. The frame still sends chicken 0's
//! hop and landing outcome, then chicken 1's, then respawns; no input rules change.

use engine_core::prelude::*;

use crate::gameplay::ripple_grid;

/// The song under the asset base: an agent's first cut, under `ai/music` until a person's
/// pass promotes it.
pub(crate) const SONG_PATH: &str = "ai/music/ai_chicken_coop.mid";
/// The song's six bindings: one per chicken on the one `galop` part, the three other moments,
/// and the pause menu's jam.
pub(crate) const HOP_BINDING: &str = "hop";
pub(crate) const HOP_TWO_BINDING: &str = "hop_two";
pub(crate) const DEATH_BINDING: &str = "death";
pub(crate) const HOME_BINDING: &str = "home";
pub(crate) const CLEAR_BINDING: &str = "clear";
pub(crate) const MENU_BINDING: &str = "menu";
/// A bar line's ripple under each active chicken: a nest's ripple, felt and not a death.
pub(crate) const BEAT_PULSE_STRENGTH: f32 = 300.0;
pub(crate) const BEAT_PULSE_RADIUS: f32 = 110.0;
/// A beat that is not a bar line ripples at this share of a bar's; zero pulses on the bars only.
pub(crate) const BEAT_PULSE_FRACTION: f32 = 0.25;

/// A moment the game plays: chicken `index`'s hop on its entity's source, a chicken lost to
/// any cause, a nest filled that leaves the round open, the round cleared by the fifth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Moment {
    Hop { chicken: EntityId, index: usize },
    Death,
    Home,
    Clear,
}

/// What the game itself sent, beside the conductor's counters: what a listener's ear is
/// checked against. The counts run for the session; a match start never resets them. The
/// jam and the pulse are counted by `GameMusic`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct MusicCounts {
    pub(crate) hops: u64,
    pub(crate) deaths: u64,
    pub(crate) homes: u64,
    pub(crate) clears: u64,
}

/// See the module docs.
#[derive(Debug, Clone)]
pub(crate) struct Music {
    pub(crate) game: GameMusic,
    pub(crate) counts: MusicCounts,
    /// The bindings this frame triggered, in the order the game sent them: production's
    /// call order recorded for the same-frame order tests, never reconstructed.
    #[cfg(test)]
    pub(crate) triggered: Vec<&'static str>,
    /// Where this frame's beat rippled the grid, one anchor an active chicken.
    #[cfg(test)]
    pub(crate) pulsed_at: Vec<Vec2>,
}

impl Default for Music {
    fn default() -> Self {
        Self {
            game: GameMusic::new(SONG_PATH, "chicken coop", "CHICKEN_COOP_TELEMETRY"),
            counts: MusicCounts::default(),
            #[cfg(test)]
            triggered: Vec::new(),
            #[cfg(test)]
            pulsed_at: Vec::new(),
        }
    }
}

impl Music {
    /// Trigger `moment`'s binding and count it: a hop on its chicken's source, the first
    /// chicken's on the harmonic triangle and the second's on the rounded sine; the rest on the game's.
    pub(crate) fn play(&mut self, ctx: &mut GameContext, moment: Moment) {
        let counts = &mut self.counts;
        let (binding, entity, count) = match moment {
            Moment::Hop { chicken, index } => (hop_binding(index), Some(chicken), &mut counts.hops),
            Moment::Death => (DEATH_BINDING, None, &mut counts.deaths),
            Moment::Home => (HOME_BINDING, None, &mut counts.homes),
            Moment::Clear => (CLEAR_BINDING, None, &mut counts.clears),
        };
        *count += 1;
        let source = if let Some(entity) = entity { ctx.sources.entity(entity) } else { ctx.sources.game() };
        #[cfg(test)]
        self.triggered.push(binding);
        ctx.sources.trigger(ctx.audio, binding, source);
    }

    /// The pause menu's frame: a move up or down jams one unit of the playful counter-line.
    pub(crate) fn follow_pause(&mut self, ctx: &mut GameContext, paused: bool) {
        if self.game.follow_pause(ctx, paused, MENU_BINDING) {
            #[cfg(test)]
            self.triggered.push(MENU_BINDING);
        }
    }

    /// A running round's frame, after the sprites are synced: a bar line ripples the grid
    /// under each of `chickens` — every active chicken where it is drawn this frame — and
    /// another beat a quarter as hard. Never called while paused (the frozen grid discards
    /// impulses), off `Playing`, or in the round-clear beat.
    pub(crate) fn pulse(&mut self, ctx: &mut GameContext, chickens: &[Vec2]) {
        let strength = match self.game.beat_pulse(ctx) {
            Some(BeatPulse::Bar) => BEAT_PULSE_STRENGTH,
            Some(BeatPulse::Beat) => BEAT_PULSE_STRENGTH * BEAT_PULSE_FRACTION,
            None => return,
        };
        #[cfg(test)]
        self.pulsed_at.extend_from_slice(chickens);
        for &chicken in chickens {
            ripple_grid(ctx.world, chicken, (strength, BEAT_PULSE_RADIUS));
        }
    }

    /// Every frame, whatever the state: the engine's frame ring, telemetry and refusal
    /// lines, the telemetry ending in the game's counts.
    pub(crate) fn update(&mut self, ctx: &mut GameContext) {
        #[cfg(test)]
        {
            self.triggered.clear();
            self.pulsed_at.clear();
        }
        let (counts, jam_units, pulsed) = (self.counts, self.game.jam_units(), self.game.pulsed());
        self.game.update(ctx, || {
            format!(
                "hops={} deaths={} homes={} clears={} jam_units={jam_units} pulsed={pulsed}",
                counts.hops, counts.deaths, counts.homes, counts.clears
            )
        });
    }
}

/// The binding chicken `index`'s hop plays on the shared galop: the round chicken's triangle,
/// the tall one's harmonic sine.
fn hop_binding(index: usize) -> &'static str {
    if index == 0 { HOP_BINDING } else { HOP_TWO_BINDING }
}
