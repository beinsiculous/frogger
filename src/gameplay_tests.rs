//! Headless gameplay tests: pure lane/frog rules, croc and dive cycles,
//! chaos scaling, and lane-table sanity. No GPU, no window, no physics.

use engine_core::prelude::*;

use crate::constants::*;
use crate::gameplay::rules::*;
use crate::types::*;

// --- Torus wrap + modular overlap (the F5 straddle cases) ---

#[test]
fn test_wrap_lane_x_keeps_positions_on_the_torus() {
    let half = LANE_PERIOD / 2.0;
    for x in [-2.5 * LANE_PERIOD, -half, -1.0, 0.0, half - 0.1, half, 3.0 * LANE_PERIOD] {
        let w = wrap_lane_x(x);
        assert!((-half..half).contains(&w), "wrap({x}) = {w} out of range");
    }
    // Wrapping is periodic: x and x + P land on the same point.
    assert!((wrap_lane_x(100.0) - wrap_lane_x(100.0 + LANE_PERIOD)).abs() < 1e-3);
}

#[test]
fn test_lane_overlap_plain_and_straddling() {
    let half = LANE_PERIOD / 2.0;
    // Plain overlap.
    assert!(lane_overlap(0.0, FROG_HALF, 20.0, 48.0));
    // Clearly apart.
    assert!(!lane_overlap(0.0, FROG_HALF, 200.0, 48.0));
    // A truck straddling the seam: center just inside +P/2, body wrapping
    // to the far side. A frog near -P/2 must still be hit.
    let truck_half = 2.0 * TILE / 2.0;
    let truck_x = half - 10.0;
    let frog_x = -half + 20.0;
    // Modular distance is 30 < frog_half + truck_half.
    assert!(lane_overlap(frog_x, FROG_HALF, truck_x, truck_half));
    // The naive (non-modular) distance would be ~P, i.e. a miss — which is
    // exactly the bug the modular test exists to prevent.
    assert!((frog_x - truck_x).abs() > truck_half + FROG_HALF);
}

#[test]
fn test_straddling_log_still_carries_the_frog() {
    let def = LaneDef { row: 3, kind: LaneKind::Log, dir: 1.0, speed: 100.0, count: 1, len_tiles: 4.0 };
    let half = LANE_PERIOD / 2.0;
    // Log centered just inside the seam; frog stands on the wrapped tail.
    let xs = [half - 20.0];
    let frog_x = -half + 40.0; // modular distance 60 < log half (96)
    let ride = platform_under(frog_x, &def, &xs, ChaosMode::Normal, 1, 0.0);
    assert_eq!(ride, Some(lane_velocity(&def, ChaosMode::Normal, 1)));
}

#[test]
fn test_initial_lane_positions_evenly_spaced_on_torus() {
    let xs = initial_lane_positions(3);
    assert_eq!(xs.len(), 3);
    let spacing = LANE_PERIOD / 3.0;
    assert!((wrapped_dx(xs[1], xs[0]) - spacing).abs() < 1e-3);
    assert!((wrapped_dx(xs[2], xs[1]) - spacing).abs() < 1e-3);
}

// --- Riding, drowning, sweeping ---

#[test]
fn test_platform_under_between_logs_is_open_water() {
    let def = LaneDef { row: 4, kind: LaneKind::Log, dir: 1.0, speed: 80.0, count: 2, len_tiles: 3.0 };
    let xs = [-200.0, 200.0];
    assert!(platform_under(0.0, &def, &xs, ChaosMode::Normal, 1, 0.0).is_none());
    assert!(platform_under(-200.0, &def, &xs, ChaosMode::Normal, 1, 0.0).is_some());
}

#[test]
fn test_ride_drift_accumulates_and_sweeps_off_the_edge() {
    let def = LaneDef { row: 4, kind: LaneKind::Log, dir: 1.0, speed: 80.0, count: 1, len_tiles: 3.0 };
    let v = lane_velocity(&def, ChaosMode::Normal, 1);
    let dt = 1.0 / 60.0;
    let mut frog_x = 300.0;
    for _ in 0..600 {
        frog_x += v * dt;
        if swept_off(frog_x) {
            return; // carried past the board edge: the Swept death fires
        }
    }
    panic!("a frog riding right at {v} px/s must eventually sweep off");
}

