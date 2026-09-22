//! All entity creation. Chicken Coop needs no physics — every entity is a plain
//! sprite (Name + Transform2D + Sprite) or one of the board's `Tilemap`s; all
//! collision is pure lane math in `gameplay/rules.rs`.
//!
//! Every lane sprite spawns as a (main, ghost) pair: the ghost sits one torus
//! period away and shows only while its segment straddles a window edge, so
//! lanes scroll seamlessly with no visual/collision mismatch.
//!
//! The crackers and the bun are the exception to the machine: they carry an
//! animation and no `ClipStateMachine`, and the rules pose them frame by
//! frame. A spawn writes the first pose through the same functions, because
//! two of the three ways a match starts never reach the per-frame sync and an
//! unposed animation would draw the wrong frame for one.

use engine_core::prelude::*;

use crate::board::{build_map, tile_center, Terrain};
use crate::constants::*;
use crate::gameplay::pose_chicken;
use crate::gameplay::poses::{bun_pose, pose_lane_crackers, write_pose, Pose};
use crate::gameplay::rules::{bun_nest, initial_lane_positions};
use crate::types::*;

/// The components every animated art entity carries: the sheet's first cell as
/// its sprite, the sheet's clips as its animation, and the anchor offset that
/// puts the reference frame's art on the entity.
fn art_components(sheet: &SpriteSheet, spec: &SheetSpec, depth: f32) -> (Sprite, SpriteAnimation) {
    (
        sheet.sprite().with_offset(spec.sprite_offset()).with_depth(depth),
        sheet.animation(),
    )
}

/// A chicken's states: the two poses in each of the four facings. A hop moves
/// the chicken a whole tile in one frame, so `jump` is a one-shot back into
/// the idle of the same facing — a hop in one direction never leaves it
/// facing another.
pub(crate) fn player_machine() -> ClipStateMachine {
    let mut states = Vec::with_capacity(Facing::ALL.len() * 2);
    for facing in Facing::ALL {
        let idle = facing.idle_state();
        let jump = facing.jump_state();
        states.push((idle.clone(), ClipState::staying(idle.clone())));
        states.push((jump.clone(), ClipState::new(jump, OnFinished::Next(idle))));
    }
    ClipStateMachine::new(Facing::START.idle_state(), states)
}

/// A nest's states: it stands empty, plays its arrival once as a chicken
/// lands, then holds the occupied pose for the rest of the round. The seated
/// chicken is a separate entity, so the nest itself never carries one.
pub(crate) fn nest_machine() -> ClipStateMachine {
    ClipStateMachine::new(
        NEST_EMPTY,
        vec![
            (NEST_EMPTY.to_string(), ClipState::staying(NEST_EMPTY)),
            (
                NEST_ARRIVAL.to_string(),
                ClipState::new(NEST_ARRIVAL, OnFinished::Next(NEST_OCCUPIED.to_string())),
            ),
            (NEST_OCCUPIED.to_string(), ClipState::staying(NEST_OCCUPIED)),
        ],
    )
}

/// Spawn one player's chicken, resting in its start facing's idle. Position is
/// set by the flow when a life starts.
pub(crate) fn spawn_player(world: &mut World, sheets: &Sheets, index: usize) -> EntityId {
    let spec = player_spec(index);
    let sheet = sheets.player(index);
    let (sprite, animation) = art_components(sheet, spec, PLAYER_DEPTH);
    world
        .spawn()
        .with(Name::new(format!("Chicken P{}", index + 1)))
        .with(Transform2D::from_parts(Vec2::ZERO, 0.0, spec.scale()))
        .with(sprite)
        .with(animation)
        .with(player_machine())
        .id()
}

/// The measured sheet one player's chicken draws from: P1 the round chicken,
/// P2 the tall one.
pub(crate) fn player_spec(index: usize) -> &'static SheetSpec {
    if index == 0 {
        &CHICKEN_ROUND
    } else {
        &CHICKEN_TALL
    }
}

/// Spawn one lane's segments at their evenly-spaced torus positions, each as
/// its sprites' own (main, ghost) pairs. `play_time` poses any cracker the
/// lane carries, so the first frame of a match draws the right clip.
pub(crate) fn spawn_lane(
    world: &mut World,
    sheets: &Sheets,
    def: LaneDef,
    mode: ChaosMode,
    play_time: f32,
) -> LaneState {
    let xs = initial_lane_positions(def.count);
    let y = tile_center(0, def.row).y;
    let sprites = xs
        .iter()
        .enumerate()
        .map(|(segment, &x)| {
            (0..def.segment.sprites)
                .map(|sprite| {
                    let center = x + def.sprite_offset(sprite);
                    spawn_lane_sprite(world, sheets, &def, segment, sprite, center, y)
                })
                .collect()
        })
        .collect();
    let state = LaneState { def, xs, sprites };
    pose_lane_crackers(world, &state, mode, play_time);
    state
}

