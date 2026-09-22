//! Headless tests for the pure rules: lane math on the torus, the dive and
//! duty cycles, home-row resolution, hop priority, chaos scaling, the lane
//! table and the match-level state. No GPU, no window, no physics, no
//! context — data in, data out.

use engine_core::prelude::*;

use crate::constants::*;
use crate::gameplay::rules::*;
use crate::test_support::{lane, playing_game};
use crate::types::*;

// --- Torus wrap + modular overlap ---

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
    assert!(lane_overlap(0.0, CHICKEN_HALF, 20.0, 48.0));
    // Clearly apart.
    assert!(!lane_overlap(0.0, CHICKEN_HALF, 200.0, 48.0));
    // A cart straddling the seam: center just inside +P/2, body wrapping to
    // the far side. A chicken near -P/2 must still be hit.
    let cart_half = 55.0 / 2.0;
    let cart_x = half - 10.0;
    let chicken_x = -half + 20.0;
    // Modular distance is 30 < chicken_half + cart_half.
    assert!(lane_overlap(chicken_x, CHICKEN_HALF, cart_x, cart_half));
    // The naive (non-modular) distance would be ~P, i.e. a miss — which is
    // exactly the bug the modular test exists to prevent.
    assert!((chicken_x - cart_x).abs() > cart_half + CHICKEN_HALF);
}

