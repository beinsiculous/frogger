//! Headless tests for match flow, driven through the engine's harness: the
//! real frame over the real synced art, with no window. Where
//! `gameplay_tests.rs` checks the pure rules with no context at all, these
//! run `FroggerGame` as a player does — keys into a frame, the game's own
//! entry points through a lent context — and read the world back.

use engine_core::prelude::*;
use engine_core::test_support::{GameHarness, InputEvent};

use crate::achievements;
use crate::board::tile_center;
use crate::constants::*;
use crate::gameplay::rules::{lane_velocity, road_hit, wrap_lane_x, wrapped_dx};
use crate::test_support::{harness, start_match, state_of, title_harness, TOLERANCE};
use crate::types::*;

const FRAME: f32 = 1.0 / 60.0;

/// The frames the beat lasts at `FRAME`, from the constant that sets it.
/// Rounded, not truncated: `FRAME` is the f32 just above a sixtieth, so the
/// quotient sits just under sixty.
const BEAT_FRAMES: usize = (ROUND_CLEAR_BEAT / FRAME).round() as usize;

/// The most frames the beat may take to run out: `BEAT_FRAMES`, with room
/// for the residue f32 subtraction can leave after them.
const BEAT_STEP_CAP: usize = 2 * BEAT_FRAMES;

/// Player one fills nest `slot` through the game's own entry point; returns
/// what the fill paid.
fn fill_nest(harness: &mut GameHarness<FroggerGame>, slot: usize) -> u32 {
    let before = harness.game().score;
    harness.context(|game, ctx| game.fill_home(ctx, 0, slot));
    harness.game().score - before
}

/// Fill all five nests, the fifth arming the beat.
fn arm_beat(harness: &mut GameHarness<FroggerGame>) {
    for slot in 0..5 {
        fill_nest(harness, slot);
    }
}

/// Step with no input until the beat has run out, or the cap; returns how
/// many steps ran.
fn step_until_beat_clears(harness: &mut GameHarness<FroggerGame>) -> usize {
    let mut steps = 0;
    while harness.game().round_clear_in.is_some() && steps < BEAT_STEP_CAP {
        harness.step(FRAME, &[]);
        steps += 1;
    }
    steps
}

fn lane_on_row(game: &FroggerGame, row: u32) -> &LaneState {
    game.lanes
        .iter()
        .find(|lane| lane.def.row == row)
        .unwrap_or_else(|| panic!("no lane on row {row}"))
}

fn lane_on_row_mut(game: &mut FroggerGame, row: u32) -> &mut LaneState {
    game.lanes
        .iter_mut()
        .find(|lane| lane.def.row == row)
        .unwrap_or_else(|| panic!("no lane on row {row}"))
}

fn round_clear_unlocked(harness: &GameHarness<FroggerGame>) -> bool {
    harness.achievements().is_unlocked(achievements::ROUND_CLEAR)
}

#[test]
fn fifth_fill_pays_the_round_and_arms_the_beat_with_the_nests_filled() {
    let mut harness = harness(GameMode::SinglePlayer, ChaosMode::Normal);
    let round = harness.game().round;

    let mut fourth_delta = 0;
    for slot in 0..4 {
        fourth_delta = fill_nest(&mut harness, slot);
    }
    let fifth_delta = fill_nest(&mut harness, 4);

    assert_eq!(
        fifth_delta,
        fourth_delta + SCORE_ROUND_CLEAR,
        "the fifth fill pays what the fourth did plus the round's reward, once"
    );
    assert!(round_clear_unlocked(&harness), "the fifth fill unlocks the round clear");
    let game = harness.game();
    assert_eq!(
        game.round_clear_in,
        Some(ROUND_CLEAR_BEAT),
        "the fifth fill arms the full beat"
    );
    assert!(game.homes.iter().all(|&home| home), "the fifth fill leaves all five nests filled");
    assert_eq!(game.round, round, "the round advances when the beat ends, not at the fill");
}

