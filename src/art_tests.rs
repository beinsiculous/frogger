//! Headless checks of the game's own tables against the committed art:
//! every sheet's measured extents, every pose window against its synced
//! sidecar, the one writer of a chicken's pose, and the fact that a spawned
//! entity is already posed. No GPU, no window, no physics, no context.

use engine_core::prelude::*;

use crate::constants::*;
use crate::gameplay::pose_chicken;
use crate::gameplay::poses::*;
use crate::gameplay::rules::*;
use crate::spawning::{
    composition_position, place_on_start_tile, spawn_bun, spawn_lane, spawn_player, spawn_seated,
};
use crate::test_support::{
    clip_length, clip_of, declared, headless_sheets, lane, sidecar, state_of, TOLERANCE,
};
use crate::types::*;

// --- The drawn danger's poses ---


#[test]
fn test_every_pose_names_a_clip_its_own_sheet_has() {
    for (spec, clip) in [
        (&CRACKER, CRACKER_IDLE),
        (&CRACKER, CRACKER_WARNING),
        (&CRACKER, CRACKER_SINK),
        (&CRACKER, CRACKER_SUBMERGED),
        (&CRACKER, CRACKER_RISE),
        (&BUN, BUN_WARNING),
        (&BUN, BUN_OPENING),
        (&BUN, BUN_OPEN),
        (&BUN, BUN_CLOSING),
    ] {
        let (frames, _) = declared(spec, clip);
        assert!(frames > 0, "{}: '{clip}' has no frames", spec.path);
    }
}

#[test]
fn test_every_pose_window_is_no_shorter_than_the_clip_it_shows() {
    // The windows are typed constants and cannot see a loaded sheet, so each
    // is checked against the art: a retime fails loudly here rather than
    // silently cutting a lead-in short.
    let cases: [(&SheetSpec, &str, f32); 7] = [
        (&CRACKER, CRACKER_RISE, CRACKER_RISE_SECS),
        (&CRACKER, CRACKER_IDLE, CRACKER_IDLE_SECS),
        (&CRACKER, CRACKER_WARNING, CRACKER_WARNING_SECS),
        (&CRACKER, CRACKER_SINK, CRACKER_SINK_SECS),
        (&BUN, BUN_WARNING, BUN_WARNING_SECS),
        (&BUN, BUN_OPENING, BUN_OPENING_SECS),
        (&BUN, BUN_CLOSING, BUN_CLOSING_SECS),
    ];
    for (spec, clip, window) in cases {
        let length = clip_length(spec, clip);
        assert!(
            window + TOLERANCE >= length,
            "{}: '{clip}' is {length}s of art in a {window}s window",
            spec.path
        );
    }
}

#[test]
fn test_the_round_clear_beat_outlasts_the_fifth_celebration() {
    // A retime cannot cut the fifth arrival's celebration short.
    for (spec, clip) in [(&NEST, NEST_ARRIVAL), (&ARRIVAL_BURST, CELEBRATE_CLIP)] {
        let length = clip_length(spec, clip);
        assert!(
            ROUND_CLEAR_BEAT + TOLERANCE >= length,
            "{}: '{clip}' is {length}s in a {ROUND_CLEAR_BEAT}s beat",
            spec.path
        );
    }
}

#[test]
fn test_the_pose_windows_tile_their_cycle_up_to_the_flip() {
    // The lead-in ends at the flip and the lead-out begins at it, so the four
    // windows of a subject must add up to exactly where the rules put it.
    let cracker: f32 =
        CRACKER_RISE_SECS + CRACKER_IDLE_SECS + CRACKER_WARNING_SECS + CRACKER_SINK_SECS;
    assert!((cracker - DIVE_FLIP_SECS).abs() < 1e-4, "cracker windows reach {cracker}");
    let bun: f32 = BUN_CLOSING_SECS + BUN_HIDDEN_SECS + BUN_WARNING_SECS + BUN_OPENING_SECS;
    assert!((bun - BUN_ABSENT_SECS).abs() < 1e-4, "bun windows reach {bun}");
    // Each warning is the one 600 ms beat that precedes its lead-in.
    assert!((CRACKER_WARNING_SECS - 0.6).abs() < 1e-6);
    assert!((BUN_WARNING_SECS - 0.6).abs() < 1e-6);
}

/// The `play_time` at which a cracker's phased cycle reaches `cycle_time`.
fn cracker_play_time(lane_row: u32, index: usize, cycle_time: f32) -> f32 {
    let phase = cracker_cycle_time(lane_row, index, 0.0);
    (cycle_time - phase).rem_euclid(DIVE_PERIOD)
}

