//! Chicken Coop's music through the real game and the engine's harness, whose audio manager
//! plays the committed song on its virtual clock: the sidecar, the bank and the score as
//! committed, the four moments and their sources, the order within a frame, the pause jam, the
//! grid's pulse, a match start deciding by the song in the slot, and a missing bank leaving the
//! game silent.

use std::path::PathBuf;
use std::time::Duration;

use engine_core::prelude::*;
use engine_core::test_support::{GameHarness, InputEvent};
use music_midi::rustysynth::SoundFont;
use music_midi::{BackgroundScore, InstrumentSource, MusicalUnit, PartMode, PositionSnapshot, Quantize, SongData, SongFile};

use crate::board::tile_center;
use crate::constants::{HOME_COLS, HOME_ROW, LANE_PERIOD, START_ROW, WIN_W};
use crate::gameplay::rules::{lane_velocity, LANES};
use crate::music::SONG_PATH;
use crate::test_support::*;
use crate::types::*;

/// What a trigger waits for before the conductor has drained it: far longer than it takes.
const DRAINED: Duration = Duration::from_secs(2);
/// Ticks in one bar of 2/4 at 480 a quarter.
const BAR_TICKS: u64 = 960;
/// The belt just above the start row.
const FIRST_BELT_ROW: u32 = START_ROW - 1;
/// The most frames a test steps waiting for the song's beats: wall time, not a hung suite.
const BEAT_FRAME_CAP: usize = 600;

fn committed_song() -> SongData {
    let bytes = std::fs::read(asset_base().join(SONG_PATH)).expect("the song is committed");
    SongData::from_smf(&bytes).expect("the song parses").0
}

fn committed_sidecar() -> SongFile {
    let text = std::fs::read_to_string(asset_base().join(format!("{SONG_PATH}.ron"))).expect("the sidecar");
    music_midi::parse_song_file("ai_chicken_coop.mid.ron", &text).expect("the sidecar parses")
}

fn units_of<'song>(song: &'song SongData, track: &str) -> &'song [MusicalUnit] {
    song.tracks().iter().find(|candidate| candidate.name() == Some(track)).expect(track).units()
}

fn conductor(harness: &GameHarness<FroggerGame>) -> PositionSnapshot {
    harness.audio().song_snapshot().expect("a song is in the slot").conductor
}

#[test]
fn the_committed_sidecar_binds_six_moments_on_five_unquantized_loop_parts() {
    let file = committed_sidecar();
    let binding = |action: &str| file.bindings.iter().find(|binding| binding.action == action).expect(action).clone();
    let mut parts: Vec<&str> = file.parts.iter().map(|part| part.name.as_str()).collect();
    parts.sort_unstable();
    assert_eq!(parts, ["clear", "fanfare", "galop", "jam", "squawk"], "the background tracks select no part");
    assert_eq!(file.bindings.len(), 6, "one death for every cause");
    for (action, instrument) in [("hop", "trumpet"), ("hop_two", "piccolo")] {
        assert_eq!((binding(action).part.as_str(), binding(action).instrument.as_str()), ("galop", instrument), "{action}");
    }
    for (action, part) in [("death", "squawk"), ("home", "fanfare"), ("clear", "clear"), ("menu", "jam")] {
        assert_eq!(binding(action).part, part, "{action}");
    }
    assert!(file.parts.iter().all(|part| part.mode == PartMode::Loop), "every part walks its units");
    assert!(file.parts.iter().all(|part| part.quantize == Quantize::Off), "no trigger is parked on the loop seam");
    assert!(file.provenance.is_some());
    assert_eq!(file.soundfont, "soundfonts/chicken_coop.sf2");
    let song = committed_song();
    for cue in ["squawk", "fanfare", "cadence"] {
        assert_eq!(units_of(&song, cue).len(), 1, "a cue is one note or one chord: Loop plays one unit a trigger ({cue})");
    }
}