/// One sprite of a segment and its seam ghost, named for the editor.
fn spawn_lane_sprite(
    world: &mut World,
    sheets: &Sheets,
    def: &LaneDef,
    segment: usize,
    sprite: usize,
    center: f32,
    y: f32,
) -> (EntityId, EntityId) {
    let lane_sprite = def.segment.sprite;
    let spec = lane_sprite.spec();
    let sheet = sheets.lane(lane_sprite);
    let (main_sprite, animation) = art_components(sheet, spec, lane_sprite.depth());
    let name = format!("{} r{} #{segment}.{sprite}", lane_sprite.label(), def.row);

    let main = world
        .spawn()
        .with(Name::new(name.clone()))
        .with(Transform2D::from_parts(Vec2::new(center, y), 0.0, spec.scale()))
        .with(main_sprite)
        .with(animation)
        .id();
    // Ghost: same look, one period away, hidden until straddling.
    let ghost = world
        .spawn()
        .with(Name::new(format!("{name} ghost")))
        .with(Transform2D::from_parts(
            Vec2::new(center - LANE_PERIOD, y),
            0.0,
            spec.scale(),
        ))
        .with(
            sheet
                .sprite()
                .with_offset(spec.sprite_offset())
                .with_depth(lane_sprite.depth())
                .with_visible(false),
        )
        .with(sheet.animation())
        .id();

    // A posed sprite is left to `pose_crackers`, which gives it and its ghost
    // the same frame so a straddling cracker does not change pose at the seam.
    if !lane_sprite.is_posed() {
        let clip = lane_sprite.travelling_clip();
        for entity in [main, ghost] {
            if let Some(animation) = world.get_mut::<SpriteAnimation>(entity) {
                let _ = animation.play(clip);
            }
        }
    }
    (main, ghost)
}

/// The five nests, standing on the home row from the first frame of a match.
pub(crate) fn spawn_nests(world: &mut World, sheets: &Sheets) -> [Option<EntityId>; 5] {
    std::array::from_fn(|slot| {
        let spec = &NEST;
        let (sprite, animation) = art_components(&sheets.nest, spec, NEST_DEPTH);
        Some(
            world
                .spawn()
                .with(Name::new(format!("Nest {slot}")))
                .with(Transform2D::from_parts(nest_center(slot), 0.0, spec.scale()))
                .with(sprite)
                .with(animation)
                .with(nest_machine())
                .id(),
        )
    })
}

/// A nest's world position: centred on its home column, with its canvas's
/// bottom edge on the home row's bottom edge, so the drawn nest rises into the
/// top band over the wall.
pub(crate) fn nest_center(slot: usize) -> Vec2 {
    Vec2::new(tile_center(HOME_COLS[slot], HOME_ROW).x, NEST_Y)
}

/// The chicken seated in nest `slot` — the arriving player's own chicken,
/// idling south on the nest's straw.
pub(crate) fn spawn_seated(
    world: &mut World,
    sheets: &Sheets,
    player: usize,
    slot: usize,
) -> EntityId {
    let spec = player_spec(player);
    let sheet = sheets.player(player);
    let (sprite, mut animation) = art_components(sheet, spec, SEATED_DEPTH);
    // The one clip a seated chicken ever shows; with no machine of its own it
    // simply loops, as it would on the board.
    let _ = animation.play(&Facing::South.idle_state());
    world
        .spawn()
        .with(Name::new(format!("Seated {slot}")))
        .with(Transform2D::from_parts(
            composition_position(slot, spec),
            0.0,
            spec.scale(),
        ))
        .with(sprite)
        .with(animation)
        .id()
}

/// Where a sprite's cell sits on a nest: the nest's own canvas carries the
/// composition (no chicken pixels live in the prop), and the sprite's cell
/// centre lands on the canvas's straw anchor.
pub(crate) fn composition_position(slot: usize, spec: &SheetSpec) -> Vec2 {
    let canvas_top_left = nest_center(slot)
        + NEST.sprite_offset()
        + Vec2::new(-NEST.cell.x / 2.0, NEST.cell.y / 2.0);
    // The anchor names a point on that canvas — and canvas y grows downward,
    // where world y grows up.
    let anchor = canvas_top_left + Vec2::new(NEST_COMPOSITION_ANCHOR.x, -NEST_COMPOSITION_ANCHOR.y);
    anchor - spec.sprite_offset()
}