#[test]
fn hop_input_does_nothing_and_lanes_advance_during_the_beat() {
    let mut harness = harness(GameMode::SinglePlayer, ChaosMode::Normal);
    arm_beat(&mut harness);
    let game = harness.game();
    let (row_before, x_before) = (game.chickens[0].row, game.chickens[0].x);
    let lane_before = lane_on_row(game, 11);
    let velocity = lane_velocity(&lane_before.def, game.chaos_mode, game.round);
    let expected_x = wrap_lane_x(lane_before.xs[0] + velocity * FRAME);

    harness.step(FRAME, &[InputEvent::KeyPressed(KeyCode::KeyW)]);

    let game = harness.game();
    assert!(game.round_clear_in.is_some(), "the step ran inside the beat");
    assert!(
        game.chickens[0].row == row_before && game.chickens[0].x == x_before,
        "an Up during the beat moves no chicken: row {} to {}, x {x_before} to {}",
        row_before,
        game.chickens[0].row,
        game.chickens[0].x
    );
    let actual_x = lane_on_row(game, 11).xs[0];
    assert!(
        wrapped_dx(actual_x, expected_x).abs() < TOLERANCE,
        "the lanes run on through the beat: segment at {actual_x}, expected {expected_x}"
    );
}

#[test]
fn the_beat_ends_with_one_reset_and_no_double_pay() {
    let mut harness = harness(GameMode::SinglePlayer, ChaosMode::Normal);
    arm_beat(&mut harness);
    let game = harness.game();
    let (score, round, nests) = (game.score, game.round, game.nests);
    let seated: Vec<EntityId> = game
        .seated
        .iter()
        .map(|seat| seat.expect("every filled nest seats a chicken"))
        .collect();

    let steps = step_until_beat_clears(&mut harness);

    assert!(
        (BEAT_FRAMES..BEAT_STEP_CAP).contains(&steps),
        "the beat runs its length and then ends: it took {steps} frames, {BEAT_FRAMES} expected"
    );
    let game = harness.game();
    assert!(game.homes.iter().all(|&home| !home), "the beat's end empties every nest");
    assert_eq!(game.nests, nests, "the nests are reset in place, not respawned");
    for nest in nests.iter().map(|nest| nest.expect("a match has five nests")) {
        assert_eq!(state_of(harness.world(), nest), NEST_EMPTY, "every nest's machine returns to empty");
    }
    assert!(game.seated.iter().all(Option::is_none), "no nest keeps a seat");
    let entities = harness.world().entities();
    assert!(
        seated.iter().all(|seat| !entities.contains(seat)),
        "the seated chickens leave the world with the round"
    );
    assert_eq!(game.round, round + 1, "the beat's end advances the round once");
    assert_eq!(game.score, score, "the beat's end pays nothing more");
}

#[test]
fn restart_and_quit_inside_the_beat_keep_unlocks_and_leave_no_entity() {
    let mut harness = title_harness();
    let title_count = harness.world().entity_count();
    start_match(&mut harness, GameMode::SinglePlayer, ChaosMode::Normal);
    let fresh_match_count = harness.world().entity_count();

    arm_beat(&mut harness);
    harness.context(|game, ctx| game.start_game(ctx));
    assert!(round_clear_unlocked(&harness), "a Restart inside the beat keeps the round clear");
    assert_eq!(harness.game().round_clear_in, None, "a Restart inside the beat ends it");
    assert_eq!(
        harness.world().entity_count(),
        fresh_match_count,
        "a Restart inside the beat leaves exactly a fresh match's entities"
    );

    arm_beat(&mut harness);
    harness.context(|game, ctx| game.reset_to_title(ctx));
    assert_eq!(
        harness.world().entity_count(),
        title_count,
        "a Quit inside the beat leaves exactly the title screen's entities"
    );
    assert!(round_clear_unlocked(&harness), "a Quit inside the beat keeps the round clear");
}