/// Whether `key` sounds on `preset` of `bank`: a preset region and one of its instrument's
/// regions both cover it (a region without a key range covers every key).
fn bank_covers(bank: &SoundFont, preset: u8, key: u8) -> bool {
    let key = i32::from(key);
    let preset = bank.get_presets().iter()
        .find(|candidate| candidate.get_bank_number() == 0 && candidate.get_patch_number() == i32::from(preset))
        .expect("the preset is in the bank");
    preset.get_regions().iter()
        .filter(|region| region.get_key_range_start() <= key && key <= region.get_key_range_end())
        .any(|region| bank.get_instruments()[region.get_instrument_id()].get_regions().iter()
            .any(|inner| inner.get_key_range_start() <= key && key <= inner.get_key_range_end()))
}

#[test]
fn every_key_of_the_galop_sounds_on_the_trumpet_and_on_the_piccolo() {
    let file = committed_sidecar();
    let bytes = std::fs::read(asset_base().join(&file.soundfont)).expect("the bank is committed");
    let bank = SoundFont::new(&mut bytes.as_slice()).expect("the bank parses");
    let song = committed_song();
    let keys: Vec<u8> = units_of(&song, "galop").iter().flat_map(|unit| unit.notes.iter().map(|note| note.key)).collect();
    assert!(keys.iter().all(|key| (67..=79).contains(key)), "the galop sits in G4 to G5, the register both carry");
    for action in ["hop", "hop_two"] {
        let binding = file.bindings.iter().find(|binding| binding.action == action).expect(action);
        let instrument = file.instruments.iter().find(|instrument| instrument.name == binding.instrument).expect("bound");
        let InstrumentSource::SoundFont { preset, .. } = instrument.source else { panic!("{action} is a SoundFont preset") };
        for &key in &keys {
            assert!(bank_covers(&bank, preset, key), "key {key} of the galop is silent on {}'s preset {preset}", binding.instrument);
        }
    }
}

#[test]
fn the_song_is_forty_eight_bars_of_two_four_at_152_ending_on_the_tubas_held_note() {
    let song = committed_song();
    let meter = song.time_signatures().first().expect("a meter is written, not the 4/4 default");
    assert_eq!((meter.tick, meter.numerator, meter.denominator), (0, 2, 4), "else the pulse's bars land on every other beat");
    let loop_end = BackgroundScore::new(&song, &Default::default()).end_tick();
    assert_eq!(loop_end, 48 * BAR_TICKS, "the loop ends on bar 49's downbeat");
    let tempo = song.tempo_map();
    assert_eq!(tempo.microseconds_per_quarter_at(0), 394_737, "152 BPM from the first tick");
    assert!((tempo.seconds_at_tick(loop_end) - 37.894_752).abs() < 1e-6, "the loop is about 37.9 seconds");
    let tuba = units_of(&song, "bass").last().expect("a unit");
    let end = tuba.tick + tuba.notes.iter().map(|note| note.duration_ticks).max().unwrap_or(0);
    assert_eq!(end, loop_end, "the tuba's last note is held to the loop's end, a written note's end");
}

#[test]
fn the_song_plays_after_init_and_a_match_start_requests_it_into_an_empty_slot_or_takes_it_to_the_top() {
    let mut harness = title_harness();
    let own = format!("{}/{SONG_PATH}", asset_base().display());
    assert!(!harness.game().music.game.song_failed());
    assert_eq!(harness.audio().song_label(), Some(own.as_str()));
    let before = conductor(&harness).blocks_rendered;
    settle(&mut harness);
    assert!(conductor(&harness).blocks_rendered > before, "the virtual clock renders the song");
    harness.audio_mut().stop_song();
    harness.context(|game, ctx| game.start_game(ctx));
    assert_eq!(harness.audio().song_label(), Some(own.as_str()), "an empty slot is requested into");
    hold_the_match(&mut harness);
    poll(&mut harness, DRAINED, "the song past its first beat", |harness| beats_into_the_song(harness) >= 1);
    let seeks = conductor(&harness).reconstructions;
    harness.context(|game, ctx| game.start_game(ctx));
    hold_the_match(&mut harness);
    settle(&mut harness);
    assert_eq!(conductor(&harness).reconstructions, seeks + 1, "the match start seeks the song to its top, once");
    let position = harness.audio().song_position().expect("playing");
    assert_eq!(position.bar, 0, "a match begins in the call's first bar");
}