/// Everything the bun's own rules decide about it at one instant: which nest
/// it guards, where on that nest's straw it stands, and the pose — or nothing
/// at all, while it draws nothing.
///
/// One rule, asked by both the spawn and the per-frame sync, so the frame a
/// match spawns already shows what the next frame would: two of the three ways
/// a match starts reach a rendered frame before the sync runs.
pub(crate) struct BunPlacement {
    /// Its world position: the guarded nest's composition anchor. With no nest
    /// to guard it is nest 0's, where it is never drawn.
    pub(crate) position: Vec2,
    /// The clip and frame it shows, or `None` while it is hidden — away from
    /// its nest in the duty cycle, or looking at a nest that is already filled.
    pub(crate) pose: Option<Pose>,
}

impl BunPlacement {
    /// Whether the bun is drawn at all this instant.
    pub(crate) fn is_visible(&self) -> bool {
        self.pose.is_some()
    }
}

/// What the bun is and shows at `play_time`, given the nests' fill state.
pub(crate) fn bun_placement(
    mode: ChaosMode,
    round: u32,
    play_time: f32,
    homes: &[bool; 5],
) -> BunPlacement {
    let nest = bun_nest(mode, round);
    let slot_filled = nest.is_some_and(|slot| homes[slot]);
    let pose = nest.and_then(|_| bun_pose(mode, round, play_time, slot_filled));
    BunPlacement { position: composition_position(nest.unwrap_or(0), &BUN), pose }
}

/// The snapping bun (Ridiculous family only). It surfaces inside the nest it
/// guards, and both the spawn and the per-frame sync place it and pose it
/// through [`bun_placement`], so the spawn frame is already right.
pub(crate) fn spawn_bun(
    world: &mut World,
    sheets: &Sheets,
    mode: ChaosMode,
    round: u32,
    play_time: f32,
    homes: &[bool; 5],
) -> EntityId {
    let placement = bun_placement(mode, round, play_time, homes);
    let spec = &BUN;
    let (sprite, animation) = art_components(&sheets.bun, spec, SEATED_DEPTH);
    let entity = world
        .spawn()
        .with(Name::new("Bun"))
        .with(Transform2D::from_parts(placement.position, 0.0, spec.scale()))
        .with(sprite.with_visible(placement.is_visible()))
        .with(animation)
        .id();
    if let Some(pose) = placement.pose {
        write_pose(world, entity, pose);
    }
    entity
}

/// Stand `entity` on the start tile of `start_col`: where the next sync would
/// put it, visible, and idling north. A chicken is not born where it stands
/// until this runs, and a spawn's own frame draws before any sync does.
pub(crate) fn place_on_start_tile(world: &mut World, entity: EntityId, start_col: u32) {
    let position = Vec2::new(tile_center(start_col, START_ROW).x, tile_center(0, START_ROW).y);
    if let Some(transform) = world.get_mut::<Transform2D>(entity) {
        transform.position = position;
    }
    if let Some(sprite) = world.get_mut::<Sprite>(entity) {
        sprite.visible = true;
    }
    pose_chicken(world, entity, Facing::START, false);
}

/// One icon per life, per player, in the bottom band — `round` for P1 and
/// `tall` for P2. Hidden as lives run out, so the row is the count.
pub(crate) fn spawn_life_icons(
    world: &mut World,
    sheets: &Sheets,
    mode: GameMode,
) -> Vec<EntityId> {
    let mut icons = Vec::new();
    for player in 0..mode.player_count() {
        for life in 0..STARTING_LIVES as usize {
            let spec = &LIFE_ICON;
            let (sprite, mut animation) = art_components(&sheets.life_icon, spec, LIFE_ICON_DEPTH);
            let _ = animation.play(Sheets::life_icon_clip(player));
            icons.push(
                world
                    .spawn()
                    .with(Name::new(format!("Life P{} {life}", player + 1)))
                    .with(Transform2D::from_parts(
                        life_icon_position(mode, player, life),
                        0.0,
                        spec.scale(),
                    ))
                    .with(sprite)
                    .with(animation)
                    .id(),
            );
        }
    }
    icons
}