/// The `play_time` at which the bun's phased cycle reaches `cycle_time`.
fn bun_play_time(round: u32, cycle_time: f32) -> f32 {
    let cycle = BUN_ABSENT_SECS + BUN_PRESENT_SECS;
    let phase = bun_cycle_time(round, 0.0);
    (cycle_time - phase).rem_euclid(cycle)
}

#[test]
fn test_a_cracker_shows_sink_until_the_flip_and_rise_from_it() {
    let (row, index) = (5, 0);
    let sink_frames = declared(&CRACKER, CRACKER_SINK).0;
    let step = 0.001;

    // One step before the flip: the lead-in's last frame, and still rideable.
    let before = cracker_play_time(row, index, DIVE_FLIP_SECS - step);
    let pose = cracker_pose(ChaosMode::Ridiculous, row, index, before);
    assert_eq!(pose.clip, CRACKER_SINK);
    assert_eq!(pose.frame, sink_frames - 1, "the lead-in's last frame is the one before the flip");

    // At the exact flip the rules are strict, so the cracker is still sinking
    // — the safe direction, and the frame the player last saw.
    let at = cracker_play_time(row, index, DIVE_FLIP_SECS);
    assert_eq!(cracker_pose(ChaosMode::Ridiculous, row, index, at).clip, CRACKER_SINK);
    assert!(!crackers_submerged(ChaosMode::Ridiculous, row, index, at));

    // One step after: submerged, from the top of the clip.
    let after = cracker_play_time(row, index, DIVE_FLIP_SECS + step);
    let pose = cracker_pose(ChaosMode::Ridiculous, row, index, after);
    assert_eq!(pose.clip, CRACKER_SUBMERGED);
    assert_eq!(pose.frame, 0, "the lead-out's first frame is the frame of the flip");
    assert!(crackers_submerged(ChaosMode::Ridiculous, row, index, after));

    // The cycle's own start is that flip, so the rise leads out of it.
    let start = cracker_play_time(row, index, 0.0);
    assert_eq!(cracker_pose(ChaosMode::Ridiculous, row, index, start).clip, CRACKER_RISE);
    assert_eq!(cracker_pose(ChaosMode::Ridiculous, row, index, start).frame, 0);
}

#[test]
fn test_a_cracker_is_unrideable_exactly_while_it_shows_submerged() {
    // The drawn danger always covers the real one: the pose and the platform
    // predicate never disagree, all cycle long.
    let def = lane(5);
    let row = def.row;
    let steps = (DIVE_PERIOD / 0.005) as usize;
    for step in 0..steps {
        let time = step as f32 * 0.005;
        let pose = cracker_pose(ChaosMode::Ridiculous, row, 0, time);
        let rideable = platform_under(0.0, &def, &[0.0], ChaosMode::Ridiculous, 1, time).is_some();
        assert_eq!(
            pose.clip == CRACKER_SUBMERGED,
            !rideable,
            "at {time}s a cracker shows {} and is {}",
            pose.clip,
            if rideable { "rideable" } else { "not" }
        );
    }
}

#[test]
fn test_the_bun_shows_opening_until_the_flip_and_open_from_it() {
    let round = 3;
    let opening_frames = declared(&BUN, BUN_OPENING).0;
    let step = 0.001;

    // One step before the flip: the lead-in's last frame, and harmless.
    let before = bun_play_time(round, BUN_ABSENT_SECS - step);
    let pose = bun_pose(ChaosMode::Ridiculous, round, before, false).expect("it draws");
    assert_eq!(pose.clip, BUN_OPENING);
    assert_eq!(pose.frame, opening_frames - 1);
    assert!(!bun_open(ChaosMode::Ridiculous, round, before));

    // At the exact flip the rules are strict: still opening, still harmless.
    let at = bun_play_time(round, BUN_ABSENT_SECS);
    let pose = bun_pose(ChaosMode::Ridiculous, round, at, false).expect("it draws");
    assert_eq!(pose.clip, BUN_OPENING);
    assert!(!bun_open(ChaosMode::Ridiculous, round, at));

    // One step after: open, from the top, and lethal.
    let after = bun_play_time(round, BUN_ABSENT_SECS + step);
    let pose = bun_pose(ChaosMode::Ridiculous, round, after, false).expect("it draws");
    assert_eq!(pose.clip, BUN_OPEN);
    assert_eq!(pose.frame, 0);
    assert!(bun_open(ChaosMode::Ridiculous, round, after));

    // The cycle's own start is that flip, so the closing leads out of it.
    let start = bun_play_time(round, 0.0);
    let pose = bun_pose(ChaosMode::Ridiculous, round, start, false).expect("it draws");
    assert_eq!(pose.clip, BUN_CLOSING);
    assert_eq!(pose.frame, 0);
}

