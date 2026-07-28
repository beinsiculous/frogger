mod achievements;
mod board;
mod constants;
mod drawing;
mod effects;
mod gameplay;
#[cfg(test)]
mod gameplay_tests;
mod menu;
mod spawning;
mod types;

use engine_core::prelude::*;
use constants::*;
use types::*;

impl Game for FroggerGame {
    fn init(&mut self, ctx: &mut GameContext) {
        let font_path = engine_core::game_root!().join("assets/fonts/font.ttf");
        if let Ok(font) = ctx.ui.load_font_file(&font_path.to_string_lossy()) {
            ctx.ui.set_default_font(font);
        }

        achievements::register_all(ctx.achievements);

        let tex = ctx.assets.create_solid_color(1, 1, [255, 255, 255, 255]).unwrap();
        self.tex_id = tex.id;

        let theme = ChaosTheme::for_mode(self.chaos_mode);
        self.background = Some(spawn_background(
            ctx.world, tex.id, theme.bg_color, Vec2::new(WIN_W, WIN_H)));

        // The board, lanes, and frogs all spawn fresh in `start_game()` —
        // the board texture is themed per chaos mode and the frog count
        // depends on the mode the player picks.
        self.grid = Some(default_playfield_grid(&theme));
    }

    fn update(&mut self, ctx: &mut GameContext) {
        self.frame_count = self.frame_count.wrapping_add(1);

        match self.state.clone() {
            GameState::TitleScreen { selection } => self.update_title_input(ctx, selection),
            GameState::ModeSelect { selection } => self.update_mode_select_input(ctx, selection),
            GameState::Achievements => self.update_achievements_input(ctx),
            _ => self.update_gameplay(ctx),
        }

        self.draw_ui(ctx);
    }
}

fn main() {
    // Anchor assets and saves to the game's directory so launching from any
    // working directory behaves the same.
    let root = engine_core::game_root!();
    let config = GameConfig::new("Insiculous Frogger")
        .with_size(WIN_W as u32, WIN_H as u32)
        .with_clear_color(0.0, 0.0, 0.0, 1.0)
        .with_fps(60)
        .with_asset_base_path(root.join("assets").to_string_lossy())
        .with_achievement_save_path(root.join("saves/frogger_achievements.json").to_string_lossy())
        .with_input_settings_path(root.join("saves/input_settings.json").to_string_lossy());

    // With `--features editor` the game runs inside the scene editor
    // (hierarchy, inspector, gizmos, play/pause/stop); without it the game
    // runs bare. Same game code either way.
    #[cfg(feature = "editor")]
    editor_integration::run_game_with_editor(FroggerGame::default(), config).unwrap();
    #[cfg(not(feature = "editor"))]
    run_game(FroggerGame::default(), config).unwrap();
}