#[test]
fn test_straddling_raft_still_carries_the_chicken() {
    let def = lane(3);
    let half = LANE_PERIOD / 2.0;
    // The raft is centered just inside the seam; the chicken stands on the
    // wrapped tail, 60 px away in modular distance and well inside its 92.5.
    let xs = [half - 20.0];
    let chicken_x = -half + 40.0;
    let ride = platform_under(chicken_x, &def, &xs, ChaosMode::Normal, 1, 0.0);
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
fn test_platform_under_between_rafts_is_open_soup() {
    let def = lane(4);
    let xs = [-200.0, 200.0];
    assert!(platform_under(0.0, &def, &xs, ChaosMode::Normal, 1, 0.0).is_none());
    assert!(platform_under(-200.0, &def, &xs, ChaosMode::Normal, 1, 0.0).is_some());
}

#[test]
fn test_ride_drift_accumulates_and_sweeps_off_the_edge() {
    let def = lane(4);
    let velocity = lane_velocity(&def, ChaosMode::Normal, 1);
    let delta_time = 1.0 / 60.0;
    let mut chicken_x = 300.0;
    for _ in 0..600 {
        chicken_x += velocity * delta_time;
        if swept_off(chicken_x) {
            return; // carried past the board edge: the Swept death fires
        }
    }
    panic!("a chicken riding right at {velocity} px/s must eventually sweep off");
}

#[test]
fn test_swept_off_only_beyond_the_board_edge() {
    assert!(!swept_off(0.0));
    assert!(!swept_off(WIN_W / 2.0 - 1.0));
    assert!(swept_off(WIN_W / 2.0 + 1.0));
    assert!(swept_off(-WIN_W / 2.0 - 1.0));
}

#[test]
fn test_road_hit_uses_the_chicken_and_segment_extents() {
    let def = lane(8);
    let hit_range = CHICKEN_HALF + def.half_len();
    assert!(road_hit(0.0, &def, &[hit_range - 1.0]));
    assert!(!road_hit(0.0, &def, &[hit_range + 1.0]));
}

// --- The measured extents ---

#[test]
fn test_every_segment_extent_is_the_measured_art() {
    // Each number is the segment's drawn length, halved: the first sprite's
    // opaque left to the last sprite's opaque right, off the synced PNGs.
    let expected: [(u32, f32); 10] = [
        (11, 27.5), // one sushi cart: 55 px of art in a 64 px cell
        (10, 27.5), // one donut cart: 55
        (9, 43.5),  // one hot dog cart: 87 in a 96 px cell
        (8, 27.5),
        (7, 43.5),
        (5, 69.0),  // three crackers: 3 x 48 px cells, 138 px of art
        (4, 62.0),  // two celery rafts: 2 x 64 px cells, 124 px of art
        (3, 92.5),  // two baguette rafts: 2 x 96 px cells, 185 px of art
        (2, 45.0),  // two crackers: 2 x 48 px cells, 90 px of art
        (1, 62.0),
    ];
    for (row, half_len) in expected {
        assert_eq!(lane(row).half_len(), half_len, "row {row}");
    }
}

#[test]
fn test_the_hitbox_is_the_narrower_body_of_both_chickens() {
    // One number for both players and every facing: a box that grew when the
    // chicken turned would punish turning, and two boxes in one co-op game is
    // unfair.
    assert_eq!(CHICKEN_HALF, 11.5);
    // Half the tall chicken's 23 px head-on body, so nothing is drawn narrower
    // than the box it collides with. The round chicken's 29 px body overhangs
    // it 3 px a side head-on — the overlap the manual pass judges, and one
    // that can only let a player off.
    const ROUND_HEAD_ON_WIDTH: f32 = 29.0;
    assert_eq!(ROUND_HEAD_ON_WIDTH / 2.0 - CHICKEN_HALF, 3.0);
    // Both sheets are wider than the box in every pose they draw.
    for spec in [&CHICKEN_ROUND, &CHICKEN_TALL] {
        assert!(opaque_size(spec).x > CHICKEN_HALF * 2.0, "{}", spec.path);
    }
}

#[test]
fn test_no_lane_s_segments_touch_the_next() {
    // Segments are evenly spaced round the torus, so a lane whose segment is
    // longer than its spacing spawns overlapping traffic.
    for def in LANES {
        let spacing = LANE_PERIOD / def.count as f32;
        let length = 2.0 * def.half_len();
        assert!(length < spacing, "row {} segments overlap at spawn", def.row);
    }
}

// --- The crackers sinking (Ridiculous family only) ---

#[test]
fn test_crackers_never_sink_outside_the_ridiculous_family() {
    for step in 0..200 {
        let time = step as f32 * 0.1;
        assert!(!crackers_submerged(ChaosMode::Normal, 5, 0, time));
        assert!(!crackers_submerged(ChaosMode::Insane, 5, 0, time));
    }
}

#[test]
fn test_ridiculous_crackers_sink_for_the_configured_window() {
    let mut submerged_time = 0.0;
    let delta_time = 0.01;
    let steps = (DIVE_PERIOD / delta_time) as usize;
    for step in 0..steps {
        if crackers_submerged(ChaosMode::Ridiculous, 5, 0, step as f32 * delta_time) {
            submerged_time += delta_time;
        }
    }
    assert!(
        (submerged_time - DIVE_DOWN_SECS).abs() < 0.1,
        "submerged {submerged_time}s of each {DIVE_PERIOD}s cycle, wanted {DIVE_DOWN_SECS}"
    );
}

#[test]
fn test_submerged_crackers_are_not_platforms() {
    let def = lane(5);
    let xs = [0.0];
    // Find one submerged and one surfaced instant of the cycle.
    let mut surfaced_at = None;
    let mut submerged_at = None;
    for step in 0..500 {
        let time = step as f32 * 0.02;
        if crackers_submerged(ChaosMode::Ridiculous, def.row, 0, time) {
            submerged_at.get_or_insert(time);
        } else {
            surfaced_at.get_or_insert(time);
        }
    }
    let (surfaced, submerged) = (surfaced_at.unwrap(), submerged_at.unwrap());
    assert!(platform_under(0.0, &def, &xs, ChaosMode::Ridiculous, 1, surfaced).is_some());
    assert!(platform_under(0.0, &def, &xs, ChaosMode::Ridiculous, 1, submerged).is_none());
    // The same instant on Normal keeps the platform (nothing sinks).
    assert!(platform_under(0.0, &def, &xs, ChaosMode::Normal, 1, submerged).is_some());
}

// --- The bun's duty cycle (every round must stay winnable) ---

#[test]
fn test_bun_only_in_the_ridiculous_family_and_nest_in_range() {
    assert_eq!(bun_nest(ChaosMode::Normal, 1), None);
    assert_eq!(bun_nest(ChaosMode::Insane, 1), None);
    for round in 1..50 {
        for mode in [ChaosMode::Ridiculous, ChaosMode::Insiculous] {
            let nest = bun_nest(mode, round).expect("the ridiculous family has a bun");
            assert!(nest < 5);
        }
        // Deterministic per round.
        assert_eq!(bun_nest(ChaosMode::Ridiculous, round), bun_nest(ChaosMode::Ridiculous, round));
    }
}

#[test]
fn test_bun_duty_cycle_has_absent_and_open_phases() {
    for round in 1..10 {
        let cycle = BUN_ABSENT_SECS + BUN_PRESENT_SECS;
        let mut absent = 0.0;
        let mut open = 0.0;
        let delta_time = 0.01;
        let steps = (cycle / delta_time) as usize;
        for step in 0..steps {
            if bun_open(ChaosMode::Ridiculous, round, step as f32 * delta_time) {
                open += delta_time;
            } else {
                absent += delta_time;
            }
        }
        assert!((absent - BUN_ABSENT_SECS).abs() < 0.1, "round {round}: absent {absent}");
        assert!((open - BUN_PRESENT_SECS).abs() < 0.1, "round {round}: open {open}");
    }
}

#[test]
fn test_ridiculous_round_is_winnable_every_nest_fillable() {
    // The winnability lock: for any round, every nest must have some moment
    // where entering it fills (the bun away, or guarding another nest).
    for round in 1..20 {
        for (slot, &col) in HOME_COLS.iter().enumerate() {
            let homes = [false; 5];
            let x = crate::board::tile_center(col, HOME_ROW).x;
            let fillable = (0..1000).any(|step| {
                let time = step as f32 * 0.05;
                home_slot_at(x, &homes, ChaosMode::Ridiculous, round, time)
                    == SlotOutcome::Fill(slot)
            });
            assert!(fillable, "round {round} nest {slot} can never be filled");
        }
    }
}

// --- Home row resolution ---

#[test]
fn test_home_slot_outcomes() {
    let mut homes = [false; 5];
    let nest_x = crate::board::tile_center(HOME_COLS[1], HOME_ROW).x;
    // An open nest fills (Normal: no bun at all).
    assert_eq!(home_slot_at(nest_x, &homes, ChaosMode::Normal, 1, 0.0), SlotOutcome::Fill(1));
    // A filled nest is a miss.
    homes[1] = true;
    assert_eq!(home_slot_at(nest_x, &homes, ChaosMode::Normal, 1, 0.0), SlotOutcome::Miss);
    // The coop wall between nests is a miss.
    let wall_x = crate::board::tile_center(2, HOME_ROW).x;
    assert_eq!(home_slot_at(wall_x, &homes, ChaosMode::Normal, 1, 0.0), SlotOutcome::Miss);
}

#[test]
fn test_bun_nest_kills_only_while_open() {
    let homes = [false; 5];
    for round in 1..10 {
        let nest = bun_nest(ChaosMode::Ridiculous, round).unwrap();
        let x = crate::board::tile_center(HOME_COLS[nest], HOME_ROW).x;
        let mut saw_bun = false;
        let mut saw_fill = false;
        for step in 0..1000 {
            let time = step as f32 * 0.05;
            match home_slot_at(x, &homes, ChaosMode::Ridiculous, round, time) {
                SlotOutcome::Bun => {
                    saw_bun = true;
                    assert!(bun_open(ChaosMode::Ridiculous, round, time));
                }
                SlotOutcome::Fill(slot) => {
                    saw_fill = true;
                    assert_eq!(slot, nest);
                    assert!(!bun_open(ChaosMode::Ridiculous, round, time));
                }
                SlotOutcome::Miss => panic!("an open bun nest never resolves to Miss"),
            }
        }
        assert!(saw_bun && saw_fill, "round {round}: the bun's nest must both kill and fill");
    }
}

#[test]
fn test_nearest_col_snaps_and_clamps() {
    assert_eq!(nearest_col(0.0), 7);
    assert_eq!(nearest_col(crate::board::tile_center(3, 0).x + 10.0), 3);
    assert_eq!(nearest_col(-10_000.0), 0);
    assert_eq!(nearest_col(10_000.0), COLS - 1);
}

// --- Hop resolution (one hop per frame, fixed priority) ---

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
    assert_eq!(apply_hop(0.0, 6, Hop::Up), (0.0, 5));
    assert_eq!(apply_hop(0.0, 0, Hop::Up), (0.0, 0), "the home row is the top");
    assert_eq!(apply_hop(0.0, 12, Hop::Down), (0.0, 12), "the start row is the bottom");
    assert_eq!(apply_hop(0.0, 6, Hop::Left), (-TILE, 6));
    assert_eq!(apply_hop(0.0, 6, Hop::Right), (TILE, 6));
    let edge = (COLS as f32 - 1.0) / 2.0 * TILE;
    assert_eq!(apply_hop(-edge, 6, Hop::Left), (-edge, 6));
    assert_eq!(apply_hop(edge, 6, Hop::Right), (edge, 6));
}