#[test]
fn test_swept_off_only_beyond_the_board_edge() {
    assert!(!swept_off(0.0));
    assert!(!swept_off(WIN_W / 2.0 - 1.0));
    assert!(swept_off(WIN_W / 2.0 + 1.0));
    assert!(swept_off(-WIN_W / 2.0 - 1.0));
}

#[test]
fn test_road_hit_uses_frog_and_obstacle_extents() {
    let def = LaneDef { row: 8, kind: LaneKind::Car, dir: -1.0, speed: 120.0, count: 1, len_tiles: 1.0 };
    let hit_range = FROG_HALF + def.half_len();
    assert!(road_hit(0.0, &def, &[hit_range - 1.0]));
    assert!(!road_hit(0.0, &def, &[hit_range + 1.0]));
}

// --- Turtles diving (Ridiculous family only) ---

#[test]
fn test_turtles_never_dive_outside_the_ridiculous_family() {
    for t in 0..200 {
        let time = t as f32 * 0.1;
        assert!(!turtles_submerged(ChaosMode::Normal, 5, 0, time));
        assert!(!turtles_submerged(ChaosMode::Insane, 5, 0, time));
    }
}

#[test]
fn test_ridiculous_turtles_dive_for_the_configured_window() {
    let mut submerged_time = 0.0;
    let dt = 0.01;
    let steps = (DIVE_PERIOD / dt) as usize;
    for i in 0..steps {
        if turtles_submerged(ChaosMode::Ridiculous, 5, 0, i as f32 * dt) {
            submerged_time += dt;
        }
    }
    assert!(
        (submerged_time - DIVE_DOWN_SECS).abs() < 0.1,
        "submerged {submerged_time}s of each {DIVE_PERIOD}s cycle, wanted {DIVE_DOWN_SECS}"
    );
}

#[test]
fn test_submerged_turtles_are_not_platforms() {
    let def = LaneDef { row: 5, kind: LaneKind::Turtles, dir: -1.0, speed: 70.0, count: 1, len_tiles: 3.0 };
    let xs = [0.0];
    // Find one submerged and one surfaced instant of the cycle.
    let mut surfaced_at = None;
    let mut submerged_at = None;
    for i in 0..500 {
        let t = i as f32 * 0.02;
        if turtles_submerged(ChaosMode::Ridiculous, def.row, 0, t) {
            submerged_at.get_or_insert(t);
        } else {
            surfaced_at.get_or_insert(t);
        }
    }
    let (surfaced, submerged) = (surfaced_at.unwrap(), submerged_at.unwrap());
    assert!(platform_under(0.0, &def, &xs, ChaosMode::Ridiculous, 1, surfaced).is_some());
    assert!(platform_under(0.0, &def, &xs, ChaosMode::Ridiculous, 1, submerged).is_none());
    // The same instant on Normal keeps the platform (no diving).
    assert!(platform_under(0.0, &def, &xs, ChaosMode::Normal, 1, submerged).is_some());
}

// --- Croc cycle (F1: rounds must stay winnable) ---

#[test]
fn test_croc_only_in_the_ridiculous_family_and_slot_in_range() {
    assert_eq!(croc_slot(ChaosMode::Normal, 1), None);
    assert_eq!(croc_slot(ChaosMode::Insane, 1), None);
    for round in 1..50 {
        for mode in [ChaosMode::Ridiculous, ChaosMode::Insiculous] {
            let slot = croc_slot(mode, round).expect("ridiculous family has a croc");
            assert!(slot < 5);
        }
        // Deterministic per round.
        assert_eq!(croc_slot(ChaosMode::Ridiculous, round), croc_slot(ChaosMode::Ridiculous, round));
    }
}

#[test]
fn test_croc_duty_cycle_has_absent_and_present_phases() {
    for round in 1..10 {
        let cycle = CROC_ABSENT_SECS + CROC_PRESENT_SECS;
        let mut absent = 0.0;
        let mut present = 0.0;
        let dt = 0.01;
        let steps = (cycle / dt) as usize;
        for i in 0..steps {
            if croc_present(round, i as f32 * dt) {
                present += dt;
            } else {
                absent += dt;
            }
        }
        assert!((absent - CROC_ABSENT_SECS).abs() < 0.1, "round {round}: absent {absent}");
        assert!((present - CROC_PRESENT_SECS).abs() < 0.1, "round {round}: present {present}");
    }
}

