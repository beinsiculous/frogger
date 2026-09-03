//! Menu screens: navigation and selection. Match lifecycle lives in
//! `gameplay`. Every player-facing string is a locale key resolved through
//! `ctx.strings` at draw time.

use engine_core::prelude::*;
use crate::types::*;

/// Panel layouts shared by the input half (mouse hit-testing here) and the
/// drawing half (`drawing.rs`) — the geometry must match or clicks land
/// beside the drawn rows. Titles only affect the label, never the layout.
pub(crate) fn title_panel(title: &str, window_size: Vec2) -> MenuPanel {
    MenuPanel::new(title, window_size / 2.0, 400.0, TITLE_ITEMS.len())
}
pub(crate) fn chaos_panel(title: &str, window_size: Vec2) -> MenuPanel {
    MenuPanel::new(title, window_size / 2.0, 400.0, ChaosMode::ALL.len())
}
pub(crate) fn achievements_panel(title: &str, window_size: Vec2) -> MenuPanel {
    // Clamp so a shrunken window can't drive the panel width negative.
    let width = (window_size.x - 120.0).max(320.0);
    MenuPanel::new(title, window_size / 2.0, width, 13)
}

/// A row on the title screen. Both halves derive from [`TITLE_ITEMS`]: the
/// input half (navigation bounds, mouse hit-testing, confirm dispatch) and
/// the drawing half (row labels) — so keyboard, mouse, and the drawn panel
/// can never disagree on row count or order.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TitleItem {
    Single,
    Coop,
    Achievements,
    Language,
    Exit,
}

/// The title rows for this build. The wasm build drops the Achievements
/// row (the game page on the site shows the achievements board instead);
/// the `GameState::Achievements` screen itself stays compiled everywhere.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const TITLE_ITEMS: &[TitleItem] = &[
    TitleItem::Single,
    TitleItem::Coop,
    TitleItem::Achievements,
    TitleItem::Language,
    TitleItem::Exit,
];
#[cfg(target_arch = "wasm32")]
pub(crate) const TITLE_ITEMS: &[TitleItem] = &[
    TitleItem::Single,
    TitleItem::Coop,
    TitleItem::Language,
    TitleItem::Exit,
];

/// Selection index of `item` on this build's title menu (0 for a row this
/// build doesn't show).
pub(crate) fn title_index(item: TitleItem) -> u8 {
    TITLE_ITEMS.iter().position(|i| *i == item).unwrap_or(0) as u8
}

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
        let mouse = title_panel("", ctx.window_size).mouse_select(ctx.input);
        let selection = mouse.hovered.unwrap_or(selection);
        let mut selection = input.navigate(selection, TITLE_ITEMS.len() as u8);
        if let Some(row) = mouse.clicked {
            selection = row;
        }
        self.state = GameState::TitleScreen { selection };

        if input.confirm || mouse.clicked.is_some() {
            match TITLE_ITEMS[selection as usize % TITLE_ITEMS.len()] {
                TitleItem::Single => {
                    self.mode = GameMode::SinglePlayer;
                    self.state = GameState::ModeSelect { selection: 0 };
                }
                TitleItem::Coop => {
                    self.mode = GameMode::TwoPlayerCoop;
                    self.state = GameState::ModeSelect { selection: 0 };
                }
                TitleItem::Achievements => self.state = GameState::Achievements,
                TitleItem::Language => {
                    // Language: cycle locale, then re-register achievements
                    // so their names/descriptions pick up the new language
                    // (id-keyed insert — unlock state is untouched).
                    ctx.strings.cycle_locale();
                    crate::achievements::register_all(ctx.achievements, ctx.strings);
                }
                TitleItem::Exit => ctx.request_exit(),
            }
        }
    }

    pub(crate) fn update_achievements_input(&mut self, ctx: &mut GameContext) {
        let input = MenuInput::read(ctx.input);
        // The page is one big non-selectable list: any click on it dismisses,
        // same as confirm/back.
        // Whole-window dismiss: clicks on headers/margins count too, not
        // just the row bands (the page is one big info sheet).
        let click_dismiss = achievements_panel("", ctx.window_size).clicked_inside(ctx.input);
        if input.back || input.confirm || click_dismiss {
            self.state = GameState::TitleScreen {
                selection: title_index(TitleItem::Achievements),
            };
        }
    }

    pub(crate) fn update_mode_select_input(&mut self, ctx: &mut GameContext, selection: u8) {
        let input = MenuInput::read(ctx.input);
        let mouse = chaos_panel("", ctx.window_size).mouse_select(ctx.input);
        let count = ChaosMode::ALL.len() as u8;
        let selection = mouse.hovered.unwrap_or(selection);
        let mut selection = input.navigate(selection, count);
        if let Some(row) = mouse.clicked {
            selection = row;
        }
        self.state = GameState::ModeSelect { selection };

        if input.back {
            self.state = GameState::TitleScreen { selection: 0 };
        } else if input.confirm || mouse.clicked.is_some() {
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
