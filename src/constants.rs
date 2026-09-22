//! Every tunable in one place. Sizes are world pixels; sprite scales divide
//! by `RENDER_UNIT` at the spawn site. There is no physics in this game —
//! all collision is pure lane math (see `gameplay/rules.rs`).

use engine_core::prelude::*;

// --- Board geometry (single source of truth: everything derives from these) ---
/// Edge length of one board tile, world pixels.
pub(crate) const TILE: f32 = 48.0;
pub(crate) const COLS: u32 = 15;
pub(crate) const ROWS: u32 = 13;
/// HUD band above and below the board (score/lanes up top, timers below).
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
/// Board columns holding a nest (all other home-row tiles are coop wall).
pub(crate) const HOME_COLS: [u32; 5] = [1, 4, 7, 10, 13];

// --- Lanes ---
/// Extra torus circumference beyond the window so obstacles have an
/// offstage margin before re-entering on the far side.
pub(crate) const LANE_MARGIN: f32 = 2.0 * TILE;
/// Torus period every lane position lives on (`[-P/2, P/2)`).
pub(crate) const LANE_PERIOD: f32 = WIN_W + LANE_MARGIN; // 816
pub(crate) const INSANE_SPEED_MULT: f32 = 1.5;
/// Per-round ramp on obstacle speed, capped so late rounds stay playable.
pub(crate) const ROUND_SPEED_RAMP: f32 = 0.15;
pub(crate) const ROUND_SPEED_MULT_MAX: f32 = 2.0;

// --- The sheets ---------------------------------------------------------------------
// One `SheetSpec` per synced sheet: where the copy lives, how big one art cell
// is, and the opaque box of the art inside that cell. Every box below was
// measured from the synced PNG at deion_assets 5789034 on 2026-09-21 as the
// union over the frames the sheet's clips play, with the bottom-right corner
// one past the last opaque pixel.

const CHICKEN_CELL: Vec2 = Vec2::new(48.0, 48.0);
/// The round chicken (P1): 40 x 39 px of art in its cell — 29 wide head-on,
/// 38 beak to tail.
const CHICKEN_ROUND_BOUNDS: (Vec2, Vec2) = (Vec2::new(4.0, 3.0), Vec2::new(44.0, 42.0));
/// The tall chicken (P2): 40 x 45 px — 23 wide head-on, 38 beak to tail.
const CHICKEN_TALL_BOUNDS: (Vec2, Vec2) = (Vec2::new(4.0, 1.0), Vec2::new(44.0, 46.0));

pub(crate) const CHICKEN_ROUND: SheetSpec = SheetSpec {
    path: "sprites/ai_chicken_round.png",
    cell: CHICKEN_CELL,
    bounds: CHICKEN_ROUND_BOUNDS,
};
pub(crate) const CHICKEN_TALL: SheetSpec = SheetSpec {
    path: "sprites/ai_chicken_tall.png",
    cell: CHICKEN_CELL,
    bounds: CHICKEN_TALL_BOUNDS,
};

/// A cart is one 64 px cell of art on a 48 px row, so a lane reads as a lane.
const CART_CELL: Vec2 = Vec2::new(64.0, 48.0);
const SUSHI_CART_BOUNDS: (Vec2, Vec2) = (Vec2::new(2.0, 7.0), Vec2::new(57.0, 42.0));
const DONUT_CART_BOUNDS: (Vec2, Vec2) = (Vec2::new(2.0, 5.0), Vec2::new(57.0, 42.0));

pub(crate) const SUSHI_CART: SheetSpec = SheetSpec {
    path: "sprites/ai_sushi_cart_64x48.png",
    cell: CART_CELL,
    bounds: SUSHI_CART_BOUNDS,
};
pub(crate) const DONUT_CART: SheetSpec = SheetSpec {
    path: "sprites/ai_donut_cart_64x48.png",
    cell: CART_CELL,
    bounds: DONUT_CART_BOUNDS,
};
pub(crate) const HOT_DOG_CART: SheetSpec = SheetSpec {
    path: "sprites/ai_hot_dog_cart_96x48.png",
    cell: Vec2::new(96.0, 48.0),
    bounds: (Vec2::new(2.0, 7.0), Vec2::new(89.0, 42.0)),
};