#[test]
fn a_hop_plays_the_galop_once_on_its_chickens_own_source_and_a_clamped_hop_too() {
    let mut harness = harness(GameMode::SinglePlayer, ChaosMode::Normal);
    settle(&mut harness);
    let hen = chicken(&harness, 0);
    let x = harness.game().chickens[0].x;
    park_lane(&mut harness, FIRST_BELT_ROW, x + LANE_PERIOD / 2.0);
    let before = conductor(&harness);
    step_with(&mut harness, &[press(KeyCode::KeyW)]);
    assert_eq!(harness.game().music.triggered, ["hop"]);
    assert_eq!(harness.game().chickens[0].row, FIRST_BELT_ROW, "the hop landed clear of the carts");
    assert!(source_of(&harness, hen).is_some(), "the chicken's own activation");
    step_with(&mut harness, &[release(KeyCode::KeyW)]);
    key(&mut harness, KeyCode::KeyS);
    assert_eq!(harness.game().chickens[0].row, START_ROW);
    step_with(&mut harness, &[press(KeyCode::KeyS)]);
    assert_eq!(harness.game().chickens[0].row, START_ROW, "a Down on the start row is clamped at the board's edge");
    assert_eq!(harness.game().music.triggered, ["hop"], "and poses, puffs and sounds like any hop");
    assert_eq!(harness.game().music.counts.hops, 3);
    hold_the_match(&mut harness);
    let rung = |harness: &GameHarness<FroggerGame>| {
        let now = conductor(harness);
        now.accepted_triggers == before.accepted_triggers + 3 && now.live_sources == before.live_sources + 1
    };
    poll(&mut harness, DRAINED, "three accepted hops on the chicken's one source", rung);
}

#[test]
fn in_co_op_the_second_chicken_answers_on_the_piccolo_on_its_own_source_and_solo_never_does() {
    let mut harness = harness(GameMode::TwoPlayerCoop, ChaosMode::Normal);
    let (round, tall) = (chicken(&harness, 0), chicken(&harness, 1));
    step_with(&mut harness, &[press(KeyCode::KeyS), press(KeyCode::ArrowDown)]);
    assert_eq!(harness.game().music.triggered, ["hop", "hop_two"], "both on one frame, in index order");
    let (first, second) = (source_of(&harness, round), source_of(&harness, tall));
    assert!(first.is_some() && second.is_some() && first != second, "each chicken on its own source");
    step_with(&mut harness, &[release(KeyCode::KeyS), release(KeyCode::ArrowDown)]);
    step_with(&mut harness, &[press(KeyCode::ArrowLeft)]);
    assert_eq!(harness.game().music.triggered, ["hop_two"]);

    let mut solo = crate::test_support::harness(GameMode::SinglePlayer, ChaosMode::Normal);
    let mut sent = Vec::new();
    for code in [KeyCode::KeyA, KeyCode::ArrowLeft, KeyCode::KeyD, KeyCode::ArrowRight, KeyCode::KeyS, KeyCode::ArrowDown] {
        step_with(&mut solo, &[press(code)]);
        sent.extend(solo.game().music.triggered.clone());
        step_with(&mut solo, &[release(code)]);
    }
    assert_eq!(sent, ["hop"; 6], "either player's keys steer the one chicken, on the trumpet; hop_two is never sent");
}

#[test]
fn a_hop_into_a_cart_plays_its_note_then_the_squawk_and_the_chicken_keeps_its_source() {
    let mut harness = harness(GameMode::SinglePlayer, ChaosMode::Normal);
    let hen = chicken(&harness, 0);
    let x = harness.game().chickens[0].x;
    park_lane(&mut harness, FIRST_BELT_ROW, x);
    step_with(&mut harness, &[press(KeyCode::KeyW)]);
    assert_eq!(harness.game().music.triggered, ["hop", "death"], "the hop's note, then the death its landing resolves");
    assert_eq!(harness.game().music.counts.deaths, 1);
    settle(&mut harness);
    assert!(source_of(&harness, hen).is_some(), "the chicken is hidden, never destroyed: its source lives on");
}

