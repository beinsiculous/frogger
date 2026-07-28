//! Pure lane/frog rules — data in, data out, no entities. This is the
//! primary test surface: every collision, death, and timing rule the game
//! depends on lives here.

use engine_core::prelude::*;

use crate::constants::*;
use crate::types::*;

/// The ten lanes, bottom of the board first. Directions alternate per band;
/// speeds are Normal-mode base values.
pub(crate) const LANES: [LaneDef; 10] = [
    // --- Road (rows 11..7, above the start row) ---
    LaneDef { row: 11, kind: LaneKind::Car, dir: 1.0, speed: 70.0, count: 3, len_tiles: 1.0 },
    LaneDef { row: 10, kind: LaneKind::Car, dir: -1.0, speed: 90.0, count: 3, len_tiles: 1.0 },
    LaneDef { row: 9, kind: LaneKind::Truck, dir: 1.0, speed: 60.0, count: 2, len_tiles: 2.0 },
    LaneDef { row: 8, kind: LaneKind::Car, dir: -1.0, speed: 120.0, count: 3, len_tiles: 1.0 },
    LaneDef { row: 7, kind: LaneKind::Truck, dir: 1.0, speed: 80.0, count: 2, len_tiles: 2.0 },
    // --- Water (rows 5..1, above the median) ---
    LaneDef { row: 5, kind: LaneKind::Turtles, dir: -1.0, speed: 70.0, count: 3, len_tiles: 3.0 },
    LaneDef { row: 4, kind: LaneKind::Log, dir: 1.0, speed: 80.0, count: 3, len_tiles: 3.0 },
    LaneDef { row: 3, kind: LaneKind::Log, dir: 1.0, speed: 110.0, count: 2, len_tiles: 4.0 },
    LaneDef { row: 2, kind: LaneKind::Turtles, dir: -1.0, speed: 90.0, count: 3, len_tiles: 2.0 },
    LaneDef { row: 1, kind: LaneKind::Log, dir: 1.0, speed: 65.0, count: 3, len_tiles: 3.0 },
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

/// Signed velocity of every obstacle in `lane` this round, px/s.
pub(crate) fn lane_velocity(def: &LaneDef, mode: ChaosMode, round: u32) -> f32 {
    def.dir * def.speed * speed_mult(mode, round)
}

/// Attempt timer for the chaos family.
pub(crate) fn attempt_timer(mode: ChaosMode) -> f32 {
    if mode.is_insane() { TIMER_INSANE } else { TIMER_NORMAL }
}

/// Even spacing of `count` obstacles around the torus.
pub(crate) fn initial_lane_positions(count: usize) -> Vec<f32> {
    let spacing = LANE_PERIOD / count as f32;
    (0..count)
        .map(|i| wrap_lane_x(-LANE_PERIOD / 2.0 + i as f32 * spacing))
        .collect()
}

/// Is turtle group `index` of `lane_row` submerged at `play_time`?
/// Only the Ridiculous family dives; phases are hash-offset per lane and
/// group so the river never blinks in lockstep.
pub(crate) fn turtles_submerged(mode: ChaosMode, lane_row: u32, index: usize, play_time: f32) -> bool {
    if !mode.is_ridiculous() {
        return false;
    }
    let phase = hash_f32(lane_row * 31 + index as u32) * DIVE_PERIOD;
    (play_time + phase).rem_euclid(DIVE_PERIOD) > DIVE_PERIOD - DIVE_DOWN_SECS
}

/// The platform (its signed velocity) under a frog at `frog_x` in a water
/// lane, or `None` = open water. `xs` are the lane's obstacle centers;
/// submerged turtle groups are not platforms.
pub(crate) fn platform_under(
    frog_x: f32,
    def: &LaneDef,
    xs: &[f32],
    mode: ChaosMode,
    round: u32,
    play_time: f32,
) -> Option<f32> {
    xs.iter().enumerate().find_map(|(i, &x)| {
        if def.kind == LaneKind::Turtles && turtles_submerged(mode, def.row, i, play_time) {
            return None;
        }
        // The frog stays aboard while its CENTER is on the platform.
        lane_overlap(frog_x, 0.0, x, def.half_len()).then(|| lane_velocity(def, mode, round))
    })
}

/// Does a frog at `frog_x` overlap any obstacle of a road lane?
pub(crate) fn road_hit(frog_x: f32, def: &LaneDef, xs: &[f32]) -> bool {
    xs.iter().any(|&x| lane_overlap(frog_x, FROG_HALF, x, def.half_len()))
}

/// Ridiculous family: which home slot (0..5) the croc guards this round.
pub(crate) fn croc_slot(mode: ChaosMode, round: u32) -> Option<usize> {
    mode.is_ridiculous().then(|| (hash_u32(round) % 5) as usize)
}

/// Is the croc surfaced (slot lethal) at `play_time`? The duty cycle starts
/// absent, hash-phased per round; every slot stays fillable across phases.
pub(crate) fn croc_present(round: u32, play_time: f32) -> bool {
    let cycle = CROC_ABSENT_SECS + CROC_PRESENT_SECS;
    let phase = hash_f32(round.wrapping_mul(97)) * cycle;
    (play_time + phase).rem_euclid(cycle) > CROC_ABSENT_SECS
}

/// What happens when a frog enters the home row near column `col`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SlotOutcome {
    /// Slot `index` (0..5) fills.
    Fill(usize),
    /// Hedge, or an already-filled slot.
    Miss,
    /// The croc-guarded slot while the croc is surfaced.
    Croc,
}

/// Resolve a home-row entry at world x `frog_x`: snap to the nearest board
/// column, then check slot / croc / hedge.
pub(crate) fn home_slot_at(
    frog_x: f32,
    homes: &[bool; 5],
    mode: ChaosMode,
    round: u32,
    play_time: f32,
) -> SlotOutcome {
    let col = nearest_col(frog_x);
    let Some(index) = HOME_COLS.iter().position(|&c| c == col) else {
        return SlotOutcome::Miss;
    };
    // A filled slot is a plain miss; the croc never surfaces in one
    // (matching the marker logic in `sync_sprites`).
    if homes[index] {
        return SlotOutcome::Miss;
    }
    if croc_slot(mode, round) == Some(index) && croc_present(round, play_time) {
        return SlotOutcome::Croc;
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

/// Apply a hop to a frog position: rows clamp to the board, x clamps to the
/// board edge columns. Returns the new `(x, row)`.
pub(crate) fn apply_hop(x: f32, row: u32, hop: Hop) -> (f32, u32) {
    let half_span = (COLS as f32 - 1.0) / 2.0 * TILE;
    match hop {
        Hop::Up => (x, row.saturating_sub(1)),
        Hop::Down => (x, (row + 1).min(START_ROW)),
        Hop::Left => ((x - TILE).max(-half_span), row),
        Hop::Right => ((x + TILE).min(half_span), row),
    }
}

/// A frog carried past the board edge is swept away (frogs never wrap —
/// only obstacles live on the torus).
pub(crate) fn swept_off(frog_x: f32) -> bool {
    frog_x.abs() > WIN_W / 2.0
}

/// Row-kind helpers over the row map.
pub(crate) fn is_water_row(row: u32) -> bool {
    (FIRST_WATER_ROW..=LAST_WATER_ROW).contains(&row)
}

pub(crate) fn is_road_row(row: u32) -> bool {
    (FIRST_ROAD_ROW..=LAST_ROAD_ROW).contains(&row)
}