#[test]
fn test_ridiculous_round_is_winnable_every_slot_fillable() {
    // The winnability lock: for any round, every slot must have some moment
    // where entering it fills (croc absent or elsewhere, slot empty).
    for round in 1..20 {
        for (slot, &col) in HOME_COLS.iter().enumerate() {
            let homes = [false; 5];
            let x = crate::board::tile_center(col, HOME_ROW).x;
            let fillable = (0..1000).any(|i| {
                let t = i as f32 * 0.05;
                home_slot_at(x, &homes, ChaosMode::Ridiculous, round, t)
                    == SlotOutcome::Fill(slot)
            });
            assert!(fillable, "round {round} slot {slot} can never be filled");
        }
    }
}

// --- Home row resolution ---

#[test]
fn test_home_slot_outcomes() {
    let mut homes = [false; 5];
    let slot1_x = crate::board::tile_center(HOME_COLS[1], HOME_ROW).x;
    // Open slot fills (Normal: no croc at all).
    assert_eq!(home_slot_at(slot1_x, &homes, ChaosMode::Normal, 1, 0.0), SlotOutcome::Fill(1));
    // Filled slot is a miss.
    homes[1] = true;
    assert_eq!(home_slot_at(slot1_x, &homes, ChaosMode::Normal, 1, 0.0), SlotOutcome::Miss);
    // Hedge (column 2 is between slots) is a miss.
    let hedge_x = crate::board::tile_center(2, HOME_ROW).x;
    assert_eq!(home_slot_at(hedge_x, &homes, ChaosMode::Normal, 1, 0.0), SlotOutcome::Miss);
}

#[test]
fn test_croc_slot_kills_only_while_present() {
    let homes = [false; 5];
    for round in 1..10 {
        let slot = croc_slot(ChaosMode::Ridiculous, round).unwrap();
        let x = crate::board::tile_center(HOME_COLS[slot], HOME_ROW).x;
        let mut saw_croc = false;
        let mut saw_fill = false;
        for i in 0..1000 {
            let t = i as f32 * 0.05;
            match home_slot_at(x, &homes, ChaosMode::Ridiculous, round, t) {
                SlotOutcome::Croc => {
                    saw_croc = true;
                    assert!(croc_present(round, t));
                }
                SlotOutcome::Fill(s) => {
                    saw_fill = true;
                    assert_eq!(s, slot);
                    assert!(!croc_present(round, t));
                }
                SlotOutcome::Miss => panic!("open croc slot never resolves to Miss"),
            }
        }
        assert!(saw_croc && saw_fill, "round {round}: croc slot must both kill and fill");
    }
}

#[test]
fn test_nearest_col_snaps_and_clamps() {
    assert_eq!(nearest_col(0.0), 7);
    assert_eq!(nearest_col(crate::board::tile_center(3, 0).x + 10.0), 3);
    assert_eq!(nearest_col(-10_000.0), 0);
    assert_eq!(nearest_col(10_000.0), COLS - 1);
}

// --- Hop resolution (F7: one hop per frame, fixed priority) ---

#[test]
fn test_resolve_hop_priority_on_conflicting_inputs() {
    assert_eq!(resolve_hop(true, true, true, true), Some(Hop::Up));
    assert_eq!(resolve_hop(false, true, true, true), Some(Hop::Down));
    assert_eq!(resolve_hop(false, false, true, true), Some(Hop::Left));
    assert_eq!(resolve_hop(false, false, false, true), Some(Hop::Right));
    assert_eq!(resolve_hop(false, false, false, false), None);
}

#[test]
fn test_apply_hop_moves_one_tile_and_clamps_at_edges() {
    // Vertical moves change the row only.
    assert_eq!(apply_hop(0.0, 6, Hop::Up), (0.0, 5));
    assert_eq!(apply_hop(0.0, 6, Hop::Down), (0.0, 7));
    // Rows clamp at home and start.
    assert_eq!(apply_hop(0.0, 0, Hop::Up), (0.0, 0));
    assert_eq!(apply_hop(0.0, START_ROW, Hop::Down), (0.0, START_ROW));
    // Horizontal moves shift one tile and clamp at the board edge.
    assert_eq!(apply_hop(0.0, 6, Hop::Right), (TILE, 6));
    let half_span = (COLS as f32 - 1.0) / 2.0 * TILE;
    assert_eq!(apply_hop(half_span, 6, Hop::Right), (half_span, 6));
    assert_eq!(apply_hop(-half_span, 6, Hop::Left), (-half_span, 6));
}