#[test]
fn a_drown_a_timeout_and_a_sweep_each_squawk_once_through_the_one_death() {
    let mut harness = harness(GameMode::SinglePlayer, ChaosMode::Normal);
    let hen = chicken(&harness, 0);
    harness.context(|game, _| game.chickens[0].timer = 0.001);
    assert_eq!(next_trace(&mut harness), ["death"], "the attempt timer ran out");
    assert_eq!(source_of(&harness, hen), None, "the squawk is the game's, never the chicken's");

    let row = raft_row();
    revive(&mut harness, 0);
    stand(&mut harness, 0, row, 0.0);
    park_lane(&mut harness, row, LANE_PERIOD / 2.0);
    assert_eq!(next_trace(&mut harness), ["death"], "open soup drowns");
    assert_eq!(harness.game().chickens[0].row, row);

    let def = LANES.iter().find(|def| def.row == row).expect("the raft lane");
    let edge = lane_velocity(def, ChaosMode::Normal, 1).signum() * (WIN_W / 2.0 - 0.01);
    revive(&mut harness, 0);
    stand(&mut harness, 0, row, edge);
    park_lane(&mut harness, row, edge);
    assert_eq!(next_trace(&mut harness), ["death"], "a raft carried past the edge sweeps");
    assert!(harness.game().chickens[0].x.abs() > WIN_W / 2.0, "swept off, not drowned");
    assert_eq!(harness.game().music.counts.deaths, 3);
}

#[test]
fn a_nest_sounds_the_fanfare_after_its_hop_and_the_fifth_the_cadence_alone() {
    let mut harness = harness(GameMode::SinglePlayer, ChaosMode::Normal);
    let hop_into_nest = |harness: &mut GameHarness<FroggerGame>, slot: usize| {
        let x = tile_center(HOME_COLS[slot], HOME_ROW).x;
        stand(harness, 0, HOME_ROW + 1, x);
        park_lane(harness, HOME_ROW + 1, x);
        step_with(harness, &[press(KeyCode::KeyW)]);
        let trace = harness.game().music.triggered.clone();
        step_with(harness, &[release(KeyCode::KeyW)]);
        trace
    };
    assert_eq!(hop_into_nest(&mut harness, 2), ["hop", "home"]);
    assert!(harness.game().homes[2]);
    assert_eq!(harness.game().music.counts.homes, 1);

    for slot in [0, 1, 3] {
        fill_nest(&mut harness, slot);
    }
    let homes = harness.game().music.counts.homes;
    assert_eq!(hop_into_nest(&mut harness, 4), ["hop", "clear"], "the fifth nest plays the cadence and no fanfare");
    assert_eq!(harness.game().music.counts.homes, homes);
    assert_eq!(harness.game().music.counts.clears, 1);
    assert!(harness.game().round_clear_in.is_some());
    assert_eq!(harness.game().state, GameState::Playing, "the round clears; the match goes on");
}

#[test]
fn a_restart_ends_the_chickens_source_and_the_next_matchs_hop_activates_a_new_one() {
    let mut harness = harness(GameMode::SinglePlayer, ChaosMode::Normal);
    let first = chicken(&harness, 0);
    key(&mut harness, KeyCode::KeyS);
    let old = source_of(&harness, first).expect("the hop activated the chicken's source");
    harness.context(|game, ctx| game.start_game(ctx));
    settle(&mut harness);
    assert_eq!(source_of(&harness, first), None, "the match's entities went, the chicken's source with them");
    let second = chicken(&harness, 0);
    assert_eq!(source_of(&harness, second), None, "nothing activates before the new chicken's first hop");
    step_with(&mut harness, &[press(KeyCode::KeyS)]);
    assert_eq!(harness.game().music.triggered, ["hop"]);
    let new = source_of(&harness, second).expect("the next match's hop activates anew");
    assert_ne!(new, old);
}

