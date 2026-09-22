//! Pure lane/chicken rules — data in, data out, no entities. This is the
//! primary test surface: every collision, death, and timing rule the game
//! depends on lives here, as does the phase each drawn pose is a function of.

use engine_core::prelude::*;

use crate::constants::*;
use crate::types::*;

/// The ten lanes, bottom of the board first. Directions alternate per band;
/// speeds are Normal-mode base values.
pub(crate) const LANES: [LaneDef; 10] = [
    // --- Road (rows 11..7, above the start row) ---
    LaneDef {
        row: 11,
        kind: LaneKind::Car,
        dir: 1.0,
        speed: 70.0,
        count: 3,
        segment: Segment { sprite: LaneSprite::SushiCart, sprites: 1 },
    },
    LaneDef {
        row: 10,
        kind: LaneKind::Car,
        dir: -1.0,
        speed: 90.0,
        count: 3,
        segment: Segment { sprite: LaneSprite::DonutCart, sprites: 1 },
    },
    LaneDef {
        row: 9,
        kind: LaneKind::Truck,
        dir: 1.0,
        speed: 60.0,
        count: 2,
        segment: Segment { sprite: LaneSprite::HotDogCart, sprites: 1 },
    },
    LaneDef {
        row: 8,
        kind: LaneKind::Car,
        dir: -1.0,
        speed: 120.0,
        count: 3,
        segment: Segment { sprite: LaneSprite::SushiCart, sprites: 1 },
    },
    LaneDef {
        row: 7,
        kind: LaneKind::Truck,
        dir: 1.0,
        speed: 80.0,
        count: 2,
        segment: Segment { sprite: LaneSprite::HotDogCart, sprites: 1 },
    },
    // --- Soup (rows 5..1, above the median) ---
    LaneDef {
        row: 5,
        kind: LaneKind::Crackers,
        dir: -1.0,
        speed: 70.0,
        count: 3,
        segment: Segment { sprite: LaneSprite::Cracker, sprites: 3 },
    },
    LaneDef {
        row: 4,
        kind: LaneKind::Log,
        dir: 1.0,
        speed: 80.0,
        count: 3,
        segment: Segment { sprite: LaneSprite::CeleryRaft, sprites: 2 },
    },
    LaneDef {
        row: 3,
        kind: LaneKind::Log,
        dir: 1.0,
        speed: 110.0,
        count: 2,
        segment: Segment { sprite: LaneSprite::BaguetteRaft, sprites: 2 },
    },
    LaneDef {
        row: 2,
        kind: LaneKind::Crackers,
        dir: -1.0,
        speed: 90.0,
        count: 3,
        segment: Segment { sprite: LaneSprite::Cracker, sprites: 2 },
    },
    LaneDef {
        row: 1,
        kind: LaneKind::Log,
        dir: 1.0,
        speed: 65.0,
        count: 3,
        segment: Segment { sprite: LaneSprite::CeleryRaft, sprites: 2 },
    },
];

/// Wrap a lane position onto the torus `[-LANE_PERIOD/2, LANE_PERIOD/2)`.
pub(crate) fn wrap_lane_x(x: f32) -> f32 {
    (x + LANE_PERIOD / 2.0).rem_euclid(LANE_PERIOD) - LANE_PERIOD / 2.0
}

/// Shortest signed distance from `b` to `a` on the lane torus. Handles
/// obstacles straddling the wrap seam exactly — the "two AABBs" of an
/// edge-straddling truck collapse into this one modular formula.
pub(crate) fn wrapped_dx(a: f32, b: f32) -> f32 {
    wrap_lane_x(a - b)
}

/// 1-D overlap test on the lane torus between centers `a` (half-extent
/// `half_a`) and `b` (half-extent `half_b`).
pub(crate) fn lane_overlap(a: f32, half_a: f32, b: f32, half_b: f32) -> bool {
    wrapped_dx(a, b).abs() < half_a + half_b
}

