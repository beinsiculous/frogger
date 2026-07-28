//! Menu screens: navigation and selection. Match lifecycle lives in
//! `gameplay`. Every player-facing string is a locale key resolved through
//! `ctx.strings` at draw time.

use engine_core::prelude::*;
use crate::types::*;

/// Locale key for a chaos mode's menu label.
pub(crate) fn chaos_label_key(mode: ChaosMode) -> &'static str {
    match mode {
        ChaosMode::Normal => "chaos.normal",
        ChaosMode::Insane => "chaos.insane",
        ChaosMode::Ridiculous => "chaos.ridiculous",
        ChaosMode::Insiculous => "chaos.insiculous",
    }
}

/// Locale key for the one-line description of what each chaos mode means.
pub(crate) fn mode_hint_key(mode: ChaosMode) -> &'static str {
    match mode {
        ChaosMode::Normal => "chaos.normal.desc",
        ChaosMode::Insane => "chaos.insane.desc",
        ChaosMode::Ridiculous => "chaos.ridiculous.desc",
        ChaosMode::Insiculous => "chaos.insiculous.desc",
    }
}

impl FroggerGame {
    pub(crate) fn update_title_input(&mut self, ctx: &mut GameContext, selection: u8) {
        let input = MenuInput::read(ctx.input);
        let selection = input.navigate(selection, 5);
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
                3 => {
                    // Language: cycle locale, then re-register achievements
                    // so their names/descriptions pick up the new language
                    // (id-keyed insert — unlock state is untouched).
                    ctx.strings.cycle_locale();
                    crate::achievements::register_all(ctx.achievements, ctx.strings);
                }
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
    fn every_mode_has_label_and_hint_keys_in_en() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/locales");
        let strings = Strings::load_dir(&dir);
        let en = strings.locale_keys("en").expect("en.ron loads");
        for mode in ChaosMode::ALL {
            assert!(en.contains(&chaos_label_key(mode)), "{} missing", chaos_label_key(mode));
            assert!(en.contains(&mode_hint_key(mode)), "{} missing", mode_hint_key(mode));
        }
    }
}