#[test]
fn the_pause_menu_jams_one_unit_a_move_and_nothing_on_open_confirm_the_pointer_or_a_reopen() {
    let mut harness = harness(GameMode::SinglePlayer, ChaosMode::Normal);
    let jams = |harness: &GameHarness<FroggerGame>| harness.game().music.game.jam_units();
    harness.step(FRAME, &[press(KeyCode::Escape), press(KeyCode::KeyW)]);
    harness.step(FRAME, &[release(KeyCode::Escape), release(KeyCode::KeyW)]);
    assert!(harness.game().pause.is_active());
    assert_eq!(jams(&harness), 0, "the frame it opens jams nothing, a move on it included");
    let accepted = conductor(&harness).accepted_triggers;
    key(&mut harness, KeyCode::KeyS);
    assert_eq!(jams(&harness), 1, "one move, one unit");
    settle(&mut harness);
    assert_eq!(conductor(&harness).accepted_triggers, accepted + 1, "the song plays on under the pause menu");
    let (resume, restart) = (drawn_at(&harness, "Resume"), drawn_at(&harness, "Restart"));
    for at in [restart, resume, restart, resume] {
        harness.step(FRAME, &[InputEvent::MouseMoved(at.x, at.y)]);
    }
    assert_eq!(jams(&harness), 1, "a pointer swept across the localized rows jams nothing");
    harness.step(FRAME, &[InputEvent::MouseMoved(resume.x, resume.y)]);
    key(&mut harness, KeyCode::Enter);
    assert!(!harness.game().pause.is_active(), "confirm on Resume resumes");
    assert_eq!(jams(&harness), 1, "and confirm jams nothing");
    harness.step(FRAME, &[press(KeyCode::Escape), press(KeyCode::KeyS)]);
    harness.step(FRAME, &[release(KeyCode::Escape), release(KeyCode::KeyS)]);
    assert!(harness.game().pause.is_active());
    assert_eq!(jams(&harness), 1, "a reopen jams nothing on its opening frame");
    assert_eq!(harness.game().music.triggered, Vec::<&str>::new());
}

/// Beats the song has crossed, counted from its top.
fn beats_into_the_song(harness: &GameHarness<FroggerGame>) -> u64 {
    harness.audio().song_position().expect("playing").beat
}

/// Step with wall time between frames, running `each_frame` before every one, until a frame
/// ripples; returns that frame's anchors.
fn next_pulse(
    harness: &mut GameHarness<FroggerGame>,
    mut each_frame: impl FnMut(&mut GameHarness<FroggerGame>),
) -> Vec<Vec2> {
    for _ in 0..BEAT_FRAME_CAP {
        each_frame(harness);
        std::thread::sleep(Duration::from_millis(3));
        harness.step(FRAME, &[]);
        if !harness.game().music.pulsed_at.is_empty() {
            return harness.game().music.pulsed_at.clone();
        }
    }
    panic!("no beat rippled within {BEAT_FRAME_CAP} frames");
}

#[test]
fn a_beat_ripples_under_each_active_chicken_where_it_is_drawn_and_never_under_a_hidden_one() {
    let mut harness = harness(GameMode::TwoPlayerCoop, ChaosMode::Normal);
    let (round, tall) = (chicken(&harness, 0), chicken(&harness, 1));
    let anchors = next_pulse(&mut harness, |_| {});
    assert_eq!(anchors, [drawn(&harness, round), drawn(&harness, tall)], "one anchor each, where this frame drew it");
    let anchors = next_pulse(&mut harness, |harness| harness.context(|game, _| game.chickens[1].respawn_timer = 1.0));
    assert!(!harness.game().chickens[1].active());
    assert_eq!(anchors, [drawn(&harness, round)], "the chicken waiting out its respawn is hidden, and gets none");
}

