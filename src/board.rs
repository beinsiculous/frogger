//! The board: a single `Tilemap` entity — the engine's first Tilemap
//! consumer. `build_board` is pure (the tileset handle comes from the
//! caller) so every terrain rule is headless-testable.
//!
//! Tile values index the 4-cell tileset strip built by `tileset_pixels`:
//! 1 = grass, 2 = water, 3 = road, 4 = home slot. Value 0 would be an empty
//! tile; this board has no holes.

use engine_core::prelude::*;
use glam::Vec4;

use crate::constants::*;

pub(crate) const TILE_GRASS: u32 = 1;
pub(crate) const TILE_WATER: u32 = 2;
pub(crate) const TILE_ROAD: u32 = 3;
pub(crate) const TILE_HOME: u32 = 4;

/// Tileset strip geometry: 4 cells of 16x16 px in one 64x16 texture.
pub(crate) const TILESET_W: u32 = 64;
pub(crate) const TILESET_H: u32 = 16;
const CELL_W: u32 = 16;

/// World position of the center of board tile (`col`, `row`). Row 0 is the
/// top row (matching the Tilemap convention); the board is centered on the
/// window origin.
pub(crate) fn tile_center(col: u32, row: u32) -> Vec2 {
    Vec2::new(
        (col as f32 - (COLS as f32 - 1.0) / 2.0) * TILE,
        ((ROWS as f32 - 1.0) / 2.0 - row as f32) * TILE,
    )
}

/// Terrain value for one board tile — the single source of the row map.
pub(crate) fn terrain_at(col: u32, row: u32) -> u32 {
    match row {
        HOME_ROW => {
            if HOME_COLS.contains(&col) {
                TILE_HOME
            } else {
                TILE_GRASS
            }
        }
        FIRST_WATER_ROW..=LAST_WATER_ROW => TILE_WATER,
        MEDIAN_ROW => TILE_GRASS,
        FIRST_ROAD_ROW..=LAST_ROAD_ROW => TILE_ROAD,
        _ => TILE_GRASS, // START_ROW and anything past it
    }
}

/// Build the full board tilemap. Pure: no textures are created here — the
/// caller passes the tileset handle (see `FroggerGame::rebuild_board`).
pub(crate) fn build_board(tileset: u32) -> Tilemap {
    let mut map = Tilemap::new(COLS, ROWS, TILE);
    map.tileset = tileset;
    map.tile_uv_size = Vec2::new(CELL_W as f32 / TILESET_W as f32, 1.0);
    for row in 0..ROWS {
        for col in 0..COLS {
            map.set_tile(col, row, terrain_at(col, row));
        }
    }
    map
}

fn cell_color(base: Vec4, tint: Vec4) -> [u8; 4] {
    let c = base * 0.85 + tint * 0.15;
    [
        (c.x.clamp(0.0, 1.0) * 255.0) as u8,
        (c.y.clamp(0.0, 1.0) * 255.0) as u8,
        (c.z.clamp(0.0, 1.0) * 255.0) as u8,
        255,
    ]
}

/// Raw RGBA for the 4-cell tileset strip, tinted by the chaos theme's accent
/// so each mode's board carries its palette. Cell order matches the
/// `TILE_*` constants (value t samples cell t-1).
pub(crate) fn tileset_pixels(theme: &ChaosTheme) -> Vec<u8> {
    let tint = theme.accent_color;
    let cells: [[u8; 4]; 4] = [
        cell_color(Vec4::new(0.10, 0.32, 0.14, 1.0), tint), // grass
        cell_color(Vec4::new(0.07, 0.13, 0.36, 1.0), tint), // water
        cell_color(Vec4::new(0.15, 0.15, 0.19, 1.0), tint), // road
        cell_color(Vec4::new(0.03, 0.06, 0.05, 1.0), tint), // home slot
    ];
    let mut rgba = Vec::with_capacity((TILESET_W * TILESET_H * 4) as usize);
    for _y in 0..TILESET_H {
        for x in 0..TILESET_W {
            rgba.extend_from_slice(&cells[(x / CELL_W) as usize]);
        }
    }
    rgba
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_board_dimensions_and_tile_count() {
        let map = build_board(0);
        assert_eq!(map.width, COLS);
        assert_eq!(map.height, ROWS);
        assert_eq!(map.tiles.len(), (COLS * ROWS) as usize);
        assert_eq!(map.tile_size, TILE);
    }

    #[test]
    fn test_terrain_rows_match_the_row_map() {
        for col in 0..COLS {
            for row in FIRST_WATER_ROW..=LAST_WATER_ROW {
                assert_eq!(terrain_at(col, row), TILE_WATER, "row {row} must be water");
            }
            for row in FIRST_ROAD_ROW..=LAST_ROAD_ROW {
                assert_eq!(terrain_at(col, row), TILE_ROAD, "row {row} must be road");
            }
            assert_eq!(terrain_at(col, MEDIAN_ROW), TILE_GRASS);
            assert_eq!(terrain_at(col, START_ROW), TILE_GRASS);
        }
    }

    #[test]
    fn test_home_row_has_slots_exactly_at_home_cols() {
        for col in 0..COLS {
            let expected = if HOME_COLS.contains(&col) { TILE_HOME } else { TILE_GRASS };
            assert_eq!(terrain_at(col, HOME_ROW), expected, "home row col {col}");
        }
    }

    #[test]
    fn test_board_has_no_empty_tiles() {
        let map = build_board(0);
        assert!(map.tiles.iter().all(|&t| t != 0), "board must have no holes");
    }

    #[test]
    fn test_tile_center_spans_the_centered_board() {
        // Middle tile of the middle row sits on the origin.
        assert_eq!(tile_center(7, 6), Vec2::ZERO);
        // Top-left tile: half the board left and up from center.
        let top_left = tile_center(0, 0);
        assert_eq!(top_left.x, -(COLS as f32 - 1.0) / 2.0 * TILE);
        assert_eq!(top_left.y, (ROWS as f32 - 1.0) / 2.0 * TILE);
        // Row 0 is the TOP row: larger y than the start row.
        assert!(tile_center(0, HOME_ROW).y > tile_center(0, START_ROW).y);
    }

    #[test]
    fn test_tileset_pixels_match_texture_dimensions() {
        let theme = ChaosTheme::for_mode(ChaosMode::Normal);
        let rgba = tileset_pixels(&theme);
        assert_eq!(rgba.len(), (TILESET_W * TILESET_H * 4) as usize);
        // Distinct terrains must produce distinct cell colors (first texel
        // of each 16px cell in the first pixel row).
        let texel = |cell: u32| {
            let i = (cell * CELL_W * 4) as usize;
            &rgba[i..i + 4]
        };
        assert_ne!(texel(0), texel(1));
        assert_ne!(texel(1), texel(2));
        assert_ne!(texel(2), texel(3));
    }

    #[test]
    fn test_tilemap_uv_selects_four_cells() {
        let map = build_board(0);
        assert_eq!(map.tile_uv_size, Vec2::new(0.25, 1.0));
    }
}