#[test]
fn the_fill_that_arms_the_beat_ends_the_frame_before_the_partner_hops() {
    let mut harness = harness(GameMode::TwoPlayerCoop, ChaosMode::Normal);
    let game = harness.game_mut();
    game.homes = [true, true, true, true, false];
    // One hop below the open nest, riding a raft: open soup would drown
    // player one before the hop runs.
    let player_one_x = tile_center(HOME_COLS[4], 0).x;
    let player_one = &mut game.chickens[0];
    player_one.row = 1;
    player_one.x = player_one_x;
    player_one.facing = Facing::North;
    lane_on_row_mut(game, 1).xs[0] = player_one_x;
    // A cart on the tile player two would land on, so a hop that slipped
    // through would be a death, not a quiet step.
    let player_two_x = tile_center(COOP_START_COLS[1], 0).x;
    let player_two = &mut game.chickens[1];
    player_two.row = START_ROW;
    player_two.x = player_two_x;
    lane_on_row_mut(game, 11).xs[0] = player_two_x;

    harness.step(
        FRAME,
        &[InputEvent::KeyPressed(KeyCode::KeyW), InputEvent::KeyPressed(KeyCode::ArrowUp)],
    );

    let game = harness.game();
    assert_eq!(
        game.round_clear_in,
        Some(ROUND_CLEAR_BEAT),
        "player one's hop filled the fifth nest and armed the beat this frame"
    );
    assert!(game.homes.iter().all(|&home| home), "all five nests are filled");
    let lane = lane_on_row(game, 11);
    assert!(
        road_hit(game.chickens[1].x, &lane.def, &lane.xs),
        "the stage is lethal: player two's hop would land under a cart"
    );
    assert!(
        game.chickens[1].row == START_ROW && game.chickens[1].active(),
        "player two's Up in the arming frame does not hop it: row {}",
        game.chickens[1].row
    );
    for (index, chicken) in game.chickens.iter().enumerate().filter(|(_, chicken)| chicken.active()) {
        let start_x = tile_center(chicken.start_col, 0).x;
        assert!(
            chicken.row == START_ROW && (chicken.x - start_x).abs() < TOLERANCE,
            "the beat strands no chicken in traffic: chicken {index} at row {}, x {}",
            chicken.row,
            chicken.x
        );
    }

    harness.step(FRAME, &[InputEvent::KeyReleased(KeyCode::ArrowUp)]);
    harness.step(FRAME, &[InputEvent::KeyPressed(KeyCode::ArrowUp)]);
    assert_eq!(
        harness.game().chickens[1].row,
        START_ROW,
        "a fresh Up during the beat does not hop player two either"
    );
}

#[test]
fn a_partner_waiting_to_respawn_stands_up_when_the_beat_ends() {
    let mut harness = harness(GameMode::TwoPlayerCoop, ChaosMode::Normal);
    let game = harness.game_mut();
    game.chickens[1].respawn_timer = RESPAWN_DELAY;
    game.chickens[1].retired = false;
    game.homes = [true, true, true, true, false];
    // The fifth fill comes from player one's hop inside a real frame, riding
    // the row-1 raft under the open nest: the respawn wait must not tick in
    // the frame the fill arms the beat, and that frame's respawn step runs
    // after the hop.
    let player_one_x = tile_center(HOME_COLS[4], 0).x;
    let player_one = &mut game.chickens[0];
    player_one.row = 1;
    player_one.x = player_one_x;
    player_one.facing = Facing::North;
    lane_on_row_mut(game, 1).xs[0] = player_one_x;

    harness.step(FRAME, &[InputEvent::KeyPressed(KeyCode::KeyW)]);
    let game = harness.game();
    assert_eq!(game.round_clear_in, Some(ROUND_CLEAR_BEAT), "player one's hop armed the beat this frame");
    assert!(
        game.chickens[1].respawn_timer == RESPAWN_DELAY,
        "the respawn wait does not count down in the frame the beat is armed: {}",
        game.chickens[1].respawn_timer
    );

    harness.steps(5, FRAME);
    let partner = &harness.game().chickens[1];
    assert!(!partner.active(), "a partner waiting to respawn stays gone during the beat");
    assert!(
        partner.respawn_timer == RESPAWN_DELAY,
        "the respawn wait does not count down during the beat: {}",
        partner.respawn_timer
    );

    let steps = step_until_beat_clears(&mut harness);
    assert!(steps < BEAT_STEP_CAP, "the beat ends: still armed after {steps} frames");
    let partner = &harness.game().chickens[1];
    let start_x = tile_center(COOP_START_COLS[1], 0).x;
    assert!(
        partner.active() && partner.row == START_ROW && (partner.x - start_x).abs() < TOLERANCE,
        "the beat's end stands the partner up on its start tile: active {}, row {}, x {}",
        partner.active(),
        partner.row,
        partner.x
    );
}