#[test]
fn no_beat_ripples_under_the_pause_menu_or_in_the_round_clear_beat() {
    let mut harness = harness(GameMode::SinglePlayer, ChaosMode::Normal);
    key(&mut harness, KeyCode::Escape);
    assert!(harness.game().pause.is_active());
    let (pulsed, beat) = (harness.game().music.game.pulsed(), beats_into_the_song(&harness));
    poll(&mut harness, Duration::from_secs(3), "two beats crossed under the pause menu", |harness| {
        assert!(harness.game().music.pulsed_at.is_empty(), "nothing rippled under the menu");
        beats_into_the_song(harness) >= beat + 2
    });
    assert_eq!(harness.game().music.game.pulsed(), pulsed, "no beat ripples a frozen grid");
    key(&mut harness, KeyCode::Escape);
    assert!(!harness.game().pause.is_active());

    arm_beat(&mut harness);
    let (pulsed, beat) = (harness.game().music.game.pulsed(), beats_into_the_song(&harness));
    for frame in 0.. {
        assert!(frame < BEAT_FRAME_CAP, "two beats within {BEAT_FRAME_CAP} frames");
        harness.context(|game, _| game.round_clear_in = Some(1.0));
        std::thread::sleep(Duration::from_millis(3));
        harness.step(FRAME, &[]);
        assert!(harness.game().music.pulsed_at.is_empty(), "nothing rippled in the round-clear second");
        if beats_into_the_song(&harness) >= beat + 2 {
            break;
        }
    }
    assert_eq!(harness.game().music.game.pulsed(), pulsed);
}

#[test]
fn no_beat_ripples_on_the_frame_the_last_chicken_retires() {
    let mut harness = harness(GameMode::SinglePlayer, ChaosMode::Normal);
    for attempt in 0.. {
        assert!(attempt < 10, "a beat on the retiring frame within ten tries");
        harness.context(|game, _| game.chickens[0].lives = 1);
        settle(&mut harness);
        let position = harness.audio().song_position().expect("playing");
        let seconds_a_beat = 60.0 / 152.0;
        let next_beat = (position.seconds / seconds_a_beat).floor() * seconds_a_beat + seconds_a_beat;
        std::thread::sleep(Duration::from_secs_f64(next_beat - position.seconds + 0.01));
        let pulsed = harness.game().music.game.pulsed();
        harness.context(|game, _| game.chickens[0].timer = 0.001);
        harness.step(FRAME, &[]);
        assert_eq!(harness.game().music.triggered, ["death"]);
        assert_eq!(harness.game().state, GameState::GameOver, "the last life retired the last chicken");
        if harness.audio().beat_crossed(Subdivision::Beat) {
            assert!(harness.game().music.pulsed_at.is_empty(), "the frame that ends the match ripples nothing");
            assert_eq!(harness.game().music.game.pulsed(), pulsed);
            return;
        }
        harness.context(|game, ctx| game.start_game(ctx));
    }
}

/// An asset folder holding every directory the game reads but `soundfonts/`, each a link to
/// the repository's own, removed however the test ends.
#[cfg(unix)]
struct AssetsWithoutBank {
    root: PathBuf,
}

#[cfg(unix)]
impl AssetsWithoutBank {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("chicken-coop-missing-bank-{}", std::process::id()));
        std::fs::remove_dir_all(&root).ok();
        let assets = root.join("assets");
        std::fs::create_dir_all(assets.join("ai")).expect("a scratch asset folder");
        for directory in ["fonts", "locales", "sprites", "ai/music"] {
            std::os::unix::fs::symlink(asset_base().join(directory), assets.join(directory)).expect("linked");
        }
        Self { root }
    }
}

#[cfg(unix)]
impl Drop for AssetsWithoutBank {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).ok();
    }
}

#[cfg(unix)]
#[test]
fn without_its_bank_the_game_runs_silent_and_counts_every_trigger_it_refuses() {
    let scratch = AssetsWithoutBank::new();
    let assets = scratch.root.join("assets");
    let config = crate::game_config(assets.to_str().expect("UTF-8"));
    let mut harness = GameHarness::new(FroggerGame::default(), config);
    harness.step(FRAME, &[]);
    harness.context(|game, ctx| game.start_game(ctx));
    assert!(harness.game().music.game.song_failed(), "the song failed at init");
    assert_eq!(harness.game().music.game.failures(), 2, "and again at the match start");
    assert_eq!(harness.audio().song_label(), None);
    let refused = harness.sound_sources().refused_triggers();
    step_with(&mut harness, &[press(KeyCode::KeyS)]);
    assert_eq!(harness.game().music.triggered, ["hop"]);
    assert_eq!(harness.sound_sources().refused_triggers(), refused + 1, "refused, and counted");
    assert_eq!(harness.game().state, GameState::Playing, "and the game runs on");
}
