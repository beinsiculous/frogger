//! Every tunable in one place. Sizes are world pixels; sprite scales divide
//! by `RENDER_UNIT` at the spawn site. There is no physics in this game —
//! all collision is pure lane math (see `gameplay/rules.rs`).

use glam::Vec4;

// --- Board geometry (single source of truth: everything derives from these) ---
/// Edge length of one board tile, world pixels.
pub(crate) const TILE: f32 = 48.0;
pub(crate) const COLS: u32 = 15;
pub(crate) const ROWS: u32 = 13;
/// HUD band above and below the board (score/lives up top, timers below).
pub(crate) const BAND: f32 = 72.0;
pub(crate) const WIN_W: f32 = COLS as f32 * TILE; // 720
pub(crate) const WIN_H: f32 = ROWS as f32 * TILE + 2.0 * BAND; // 768

// --- Row map (tilemap row indices, row 0 = top) ---
pub(crate) const HOME_ROW: u32 = 0;
pub(crate) const FIRST_WATER_ROW: u32 = 1;
pub(crate) const LAST_WATER_ROW: u32 = 5;
pub(crate) const MEDIAN_ROW: u32 = 6;
pub(crate) const FIRST_ROAD_ROW: u32 = 7;
pub(crate) const LAST_ROAD_ROW: u32 = 11;
pub(crate) const START_ROW: u32 = 12;
/// Board columns holding a home slot (all other home-row tiles are hedge).
pub(crate) const HOME_COLS: [u32; 5] = [1, 4, 7, 10, 13];

// --- Lanes ---
/// Extra torus circumference beyond the window so obstacles have an
/// offstage margin before re-entering on the far side.
pub(crate) const LANE_MARGIN: f32 = 2.0 * TILE;
/// Torus period every lane position lives on (`[-P/2, P/2)`).
pub(crate) const LANE_PERIOD: f32 = WIN_W + LANE_MARGIN; // 816
/// Obstacle sprite height, slightly under a tile so lanes read as lanes.
pub(crate) const OBSTACLE_H: f32 = 40.0;
pub(crate) const INSANE_SPEED_MULT: f32 = 1.5;
/// Per-round ramp on obstacle speed, capped so late rounds stay playable.
pub(crate) const ROUND_SPEED_RAMP: f32 = 0.15;
pub(crate) const ROUND_SPEED_MULT_MAX: f32 = 2.0;

// --- Frog ---
pub(crate) const FROG_SIZE: f32 = 36.0;
/// Half-width used for all frog-vs-obstacle overlap tests.
pub(crate) const FROG_HALF: f32 = FROG_SIZE / 2.0;
pub(crate) const STARTING_LIVES: u32 = 3;
/// Seconds between death and respawn at the start row.
pub(crate) const RESPAWN_DELAY: f32 = 1.0;
/// Attempt timer, seconds (Insane family shortens it).
pub(crate) const TIMER_NORMAL: f32 = 40.0;
pub(crate) const TIMER_INSANE: f32 = 25.0;
/// Player 2's frog color in co-op (player 1 uses the classic green below).
pub(crate) const FROG1_COLOR: Vec4 = Vec4::new(0.35, 1.0, 0.4, 1.0);
pub(crate) const FROG2_COLOR: Vec4 = Vec4::new(1.0, 0.75, 0.25, 1.0);
pub(crate) const FROG_EMISSIVE: f32 = 1.6;
/// Start columns: solo spawns center; co-op splits left/right of center.
pub(crate) const SOLO_START_COL: u32 = 7;
pub(crate) const COOP_START_COLS: [u32; 2] = [5, 9];

// --- Homes / croc (Ridiculous) ---
/// Croc duty cycle: absent (slot fillable) then present (slot kills).
pub(crate) const CROC_ABSENT_SECS: f32 = 4.0;
pub(crate) const CROC_PRESENT_SECS: f32 = 3.0;
pub(crate) const CROC_COLOR: Vec4 = Vec4::new(0.75, 0.2, 0.15, 1.0);

// --- Turtles (dive only in the Ridiculous family) ---
/// Full dive cycle; turtles are submerged for the last `DIVE_DOWN_SECS`.
pub(crate) const DIVE_PERIOD: f32 = 5.0;
pub(crate) const DIVE_DOWN_SECS: f32 = 1.5;

// --- Scoring (pooled between co-op players) ---
pub(crate) const SCORE_PER_ROW: u32 = 10;
pub(crate) const SCORE_HOME: u32 = 200;
/// Time bonus: points per whole second left on the attempt timer.
pub(crate) const SCORE_PER_SEC_LEFT: u32 = 5;
pub(crate) const SCORE_ROUND_CLEAR: u32 = 1000;

// --- Achievements thresholds ---
pub(crate) const SPEEDY_SECS_LEFT: f32 = 15.0;
pub(crate) const HOMES_MILESTONE: u32 = 25;
pub(crate) const SCORE_TIER: u32 = 5_000;

// --- Grid impulses (strength, radius) ---
pub(crate) const GRID_IMPULSE_DEATH: (f32, f32) = (500.0, 140.0);
pub(crate) const GRID_IMPULSE_HOME: (f32, f32) = (300.0, 110.0);