// --- Chaos scaling + timers ---

#[test]
fn test_speed_mult_chaos_and_round_ramp() {
    assert_eq!(speed_mult(ChaosMode::Normal, 1), 1.0);
    assert_eq!(speed_mult(ChaosMode::Insane, 1), INSANE_SPEED_MULT);
    assert_eq!(speed_mult(ChaosMode::Ridiculous, 1), 1.0);
    assert_eq!(speed_mult(ChaosMode::Insiculous, 1), INSANE_SPEED_MULT);
    // Round ramp: +15% per round, capped.
    assert!((speed_mult(ChaosMode::Normal, 2) - 1.15).abs() < 1e-4);
    assert_eq!(speed_mult(ChaosMode::Normal, 100), ROUND_SPEED_MULT_MAX);
    assert_eq!(
        speed_mult(ChaosMode::Insane, 100),
        INSANE_SPEED_MULT * ROUND_SPEED_MULT_MAX
    );
}

#[test]
fn test_attempt_timer_per_chaos_family() {
    assert_eq!(attempt_timer(ChaosMode::Normal), TIMER_NORMAL);
    assert_eq!(attempt_timer(ChaosMode::Ridiculous), TIMER_NORMAL);
    assert_eq!(attempt_timer(ChaosMode::Insane), TIMER_INSANE);
    assert_eq!(attempt_timer(ChaosMode::Insiculous), TIMER_INSANE);
}

// --- Lane table sanity ---

#[test]
fn test_lane_table_covers_every_water_and_road_row_once() {
    let mut rows: Vec<u32> = LANES.iter().map(|l| l.row).collect();
    rows.sort_unstable();
    let mut expected: Vec<u32> = (FIRST_WATER_ROW..=LAST_WATER_ROW)
        .chain(FIRST_ROAD_ROW..=LAST_ROAD_ROW)
        .collect();
    expected.sort_unstable();
    assert_eq!(rows, expected, "exactly one lane per water/road row");
}

#[test]
fn test_lane_kinds_match_their_rows() {
    for def in LANES {
        let water_kind = matches!(def.kind, LaneKind::Log | LaneKind::Turtles);
        assert_eq!(
            is_water_row(def.row),
            water_kind,
            "row {} carries {:?}",
            def.row,
            def.kind
        );
        assert!(def.dir == 1.0 || def.dir == -1.0);
        assert!(def.count > 0 && def.speed > 0.0 && def.len_tiles > 0.0);
        // Obstacles must fit their lane with open water/road between them.
        assert!(
            def.count as f32 * def.len_tiles * TILE < LANE_PERIOD,
            "row {} overfilled",
            def.row
        );
    }
}

// --- Match-level state (no GPU, no ctx) ---

fn playing_game(mode: GameMode, chaos: ChaosMode) -> FroggerGame {
    let mut game = FroggerGame {
        mode,
        chaos_mode: chaos,
        state: GameState::Playing,
        ..FroggerGame::default()
    };
    let timer = attempt_timer(chaos);
    let count = mode.player_count();
    game.frogs = (0..count)
        .map(|i| {
            let col = crate::gameplay::start_col(mode, i);
            let mut frog = FrogState::new(col, timer);
            frog.x = crate::board::tile_center(col, 0).x;
            frog
        })
        .collect();
    game
}

#[test]
fn test_coop_frogs_spawn_on_distinct_columns() {
    let game = playing_game(GameMode::TwoPlayerCoop, ChaosMode::Normal);
    assert_eq!(game.frogs.len(), 2);
    assert_ne!(game.frogs[0].x, game.frogs[1].x);
    let solo = playing_game(GameMode::SinglePlayer, ChaosMode::Normal);
    assert_eq!(solo.frogs.len(), 1);
    assert_eq!(solo.frogs[0].x, crate::board::tile_center(SOLO_START_COL, 0).x);
}

#[test]
fn test_frog_active_state_transitions() {
    let mut frog = FrogState::new(7, TIMER_NORMAL);
    assert!(frog.active());
    frog.respawn_timer = RESPAWN_DELAY;
    assert!(!frog.active());
    frog.respawn_timer = 0.0;
    frog.retired = true;
    assert!(!frog.active());
}
