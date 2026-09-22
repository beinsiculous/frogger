//! Chicken Coop — game crate.
//!
//! The library owns the whole game (`FroggerGame` + its `Game` impl) so both
//! entry points stay thin: `main.rs` (native window, filesystem saves,
//! optional editor) and `web_entry.rs` (wasm-bindgen start: fetch assets,
//! then the same `run_game`). This split also keeps `editor_integration`
//! behind the `editor` feature in both entry points — `main.rs` for the native
//! window, `web_entry.rs` for the browser's editor bundle.

mod achievements;
#[cfg(test)]
mod art_tests;
mod board;
mod constants;
mod drawing;
mod effects;
mod gameplay;
mod hud;
#[cfg(test)]
mod gameplay_tests;
mod menu;
mod spawning;
#[cfg(test)]
mod test_support;
mod types;

#[cfg(target_arch = "wasm32")]
mod web_entry;

use engine_core::prelude::*;
use constants::*;
use types::*;

pub use types::FroggerGame;

/// The shared `GameConfig` for every target. Entry points add their own
/// platform extras on top — both set save paths (native: JSON files
/// anchored to the game dir; web: `beinsiculous.games.frogger.*`
/// localStorage keys per the engine's `docs/WEB_SAVES.md`).
///
/// `asset_base` must be an ANCHORED base: native callers pass an absolute
/// path (`main.rs` derives it from `game_root!()` so the cwd never
/// matters); the web entry passes the deploy URL base. Passing a bare
/// relative path like `"assets"` would silently resolve against the
/// current working directory.
pub fn game_config(asset_base: &str) -> GameConfig {
    GameConfig::new("Chicken Coop")
        .with_size(WIN_W as u32, WIN_H as u32)
        .with_clear_color(0.0, 0.0, 0.0, 1.0)
        .with_fps(60)
        // The art is 1x with nearest filtering, so snapping every sprite's
        // origin to a whole device pixel is what keeps it crisp.
        .with_pixel_snap(true)
        .with_asset_base_path(asset_base)
}

/// Load one synced sheet by the path its spec names. Fail-loud: the sheets
/// ship with the game, so a missing or malformed one is a broken build rather
/// than a game that silently draws nothing.
fn load_sheet(assets: &mut AssetManager, spec: &SheetSpec) -> SpriteSheet {
    assets
        .load_sprite_sheet(spec.path)
        .unwrap_or_else(|error| panic!("{} does not load: {error}", spec.path))
}

impl Game for FroggerGame {
    fn register_achievements(&self, achievements: &mut AchievementManager, strings: &Strings) {
        // Names and descriptions come from the locale tables; the title menu's Language item
        // re-registers on a locale switch.
        achievements::register_all(achievements, strings);
    }

    fn init(&mut self, ctx: &mut GameContext) {
        // Resolve against the configured asset base so the same relative
        // path works natively (game dir) and on the web (VFS keys).
        let font_path = std::path::Path::new(ctx.assets.base_path()).join("fonts/font.ttf");
        if let Ok(font) = ctx.ui.load_font_file(&font_path.to_string_lossy()) {
            ctx.ui.set_default_font(font);
        }

        let tex = ctx.assets.create_solid_color(1, 1, [255, 255, 255, 255]).unwrap();
        self.sheets.white = tex.id;

        // Every sheet's path, cell and measured bounds is in `constants.rs`'s
        // sheets block; each PNG and its `.sheet.ron` sidecar is a synced copy
        // under `assets/sprites/`.
        self.sheets.chicken_round = load_sheet(ctx.assets, &CHICKEN_ROUND);
        self.sheets.chicken_tall = load_sheet(ctx.assets, &CHICKEN_TALL);
        self.sheets.sushi_cart = load_sheet(ctx.assets, &SUSHI_CART);
        self.sheets.donut_cart = load_sheet(ctx.assets, &DONUT_CART);
        self.sheets.hot_dog_cart = load_sheet(ctx.assets, &HOT_DOG_CART);
        self.sheets.celery_raft = load_sheet(ctx.assets, &CELERY_RAFT);
        self.sheets.baguette_raft = load_sheet(ctx.assets, &BAGUETTE_RAFT);
        self.sheets.cracker = load_sheet(ctx.assets, &CRACKER);
        self.sheets.bun = load_sheet(ctx.assets, &BUN);
        self.sheets.nest = load_sheet(ctx.assets, &NEST);
        self.sheets.arrival_burst = load_sheet(ctx.assets, &ARRIVAL_BURST);
        self.sheets.feather_poof = load_sheet(ctx.assets, &FEATHER_POOF);
        self.sheets.life_icon = load_sheet(ctx.assets, &LIFE_ICON);
        self.sheets.conveyor_belt = load_sheet(ctx.assets, &CONVEYOR_BELT);
        self.sheets.tomato_soup = load_sheet(ctx.assets, &TOMATO_SOUP);
        self.sheets.coop_floor = load_sheet(ctx.assets, &COOP_FLOOR);
        self.sheets.coop_wall = load_sheet(ctx.assets, &COOP_WALL);

        let theme = ChaosTheme::for_mode(self.chaos_mode);
        self.background = Some(spawn_background(
            ctx.world, tex.id, theme.bg_color, Vec2::new(WIN_W, WIN_H)));

        // The board, the lanes, the nests and the chickens all spawn fresh in
        // `start_game()` — the chicken count depends on the mode the player
        // picks and the whole board is match-scoped.
    }

    fn update(&mut self, ctx: &mut GameContext) {
        match self.state.clone() {
            GameState::TitleScreen { selection } => self.update_title_input(ctx, selection),
            GameState::ModeSelect { selection } => self.update_mode_select_input(ctx, selection),
            GameState::Achievements => self.update_achievements_input(ctx),
            _ => self.update_gameplay(ctx),
        }

        self.draw_ui(ctx);
    }
}