/// Combined speed multiplier: chaos family x round ramp.
pub(crate) fn speed_mult(mode: ChaosMode, round: u32) -> f32 {
    let chaos = if mode.is_insane() { INSANE_SPEED_MULT } else { 1.0 };
    let ramp = (1.0 + ROUND_SPEED_RAMP * (round.saturating_sub(1)) as f32)
        .min(ROUND_SPEED_MULT_MAX);
    chaos * ramp
}

/// Signed velocity of every segment in `lane` this round, px/s.
pub(crate) fn lane_velocity(def: &LaneDef, mode: ChaosMode, round: u32) -> f32 {
    def.dir * def.speed * speed_mult(mode, round)
}

/// Attempt timer for the chaos family.
pub(crate) fn attempt_timer(mode: ChaosMode) -> f32 {
    if mode.is_insane() { TIMER_INSANE } else { TIMER_NORMAL }
}

/// Even spacing of `count` segments around the torus.
pub(crate) fn initial_lane_positions(count: usize) -> Vec<f32> {
    let spacing = LANE_PERIOD / count as f32;
    (0..count)
        .map(|i| wrap_lane_x(-LANE_PERIOD / 2.0 + i as f32 * spacing))
        .collect()
}

/// The cycle offset a cracker group's dive is hashed to: staggered per lane
/// and group so the soup never sinks in lockstep.
fn dive_phase(lane_row: u32, index: usize) -> f32 {
    hash_f32(lane_row * 31 + index as u32) * DIVE_PERIOD
}

/// Where `play_time` sits in cracker group `index`'s phased dive cycle, in
/// `[0, DIVE_PERIOD)` — the clock the drawn poses are functions of.
pub(crate) fn cracker_cycle_time(lane_row: u32, index: usize, play_time: f32) -> f32 {
    (play_time + dive_phase(lane_row, index)).rem_euclid(DIVE_PERIOD)
}

/// Is cracker group `index` of `lane_row` submerged at `play_time`?
/// Only the Ridiculous family sinks, and the flip is this predicate's alone —
/// every drawn pose asks it rather than restating the inequality.
pub(crate) fn crackers_submerged(
    mode: ChaosMode,
    lane_row: u32,
    index: usize,
    play_time: f32,
) -> bool {
    if !mode.is_ridiculous() {
        return false;
    }
    cracker_cycle_time(lane_row, index, play_time) > DIVE_FLIP_SECS
}

/// The platform (its signed velocity) under a chicken at `chicken_x` in a
/// soup lane, or `None` = open soup. `xs` are the lane's segment centers;
/// submerged cracker groups are not platforms.
pub(crate) fn platform_under(
    chicken_x: f32,
    def: &LaneDef,
    xs: &[f32],
    mode: ChaosMode,
    round: u32,
    play_time: f32,
) -> Option<f32> {
    xs.iter().enumerate().find_map(|(i, &x)| {
        if def.kind == LaneKind::Crackers && crackers_submerged(mode, def.row, i, play_time) {
            return None;
        }
        // The chicken stays aboard while its CENTER is on the platform.
        lane_overlap(chicken_x, 0.0, x, def.half_len()).then(|| lane_velocity(def, mode, round))
    })
}

/// Does a chicken at `chicken_x` overlap any segment of a road lane?
pub(crate) fn road_hit(chicken_x: f32, def: &LaneDef, xs: &[f32]) -> bool {
    xs.iter().any(|&x| lane_overlap(chicken_x, CHICKEN_HALF, x, def.half_len()))
}

/// Ridiculous family: which nest (0..5) the bun guards this round.
pub(crate) fn bun_nest(mode: ChaosMode, round: u32) -> Option<usize> {
    mode.is_ridiculous().then(|| (hash_u32(round) % 5) as usize)
}

/// The cycle offset the bun's duty is hashed to, per round.
fn bun_phase(round: u32) -> f32 {
    hash_f32(round.wrapping_mul(97)) * (BUN_ABSENT_SECS + BUN_PRESENT_SECS)
}

/// Where `play_time` sits in the bun's phased duty cycle, in `[0, cycle)` —
/// the clock the bun's drawn pose is a function of.
pub(crate) fn bun_cycle_time(round: u32, play_time: f32) -> f32 {
    (play_time + bun_phase(round)).rem_euclid(BUN_ABSENT_SECS + BUN_PRESENT_SECS)
}