const RAFT_CELL: Vec2 = Vec2::new(64.0, 32.0);
const CELERY_RAFT_BOUNDS: (Vec2, Vec2) = (Vec2::new(2.0, 3.0), Vec2::new(62.0, 30.0));

pub(crate) const CELERY_RAFT: SheetSpec = SheetSpec {
    path: "sprites/ai_celery_raft_64x32.png",
    cell: RAFT_CELL,
    bounds: CELERY_RAFT_BOUNDS,
};
pub(crate) const BAGUETTE_RAFT: SheetSpec = SheetSpec {
    path: "sprites/ai_baguette_raft_96x32.png",
    cell: Vec2::new(96.0, 32.0),
    bounds: (Vec2::new(3.0, 5.0), Vec2::new(92.0, 30.0)),
};

pub(crate) const CRACKER: SheetSpec = SheetSpec {
    path: "sprites/ai_sinking_cracker_48x48.png",
    cell: Vec2::new(48.0, 48.0),
    bounds: (Vec2::new(3.0, 7.0), Vec2::new(45.0, 40.0)),
};
pub(crate) const BUN: SheetSpec = SheetSpec {
    path: "sprites/ai_snapping_bun_64x48.png",
    cell: Vec2::new(64.0, 48.0),
    bounds: (Vec2::new(5.0, 1.0), Vec2::new(60.0, 45.0)),
};
/// The nest: 75 x 72 px of art, 3 px clear of its canvas's bottom edge.
pub(crate) const NEST: SheetSpec = SheetSpec {
    path: "sprites/ai_breadbox_nest_96x80.png",
    cell: Vec2::new(96.0, 80.0),
    bounds: (Vec2::new(10.0, 5.0), Vec2::new(85.0, 77.0)),
};
/// The one-shot a fifth arrival plays; the whole nest's width of celebration.
pub(crate) const ARRIVAL_BURST: SheetSpec = SheetSpec {
    path: "sprites/ai_coop_arrival_burst_64x64.png",
    cell: Vec2::new(64.0, 64.0),
    bounds: (Vec2::new(6.0, 5.0), Vec2::new(59.0, 46.0)),
};
/// The one-shot every death leaves behind.
pub(crate) const FEATHER_POOF: SheetSpec = SheetSpec {
    path: "sprites/ai_coop_feather_poof_64x64.png",
    cell: Vec2::new(64.0, 64.0),
    bounds: (Vec2::new(6.0, 13.0), Vec2::new(57.0, 60.0)),
};
/// One chicken's head, drawn in the band once per life.
pub(crate) const LIFE_ICON: SheetSpec = SheetSpec {
    path: "sprites/ai_coop_life_icon_32x32.png",
    cell: Vec2::new(32.0, 32.0),
    bounds: (Vec2::new(6.0, 4.0), Vec2::new(27.0, 28.0)),
};

const TILE_CELL: Vec2 = Vec2::new(48.0, 48.0);
const WHOLE_TILE: (Vec2, Vec2) = (Vec2::ZERO, TILE_CELL);

pub(crate) const CONVEYOR_BELT: SheetSpec = SheetSpec {
    path: "sprites/ai_conveyor_belt_48x48.png",
    cell: TILE_CELL,
    bounds: WHOLE_TILE,
};
pub(crate) const TOMATO_SOUP: SheetSpec = SheetSpec {
    path: "sprites/ai_tomato_soup_48x48.png",
    cell: TILE_CELL,
    bounds: WHOLE_TILE,
};
pub(crate) const COOP_FLOOR: SheetSpec = SheetSpec {
    path: "sprites/ai_coop_floor_48x48.png",
    cell: TILE_CELL,
    bounds: WHOLE_TILE,
};
pub(crate) const COOP_WALL: SheetSpec = SheetSpec {
    path: "sprites/ai_coop_wall_48x48.png",
    cell: TILE_CELL,
    bounds: WHOLE_TILE,
};