/// Where life `life` of `player` sits: the row centred over that player's
/// band slot, one life per pitch.
pub(crate) fn life_icon_position(mode: GameMode, player: usize, life: usize) -> Vec2 {
    let slot_width = WIN_W / mode.player_count() as f32;
    let slot_center = -WIN_W / 2.0 + slot_width * (player as f32 + 0.5);
    let first = -(STARTING_LIVES as f32 - 1.0) / 2.0;
    Vec2::new(slot_center + (life as f32 + first) * LIFE_ICON_PITCH, LIFE_ICON_Y)
}

/// The board: one `Tilemap` per sheet, all anchored at the center of tile
/// (0, 0). The soup and the belts are animated by `board::animate`.
pub(crate) fn spawn_board(world: &mut World, sheets: &Sheets) -> Vec<BoardMap> {
    Terrain::ALL
        .iter()
        .map(|&terrain| {
            let entity = world
                .spawn()
                .with(Name::new(format!("Board {}", terrain.label())))
                .with(Transform2D::from_parts(tile_center(0, 0), 0.0, Vec2::ONE))
                .with(build_map(terrain, terrain.sheet(sheets), 1))
                .id();
            BoardMap { terrain, entity }
        })
        .collect()
}

/// Spawn the deforming grid the board is drawn under: the engine simulates
/// and draws it, gameplay events ripple it.
pub(crate) fn spawn_backdrop(world: &mut World, theme: &ChaosTheme) -> EntityId {
    world
        .spawn()
        .with(Name::new("Grid Backdrop"))
        .with(Transform2D::new(Vec2::ZERO))
        .with(GridBackdrop {
            draw_order: GridDrawOrder::OverSprites,
            color: backdrop_color(theme),
            ..GridBackdrop::default()
        })
        .id()
}

/// The grid's own colour at low alpha: the theme decides the hue, and the
/// lattice reads over the art without veiling it.
pub(crate) fn backdrop_color(theme: &ChaosTheme) -> Vec4 {
    let grid = theme.grid_color;
    Vec4::new(grid.x, grid.y, grid.z, BACKDROP_ALPHA)
}

