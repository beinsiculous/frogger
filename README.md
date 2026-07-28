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
cargo test                   # 40 headless tests
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

## What was learned

- `Tilemap` + `Transform2D` renders through the engine's default render path
  with zero game-side render code; one 15x13 map batches into a single draw.
- A per-tile-color tilemap isn't needed when a tiny procedural tileset strip
  (nearest-filtered so cells don't bleed) does the job.
- No physics crate at all (Snake precedent): pure-function lane math
  (`gameplay/rules.rs`) made every death path, the croc duty cycle, and the
  round-winnability guarantee unit-testable headlessly.
