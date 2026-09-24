# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository. (`AGENTS.md` is a symlink to this file.)

## Commands

```bash
cargo run                     # play the game
cargo run --features editor   # run the game inside the engine's scene editor
cargo build                   # compile check
cargo test                    # 84 headless tests (no GPU, no window)
cargo test <test_name>        # run a single test
```

The game depends on the `insiculous_2d` engine by relative path (`../../insiculous_2d`); both checkouts must sit side by side or nothing builds. Engine crates used: `engine_core` (always) and `editor_integration` (only behind the `editor` feature). The `deion_assets` symlink at the repo root points to the shared art repo (`../deion_assets`) and assumes the same side-by-side layout — it is a **read-only reference**, never write through it.

## Architecture

This is a single-crate game (`insiculous_frogger`) built on the in-house `insiculous_2d` ECS engine — game 6 of the 20 Games Challenge. `FroggerGame` (in `src/types.rs`) implements the engine's `Game` trait in `src/main.rs`: `init()` loads the font and the sheets and spawns the persistent background; achievements register in `register_achievements()`, which the engine calls before the window opens (`cargo run -- --achievements-manifest <path>` exports the list); everything match-scoped spawns fresh in `start_game()`. With `--features editor` the identical game runs inside the engine's scene editor via `editor_integration::run_game_with_editor`, and at `/playground/frogger/` in the browser (the same feature, built by the engine's `build_wasm.sh --kind editor` and served from the site); no game code changes between the modes.

**State machine drives everything.** `GameState` (types.rs) is matched at the top of `update()` in main.rs: `TitleScreen`, `ModeSelect` (chaos select), and `Achievements` dispatch to input handlers in `menu.rs`; `Playing` and `GameOver` fall through to `update_gameplay()` in `gameplay/mod.rs`. Flow is Title → ModeSelect → Playing → GameOver (endless rounds — the only ending is every chicken out of lives). Match lifecycle (start, deaths, nest fills, the round-clear beat, reset-to-title) lives in `gameplay/flow.rs`; the **pure rules** (lane math, platform riding, the dive and duty cycles, hop resolution) live in `gameplay/rules.rs`, and the poses the drawn danger takes live in `gameplay/poses.rs` — data in, data out, no entities, and that's the primary test surface. All UI/HUD drawing is in `drawing.rs`, called once per frame from `update()` regardless of state.

**No physics crate at all** (Snake precedent). Every entity is a plain sprite (`Name` + `Transform2D` + `Sprite`) or one of the board's four `Tilemap`s; there are no `RigidBody`/`Collider` components and no `PhysicsSystem`. All collision is pure 1-D lane math in `rules.rs`:

- Each lane's obstacles live on a **torus** of period `LANE_PERIOD` (window width + a 2-tile offstage margin = 816 px), positions kept in `[-P/2, P/2)`. `wrapped_dx`/`lane_overlap` use modular distance, so a segment straddling the wrap seam still hits — no two-AABB special case.
- Logical state is authoritative: chickens are `(x, row)` in `ChickenState`, obstacles are `Vec<f32>` centers in `LaneState`. `sync_sprites()` (gameplay/mod.rs) pushes logic into the sprite world each frame; sprites never drive logic.
- Every lane **sprite** spawns as a (main, ghost) pair: the ghost sits one torus period away and is visible only while its segment straddles a window edge, so lanes scroll seamlessly with no visual/collision mismatch.
- A **segment** is one lane obstacle as drawn — a car is one cart, a truck one hot dog cart, a log two rafts abutted, a group of crackers its two or three. Its sprites stand one cell apart, so `LaneDef::half_len()` is a measured half-length: the first sprite's opaque left to the last sprite's opaque right.
- Road lanes kill on overlap (`road_hit`, chicken half-width `CHICKEN_HALF`); soup lanes are inverted — the obstacles ARE the platforms (`platform_under`, chicken **center** must be aboard), open soup drowns, riding off the board edge is a `Swept` death, and chickens never wrap (only obstacles live on the torus).

**The board is four `Tilemap` entities** (`board.rs` + `spawning::spawn_board`), one per tile sheet: floor (the median, the start row, and the ground the nests stand on), coop wall (the rest of the home row), soup (rows 1–5) and conveyor belt (rows 7–11). `terrain_at(col, row)` is the single source of the row map, and `build_map` is pure (the loaded sheet comes from the caller) so every terrain rule is headless-testable. Each map holds only the cells its own terrain owns, so the four never overlap, and each map's `tile_uv_size` comes from its loaded sheet's grid — never typed. `board::animate` rewrites the soup's and the belts' tile values when the frame turns, reading the clip's own cells and fps from the sidecar; a belt row whose lane runs `dir = -1` plays its frames reversed, so the slats always run the way the carts above them do. **The tiles are never theme-tinted** — the art is the art, and the chaos theme reaches the band, the grid and the particles only.

**Board row map** (tilemap convention: row 0 = top): row 0 home row (5 nests at `HOME_COLS = [1, 4, 7, 10, 13]`, coop wall elsewhere), rows 1–5 soup, row 6 median, rows 7–11 belts, row 12 start row — 13 rows total. The ten `LaneDef`s (5 belt + 5 soup: carts, trucks, rafts, cracker groups) are the compile-time `LANES` table in rules.rs.

**Per-frame gameplay order** (`update_gameplay`): pause gate → state input → `advance_lanes` (lanes also scroll during GameOver, behind the overlay) → `step_chickens` (timer, riding/drowning/sweeping, belt hits, then at most one hop with fixed priority Up > Down > Left > Right) or `step_round_clear` → `step_respawns`, unless a fill armed the beat this frame → `sync_sprites` → `board::animate`. Hop landings resolve **immediately** — no one-frame grace: home-row entry snaps to the nearest column and resolves via `home_slot_at` (`Fill`/`Miss`/`Bun`); land rows re-snap the chicken to the grid while soup rows keep the platform drift. The pause gate follows the engine Pause Pattern exactly (only `Playing` is pausable; `ctx.time_scale` freezes particles and the backdrop grid; `Resumed` skips the rest of the frame).

**The round-clear beat (D24).** The fifth arrival arms `round_clear_in = ROUND_CLEAR_BEAT` instead of clearing the round on the spot, so the round's last arrival is not erased before it renders. `step_chickens` **breaks out of its player loop** the moment the beat arms — in co-op P2's Up in the same frame must not hop it off the start tile it was just placed on — and `step_respawns` does not run on a frame in which the beat is armed, the arming frame included. While the beat runs the match is still `Playing` (the pause menu works), lanes, belts, poses and one-shots all advance, and no chicken is simulated. When the beat runs out `advance_round` moves the round number, the nests, the per-round counters and the chickens' placement in one frame, and the seated chickens despawn with them — and it stands **every non-retired chicken** back on its start tile **alive**, out of any respawn wait (`Revive::Waiting`), so a partner who died just before the clear does not sit out the new round. The beat's *opening* placement is `Revive::Never`: a partner still in its respawn wait stays gone through the celebration while its poof plays, and its revival is the round's end. A retired chicken stays retired. The round's rewards are read **at the fifth fill** rather than at the beat's end, so a Restart or a quit inside the beat cannot forfeit a deathless clear, and they are paid exactly once.

**The drawn danger is posed, not machined (D22).** The crackers and the bun carry a `SpriteAnimation` and **no** `ClipStateMachine`: `cracker_pose` and `bun_pose` are pure functions of the rules' own cycle and return the clip **and the frame within it**, which the game writes as `current_clip`/`current_frame` with `playing = false`. A lead-in ends at the flip and a lead-out begins at it, so a cracker drawn rising is already rideable and one drawn sinking is not yet lethal: looking dangerous while safe can only let a player off, and looking safe while dangerous never happens. The lethal window is never restated — each pose asks `crackers_submerged`/`bun_open`, which own the flip, and only the lead windows are the pose module's own arithmetic. A filled nest draws no bun. Outside the Ridiculous family neither leaves its rest pose. **A spawn asks the same rule** — `bun_placement` decides the bun's nest, place and visibility as well as its pose, and `spawning` writes a chicken's start tile through the same helper a round's end uses — because two of the three ways a match starts reach a rendered frame before the per-frame sync does, and an unwritten spawn frame is a frame at the world origin.

**One writer of the chicken's pose (D21).** `ChickenState.facing` is the single source; `pose_chicken` does `transition_to(<clip>_<facing>)` **and then** `play` of the same clip. Both calls are needed: `transition_to` alone is a no-op on the state the machine is already in (three fast hops would play one jump), and `play` alone is reverted the same frame because the machine re-selects its own state's clip (a turn mid-jump would keep drawing the old direction). A hop sets the facing and calls it with `jump`; a respawn, a round clear and a match start set north and call it with `idle`. Riding never turns the chicken. Hops are instant — no tween.

**2-player co-op** shares the board, nests, and pooled score but keeps per-chicken lives, attempt timers, and respawn columns (`COOP_START_COLS`); a chicken out of lives retires (hidden, no input) while the partner plays on, and the match ends only when all are retired. Input follows the engine convention: 1P merges both player slots (WASD, arrows, and either pad all steer the one chicken); co-op routes P1/P2 by index. Gameplay never reads raw `KeyCode`s; menus use `MenuInput::read`.

**Chaos modes** map onto the family predicates: Insane (`is_insane`) = 1.5× lane speed + 25 s attempt timer; Ridiculous (`is_ridiculous`) = crackers sink on hash-phased cycles (submerged = not a platform) + a bun in one hash-picked nest per round on an absent/present duty cycle (starts absent, so every round stays winnable); Insiculous = both. Speeds also ramp +15% per round, capped at 2×. All cycle logic is pure functions of `play_time`/`round` (`crackers_submerged`, `bun_nest`, `bun_open`) — no timers to desync. The background, the backdrop grid and the menu chrome all derive from `ChaosTheme::for_mode`.

**Coordinate and scale conventions:**
- World origin is screen center. The window is `WIN_W × WIN_H` = 720×768, **and the bare native game's window is not resizable** (`main.rs` sets `resizable = false` on the non-editor path only): the world never scales, so any other size would crop or float the board and the HUD alike — the web page scales the whole canvas instead, and the editor build keeps its resizable window because it needs room for its panels. The HUD is laid out from the board's origin (`hud.rs`), never from the window's edges: a 15×13 board of `TILE = 48` world-pixel tiles, plus a 72 px HUD `BAND` above (score, round/nests) and below (per-player timer bars and life icons).
- **The HUD is anchored to the board, not the window** (`src/hud.rs`). Every element — the score, the round/nests line, the pause hint, the player labels, the timer bars and the chaos banner — is laid out in board space and translated by `hud::board_origin`, and a slot's width is `WIN_W` over the player count, never the window's width. A window that is not the board's size then crops the world the same way for the HUD as for the art; a bar laid out from the window's edges would stretch away from the life icons above it, which are world sprites and crop with the board.
- `board::tile_center(col, row)` is the one mapping from board cells to world positions (row 0 = top, matching the Tilemap convention; the board is centered on the origin).
- The renderer multiplies `Transform2D.scale` by `RENDER_UNIT = 80.0` to get pixel size — sprite scales are `cell / RENDER_UNIT` at spawn sites. With no physics there is no collider/sprite divergence to worry about; collision extents come straight from the measured constants (`CHICKEN_HALF`, `LaneDef::half_len()`).
- Chicken `x` is continuous (platforms drift it off-grid); `row` is discrete. Only obstacle positions wrap.

**All tuning lives in `src/constants.rs`** (board geometry, the sheets block, lane margins, speeds, timers, scoring, achievement thresholds, draw depths, band layout, grid impulses), the HUD's placement in `src/hud.rs`, and all entity creation lives in `src/spawning.rs`. Every spawned entity gets a `Name` component ("Chicken P1", "Cart r11 #0.0", "Nest 0", "Seated 0", "Board soup") so the editor hierarchy is readable — keep this for new entities. Particle looks are centralized in `effects.rs` (the hop puff and the soup splash; deaths and arrivals are drawn one-shots now, not particle bursts).

**Localization:** fully localized (en + pirate) on the engine's `ctx.strings` pattern. Every player-facing string is a locale key resolved at draw time via `ctx.strings.tr(...)`; tables live in `assets/locales/{en,pirate}.ron` (loaded from the `locales` dir under the asset base set in main.rs). Both files MUST define the same key set — `locale_files_have_matching_keys` in achievements.rs enforces it. The title menu's Language item calls `ctx.strings.cycle_locale()` and then **re-registers achievements** (`achievements::register_all` — id-keyed insert refreshes names/descriptions from `ach.<id>.name`/`ach.<id>.desc` keys without touching unlock state). `pirate.ron` declares `font: Some("fonts/BlackSamsGold-ej5e.ttf")`, so switching locale also swaps the game font. The pause overlay localizes via `PauseMenu::draw_labeled` + `PauseMenuLabels`; chaos labels/hints come from `chaos_label_key`/`mode_hint_key` in menu.rs; achievement page sections use the `DISPLAY_SECTIONS` locale keys.

**Achievements:** ids are `&'static str` consts in `achievements.rs` (8 total), registered with locale-table names in `register_achievements()`, which the engine calls before the window opens (`cargo run -- --achievements-manifest <path>` exports the list). They unlock from `fill_home`/`award_round_clear` in flow.rs and persist to `saves/frogger_achievements.json` — the **ids and the three save keys are frozen**: players' saves hang off them, so a re-skin never renames them. Tests pin the id↔locale-key contract and the display-section coverage.

**Tests (84):** `flow_tests.rs` (6 — match flow through the engine's harness: the fifth fill and the beat, input and lanes during it, its one reset, restart and quit inside it, and the co-op partner in the frame it arms and when it ends), `gameplay_tests.rs` (32 — torus wrap/straddle cases, every death path, the dive and duty cycles, the measured extents, home-row resolution, hop priority, chaos scaling, lane-table sanity, and what a round's end does to the chickens), `art_tests.rs` (19 — every sheet read back against its synced sidecar: the pose windows, the frame counts, the machine's clip table, the pose writer, the spawn pose and placement), `board.rs` (8 — the four maps, the row map, uv sizes, the belt's frame direction), `spawning.rs` (8 — segment pairs, naming, the nests and the composition anchor, the life-icon row), `achievements.rs` (6 — locale parity, re-register semantics), `hud.rs` (4 — the board origin and every element's place on it), `menu.rs` (1 — chaos label keys). `test_support.rs` holds the shared fixtures: the sheets read back through the engine's GPU-free load path, and the whole game driven through the engine's `GameHarness`. All headless; run `cargo test` before claiming anything done.

**Paths:** assets and saves resolve through `engine_core::game_root!()` (exe dir if it contains `assets/`, else `CARGO_MANIFEST_DIR`), so `cargo run` works from any cwd. Input bindings persist to `saves/input_settings.json`.

## Chicken Coop (landed 2026-09-21)

Frogger is the **second of the six Phase G Deion re-skins**, after Tong. The build is no longer neon:
the players are **chickens** crossing a kitchen, the traffic is **food carts on conveyor belts**, the
river is **tomato soup** crossed on celery and baguette rafts, the turtles are **sinking crackers**,
the croc is a **snapping bun** in a breadbox nest, and the home row is five **nest boxes** in a coop
wall. In-game the game is **Chicken Coop** (`title.window` in both locale files). The crate is still
`insiculous_frogger`, `FroggerGame` is still the game type, the site still lists it as *Insiculous
Frogger* and its slug is still `frogger` — those are frozen with the save keys.

- **Art enters only through the sync.** `assets/sprites/sync.list` pins the `deion_assets` commit
  and lists the seventeen sources; `python3 deion_assets/scripts/sync_sprites.py .` copies each PNG
  and its `.sheet.ron` sidecar in, and `--check` hashes the copies against the pinned blobs. Never
  hand-copy or hand-edit a file there — fix the master in `deion_assets` and re-sync. The working
  set's `scripts/check-sprite-sync.sh` runs that check over every game and prints `pong OK`,
  `frogger OK`.
- **The sheets block** in `src/constants.rs` names each sheet once as the engine's `SheetSpec` (from
  the prelude — every re-skin shares the one type): its path, its cell, and the opaque box of the
  reference frame. Each box was **measured** from the synced PNG as the union over the frames the
  sheet's clips play, with the bottom-right corner one past the last opaque pixel. The cell drives
  the draw scale; the box drives the anchor and every extent derived from the art.

  | subject | cell | drawn | extent / anchor |
  |---|---|---|---|
  | round chicken (P1) | 48×48 | 40×39 | 29 px head-on, 38 beak to tail |
  | tall chicken (P2) | 48×48 | 40×45 | 23 px head-on, 38 beak to tail |
  | sushi / donut cart | 64×48 | 55×35 / 55×37 | half-length 27.5 |
  | hot dog cart | 96×48 | 87×35 | half-length 43.5 |
  | celery / baguette raft | 64×32 / 96×32 | 60×27 / 89×25 | two abutted: half-length 62 / 92.5 |
  | cracker | 48×48 | 42×33 | two: 45; three: 69 |
  | bun | 64×48 | 55×44 | posed, never machine-played |
  | nest | 96×80 | 75×72 at (10, 5) | canvas bottom on the home row's bottom edge |
  | floor / wall / soup / belt | 48×48 | the whole cell | the four maps' tiles |

- **1× on 48 px rows, pixel-snapped** (`GameConfig::with_pixel_snap(true)`, D15): one art pixel per
  window pixel at `RENDER_UNIT = 80`, nearest filtering, no faked scale. Every art sprite is drawn
  white with emissive 0 — the neon tints are gone with the neon build.
- **`CHICKEN_HALF = 11.5`, one number for both players and every facing.** It is half the tall
  chicken's head-on body (23 px), so nothing is ever drawn narrower than the box it collides with;
  the round chicken's 29 px body overhangs it 3 px a side head-on, and both overhang it beak to
  tail, which can only let a player off. A box that grew when the chicken turned would punish
  turning, and two boxes in one co-op game is unfair.
- **The round-clear beat** and **the posed danger** are the two behaviours this re-skin added; the
  Architecture section above carries both. The turtle and bun *timings* did not change — they gained
  drawn lead-ins.
- **Style SSOT:** `deion_assets/DEION_STYLE.md` via the `deion_assets -> ../../deion_assets`
  symlink. AI art is quarantined (`ai_` prefix, `deion_assets/ai/` only) — tiered ship rule
  (DEION_STYLE.md §6, Aug 19 2026): may ship in FREE web builds, never in paid/marketplace builds.
  **Chicken Coop is free-tier only until Jesse's cleanup pass** (D3): the synced copies keep their
  `ai_` prefix, so `deion_assets/scripts/check_no_ai_assets.sh assets` fails on a paid build, as it
  must. The exit is a hand-cleaned master under `deion_assets/topdown/…`, re-exported without the
  prefix, with the `sync.list` line moved to it.

## Work tracking

Open work lives on the **Studio Board** (https://github.com/orgs/beinsiculous/projects/1)
as issues in this repo. **Always pass `-R beinsiculous/frogger`** — a bare `gh` command
resolves against the session's working directory, which is often the working-set root, so
it lists and files against the wrong repository.

```sh
gh issue list -R beinsiculous/frogger
gh api repos/beinsiculous/frogger/milestones --jq '.[] | "\(.title): \(.description)"'
```

Issues are grouped into **sprint milestones**; each description records the batch's
internal order and its gates. Take the next unblocked issue in a sprint, not an arbitrary
one. Claim by assigning yourself; close with `fixes beinsiculous/frogger#N` in the commit.

**Unfinished work becomes an issue.** Anything you don't finish — work you deferred, debt
you created, a follow-up you spotted — is filed before you report done. Never buried in a
doc, never left as a bare `TODO:`, never dropped. The `file-issue` skill carries the shape;
`sprint-planning` groups issues into shippable batches.

## Review workflow

- The adversarial-review skill lives in `.claude/skills/` (author here, headless kimi as reviewer; prompts in `prompts/` are fixed — never edit them mid-review).
- Approved plans go to `review/plan.md` and are reviewed via `scripts/request-review.sh plan review/plan.md --reviewer=kimi` **before** implementation.
- Commits over 100 changed lines are gated by `scripts/commit-review-hook.sh` (PreToolUse hook in `.claude/settings.json`). The `ADV_REVIEWED=1` prefix is used only after a code-mode review adjudicated with the user, or when the user explicitly skipped review.
- `review/` holds gitignored transients (`plan.md`, `review-N.md`, `rebuttal-N.md`, `draft.diff`); fold anything durable into real docs, then clear it when the subject settles.
- NOTE: `scripts/request-review.sh` and `scripts/commit-review-hook.sh` are copies — the canonical ones live in the working-set root, not in `insiculous_2d`. Never edit a copy: fix the root's and re-copy, and `scripts/check-skill-parity.sh` there reports any repo that drifted.
