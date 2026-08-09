# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository. (`AGENTS.md` is a symlink to this file.)

## Commands

```bash
cargo run                     # play the game
cargo run --features editor   # run the game inside the engine's scene editor
cargo build                   # compile check
cargo test                    # 43 headless tests (no GPU, no window)
cargo test <test_name>        # run a single test
```

The game depends on the `insiculous_2d` engine by relative path (`../../insiculous_2d`); both checkouts must sit side by side or nothing builds. Engine crates used: `engine_core` (always) and `editor_integration` (only behind the `editor` feature). The `deion_assets` symlink at the repo root points to the shared art repo (`../deion_assets`) and assumes the same side-by-side layout — it is a **read-only reference**, never write through it.

## Architecture

This is a single-crate game (`insiculous_frogger`) built on the in-house `insiculous_2d` ECS engine — game 6 of the 20 Games Challenge and the engine's **first Tilemap consumer**. `FroggerGame` (in `src/types.rs`) implements the engine's `Game` trait in `src/main.rs`: `init()` loads the font, registers achievements, and spawns the persistent scenery (background + deforming grid); everything match-scoped spawns fresh in `start_game()`. With `--features editor` the identical game runs inside the engine's scene editor via `editor_integration::run_game_with_editor`; no game code changes between the two modes.

**State machine drives everything.** `GameState` (types.rs) is matched at the top of `update()` in main.rs: `TitleScreen`, `ModeSelect` (chaos select), and `Achievements` dispatch to input handlers in `menu.rs`; `Playing` and `GameOver` fall through to `update_gameplay()` in `gameplay/mod.rs`. Flow is Title → ModeSelect → Playing → GameOver (endless rounds — the only ending is every frog out of lives). Match lifecycle (start, deaths, home fills, round clears, reset-to-title) lives in `gameplay/flow.rs`; the **pure rules** (lane math, platform riding, croc/dive cycles, hop resolution) live in `gameplay/rules.rs` — data in, data out, no entities, and that's the primary test surface. All UI/HUD drawing is in `drawing.rs`, called once per frame from `update()` regardless of state.

**No physics crate at all** (Snake precedent). Every entity is a plain sprite (`Name` + `Transform2D` + `Sprite`) or the board `Tilemap`; there are no `RigidBody`/`Collider` components and no `PhysicsSystem`. All collision is pure 1-D lane math in `rules.rs`:

- Each lane's obstacles live on a **torus** of period `LANE_PERIOD` (window width + a 2-tile offstage margin = 816 px), positions kept in `[-P/2, P/2)`. `wrapped_dx`/`lane_overlap` use modular distance, so an obstacle straddling the wrap seam still hits — no two-AABB special case.
- Logical state is authoritative: frogs are `(x, row)` in `FrogState`, obstacles are `Vec<f32>` centers in `LaneState`. `sync_sprites()` (gameplay/mod.rs) pushes logic into the sprite world each frame; sprites never drive logic.
- Every obstacle spawns as a **(main, ghost) sprite pair**: the ghost sits one torus period away and is visible only while the obstacle straddles a window edge, so lanes scroll seamlessly with no visual/collision mismatch.
- Road lanes kill on overlap (`road_hit`, frog half-width `FROG_HALF`); water lanes are inverted — the obstacles ARE the platforms (`platform_under`, frog **center** must be aboard), open water drowns, riding off the board edge is a `Swept` death, and frogs never wrap (only obstacles live on the torus).

**The board is one `Tilemap` entity** (`board.rs` + `spawning::spawn_board`). `terrain_at(col, row)` is the single source of the row map; `build_board` is pure (tileset handle passed in) so every terrain rule is headless-testable. The tileset is a **procedural 4-cell strip** (grass/water/road/home, 16×16 px cells in one 64×16 texture) built at runtime by `tileset_pixels()` — raw RGBA tinted by the chaos theme's accent — and uploaded via `AssetManager::create_texture_from_rgba` (nearest-filtered by that API, so cells don't bleed). Tile values 1–4 index the strip (`tile_uv_size = (0.25, 1.0)`); 0 would be a hole and the board has none. The board is rebuilt per match (`rebuild_board` in flow.rs) because the tint is per chaos mode; on texture failure it falls back to the plain white texture instead of crashing. Rendering is entirely the engine's default render path (tilemap pass) — zero game-side render code, the whole 15×13 map batches into one draw.

