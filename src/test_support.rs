//! Shared test fixtures.
//!
//! The synced sheets are read back through the engine's own GPU-free load
//! path, so a test can check the game's tables against the committed art
//! without a window, a GPU or a `GameContext`.

use engine_core::assets::sprite_sheet::{prepare_sheet, PreparedSheet};
use engine_core::prelude::*;

use crate::constants::*;
use crate::gameplay::rules::{attempt_timer, LANES};
use crate::types::*;

/// The lane the table declares for `row`.
pub(crate) fn lane(row: u32) -> LaneDef {
    *LANES
        .iter()
        .find(|def| def.row == row)
        .unwrap_or_else(|| panic!("no lane covers row {row}"))
}

/// A synced sheet read through the engine's own GPU-free load path — the one
/// check that ties the game's tables to the committed art.
pub(crate) fn sidecar(spec: &SheetSpec) -> PreparedSheet {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    prepare_sheet(&base, spec.path)
        .unwrap_or_else(|error| panic!("{} does not load: {error}", spec.path))
}

/// The frames and rate of a clip, as the sidecar declares them.
pub(crate) fn declared(spec: &SheetSpec, clip: &str) -> (usize, f32) {
    let prepared = sidecar(spec);
    let (_, declared) = prepared
        .sheet
        .clips
        .iter()
        .find(|(name, _)| name == clip)
        .unwrap_or_else(|| panic!("{} has no clip '{clip}'", spec.path));
    (declared.frame_indices.len(), declared.fps)
}

/// How long a clip's art runs for. The exporter writes 150 ms a frame as
/// `fps: 6.667`, so a three-frame clip is 0.44998 s — never compared with `==`.
pub(crate) fn clip_length(spec: &SheetSpec, clip: &str) -> f32 {
    let (frames, fps) = declared(spec, clip);
    frames as f32 / fps
}

/// How far off a typed window and a sidecar-derived length may be.
pub(crate) const TOLERANCE: f32 = 0.005;

/// A sheet shaped exactly like the real one but with no texture: its grid and
/// clips come from the synced sidecar, so a test drives the machine's real
/// table without a GPU.
pub(crate) fn headless_sheet(spec: &SheetSpec) -> SpriteSheet {
    let prepared = sidecar(spec);
    SpriteSheet {
        texture: TextureHandle { id: 0 },
        grid: prepared.sheet.grid,
        clips: prepared.sheet.clips,
        path: spec.path.to_string(),
    }
}

/// The sheets a test needs to drive a chicken, off their sidecars.
pub(crate) fn headless_sheets() -> Sheets {
    Sheets {
        chicken_round: headless_sheet(&CHICKEN_ROUND),
        chicken_tall: headless_sheet(&CHICKEN_TALL),
        ..Sheets::default()
    }
}

pub(crate) fn clip_of(world: &World, entity: EntityId) -> Option<String> {
    world.get::<SpriteAnimation>(entity).and_then(|animation| animation.current_clip.clone())
}

pub(crate) fn state_of(world: &World, entity: EntityId) -> String {
    world
        .get::<ClipStateMachine>(entity)
        .expect("the chicken carries a machine")
        .state()
        .to_string()
}

/// A game mid-match with its chickens already placed — the state a round
/// starts from, with no context and no world.
pub(crate) fn playing_game(mode: GameMode, chaos: ChaosMode) -> FroggerGame {
    let mut game = FroggerGame {
        mode,
        chaos_mode: chaos,
        state: GameState::Playing,
        ..FroggerGame::default()
    };
    let timer = attempt_timer(chaos);
    game.chickens = (0..mode.player_count())
        .map(|index| {
            let col = crate::gameplay::start_col(mode, index);
            let mut chicken = ChickenState::new(col, timer);
            chicken.x = crate::board::tile_center(col, 0).x;
            chicken
        })
        .collect();
    game
}