#[test]
fn test_the_bun_is_lethal_exactly_while_it_shows_open() {
    let round = 4;
    let cycle = BUN_ABSENT_SECS + BUN_PRESENT_SECS;
    let steps = (cycle / 0.005) as usize;
    for step in 0..steps {
        let time = step as f32 * 0.005;
        let open = bun_open(ChaosMode::Ridiculous, round, time);
        match bun_pose(ChaosMode::Ridiculous, round, time, false) {
            Some(pose) => assert_eq!(
                pose.clip == BUN_OPEN,
                open,
                "at {time}s the bun shows {} while open is {open}",
                pose.clip
            ),
            None => assert!(!open, "a bun away from its nest is never lethal"),
        }
    }
}

#[test]
fn test_a_filled_nest_draws_no_bun_in_any_window() {
    let round = 2;
    let cycle = BUN_ABSENT_SECS + BUN_PRESENT_SECS;
    let steps = (cycle / 0.01) as usize;
    for step in 0..steps {
        let time = step as f32 * 0.01;
        assert!(
            bun_pose(ChaosMode::Ridiculous, round, time, true).is_none(),
            "a filled nest drew a bun at {time}s"
        );
    }
}

#[test]
fn test_outside_the_ridiculous_family_both_stay_in_their_rest_pose() {
    for step in 0..200 {
        let time = step as f32 * 0.1;
        for mode in [ChaosMode::Normal, ChaosMode::Insane] {
            assert_eq!(cracker_pose(mode, 5, 0, time).clip, CRACKER_IDLE);
            assert!(bun_pose(mode, 1, time, false).is_none());
        }
    }
}

#[test]
fn test_a_rehashed_bun_cycle_lands_mid_clip_on_the_right_frame() {
    // A round change re-hashes the bun's phase. The pose is a function of the
    // cycle time, so the same instant of the cycle draws the same frame
    // whichever round reached it — never a lead-in restarted from the top.
    let cycle_time = BUN_ABSENT_SECS + 0.2;
    let open_frames = declared(&BUN, BUN_OPEN).0;
    for round in 1..8 {
        let time = bun_play_time(round, cycle_time);
        let pose = bun_pose(ChaosMode::Ridiculous, round, time, false).expect("it draws");
        assert_eq!(pose.clip, BUN_OPEN, "round {round}");
        assert_eq!(pose.frame, 1 % open_frames, "round {round} landed mid-clip");
    }
}

// --- One writer of the chicken's pose ---

/// Run every animation `seconds` forward at `delta_time` a step and let the
/// machines finish what that ends, exactly as the engine's frame tail does.
fn run(world: &mut World, seconds: f32, delta_time: f32) {
    let mut machines = ClipStateMachineSystem;
    let steps = (seconds / delta_time).round() as usize;
    for _ in 0..steps {
        for entity in world.entities() {
            if let Some(animation) = world.get_mut::<SpriteAnimation>(entity) {
                animation.update(delta_time);
            }
        }
        System::update(&mut machines, world, delta_time);
    }
}


#[test]
fn test_a_second_hop_restarts_the_jump_in_the_same_direction() {
    let mut world = World::new();
    let sheets = headless_sheets();
    let entity = spawn_player(&mut world, &sheets, 0);

    pose_chicken(&mut world, entity, Facing::North, true);
    assert_eq!(clip_of(&world, entity), Some(Facing::North.jump_state()));
    assert!(world.get::<SpriteAnimation>(entity).unwrap().playing);
    // The clip runs on for 200 ms — not far enough to finish a 400 ms jump.
    run(&mut world, 0.2, 0.01);
    assert!(world.get::<SpriteAnimation>(entity).unwrap().current_frame > 0, "the jump advanced");

    // The second hop starts it over, so three fast hops are three jumps.
    pose_chicken(&mut world, entity, Facing::North, true);
    let animation = world.get::<SpriteAnimation>(entity).unwrap();
    assert_eq!(animation.current_frame, 0, "the hop restarted the clip");
    assert_eq!(animation.current_clip.as_deref(), Some(Facing::North.jump_state().as_str()));

    // And it still settles back into the idle of that direction.
    run(&mut world, 0.5, 0.01);
    assert_eq!(state_of(&world, entity), Facing::North.idle_state());
    assert_eq!(clip_of(&world, entity), Some(Facing::North.idle_state()));
}

