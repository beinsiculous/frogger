//! Menu screens: navigation and selection. Match lifecycle lives in
//! `gameplay`.

use engine_core::prelude::*;
use crate::types::*;

/// One-line description of what each chaos mode means in this game.
pub(crate) fn mode_hint(mode: ChaosMode) -> &'static str {
    match mode {
        ChaosMode::Normal => "The classic crossing. Steady traffic, honest turtles.",
        ChaosMode::Insane => "Traffic 50% faster and the clock runs short.",
        ChaosMode::Ridiculous => "Turtles dive under you - and a croc squats in one home.",
        ChaosMode::Insiculous => "Diving turtles, croc, fast lanes, short clock. Good luck.",
    }
}

impl FroggerGame {
    pub(crate) fn update_title_input(&mut self, ctx: &mut GameContext, selection: u8) {
        let input = MenuInput::read(ctx.input);
        let selection = input.navigate(selection, 4);
        self.state = GameState::TitleScreen { selection };

        if input.confirm {
            match selection {
                0 => {
                    self.mode = GameMode::SinglePlayer;
                    self.state = GameState::ModeSelect { selection: 0 };
                }
                1 => {
                    self.mode = GameMode::TwoPlayerCoop;
                    self.state = GameState::ModeSelect { selection: 0 };
                }
                2 => self.state = GameState::Achievements,
                _ => ctx.exit_requested = true,
            }
        }
    }

    pub(crate) fn update_achievements_input(&mut self, ctx: &mut GameContext) {
        let input = MenuInput::read(ctx.input);
        if input.back || input.confirm {
            self.state = GameState::TitleScreen { selection: 2 };
        }
    }

    pub(crate) fn update_mode_select_input(&mut self, ctx: &mut GameContext, selection: u8) {
        let input = MenuInput::read(ctx.input);
        let count = ChaosMode::ALL.len() as u8;
        let selection = input.navigate(selection, count);
        self.state = GameState::ModeSelect { selection };

        if input.back {
            self.state = GameState::TitleScreen { selection: 0 };
        } else if input.confirm {
            self.chaos_mode = ChaosMode::ALL[selection as usize];
            // Mirror the runtime selection into the engine context so any
            // code reading ctx.chaos_mode agrees with self.chaos_mode.
            ctx.chaos_mode = self.chaos_mode;
            self.start_game(ctx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mode_has_a_nonempty_hint() {
        for mode in ChaosMode::ALL {
            assert!(!mode_hint(mode).is_empty());
        }
    }
}
