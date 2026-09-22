use insiculous_frogger::{game_config, FroggerGame};

fn main() {
    // Anchor assets and saves to the game's directory so launching from any
    // working directory behaves the same.
    let root = engine_core::game_root!();
    let config = game_config(&root.join("assets").to_string_lossy())
        .with_achievement_save_path(root.join("saves/frogger_achievements.json").to_string_lossy())
        .with_input_settings_path(root.join("saves/input_settings.json").to_string_lossy())
        .with_score_save_path(root.join("saves/frogger_scores.json").to_string_lossy());

    // With `--features editor` the game runs inside the scene editor
    // (hierarchy, inspector, gizmos, play/pause/stop); without it the game
    // runs bare. Same game code either way.
    #[cfg(feature = "editor")]
    editor_integration::run_game_with_editor(FroggerGame::default(), config).unwrap();
    #[cfg(not(feature = "editor"))]
    {
        // One design size: the board and the HUD are laid out for 720x768 and
        // the world never scales with a native window, so a smaller one would
        // crop them both and a larger one would float them. Only the bare game
        // is locked — the editor needs room for its panels, and the web page
        // scales the whole canvas instead.
        let mut config = config;
        config.resizable = false;
        engine_core::prelude::run_game(FroggerGame::default(), config).unwrap();
    }
}