**Board row map** (tilemap convention: row 0 = top): row 0 home row (5 slots at `HOME_COLS = [1, 4, 7, 10, 13]`, hedge elsewhere), rows 1–5 water, row 6 median, rows 7–11 road, row 12 start row — 13 rows total. The ten `LaneDef`s (5 road + 5 water: cars, trucks, logs, turtle groups) are the compile-time `LANES` table in rules.rs.

**Per-frame gameplay order** (`update_gameplay`): F1 debug toggle → pause gate → state input → `advance_lanes` (lanes also scroll during GameOver, behind the overlay) → `step_frogs` (timer, riding/drowning/sweeping, road hits, then at most one hop with fixed priority Up > Down > Left > Right) → `step_respawns` → `sync_sprites` → grid step. Hop landings resolve **immediately** — no one-frame grace: home-row entry snaps to the nearest column and resolves via `home_slot_at` (`Fill`/`Miss`/`Croc`); land rows re-snap the frog to the grid while water rows keep the platform drift. The pause gate follows the engine Pause Pattern exactly (only `Playing` is pausable; `ctx.time_scale` freezes particles; `Resumed` skips the rest of the frame).

**2-player co-op** shares the board, home slots, and pooled score but keeps per-frog lives, attempt timers, and respawn columns (`COOP_START_COLS`); a frog out of lives retires (hidden, no input) while the partner plays on, and the match ends only when all frogs are retired. Input follows the engine convention: 1P merges both player slots (WASD, arrows, and either pad all steer the one frog); co-op routes P1/P2 by frog index. Gameplay never reads raw `KeyCode`s (F1 debug is the sanctioned exception); menus use `MenuInput::read`.

**Chaos modes** map onto the family predicates: Insane (`is_insane`) = 1.5× lane speed + 25 s attempt timer; Ridiculous (`is_ridiculous`) = turtles dive on hash-phased cycles (submerged = not a platform) + a croc guarding one hash-picked home slot per round on an absent/present duty cycle (starts absent, so every round stays winnable); Insiculous = both. Speeds also ramp +15% per round, capped at 2×. All cycle logic is pure functions of `play_time`/`round` (`turtles_submerged`, `croc_slot`, `croc_present`) — no timers to desync. The board tint, grid, background, and menu chrome all derive from `ChaosTheme::for_mode`.

**Coordinate and scale conventions:**
- World origin is screen center. The window is `WIN_W × WIN_H` = 720×768: a 15×13 board of `TILE = 48` world-pixel tiles, plus a 72 px HUD `BAND` above (score/lives) and below (per-frog timer bars).
- `board::tile_center(col, row)` is the one mapping from board cells to world positions (row 0 = top, matching the Tilemap convention; the board is centered on the origin).
- The renderer multiplies `Transform2D.scale` by `RENDER_UNIT = 80.0` to get pixel size — sprite scales are `size / RENDER_UNIT` at spawn sites. With no physics there is no collider/sprite divergence to worry about; collision extents come straight from the same pixel constants (`FROG_HALF`, `LaneDef::half_len()`).
- Frog `x` is continuous (platforms drift it off-grid); `row` is discrete. Only obstacle positions wrap.

**All tuning lives in `src/constants.rs`** (board geometry, lane margins, speeds, timers, scoring, achievement thresholds, grid impulses) and all entity creation lives in `src/spawning.rs`. Every spawned entity gets a `Name` component ("Frog P1", "Truck r9 #0", "Board") so the editor hierarchy is readable — keep this for new entities. Particle looks are centralized in `effects.rs` (hop puff, cause-tinted death burst, home-fill shower).

**Localization:** fully localized (en + pirate) on the engine's `ctx.strings` pattern. Every player-facing string is a locale key resolved at draw time via `ctx.strings.tr(...)`; tables live in `assets/locales/{en,pirate}.ron` (loaded from the `locales` dir under the asset base set in main.rs). Both files MUST define the same key set — `locale_files_have_matching_keys` in achievements.rs enforces it. The title menu's Language item calls `ctx.strings.cycle_locale()` and then **re-registers achievements** (`achievements::register_all` — id-keyed insert refreshes names/descriptions from `ach.<id>.name`/`ach.<id>.desc` keys without touching unlock state). `pirate.ron` declares `font: Some("fonts/BlackSamsGold-ej5e.ttf")`, so switching locale also swaps the game font. The pause overlay localizes via `PauseMenu::draw_labeled` + `PauseMenuLabels`; chaos labels/hints come from `chaos_label_key`/`mode_hint_key` in menu.rs; achievement page sections use the `DISPLAY_SECTIONS` locale keys.