#[test]
fn test_a_turn_inside_the_jump_plays_the_new_direction() {
    let mut world = World::new();
    let sheets = headless_sheets();
    let entity = spawn_player(&mut world, &sheets, 0);

    pose_chicken(&mut world, entity, Facing::North, true);
    run(&mut world, 0.1, 0.01);
    assert_eq!(clip_of(&world, entity), Some(Facing::North.jump_state()));

    // A hop east mid-jump: the machine moves and the drawn clip is the new
    // direction's, not the one the machine would rather re-select.
    pose_chicken(&mut world, entity, Facing::East, true);
    assert_eq!(state_of(&world, entity), Facing::East.jump_state());
    assert_eq!(clip_of(&world, entity), Some(Facing::East.jump_state()));

    run(&mut world, 0.5, 0.01);
    assert_eq!(state_of(&world, entity), Facing::East.idle_state());
    assert_eq!(clip_of(&world, entity), Some(Facing::East.idle_state()));
}

#[test]
fn test_a_new_life_and_a_round_clear_both_face_north_in_their_idle() {
    // A respawn and a round clear both place the chicken back on its start
    // tile through the same writer, which is the only thing that sets a pose.
    let mut world = World::new();
    let sheets = headless_sheets();
    let entity = spawn_player(&mut world, &sheets, 0);

    for turn in [Facing::East, Facing::South, Facing::West] {
        pose_chicken(&mut world, entity, turn, true);
    }
    pose_chicken(&mut world, entity, Facing::START, false);
    assert_eq!(state_of(&world, entity), Facing::North.idle_state());
    assert_eq!(clip_of(&world, entity), Some(Facing::North.idle_state()));
    assert!(world.get::<SpriteAnimation>(entity).unwrap().playing);
}

#[test]
fn test_every_chicken_state_plays_a_clip_its_own_sheet_has() {
    // The table the machine declares, against the sidecar that shipped with
    // the PNG: a renamed clip in the art fails here rather than drawing a
    // frame of something else.
    let mut world = World::new();
    let sheets = headless_sheets();
    for player in 0..2 {
        let entity = spawn_player(&mut world, &sheets, player);
        let spec = if player == 0 { &CHICKEN_ROUND } else { &CHICKEN_TALL };
        let prepared = sidecar(spec);
        let clips: Vec<&str> = prepared.sheet.clips.iter().map(|(name, _)| name.as_str()).collect();
        let machine = world.get::<ClipStateMachine>(entity).expect("a machine");
        assert_eq!(machine.states().len(), 8, "two poses in each of four facings");
        for (state, row) in machine.states() {
            assert!(
                clips.contains(&row.clip.as_str()),
                "{}: state '{state}' plays '{}', which the sheet does not have ({clips:?})",
                spec.path,
                row.clip
            );
        }
    }
}

#[test]
fn test_a_spawned_lane_and_nest_are_owned_by_the_game() {
    let mut world = World::new();
    let sheets = headless_sheets();
    let def = lane(5);
    let state = spawn_lane(&mut world, &sheets, def, ChaosMode::Ridiculous, 0.0);
    // Every sprite of the group, and every ghost, is reachable from the lane.
    let owned: usize = state.sprites.iter().map(|pairs| pairs.len() * 2).sum();
    assert_eq!(owned, def.count * def.segment.sprites * 2);
    for pairs in &state.sprites {
        for &(main, ghost) in pairs {
            assert!(world.get::<Sprite>(main).is_some());
            assert!(world.get::<Sprite>(ghost).is_some());
        }
    }
    // A cracker lane spawns already posed: two of the three ways a match
    // starts never reach the per-frame sync.
    for pairs in &state.sprites {
        let animation = world.get::<SpriteAnimation>(pairs[0].0).unwrap();
        assert!(animation.current_clip.is_some(), "a spawned cracker is posed");
        assert!(!animation.playing, "a posed cracker plays no clip of its own");
    }
    let seated = spawn_seated(&mut world, &sheets, 0, 0);
    assert!(world.get::<SpriteAnimation>(seated).unwrap().current_clip.is_some());
}