/// Spawn a detached one-shot at `position` — a death's feather poof, or the
/// burst a fifth arrival leaves. Sprite-only, and it despawns itself when its
/// clip runs out; the caller keeps the handle so a reset can end it early.
pub(crate) fn spawn_oneshot(
    world: &mut World,
    name: &str,
    spec: &SheetSpec,
    sheet: &SpriteSheet,
    position: Vec2,
    clip: &str,
) -> EntityId {
    let (sprite, mut animation) = art_components(sheet, spec, EFFECT_DEPTH);
    let _ = animation.play(clip);
    world
        .spawn()
        .with(Name::new(name))
        .with(Transform2D::from_parts(position, 0.0, spec.scale()))
        .with(sprite)
        .with(animation)
        .with(ClipStateMachine::new(
            clip,
            vec![(clip.to_string(), ClipState::new(clip, OnFinished::Despawn))],
        ))
        .id()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_lane_creates_a_main_and_a_ghost_per_sprite() {
        let mut world = World::new();
        let sheets = Sheets::default();
        for def in crate::gameplay::rules::LANES {
            let lane = spawn_lane(&mut world, &sheets, def, ChaosMode::Normal, 0.0);
            assert_eq!(lane.xs.len(), def.count);
            assert_eq!(lane.sprites.len(), def.count);
            for (segment, pairs) in lane.sprites.iter().enumerate() {
                assert_eq!(pairs.len(), def.segment.sprites, "row {} segment {segment}", def.row);
                for &(main, ghost) in pairs {
                    assert!(world.get::<Sprite>(main).unwrap().visible);
                    assert!(!world.get::<Sprite>(ghost).unwrap().visible);
                }
            }
        }
    }

    #[test]
    fn test_every_lane_entity_is_named() {
        let mut world = World::new();
        let sheets = Sheets::default();
        for def in crate::gameplay::rules::LANES {
            spawn_lane(&mut world, &sheets, def, ChaosMode::Normal, 0.0);
        }
        for entity in world.entities() {
            assert!(world.get::<Name>(entity).is_some(), "unnamed entity {entity:?}");
        }
    }

    #[test]
    fn test_a_lane_segment_spreads_its_sprites_one_cell_apart() {
        let mut world = World::new();
        let sheets = Sheets::default();
        let raft = crate::gameplay::rules::LANES
            .iter()
            .find(|def| def.segment.sprites > 1)
            .copied()
            .expect("a lane carries a multi-sprite segment");
        let lane = spawn_lane(&mut world, &sheets, raft, ChaosMode::Normal, 0.0);
        let cell = raft.segment.sprite.spec().cell.x;
        for (segment, pairs) in lane.sprites.iter().enumerate() {
            let centers: Vec<f32> = pairs
                .iter()
                .map(|&(main, _)| world.get::<Transform2D>(main).unwrap().position.x)
                .collect();
            assert_eq!(centers.len(), raft.segment.sprites);
            for pair in centers.windows(2) {
                assert_eq!(pair[1] - pair[0], cell, "segment {segment} sprites abut");
            }
        }
    }

    #[test]
    fn test_nests_stand_on_the_home_row_from_the_first_frame() {
        let mut world = World::new();
        let sheets = Sheets::default();
        let nests = spawn_nests(&mut world, &sheets);
        for (slot, entity) in nests.iter().enumerate() {
            let entity = entity.expect("every nest is spawned");
            let transform = world.get::<Transform2D>(entity).unwrap();
            assert_eq!(transform.position, nest_center(slot));
            assert_eq!(transform.position.x, tile_center(HOME_COLS[slot], HOME_ROW).x);
            assert_eq!(transform.position.y, NEST_Y);
            let machine = world.get::<ClipStateMachine>(entity).expect("a nest machine");
            assert_eq!(machine.state(), NEST_EMPTY);
        }
    }

    #[test]
    fn test_a_seated_chicken_lands_on_its_nest_straw() {
        let mut world = World::new();
        let sheets = Sheets::default();
        // The composition anchor is a point on the nest's canvas: a sprite's
        // cell centre lands on it, whichever sheet it comes from.
        for player in 0..2 {
            let spec = player_spec(player);
            let seated = spawn_seated(&mut world, &sheets, player, 0);
            let position = world.get::<Transform2D>(seated).unwrap().position;
            let cell_center = position + spec.sprite_offset();
            let canvas_top_left = nest_center(0)
                + NEST.sprite_offset()
                + Vec2::new(-NEST.cell.x / 2.0, NEST.cell.y / 2.0);
            let anchor_world = canvas_top_left
                + Vec2::new(NEST_COMPOSITION_ANCHOR.x, -NEST_COMPOSITION_ANCHOR.y);
            assert_eq!(cell_center, anchor_world, "player {player}'s chicken sits on the straw");
        }
    }

    #[test]
    fn test_the_chicken_composition_anchor_is_the_one_the_nest_names() {
        // The section's anchor, in the nest canvas's own coordinates: the
        // chicken's 48 px cell has its top-left at (24, 22).
        let chicken = player_spec(0);
        let top_left =
            NEST_COMPOSITION_ANCHOR - Vec2::new(chicken.cell.x / 2.0, chicken.cell.y / 2.0);
        assert_eq!(top_left, Vec2::new(24.0, 22.0));
    }

    #[test]
    fn test_the_life_icon_row_is_centred_over_each_players_slot() {
        let solo = GameMode::SinglePlayer;
        let row: Vec<f32> = (0..STARTING_LIVES as usize)
            .map(|life| life_icon_position(solo, 0, life).x)
            .collect();
        assert_eq!(row.len(), 3);
        assert!((row[1] - row[0] - LIFE_ICON_PITCH).abs() < f32::EPSILON);
        let center = (row[0] + row[2]) / 2.0;
        assert!(center.abs() < f32::EPSILON, "solo's row is centred on the window");

        let coop = GameMode::TwoPlayerCoop;
        let p1 = life_icon_position(coop, 0, 0).x;
        let p2 = life_icon_position(coop, 1, 0).x;
        assert!(p1 < 0.0 && p2 > 0.0, "co-op splits the band between the players");
        assert!((p2 - p1 - WIN_W / 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_the_board_is_four_maps_that_do_not_overlap() {
        let mut world = World::new();
        let sheets = Sheets::default();
        let maps = spawn_board(&mut world, &sheets);
        assert_eq!(maps.len(), 4);
        for map in &maps {
            assert!(world.get::<Tilemap>(map.entity).is_some());
            assert!(world.get::<Name>(map.entity).is_some());
        }
        let terrains: Vec<Terrain> = maps.iter().map(|map| map.terrain).collect();
        assert_eq!(terrains, Terrain::ALL.to_vec());
    }
}