// --- The chickens -------------------------------------------------------------
/// The tall chicken's head-on body: 23 px between its wings, measured across
/// its `idle_south` frames (13..36). The narrowest the chicken is ever drawn.
const CHICKEN_HEAD_ON_WIDTH: f32 = 23.0;
/// The chicken's hitbox: half the tall chicken's head-on body. One number for
/// both players and every facing — a box that grew when the chicken turned
/// would punish turning, and two boxes in one co-op game is unfair. The round
/// chicken's 29 px body overhangs it 3 px a side head-on and both overhang it
/// beak to tail, which can only let a player off.
pub(crate) const CHICKEN_HALF: f32 = CHICKEN_HEAD_ON_WIDTH / 2.0;
pub(crate) const STARTING_LIVES: u32 = 3;
/// Seconds between death and respawn at the start row.
pub(crate) const RESPAWN_DELAY: f32 = 1.0;
/// Attempt timer, seconds (Insane family shortens it).
pub(crate) const TIMER_NORMAL: f32 = 40.0;
pub(crate) const TIMER_INSANE: f32 = 25.0;
/// Start columns: solo spawns center; co-op splits left/right of center.
pub(crate) const SOLO_START_COL: u32 = 7;
pub(crate) const COOP_START_COLS: [u32; 2] = [5, 9];

// --- The nests and the round-clear beat ---------------------------------------
/// The home row's bottom edge, world y — the board's top row ends here.
const HOME_ROW_BOTTOM_Y: f32 = (ROWS as f32 - 1.0) / 2.0 * TILE - TILE / 2.0;
/// A nest's world y: its canvas's bottom edge sits on the home row's bottom
/// edge, so the drawn nest rises 27 px into the top band, to band y 45. That
/// is what leaves the band's first text row clear of the nest roofs.
pub(crate) const NEST_Y: f32 =
    HOME_ROW_BOTTOM_Y + NEST.cell.y - (NEST.bounds.0.y + NEST.bounds.1.y) / 2.0;
/// Where a seated sprite's cell centre lands on the nest canvas — the straw.
/// The chicken's 48 px cell therefore has its top-left at (24, 22) of the
/// 96 x 80 canvas, and the wider bun's at (16, 22).
pub(crate) const NEST_COMPOSITION_ANCHOR: Vec2 = Vec2::new(48.0, 46.0);

/// The round-clear beat: the fifth arrival plays out with the traffic still
/// moving and no input, then the round advances. Never shorter than the nest's
/// `arrival` (4 frames) or the burst's `celebrate` (6) at 100 ms a frame, which
/// the guard test re-derives from the sidecars so a retime cannot cut the
/// fifth celebration short.
pub(crate) const ROUND_CLEAR_BEAT: f32 = 1.0;

// --- Bands and HUD layout ------------------------------------------------------------
/// The score's and the round/homes line's text row, at the top band's leading
/// edge — the only row the nests leave clear.
pub(crate) const TOP_BAND_TEXT_Y: f32 = 16.0;
pub(crate) const HUD_TEXT_MARGIN: f32 = 24.0;
/// The pause hint's row, in the bottom band beside the timer bars.
pub(crate) const PAUSE_HINT_Y: f32 = WIN_H - BAND + 16.0;
pub(crate) const TIMER_BAR_H: f32 = 14.0;
const TIMER_BAR_HALF_H: f32 = TIMER_BAR_H / 2.0;
pub(crate) const TIMER_BAR_MARGIN: f32 = 60.0;
/// How much room a player's label takes off the left of their timer bar.
pub(crate) const TIMER_LABEL_GUTTER: f32 = 28.0;
/// How far the chaos mode's banner sits under the board's bottom edge.
pub(crate) const BANNER_INSET: f32 = 14.0;
/// The life-icon row: one world sprite per life, centred over its player's
/// slot on the band's inner half — above the timer bar, so the art clears the
/// bar below it and the chaos banner under the window edge.
pub(crate) const LIFE_ICON_PITCH: f32 = 40.0;
const LIFE_ICON_GAP: f32 = 2.0;
pub(crate) const LIFE_ICON_Y: f32 = -(WIN_H / 2.0)
    + BAND / 2.0
    + TIMER_BAR_HALF_H
    + LIFE_ICON_GAP
    + (LIFE_ICON.bounds.1.y - LIFE_ICON.bounds.0.y) / 2.0;