#[test]
fn test_every_hop_turns_the_chicken_that_way() {
    assert_eq!(Hop::Up.facing(), Facing::North);
    assert_eq!(Hop::Down.facing(), Facing::South);
    assert_eq!(Hop::Left.facing(), Facing::West);
    assert_eq!(Hop::Right.facing(), Facing::East);
}

// --- Chaos scaling ---

#[test]
fn test_speed_mult_chaos_and_round_ramp() {
    assert_eq!(speed_mult(ChaosMode::Normal, 1), 1.0);
    assert!((speed_mult(ChaosMode::Insane, 1) - INSANE_SPEED_MULT).abs() < 1e-6);
    // Ramps per round and caps out.
    assert!(speed_mult(ChaosMode::Normal, 2) > speed_mult(ChaosMode::Normal, 1));
    assert_eq!(speed_mult(ChaosMode::Normal, 1000), ROUND_SPEED_MULT_MAX);
    assert_eq!(
        speed_mult(ChaosMode::Insiculous, 1000),
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
    let mut rows: Vec<u32> = LANES.iter().map(|def| def.row).collect();
    rows.sort_unstable();
    let mut expected: Vec<u32> = (FIRST_WATER_ROW..=LAST_WATER_ROW)
        .chain(FIRST_ROAD_ROW..=LAST_ROAD_ROW)
        .collect();
    expected.sort_unstable();
    assert_eq!(rows, expected, "exactly one lane per soup/belt row");
}

#[test]
fn test_lane_kinds_match_their_rows() {
    for def in LANES {
        let soup_kind = matches!(def.kind, LaneKind::Log | LaneKind::Crackers);
        assert_eq!(is_water_row(def.row), soup_kind, "row {} carries {:?}", def.row, def.kind);
        assert!(def.dir == 1.0 || def.dir == -1.0);
        assert!(def.count > 0 && def.speed > 0.0 && def.segment.sprites > 0);
        // A cart lane is one sprite; a raft or a group is more than one.
        match def.kind {
            LaneKind::Car | LaneKind::Truck => assert_eq!(def.segment.sprites, 1),
            LaneKind::Log | LaneKind::Crackers => assert!(def.segment.sprites > 1),
        }
    }
}

// --- Match-level state (no GPU, no context) ---

#[test]
fn test_coop_chickens_spawn_on_distinct_columns() {
    let game = playing_game(GameMode::TwoPlayerCoop, ChaosMode::Normal);
    assert_eq!(game.chickens.len(), 2);
    assert_ne!(game.chickens[0].x, game.chickens[1].x);
    let solo = playing_game(GameMode::SinglePlayer, ChaosMode::Normal);
    assert_eq!(solo.chickens.len(), 1);
    assert_eq!(solo.chickens[0].x, crate::board::tile_center(SOLO_START_COL, 0).x);
}

#[test]
fn test_chicken_active_state_transitions() {
    let mut chicken = ChickenState::new(7, TIMER_NORMAL);
    assert!(chicken.active());
    chicken.respawn_timer = RESPAWN_DELAY;
    assert!(!chicken.active());
    chicken.respawn_timer = 0.0;
    chicken.retired = true;
    assert!(!chicken.active());
}

#[test]
fn test_a_fresh_match_owns_nothing_yet() {
    // Every per-match entity has a named owner field, and a game that has not
    // started holds none of them — the state `clear_match_entities` restores.
    let game = FroggerGame::default();
    assert!(game.board_maps.is_empty());
    assert_eq!(game.nests, [None; 5]);
    assert_eq!(game.seated, [None; 5]);
    assert!(game.bun.is_none());
    assert!(game.life_icons.is_empty());
    assert!(game.backdrop.is_none());
    assert!(game.transient_visuals.is_empty());
    assert!(game.lanes.is_empty());
    assert!(game.chickens.is_empty());
    assert!(game.round_clear_in.is_none());
}

#[test]
fn test_a_round_clear_stands_every_live_chicken_back_alive() {
    // A partner who was dead and waiting when the round cleared must not sit
    // out the start of the next one: the round's end wakes them with the rest.
    let mut game = playing_game(GameMode::TwoPlayerCoop, ChaosMode::Normal);
    let timer = attempt_timer(ChaosMode::Normal);

    game.chickens[0].respawn_timer = RESPAWN_DELAY;
    game.chickens[0].row = 4;
    game.chickens[0].x = -123.0;
    game.chickens[0].facing = Facing::East;
    game.chickens[1].retired = true;
    game.chickens[1].row = 3;

    for index in 0..game.chickens.len() {
        game.stand_chicken_at_start(index, timer, Revive::Waiting);
    }

    let first = &game.chickens[0];
    assert_eq!(first.respawn_timer, 0.0, "out of the respawn wait");
    assert!(first.active(), "alive and taking input again");
    assert_eq!(first.row, START_ROW);
    assert_eq!(first.x, crate::board::tile_center(COOP_START_COLS[0], START_ROW).x);
    assert_eq!(first.facing, Facing::START, "facing north");
    assert_eq!(first.timer, timer, "a full attempt timer");
    assert_eq!(first.furthest_row, START_ROW);

    let partner = &game.chickens[1];
    assert!(partner.retired, "a retired chicken stays retired");
    assert_eq!(partner.row, 3, "and is left exactly where it was");
}

#[test]
fn test_the_beats_opening_leaves_a_waiting_partner_gone_until_the_round_ends() {
    // A partner who died just before the fifth fill stays gone through the
    // celebration — its poof plays out — and the round's end is what revives
    // it, so the two never coexist on screen.
    let mut game = playing_game(GameMode::TwoPlayerCoop, ChaosMode::Normal);
    let timer = attempt_timer(ChaosMode::Normal);
    game.chickens[1].respawn_timer = RESPAWN_DELAY;
    game.chickens[1].row = 5;

    assert!(!game.stand_chicken_at_start(1, timer, Revive::Never), "the beat's opening skips it");
    let waiting = &game.chickens[1];
    assert_eq!(waiting.respawn_timer, RESPAWN_DELAY, "still waiting");
    assert_eq!(waiting.row, 5, "and left where it died");

    assert!(game.stand_chicken_at_start(1, timer, Revive::Waiting), "the round's end revives it");
    let revived = &game.chickens[1];
    assert_eq!(revived.respawn_timer, 0.0);
    assert!(revived.active());
    assert_eq!(revived.row, START_ROW);
}