#[test]
fn test_every_pose_frame_count_is_the_one_its_sidecar_declares() {
    // The poses are pure and cannot see a loaded sheet, so their frame counts
    // are typed. A count that is long for its clip asks for a frame the sheet
    // does not have, and the engine then leaves the previous clip's cell on
    // screen — a cracker drawn mid-sink while the rules have it submerged.
    let cases: [(&SheetSpec, &str, usize); 9] = [
        (&CRACKER, CRACKER_RISE, CRACKER_RISE_FRAMES),
        (&CRACKER, CRACKER_IDLE, CRACKER_IDLE_FRAMES),
        (&CRACKER, CRACKER_WARNING, CRACKER_WARNING_FRAMES),
        (&CRACKER, CRACKER_SINK, CRACKER_SINK_FRAMES),
        (&CRACKER, CRACKER_SUBMERGED, CRACKER_SUBMERGED_FRAMES),
        (&BUN, BUN_WARNING, BUN_WARNING_FRAMES),
        (&BUN, BUN_OPENING, BUN_OPENING_FRAMES),
        (&BUN, BUN_OPEN, BUN_OPEN_FRAMES),
        (&BUN, BUN_CLOSING, BUN_CLOSING_FRAMES),
    ];
    for (spec, clip, frames) in cases {
        assert_eq!(frames, declared(spec, clip).0, "{}: '{clip}'", spec.path);
    }
}

#[test]
fn test_a_spawned_bun_is_already_where_and_what_the_next_frame_would_be() {
    // Two of the three ways a match starts reach a rendered frame before the
    // per-frame sync runs, so a spawn places and poses the bun itself.
    let mode = ChaosMode::Ridiculous;
    // Any round whose bun does not happen to guard nest 0, so the spawn's
    // place is a real choice and not the old default.
    let round = (1..=1000)
        .find(|&round| bun_nest(mode, round) != Some(0))
        .expect("some round in the first thousand guards a nest other than 0");
    let homes = [false; 5];
    let nest = bun_nest(mode, round).expect("the ridiculous family has a bun");
    let guarded = composition_position(nest, &BUN);
    let mut world = World::new();
    let sheets = Sheets::default();

    // A spawn inside the hidden window: on its nest's straw, drawn by nothing.
    let hidden = bun_play_time(round, 1.0);
    let bun = spawn_bun(&mut world, &sheets, mode, round, hidden, &homes);
    assert_eq!(world.get::<Transform2D>(bun).unwrap().position, guarded);
    assert!(!world.get::<Sprite>(bun).unwrap().visible, "hidden away from its nest");
    assert!(
        world.get::<SpriteAnimation>(bun).unwrap().current_clip.is_none(),
        "a hidden bun shows no clip"
    );

    // A spawn inside the open window: visible, open, and on the same nest.
    let open = bun_play_time(round, BUN_ABSENT_SECS + 1.0);
    let bun = spawn_bun(&mut world, &sheets, mode, round, open, &homes);
    assert_eq!(world.get::<Transform2D>(bun).unwrap().position, guarded);
    assert!(world.get::<Sprite>(bun).unwrap().visible);
    let animation = world.get::<SpriteAnimation>(bun).unwrap();
    assert_eq!(animation.current_clip.as_deref(), Some(BUN_OPEN));
    assert!(!animation.playing, "a posed bun plays no clip of its own");

    // A nest that is already filled draws no bun, at any instant.
    let mut filled = [false; 5];
    filled[nest] = true;
    let bun = spawn_bun(&mut world, &sheets, mode, round, open, &filled);
    assert!(!world.get::<Sprite>(bun).unwrap().visible, "a filled nest draws no bun");

    // And outside the Ridiculous family there is never one to draw.
    let bun = spawn_bun(&mut world, &sheets, ChaosMode::Normal, round, open, &homes);
    assert!(!world.get::<Sprite>(bun).unwrap().visible);
    assert!(bun_nest(ChaosMode::Normal, round).is_none());
}

#[test]
fn test_a_chicken_stands_on_its_start_tile_the_moment_it_spawns() {
    // A spawn frame draws before any sync runs, so the spawn writes the
    // chicken's tile itself — otherwise the first frame of a match draws it at
    // the world origin, in the middle of the board, facing whatever cell zero is.
    let mut world = World::new();
    let sheets = headless_sheets();
    let entity = spawn_player(&mut world, &sheets, 0);
    assert_eq!(
        world.get::<Transform2D>(entity).unwrap().position,
        Vec2::ZERO,
        "an unplaced spawn sits at the origin"
    );

    place_on_start_tile(&mut world, entity, COOP_START_COLS[0]);

    let position = world.get::<Transform2D>(entity).unwrap().position;
    let col = COOP_START_COLS[0];
    assert_eq!(position.x, crate::board::tile_center(col, START_ROW).x);
    assert_eq!(position.y, crate::board::tile_center(0, START_ROW).y);
    assert!(world.get::<Sprite>(entity).unwrap().visible, "and visible");
    assert_eq!(state_of(&world, entity), Facing::North.idle_state(), "facing north");
}
