# Insiculous Frogger

Game 6 of the 20 Games Challenge, built on the `insiculous_2d` engine — and
the engine's **first Tilemap consumer**: the whole board (grass, road, water,
home slots) is one `Tilemap` entity over a procedural 4-cell tileset strip
built at runtime with `AssetManager::create_texture_from_rgba` and tinted by
the chaos theme.

Run from this directory:

```bash
cargo run                    # play
cargo run --features editor  # play inside the scene editor
cargo test                   # 43 headless tests
```

## Controls

| Action | P1 | P2 | Gamepad |
|--------|----|----|---------|
| Hop | WASD | Arrows | D-pad / stick |
| Menus | W/S + Space | Arrows + Enter | D-pad + A |
| Pause | Esc | Esc | Start |

Single player: WASD, arrows, and either pad all steer the one frog.
Co-op: two frogs at once — shared home slots and score, individual lives.

## Mechanics

- 13-row board: start row, 5 traffic lanes, median, 5 water lanes, home row
  with 5 slots. Fill all five to clear the round; each round the lanes speed
  up (capped at 2x).
- Traffic kills on touch. Water kills unless you're on a log or turtle
  group; platforms carry you sideways, and riding off the board edge kills.
  Every attempt runs on a timer (bars in the bottom band).
- Lanes scroll on a torus: obstacles wrap seamlessly across the edges
  (collision uses modular distance, so edge-straddling trucks still hit).

## Localization

Fully localized (English + Pirate) following the engine's `ctx.strings`
pattern: locale tables in `assets/locales/*.ron`, a `Language` title-menu
item that cycles locales live (achievements re-register with translated
names, unlock state untouched), and a per-locale font (Pirate uses
BlackSamsGold).

## Chaos modes

- **Normal** — the classic crossing.
- **Insane** — traffic 50% faster, 25-second timer.
- **Ridiculous** — turtles dive on staggered cycles, and a crocodile
  periodically guards one home slot (enter while it's surfaced and you're
  lunch; it leaves — every round stays winnable).
- **Insiculous** — all of the above at once.

## The Deion Pivot: Chicken Coop

Why did the chicken cross the road? Because in Phase G this game becomes
**Chicken Coop** — the planned re-skin under the Deion pivot. (The neon
frog skin above is what ships today; none of this is implemented yet.)

- **The player is a chicken.** The oldest joke in the world, finally
  playable: a chicken crossing the food-world traffic to get to the other
  side. The title screen is allowed — encouraged — to literally ask the
  question. Level names, achievements, and flavor text should mine every
  crossing-the-road pun they can carry.
- **Co-op is the pun.** Two players = two chickens = a co-op that is,
  in fact, a chicken coop. Same mechanics as today: shared nest boxes and
  score, individual lives.
- **Home slots become the coop's nest boxes.** Proposal: each filled slot
  shows a settled hen (or an egg) instead of today's glowing marker.
- **The food traffic stays.** The DEION_STYLE §5 traffic proposals remain
  live: road lanes as conveyor belts of rolling food carts (sushi rolls,
  hot dogs, donuts), the river as soup, celery-stick and baguette logs,
  snapping hot-dog buns for crocs, cracker turtles that sink into the
  broth. The chicken replaces only the player and the home-row theming —
  this supersedes §5's original casting (Deion hopping to the ice-cube
  tray).
- **Art rules:** style SSOT is `deion_assets/DEION_STYLE.md` (root
  symlink, read-only; assumes the standard side-by-side checkout). 16 px
  base cell, nearest filtering, 5x integer scale to `RENDER_UNIT = 80`.
  Runtime assets arrive only via the F2 sync into `assets/sprites/`; AI
  art is quarantined in `deion_assets/ai/` and never ships. The re-skin
  also retires the in-code RGBA tileset in favor of real tile sheets
  (roadmap F3 `gen_tiles`).

**Open questions** (answered questions move up into the theme spec above
and get DELETED from this list — live-docs convention):

- Does Deion/Cubert cameo at all (background, achievement art, a secret
  skin)? Jesse's call.
- Final food-traffic castings — which of the §5 proposals ship?
- The chicken's design — a new character needing Jesse's drawing and
  castings sign-off.
- Do the 2P chickens get distinct looks (e.g. hen/rooster)?
- Egg mechanics — do eggs do anything (scoring, extra lives, laid on
  logs?) or stay purely visual?

## What was learned

- `Tilemap` + `Transform2D` renders through the engine's default render path
  with zero game-side render code; one 15x13 map batches into a single draw.
- A per-tile-color tilemap isn't needed when a tiny procedural tileset strip
  (nearest-filtered so cells don't bleed) does the job.
- No physics crate at all (Snake precedent): pure-function lane math
  (`gameplay/rules.rs`) made every death path, the croc duty cycle, and the
  round-winnability guarantee unit-testable headlessly.