**Achievements:** ids are `&'static str` consts in `achievements.rs` (8 total), registered in `init()` with locale-table names, unlocked from `fill_home`/`clear_round` in flow.rs, persisted to `saves/frogger_achievements.json`. Tests pin the id↔locale-key contract and the display-section coverage.

**Tests (43):** `gameplay_tests.rs` (25 — torus wrap/straddle cases, every death path, croc + dive cycles, chaos scaling, lane-table sanity), `board.rs` (7 — row map, tileset strip, UVs), `achievements.rs` (6 — locale parity, re-register semantics), `spawning.rs` (4 — main/ghost pairs, naming, marker placement), `menu.rs` (1 — chaos label keys). All headless; run `cargo test` before claiming anything done.

**Paths:** assets and saves resolve through `engine_core::game_root!()` (exe dir if it contains `assets/`, else `CARGO_MANIFEST_DIR`), so `cargo run` works from any cwd. Input bindings persist to `saves/input_settings.json`.

## The Deion Re-skin (Phase G): Chicken Coop

Planned identity for this game under the Deion pivot — the current neon skin still ships today; nothing below is implemented yet.

- **New title: Chicken Coop.** The game goes chicken-themed, and every "why did the chicken cross the road" joke and pun is fair game — level names, achievements, the title screen can literally ask the question.
- **Player = a chicken** crossing the food traffic; co-op = **2 chickens** (the title pun: it's a co-op AND a chicken coop). Home slots become the coop's **nest boxes** (proposal: each filled slot shows a settled hen / an egg).
- This **supersedes the earlier DEION_STYLE §5 casting** (Deion hopping home to the ice-cube tray). Whether Deion/Cubert cameo at all (background, achievement art, a secret skin?) is TBD by Jesse.
- **Food-world traffic stays on-theme.** The existing §5 proposals — rolling food carts, soup river, celery/baguette logs, snapping hot-dog buns for crocs, cracker turtles — remain live proposals; the chicken replaces only the PLAYER and the home theming.
- **Style SSOT:** `deion_assets/DEION_STYLE.md` via the root symlink (the symlink assumes the standard side-by-side checkout — the same requirement the Cargo path dep already imposes). Settled metrics: 16 px base cell, nearest filtering, 5× integer scale to `RENDER_UNIT = 80`. The chicken is a **new character** needing Jesse's design and castings sign-off.
- **Asset intake rules:** runtime assets arrive ONLY via the deion_assets sync copy into `assets/sprites/` (F2 — not yet built); never symlink or hand-copy art in. AI art is quarantined (`ai_` prefix, `deion_assets/ai/` only) and NEVER ships; `deion_assets/scripts/check_no_ai_assets.sh` must pass on shipping asset trees. Sheet clip names are the stable API between art and code.
- The re-skin also **migrates the in-code RGBA tileset** (`board::tileset_pixels` + `create_texture_from_rgba`) to real tile sheets — roadmap F3 `gen_tiles` is the planned source.

## Review workflow

- The adversarial-review skill lives in `.claude/skills/` (author here, headless kimi as reviewer; prompts in `prompts/` are fixed — never edit them mid-review).
- Approved plans go to `review/plan.md` and are reviewed via `scripts/request-review.sh plan review/plan.md --reviewer=kimi` **before** implementation.
- Commits over 100 changed lines are gated by `scripts/commit-review-hook.sh` (PreToolUse hook in `.claude/settings.json`). The `ADV_REVIEWED=1` prefix is used only after a code-mode review adjudicated with the user, or when the user explicitly skipped review.
- `review/` holds gitignored transients (`plan.md`, `review-N.md`, `rebuttal-N.md`, `draft.diff`); fold anything durable into real docs, then clear it when the subject settles.
- NOTE: `scripts/request-review.sh` and `scripts/commit-review-hook.sh` are copies of `../../insiculous_2d/scripts/*` — re-copy when the engine master changes.