// --- Draw depths, in one nesting order ----------------------------------------------
/// The floor and the coop wall. They never overlap — the wall is the home row
/// between the nests, the floor is everything else — so they share a depth.
pub(crate) const GROUND_DEPTH: f32 = -6.0;
pub(crate) const SOUP_DEPTH: f32 = -5.0;
pub(crate) const BELT_DEPTH: f32 = -4.0;
/// Rafts and crackers, which the cart lanes and the nests stand above.
pub(crate) const PLATFORM_DEPTH: f32 = -3.0;
pub(crate) const CART_DEPTH: f32 = -2.0;
pub(crate) const NEST_DEPTH: f32 = -1.0;
/// The seated chicken and the bun, drawn on their nest's straw.
pub(crate) const SEATED_DEPTH: f32 = 0.0;
pub(crate) const PLAYER_DEPTH: f32 = 1.0;
pub(crate) const EFFECT_DEPTH: f32 = 2.0;
pub(crate) const LIFE_ICON_DEPTH: f32 = 3.0;

/// The backdrop grid's alpha, well under the preset's resting value: the
/// lattice reads over the soup and the belts without veiling them.
pub(crate) const BACKDROP_ALPHA: f32 = 0.25;

// --- The crackers' dive and the bun's duty cycle ------------------------------
/// Turtle dive cycle: they are under for the last `DIVE_DOWN_SECS`, so the art
/// goes under at this cycle time and the rules' own predicate owns that flip.
pub(crate) const DIVE_PERIOD: f32 = 5.0;
pub(crate) const DIVE_DOWN_SECS: f32 = 1.5;
pub(crate) const DIVE_FLIP_SECS: f32 = DIVE_PERIOD - DIVE_DOWN_SECS;
/// The bun's duty cycle: absent (harmless) then open (lethal).
pub(crate) const BUN_ABSENT_SECS: f32 = 4.0;
pub(crate) const BUN_PRESENT_SECS: f32 = 3.0;
/// Both danger sheets are authored a frame every 150 ms, which its sidecar
/// writes as 6.667 fps; the pose guard test re-derives the rate from them.
pub(crate) const DANGER_POSE_FPS: f32 = 6.667;
/// The cracker's drawn dive cycle, in cycle seconds, from the flip onward.
/// Rise is the lead-out and sink the lead-in, so a cracker drawn rising is
/// already rideable and one drawn sinking is not yet lethal.
pub(crate) const CRACKER_RISE_SECS: f32 = 0.45;
pub(crate) const CRACKER_IDLE_SECS: f32 = 2.0;
/// One 600 ms loop, ending where the lead-in begins.
pub(crate) const CRACKER_WARNING_SECS: f32 = 0.6;
pub(crate) const CRACKER_SINK_SECS: f32 = 0.45;
/// The bun's, from the flip onward: closing is the lead-out, opening the
/// lead-in, and a filled nest draws no bun at all.
pub(crate) const BUN_CLOSING_SECS: f32 = 0.45;
pub(crate) const BUN_HIDDEN_SECS: f32 = 2.5;
/// Two 600 ms loops, ending where the lead-in begins.
pub(crate) const BUN_WARNING_SECS: f32 = 0.6;
pub(crate) const BUN_OPENING_SECS: f32 = 0.45;

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

// --- Soup splash (the particle a drowning or a sweeping leaves under its poof) ---
/// The soup's sauce red, so a splash reads as the river rather than as water.
pub(crate) const SPLASH_COLOR: Vec4 = Vec4::new(0.72, 0.19, 0.12, 1.0);
