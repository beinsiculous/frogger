# Chicken Coop

Game 6 of the 20 Games Challenge, built on the `insiculous_2d` engine, and the
**second of the six Deion re-skins**. Why did the chicken cross the road?
Because the road is a row of conveyor belts carrying food carts, the river is
tomato soup crossed on celery and baguette rafts, and the far side is five
nest boxes in a coop wall.

Run from this directory:

```bash
cargo run                    # play
cargo run --features editor  # play inside the scene editor
cargo test                   # 84 headless tests
```

The same build runs in the browser at [beinsiculous.com/playground/frogger/](https://beinsiculous.com/playground/frogger/): the game inside the editor, layout only — the rules are compiled in and nothing you change there persists.

## Controls

| Action | P1 | P2 | Gamepad |
|--------|----|----|---------|
| Hop | WASD | Arrows | D-pad / stick |
| Menus | W/S + Space | Arrows + Enter | D-pad + A |
| Pause | Esc | Esc | Start |

Single player: WASD, arrows, and either pad all steer the one chicken.
Co-op: two chickens at once — shared nests and score, individual lives.

## Mechanics

- 13-row board: start row, 5 belt lanes, median, 5 soup lanes, home row with 5
  nests. Fill all five to clear the round; each round the belts speed up
  (capped at 2x).
- Carts kill on touch. Soup kills unless you're on a raft or a cracker group;
  platforms carry you sideways, and riding off the board edge kills. Every
  attempt runs on a timer (bars in the bottom band).
- Lanes scroll on a torus: obstacles wrap seamlessly across the edges
  (collision uses modular distance, so edge-straddling carts still hit).
- **The drawn danger always covers the real one.** A cracker that looks like
  it is rising is already rideable, and one that looks like it is sinking is
  not yet lethal; the bun in a nest warns before it snaps. So the art can never
  tell you a lane is safe when it is not.
- **A round clear is a beat.** The fifth arrival celebrates for a second with
  the traffic still moving and no input, then the round advances in one frame —
  the nests empty and the seated chickens go with them.

## Localization

Fully localized (English + Pirate) following the engine's `ctx.strings`
pattern: locale tables in `assets/locales/*.ron`, a `Language` title-menu
item that cycles locales live (achievements re-register with translated
names, unlock state untouched), and a per-locale font (Pirate uses
BlackSamsGold).

## Chaos modes

- **Normal** — the classic crossing.
- **Insane** — carts 50% faster, 25-second timer.
- **Ridiculous** — crackers sink on staggered cycles, and a snapping bun
  periodically guards one nest (enter while it is open and you're lunch;
  it closes again — every round stays winnable).
- **Insiculous** — all of the above at once.

## The Deion re-skin

The neon frog build is gone; this is the Chicken Coop build.

- **The player is a chicken.** The oldest joke in the world, finally playable.
- **Co-op is the pun.** Two players = two chickens = a co-op that is, in fact,
  a chicken coop. Shared nests and score, individual lives.
- **The cast is signed off.** The round chicken is P1 and the tall one P2 (they
  are drawn *and* collided with differently — see `CHICKEN_HALF`); the traffic
  is sushi, hot dog and donut carts; the rafts are celery and baguette; the
  crackers sink; the bun snaps; the nests are breadboxes.
- **No cameo, no egg mechanics.** Deion and Cubert do not appear, and eggs stay
  visual — the nest's own broadcast flag. Both are filed as issues rather than
  half-built.
- **Art rules:** style SSOT is `deion_assets/DEION_STYLE.md` (the
  `deion_assets -> ../../deion_assets` symlink, read-only). 16 px base cell,
  nearest filtering, one art pixel per window pixel (`RENDER_UNIT = 80`).
  Runtime assets arrive only via the sync into `assets/sprites/`; AI art is
  quarantined in `deion_assets/ai/`; it may ship in FREE web builds, never in
  paid/marketplace builds (DEION_STYLE.md §6, tiered Aug 19 2026).
- **The in-code RGBA tileset is gone.** The board is four `Tilemap` entities,
  one per real tile sheet, with the soup and the belts animating off the clip
  their sidecar declares.

## What was learned

- `Tilemap` + `Transform2D` renders through the engine's default render path
  with zero game-side render code — one map per tileset batches into a single
  draw, so four maps cost four draws and buy four sheets.
- A measured sheet's opaque box is the one number everything downstream can
  rest on: the draw anchor, the collision extent, and the composition of a
  seated sprite inside a prop all come off the same measurement, so none of
  them can drift from the PNG.
- No physics crate at all (Snake precedent): pure-function lane math
  (`gameplay/rules.rs`) made every death path, the dive and duty cycles, and the
  round-winnability guarantee unit-testable headlessly.
- **Pose the art from the rules, don't machine it.** A clip played by a state
  machine drifts from the phase the rules run on, because a round change
  re-hashes that phase mid-clip. A pure function of the phase cannot drift —
  and it is what makes "the drawn danger always covers the real one" true by
  construction rather than by careful timing.