/// Is the bun open (its nest lethal) at `play_time`? The duty cycle starts
/// absent, hash-phased per round; every nest stays fillable across phases.
/// This predicate owns the flip; the drawn pose asks it.
pub(crate) fn bun_open(mode: ChaosMode, round: u32, play_time: f32) -> bool {
    if !mode.is_ridiculous() {
        return false;
    }
    bun_cycle_time(round, play_time) > BUN_ABSENT_SECS
}

/// What happens when a chicken enters the home row near column `col`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SlotOutcome {
    /// Nest `index` (0..5) fills.
    Fill(usize),
    /// Wall, or an already-filled nest.
    Miss,
    /// The bun's nest while the bun is open.
    Bun,
}

/// Resolve a home-row entry at world x `chicken_x`: snap to the nearest board
/// column, then check nest / bun / wall.
pub(crate) fn home_slot_at(
    chicken_x: f32,
    homes: &[bool; 5],
    mode: ChaosMode,
    round: u32,
    play_time: f32,
) -> SlotOutcome {
    let col = nearest_col(chicken_x);
    let Some(index) = HOME_COLS.iter().position(|&c| c == col) else {
        return SlotOutcome::Miss;
    };
    // A filled nest is a plain miss; the bun never opens in one (matching
    // the poses `sync_sprites` writes).
    if homes[index] {
        return SlotOutcome::Miss;
    }
    if bun_nest(mode, round) == Some(index) && bun_open(mode, round, play_time) {
        return SlotOutcome::Bun;
    }
    SlotOutcome::Fill(index)
}

/// Nearest board column to world x, clamped into the board.
pub(crate) fn nearest_col(x: f32) -> u32 {
    let col = (x / TILE + (COLS as f32 - 1.0) / 2.0).round();
    col.clamp(0.0, COLS as f32 - 1.0) as u32
}

/// One hop direction, resolved from possibly-conflicting same-frame inputs
/// with fixed priority Up > Down > Left > Right (at most one hop per frame).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Hop {
    Up,
    Down,
    Left,
    Right,
}

impl Hop {
    /// The facing a chicken wears after this hop — the one source of the
    /// drawn pose.
    pub(crate) fn facing(self) -> Facing {
        match self {
            Hop::Up => Facing::North,
            Hop::Down => Facing::South,
            Hop::Left => Facing::West,
            Hop::Right => Facing::East,
        }
    }
}

pub(crate) fn resolve_hop(up: bool, down: bool, left: bool, right: bool) -> Option<Hop> {
    if up {
        Some(Hop::Up)
    } else if down {
        Some(Hop::Down)
    } else if left {
        Some(Hop::Left)
    } else if right {
        Some(Hop::Right)
    } else {
        None
    }
}

/// Apply a hop to a chicken position: rows clamp to the board, x clamps to
/// the board edge columns. Returns the new `(x, row)`.
pub(crate) fn apply_hop(x: f32, row: u32, hop: Hop) -> (f32, u32) {
    let half_span = (COLS as f32 - 1.0) / 2.0 * TILE;
    match hop {
        Hop::Up => (x, row.saturating_sub(1)),
        Hop::Down => (x, (row + 1).min(START_ROW)),
        Hop::Left => ((x - TILE).max(-half_span), row),
        Hop::Right => ((x + TILE).min(half_span), row),
    }
}

/// A chicken carried past the board edge is swept away (chickens never wrap —
/// only obstacles live on the torus).
pub(crate) fn swept_off(chicken_x: f32) -> bool {
    chicken_x.abs() > WIN_W / 2.0
}

/// Row-kind helpers over the row map.
pub(crate) fn is_water_row(row: u32) -> bool {
    (FIRST_WATER_ROW..=LAST_WATER_ROW).contains(&row)
}

pub(crate) fn is_road_row(row: u32) -> bool {
    (FIRST_ROAD_ROW..=LAST_ROAD_ROW).contains(&row)
}
