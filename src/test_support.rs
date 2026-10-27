//! Shared test fixtures.
//!
//! The synced sheets are read back through the engine's own GPU-free load
//! path, so a test can check the game's tables against the committed art
//! without a window, a GPU or a `GameContext`. The fixtures also drive the
//! whole game through the engine's harness: the real frame, with no window.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use engine_core::assets::sprite_sheet::{prepare_sheet, PreparedSheet};
use engine_core::prelude::*;
use engine_core::test_support::{DrawCommand, GameHarness, InputEvent};

use crate::constants::*;
use crate::gameplay::rules::{attempt_timer, is_water_row, wrap_lane_x, LANES};
use crate::types::*;

pub(crate) const FRAME: f32 = 1.0 / 60.0;

/// The lane the table declares for `row`.
pub(crate) fn lane(row: u32) -> LaneDef {
    *LANES
        .iter()
        .find(|def| def.row == row)
        .unwrap_or_else(|| panic!("no lane covers row {row}"))
}

/// The synced art's directory, anchored to the crate so the working
/// directory never matters.
pub(crate) fn asset_base() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("assets")
}

/// A synced sheet read through the engine's own GPU-free load path — the one
/// check that ties the game's tables to the committed art.
pub(crate) fn sidecar(spec: &SheetSpec) -> PreparedSheet {
    prepare_sheet(&asset_base(), spec.path)
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

/// The title screen's world: the game's own config over the synced art, with
/// no save paths so achievements and scores stay in memory, and one frame run
/// so `init` has loaded the sheets and spawned what the title shows.
pub(crate) fn title_harness() -> GameHarness<FroggerGame> {
    let base = asset_base();
    let config = crate::game_config(base.to_str().expect("the asset path is UTF-8"));
    let mut harness = GameHarness::new(FroggerGame::default(), config);
    harness.step(1.0 / 60.0, &[]);
    harness
}

/// Start a match as the menus do: the title item sets the mode, the mode
/// menu's confirm sets the chaos mode on the game and the context, then
/// `start_game`.
pub(crate) fn start_match(harness: &mut GameHarness<FroggerGame>, mode: GameMode, chaos: ChaosMode) {
    harness.context(|game, ctx| {
        game.mode = mode;
        game.chaos_mode = chaos;
        ctx.chaos_mode = chaos;
        game.start_game(ctx)
    });
}

/// A match just started from the title screen, through the real frame.
pub(crate) fn harness(mode: GameMode, chaos: ChaosMode) -> GameHarness<FroggerGame> {
    let mut harness = title_harness();
    start_match(&mut harness, mode, chaos);
    harness
}

/// Player one fills nest `slot` through the game's own entry point; returns
/// what the fill paid.
pub(crate) fn fill_nest(harness: &mut GameHarness<FroggerGame>, slot: usize) -> u32 {
    let before = harness.game().score;
    harness.context(|game, ctx| game.fill_home(ctx, 0, slot));
    harness.game().score - before
}

/// Fill all five nests, the fifth arming the beat.
pub(crate) fn arm_beat(harness: &mut GameHarness<FroggerGame>) {
    for slot in 0..5 {
        fill_nest(harness, slot);
    }
}

pub(crate) fn press(key: KeyCode) -> InputEvent {
    InputEvent::KeyPressed(key)
}

pub(crate) fn release(key: KeyCode) -> InputEvent {
    InputEvent::KeyReleased(key)
}

/// Step one frame with `events` arriving before it.
pub(crate) fn step_with(harness: &mut GameHarness<FroggerGame>, events: &[InputEvent]) {
    harness.step(FRAME, events);
}

/// A key pressed on one frame and released on the next.
pub(crate) fn key(harness: &mut GameHarness<FroggerGame>, code: KeyCode) {
    harness.step(FRAME, &[press(code)]);
    harness.step(FRAME, &[release(code)]);
}

/// Frames with wall time between them, so the song's virtual clock renders blocks and the
/// conductor drains what the frames sent.
pub(crate) fn settle(harness: &mut GameHarness<FroggerGame>) {
    for _ in 0..6 {
        std::thread::sleep(Duration::from_millis(3));
        harness.step(FRAME, &[]);
    }
}

/// Step with wall time between frames until `done` holds, failing past `limit`.
pub(crate) fn poll(
    harness: &mut GameHarness<FroggerGame>,
    limit: Duration,
    what: &str,
    done: impl Fn(&GameHarness<FroggerGame>) -> bool,
) {
    let started = Instant::now();
    while !done(harness) {
        assert!(started.elapsed() < limit, "{what} within {limit:?}");
        std::thread::sleep(Duration::from_millis(3));
        harness.step(FRAME, &[]);
    }
}

/// Where the pause menu drew `label` in the last frame.
pub(crate) fn drawn_at(harness: &GameHarness<FroggerGame>, label: &str) -> Vec2 {
    harness.ui_commands().iter().find_map(|command| match command {
        DrawCommand::Text { data, .. } if data.text.contains(label) => {
            Some(data.position + Vec2::new(4.0, data.height / 2.0))
        }
        _ => None,
    }).unwrap_or_else(|| panic!("'{label}' was drawn"))
}

pub(crate) fn source_of(harness: &GameHarness<FroggerGame>, entity: EntityId) -> Option<SourceId> {
    harness.sound_sources().source_of(entity)
}

/// Chicken `index`'s entity, read now: a match start replaces it.
pub(crate) fn chicken(harness: &GameHarness<FroggerGame>, index: usize) -> EntityId {
    harness.game().chickens[index].entity.expect("the chicken is spawned")
}

/// The match stops running while the song plays on: no chicken is simulated and nothing
/// dies, so a test that waits on wall time hears only what it sent.
pub(crate) fn hold_the_match(harness: &mut GameHarness<FroggerGame>) {
    harness.context(|game, _| game.state = GameState::GameOver);
}

/// Stand chicken `index` on `row` at world `x`, alive.
pub(crate) fn stand(harness: &mut GameHarness<FroggerGame>, index: usize, row: u32, x: f32) {
    harness.context(|game, _| {
        let chicken = &mut game.chickens[index];
        chicken.row = row;
        chicken.x = x;
        chicken.furthest_row = row;
    });
}

/// Put every segment of the lane on `row` at `x` on its torus: under a chicken there, or
/// half a torus away from it.
pub(crate) fn park_lane(harness: &mut GameHarness<FroggerGame>, row: u32, x: f32) {
    harness.context(|game, _| {
        let lane = game.lanes.iter_mut().find(|lane| lane.def.row == row).expect("a lane on the row");
        for segment in &mut lane.xs {
            *segment = wrap_lane_x(x);
        }
    });
}

/// Step until a frame's trace holds a trigger, and return it.
pub(crate) fn next_trace(harness: &mut GameHarness<FroggerGame>) -> Vec<&'static str> {
    for _ in 0..60 {
        harness.step(FRAME, &[]);
        let trace = harness.game().music.triggered.clone();
        if !trace.is_empty() {
            return trace;
        }
    }
    panic!("no trigger in a second of frames");
}

/// Where `entity` was drawn in the last frame.
pub(crate) fn drawn(harness: &GameHarness<FroggerGame>, entity: EntityId) -> Vec2 {
    harness.world().get::<Transform2D>(entity).map(|transform| transform.position).expect("the entity exists")
}

/// Chicken `index` back on its feet with a full timer and three lives, wherever it stands.
pub(crate) fn revive(harness: &mut GameHarness<FroggerGame>, index: usize) {
    harness.context(|game, _| {
        let chicken = &mut game.chickens[index];
        chicken.respawn_timer = 0.0;
        chicken.timer = attempt_timer(ChaosMode::Normal);
        chicken.lives = STARTING_LIVES;
    });
}

/// The first soup lane of rafts: a platform in every chaos mode.
pub(crate) fn raft_row() -> u32 {
    LANES.iter().find(|def| is_water_row(def.row) && def.kind != LaneKind::Crackers).expect("a raft lane").row
}
