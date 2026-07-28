//! All entity creation. Frogger needs no physics — every entity is a plain
//! sprite (Name + Transform2D + Sprite) or the board `Tilemap`; all
//! collision is pure lane math in `gameplay/rules.rs`.
//!
//! Every obstacle spawns as a (main, ghost) sprite pair: the ghost sits one
//! torus period away and shows only while the obstacle straddles a window
//! edge, so lanes scroll seamlessly with no visual/collision mismatch.

use engine_core::prelude::*;
use glam::Vec4;

use crate::board::{build_board, tile_center};
use crate::constants::*;
use crate::gameplay::rules::initial_lane_positions;
use crate::types::*;

const CAR_COLORS: [Vec4; 3] = [
    Vec4::new(1.0, 0.35, 0.3, 1.0),
    Vec4::new(1.0, 0.85, 0.3, 1.0),
    Vec4::new(0.4, 0.85, 1.0, 1.0),
];
const TRUCK_COLOR: Vec4 = Vec4::new(0.82, 0.82, 0.9, 1.0);
const LOG_COLOR: Vec4 = Vec4::new(0.58, 0.36, 0.18, 1.0);
const TURTLE_COLOR: Vec4 = Vec4::new(0.25, 0.8, 0.5, 1.0);
const OBSTACLE_EMISSIVE: f32 = 1.2;
const MARKER_EMISSIVE: f32 = 1.8;
/// Visual gap so back-to-back obstacles read as separate vehicles.
const OBSTACLE_GAP: f32 = 6.0;

fn obstacle_color(kind: LaneKind, index: usize) -> Vec4 {
    match kind {
        LaneKind::Car => CAR_COLORS[index % CAR_COLORS.len()],
        LaneKind::Truck => TRUCK_COLOR,
        LaneKind::Log => LOG_COLOR,
        LaneKind::Turtles => TURTLE_COLOR,
    }
}

/// Spawn one frog sprite (position is set by the flow when a life starts).
pub(crate) fn spawn_frog(world: &mut World, tex: u32, player: usize) -> EntityId {
    let color = if player == 0 { FROG1_COLOR } else { FROG2_COLOR };
    world.spawn()
        .with(Name::new(format!("Frog P{}", player + 1)))
        .with(Transform2D::from_parts(Vec2::ZERO, 0.0, Vec2::splat(FROG_SIZE / RENDER_UNIT)))
        .with(Sprite::new(tex).with_color(color).with_emissive(FROG_EMISSIVE))
        .id()
}

/// Spawn one lane's obstacles at their evenly-spaced torus positions.
pub(crate) fn spawn_lane(world: &mut World, tex: u32, def: LaneDef) -> LaneState {
    let xs = initial_lane_positions(def.count);
    let y = tile_center(0, def.row).y;
    let size = Vec2::new(def.len_tiles * TILE - OBSTACLE_GAP, OBSTACLE_H);
    let sprites = xs
        .iter()
        .enumerate()
        .map(|(i, &x)| {
            let color = obstacle_color(def.kind, i);
            let name = format!("{:?} r{} #{i}", def.kind, def.row);
            let main = world.spawn()
                .with(Name::new(name.clone()))
                .with(Transform2D::from_parts(Vec2::new(x, y), 0.0, size / RENDER_UNIT))
                .with(Sprite::new(tex).with_color(color).with_emissive(OBSTACLE_EMISSIVE))
                .id();
            // Ghost: same look, one period away, hidden until straddling.
            let ghost = world.spawn()
                .with(Name::new(format!("{name} ghost")))
                .with(Transform2D::from_parts(
                    Vec2::new(x - LANE_PERIOD, y), 0.0, size / RENDER_UNIT))
                .with(Sprite::new(tex)
                    .with_color(color)
                    .with_emissive(OBSTACLE_EMISSIVE)
                    .with_visible(false))
                .id();
            (main, ghost)
        })
        .collect();
    LaneState { def, xs, sprites }
}

/// Spawn the five home fill markers (hidden until their slot fills).
pub(crate) fn spawn_home_markers(world: &mut World, tex: u32) -> Vec<EntityId> {
    HOME_COLS
        .iter()
        .enumerate()
        .map(|(i, &col)| {
            world.spawn()
                .with(Name::new(format!("Home marker {i}")))
                .with(Transform2D::from_parts(
                    tile_center(col, HOME_ROW), 0.0, Vec2::splat(FROG_SIZE / RENDER_UNIT)))
                .with(Sprite::new(tex)
                    .with_color(FROG1_COLOR)
                    .with_emissive(MARKER_EMISSIVE)
                    .with_visible(false))
                .id()
        })
        .collect()
}

/// Spawn the croc sprite (Ridiculous family). Placed onto the guarded slot
/// each round; visible only while the croc is surfaced.
pub(crate) fn spawn_croc(world: &mut World, tex: u32) -> EntityId {
    world.spawn()
        .with(Name::new("Croc"))
        .with(Transform2D::from_parts(
            tile_center(HOME_COLS[0], HOME_ROW), 0.0,
            Vec2::new(TILE - 6.0, OBSTACLE_H) / RENDER_UNIT))
        .with(Sprite::new(tex)
            .with_color(CROC_COLOR)
            .with_emissive(OBSTACLE_EMISSIVE)
            .with_visible(false))
        .id()
}

/// Spawn the board tilemap entity anchored at the center of tile (0, 0).
pub(crate) fn spawn_board(world: &mut World, tileset: u32) -> EntityId {
    world.spawn()
        .with(Name::new("Board"))
        .with(Transform2D::from_parts(tile_center(0, 0), 0.0, Vec2::ONE))
        .with(build_board(tileset))
        .id()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gameplay::rules::LANES;

    #[test]
    fn test_spawn_lane_creates_main_and_ghost_per_obstacle() {
        let mut world = World::new();
        let lane = spawn_lane(&mut world, 0, LANES[0]);
        assert_eq!(lane.xs.len(), LANES[0].count);
        assert_eq!(lane.sprites.len(), LANES[0].count);
        // Ghosts start hidden; mains start visible.
        for &(main, ghost) in &lane.sprites {
            assert!(world.get::<Sprite>(main).unwrap().visible);
            assert!(!world.get::<Sprite>(ghost).unwrap().visible);
        }
    }

    #[test]
    fn test_all_lane_entities_are_named() {
        let mut world = World::new();
        for def in LANES {
            spawn_lane(&mut world, 0, def);
        }
        for entity in world.entities() {
            assert!(world.get::<Name>(entity).is_some(), "unnamed entity {entity:?}");
        }
    }

    #[test]
    fn test_home_markers_sit_on_home_slots_hidden() {
        let mut world = World::new();
        let markers = spawn_home_markers(&mut world, 0);
        assert_eq!(markers.len(), HOME_COLS.len());
        for (i, &m) in markers.iter().enumerate() {
            assert!(!world.get::<Sprite>(m).unwrap().visible);
            let t = world.get::<Transform2D>(m).unwrap();
            assert_eq!(t.position, tile_center(HOME_COLS[i], HOME_ROW));
        }
    }

    #[test]
    fn test_board_entity_carries_the_tilemap_at_the_anchor() {
        let mut world = World::new();
        let board = spawn_board(&mut world, 3);
        let map = world.get::<Tilemap>(board).expect("board must carry a Tilemap");
        assert_eq!(map.tileset, 3);
        let t = world.get::<Transform2D>(board).unwrap();
        assert_eq!(t.position, tile_center(0, 0));
    }
}
