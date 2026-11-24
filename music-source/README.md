# Crosswalk Carnival

Original AI-authored G-major electro-swing/ska-hop for Chicken Coop, CC0-1.0. No
borrowed melody, recording or sample bank. Long-short swung pairs and descending
calls answer across two-beat bars; bass and chords emphasize the late offbeats.
Bars 1–8 introduce, 9–16 extend the lead upward, 17–24 reduce and leave alternate
bars open, 25–32 reprise; tom/snare fills close each eight-bar sentence. This is a
new hook/arrangement, not a transposition of previous games. G/B/D/E throughout
keeps arbitrary asynchronous cues free of semitone/tritone clashes with the backing.

152 BPM, explicit 2/4, 480 PPQ, 32 bars. Integer MIDI tempo 394737 us/quarter yields
25.263168 seconds; last actual backing note-off and loaded end_tick are 30720.
Background track names are distinct from galop/squawk/fanfare/cadence/jam selectors.

`crosswalk_carnival.evil` is editable in MIDI Evil. **compose.py replaces hand-edited
project/patch sources when explicitly run.** Ten original patches cover filtered bass,
square lead, clipped offbeat chords, kick/snare/hat, distinct triangle/sine hops,
low saw death and pitched toms. Presets 56/72/57/58/47 retain every declared map
instrument; the two unbound slots (58/47) have backing roles and must still load.
Hop notes are short (70 ticks, ~58 ms) and quieter than the three-note home and
four-note clear rewards (360/480 ticks). Death is a lower-register two-note chord.
All six existing action names, source ownership and Loop/Off modes are unchanged,
including clamped-hop requests. No new movement logic.

`render.json` records seed base 11600 (instrument index added), 32000 Hz, velocity
127, master gain 0.8 and 720-tick render gates. Exact-key nonlooping bank zones have
50 ms release and 0.40 s longest-note budgets. Longest composed gate is 480 ticks
(~0.394737 s); rendered notes last 20547 frames (~0.6421 s). No transposition or
sample truncation. Five declared slots (<8), one layer/key (<6), bank below 4 MiB.

## Rebuild and install

From games/frogger, with normal working-set sibling layout:

```sh
python3 music-source/compose.py
python3 music-source/build.py /tmp/frogger-build-a
python3 music-source/build.py /tmp/frogger-build-b
cmp /tmp/frogger-build-a/ai_chicken_coop.mid /tmp/frogger-build-b/ai_chicken_coop.mid
cmp /tmp/frogger-build-a/ai_chicken_coop.sf2 /tmp/frogger-build-b/ai_chicken_coop.sf2
cp /tmp/frogger-build-a/ai_chicken_coop.mid assets/ai/music/ai_chicken_coop.mid
cp /tmp/frogger-build-a/ai_chicken_coop.sf2 assets/soundfonts/ai_chicken_coop.sf2
cargo run --manifest-path ../../insiculous_2d/Cargo.toml -p editor --example remap_soundfont -- \
  assets ai/music/ai_chicken_coop.mid soundfonts/ai_chicken_coop.sf2 \
  'Original AI composition and MIDI Evil synthesis for Be Insiculous, 2026' CC0-1.0
```

Retain the existing .mid.ron during export. The remapper applies bank/provenance
edits and saves through SongDocument, validating exact key coverage and native
Conductor load for all declared instruments. Never serialize the map/fingerprint
manually. Remove the old bank only after closure validation. build.py and
prepare_soundfont.sh are thin wrappers around shared engine build_music.py; default
output is disposable/ignored `rendered/`, never runtime assets. Install only after
success. The bank recipe's exact paths and roots match its render recipe.

Tool baselines: MIDI Evil project_audio 46ae908 (actual source revision
3d061afafc64c71266e643271cb25ee88f4e84e8), engine packer/remap/audition
3623e73c04906f6a4d601908c366c1bc5b6b6f5d. The shared builder and its finalized
source-directory/hardlink/integer/path/root guards are pinned to engine **7c8fed0**;
MIDI Evil is pinned to **3d061af** (full revision above). Python 3.12.3, Rust 1.94.0,
x86_64 Linux; existing Cargo.lock dependencies. Two fresh same-platform builds
must be byte-identical; cross-platform floating-point identity is not promised.
Runtime needs only its committed MIDI/map/bank, not these source tools.

Historical editor scores need their original bank and LICENSE, not a retarget to
new synthesis. Pre-migration revision 314b112eb9caaac2b529fab93543fc988a5c054c retains
the old closure. Restore it separately, e.g.:

```sh
git worktree add --detach /tmp/chicken-coop-historical 314b112eb9caaac2b529fab93543fc988a5c054c
```

Use that worktree's assets with the engine editor/audition. Building that detached
game also needs the usual relative engine layout.

## Auditions

```sh
python3 music-source/preview.py
```

The script reads all authored short TSVs without rewriting them. Edit idle.tsv,
sparse.tsv or hop/hop_two/death/home/clear/menu.tsv directly; rerunning preserves
those edits. Demo times must fit the eight-second renders. Only the long synthetic
dense recipe is generated, under ignored `previews/dense.tsv`. Metrics derive demo
onset times from the actual TSVs, not hardcoded example timestamps.

The real engine conductor, committed bank, unity runtime buses and production
limiter render `previews/idle.wav`, `sparse.wav`, `dense.wav` for 52 seconds (two full
wraps). Sparse/dense exercise all six actions. Dense alternates busy two-player
hop bursts with recovery gaps, plus deaths/home/clear/menu. Eight-second `demo-*.wav`
files and matching `isolated-*.wav` demonstrate each action at final gain. Isolation
subtracts identical idle engine PCM only when neither limiter operated (at most
one PCM16 step of quantization error); it is not another synthesizer. Logs include
peak/RMS and first-100-ms cue/backing onset energy. These are synthetic event mixes,
not recorded gameplay; browser interaction and human listening remain planner gates.

Dense voice estimates: at 20 hops/s per chicken, ~58 ms gates plus 50 ms release
need at most three voices per hop slot. Home adds three and clear four to the first
slot (ten including overlapping hops); menu adds one to the second. Death uses two
voices separately. All stay below 24 voices/slot; measured background peak is six.
No limiting/clipping/nonfinite/rejected/late/stale/silent triggers are acceptable.

Runtime SHA-256 values recorded for this score revision (compare regenerated outputs before installation):

- `assets/ai/music/ai_chicken_coop.mid`: `8fcd155c3cda0b9c58792a6b7b6e24d2f2b1133cc57964eaf5e9a22dc28629a0`
- `assets/soundfonts/ai_chicken_coop.sf2`: `8b873bc98eb967ddd8668ec6f4508876a50cbc4f83af781166aaf7e49272778b`
