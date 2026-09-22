//! The board: four `Tilemap` entities, one per sheet — the floor and the coop
//! wall, the soup, the conveyor belts. `build_map` is pure (the sheet comes
//! from the caller) so every terrain rule is headless-testable.
//!
//! Each map holds only the cells its own terrain owns, so the four never
//! overlap and never fight over a tile. A tile value is a cell index plus one,
//! because 0 is a hole and the board has none.

use engine_core::prelude::*;

use crate::constants::*;
use crate::gameplay::rules::LANES;
use crate::types::*;

/// What a board cell is made of. One kind is one tile sheet, which is why the
/// board is four maps rather than one map of four tinted cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Terrain {
    /// The median, the start row, and the ground the nests stand on.
    Floor,
    /// The coop wall between the nests — the row where a miss kills.
    Wall,
    /// The river of soup.
    Soup,
    /// The conveyor belts the carts run on.
    Belt,
}

impl Terrain {
    /// Every terrain, in the order the maps are built and drawn.
    pub(crate) const ALL: [Terrain; 4] = [Terrain::Floor, Terrain::Wall, Terrain::Soup, Terrain::Belt];

    /// The clip each of these cells plays, and so what the map's own name is.
    pub(crate) fn clip(self) -> &'static str {
        match self {
            Terrain::Floor => FLOOR_CLIP,
            Terrain::Wall => WALL_CLIP,
            Terrain::Soup => SOUP_CLIP,
            Terrain::Belt => BELT_CLIP,
        }
    }

    pub(crate) fn depth(self) -> f32 {
        match self {
            Terrain::Floor | Terrain::Wall => GROUND_DEPTH,
            Terrain::Soup => SOUP_DEPTH,
            Terrain::Belt => BELT_DEPTH,
        }
    }

    /// The tile sheet these cells draw from.
    pub(crate) fn sheet(self, sheets: &Sheets) -> &SpriteSheet {
        match self {
            Terrain::Floor => &sheets.coop_floor,
            Terrain::Wall => &sheets.coop_wall,
            Terrain::Soup => &sheets.tomato_soup,
            Terrain::Belt => &sheets.conveyor_belt,
        }
    }

    /// What the editor hierarchy calls this map.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Terrain::Floor => "floor",
            Terrain::Wall => "wall",
            Terrain::Soup => "soup",
            Terrain::Belt => "belt",
        }
    }
}

/// World position of the center of board tile (`col`, `row`). Row 0 is the
/// top row (matching the Tilemap convention); the board is centered on the
/// window origin.
pub(crate) fn tile_center(col: u32, row: u32) -> Vec2 {
    Vec2::new(
        (col as f32 - (COLS as f32 - 1.0) / 2.0) * TILE,
        ((ROWS as f32 - 1.0) / 2.0 - row as f32) * TILE,
    )
}

/// What a board cell is made of — the single source of the row map.
pub(crate) fn terrain_at(col: u32, row: u32) -> Terrain {
    match row {
        HOME_ROW => {
            if HOME_COLS.contains(&col) {
                Terrain::Floor
            } else {
                Terrain::Wall
            }
        }
        FIRST_WATER_ROW..=LAST_WATER_ROW => Terrain::Soup,
        MEDIAN_ROW => Terrain::Floor,
        FIRST_ROAD_ROW..=LAST_ROAD_ROW => Terrain::Belt,
        _ => Terrain::Floor, // START_ROW and anything past it
    }
}

/// Build one map holding only the cells `terrain` owns, each showing the
/// sheet's cell `tile` (its index plus one). Pure: no textures are created
/// here, and the uv size comes from the loaded sheet's own grid rather than
/// from a typed-in cell count.
pub(crate) fn build_map(terrain: Terrain, sheet: &SpriteSheet, tile: u32) -> Tilemap {
    let mut map = Tilemap::new(COLS, ROWS, TILE);
    map.tileset = sheet.texture.id;
    map.tile_uv_size = Vec2::new(1.0 / sheet.grid.cols as f32, 1.0 / sheet.grid.rows as f32);
    map.depth = terrain.depth();
    for row in 0..ROWS {
        for col in 0..COLS {
            if terrain_at(col, row) == terrain {
                map.set_tile(col, row, tile);
            }
        }
    }
    map
}

/// The clip a sheet declares under `name`, as its sidecar loaded it.
fn declared_clip<'a>(sheet: &'a SpriteSheet, name: &str) -> Option<&'a AnimationClip> {
    sheet.clips.iter().find(|(clip, _)| clip == name).map(|(_, clip)| clip)
}

/// The cell a clip's animation is showing `elapsed` seconds in, at the rate
/// the sidecar declares. `reversed` walks the same cells the other way.
fn clip_cell(clip: &AnimationClip, elapsed: f32, reversed: bool) -> Option<u32> {
    let frames = clip.frame_indices.len() as u32;
    if frames == 0 {
        return None;
    }
    let reached = (elapsed.max(0.0) * clip.fps) as u32 % frames;
    let position = if reversed { frames - 1 - reached } else { reached };
    clip.frame_indices.get(position as usize).copied()
}

/// Rewrite one row's tiles, and only when the value actually changed — a
/// frame that has not turned costs nothing.
fn set_row(tilemap: &mut Tilemap, row: u32, tile: u32) {
    let start = (row * tilemap.width) as usize;
    let Some(tiles) = tilemap.tiles.get_mut(start..start + tilemap.width as usize) else {
        return;
    };
    if tiles.iter().all(|&value| value == tile) {
        return;
    }
    tiles.fill(tile);
}

/// Which way the lane covering `row` runs: +1 right, -1 left.
fn lane_direction(row: u32) -> f32 {
    LANES
        .iter()
        .find(|lane| lane.row == row)
        .map_or(1.0, |lane| lane.dir)
}

/// Advance the soup and the belts to the frame `play_time` is on, reading
/// each clip's own rate and cells from the loaded sheet. A belt row whose lane
/// runs leftward plays its frames reversed, so the slats always run the way
/// the carts above them do.
pub(crate) fn animate(world: &mut World, maps: &[BoardMap], sheets: &Sheets, play_time: f32) {
    for map in maps {
        let rows: &[u32] = match map.terrain {
            Terrain::Soup => &SOUP_ROWS,
            Terrain::Belt => &BELT_ROWS,
            Terrain::Floor | Terrain::Wall => continue,
        };
        let sheet = map.terrain.sheet(sheets);
        let Some(clip) = declared_clip(sheet, map.terrain.clip()) else {
            continue;
        };
        let Some(tilemap) = world.get_mut::<Tilemap>(map.entity) else {
            continue;
        };
        for &row in rows {
            let reversed = lane_direction(row) < 0.0;
            let Some(cell) = clip_cell(clip, play_time, reversed) else {
                continue;
            };
            set_row(tilemap, row, cell + 1);
        }
    }
}

/// The rows each animated terrain covers, as the one loop each map runs.
const SOUP_ROWS: [u32; 5] = [
    FIRST_WATER_ROW,
    FIRST_WATER_ROW + 1,
    FIRST_WATER_ROW + 2,
    FIRST_WATER_ROW + 3,
    LAST_WATER_ROW,
];
const BELT_ROWS: [u32; 5] = [
    FIRST_ROAD_ROW,
    FIRST_ROAD_ROW + 1,
    FIRST_ROAD_ROW + 2,
    FIRST_ROAD_ROW + 3,
    LAST_ROAD_ROW,
];

#[cfg(test)]
mod tests {
    use super::*;

    /// A sheet stand-in: one texture handle and the grid its sidecar declares.
    fn sheet(grid: SheetGrid, clips: Vec<(&str, AnimationClip)>) -> SpriteSheet {
        SpriteSheet {
            texture: TextureHandle { id: 7 },
            grid,
            clips: clips
                .into_iter()
                .map(|(name, clip)| (name.to_string(), clip))
                .collect(),
            path: String::new(),
        }
    }

    fn static_sheet() -> SpriteSheet {
        sheet(SheetGrid::new(1, 1), vec![(FLOOR_CLIP, AnimationClip::new(vec![0], 5.0))])
    }

    #[test]
    fn test_a_map_holds_only_its_own_terrain() {
        for terrain in Terrain::ALL {
            let map = build_map(terrain, &static_sheet(), 1);
            assert_eq!(map.width, COLS);
            assert_eq!(map.height, ROWS);
            assert_eq!(map.tile_size, TILE);
            for row in 0..ROWS {
                for col in 0..COLS {
                    let owned = terrain_at(col, row) == terrain;
                    let value = map.tiles[(row * COLS + col) as usize];
                    assert_eq!(
                        value != 0,
                        owned,
                        "{terrain:?} at ({col}, {row}) should be {}",
                        if owned { "set" } else { "empty" }
                    );
                }
            }
        }
    }

    #[test]
    fn test_every_board_cell_is_owned_by_exactly_one_map() {
        for row in 0..ROWS {
            for col in 0..COLS {
                let owners = Terrain::ALL
                    .iter()
                    .filter(|terrain| terrain_at(col, row) == **terrain)
                    .count();
                assert_eq!(owners, 1, "({col}, {row}) has {owners} owners");
            }
        }
    }

    #[test]
    fn test_the_row_map_is_the_one_the_rules_name() {
        for col in 0..COLS {
            for row in FIRST_WATER_ROW..=LAST_WATER_ROW {
                assert_eq!(terrain_at(col, row), Terrain::Soup, "row {row} is soup");
            }
            for row in FIRST_ROAD_ROW..=LAST_ROAD_ROW {
                assert_eq!(terrain_at(col, row), Terrain::Belt, "row {row} is belt");
            }
            assert_eq!(terrain_at(col, MEDIAN_ROW), Terrain::Floor);
            assert_eq!(terrain_at(col, START_ROW), Terrain::Floor);
            let expected = if HOME_COLS.contains(&col) { Terrain::Floor } else { Terrain::Wall };
            assert_eq!(terrain_at(col, HOME_ROW), expected, "home row col {col}");
        }
    }

    #[test]
    fn test_each_map_cuts_the_sheet_into_the_cells_its_sidecar_declares() {
        // The uv size is taken from the loaded sheet, so a map can only sample
        // the cells its own PNG was cut into.
        let four = sheet(SheetGrid::new(4, 1), vec![(SOUP_CLIP, AnimationClip::new(vec![0, 1, 2, 3], 6.667))]);
        let soup = build_map(Terrain::Soup, &four, 1);
        assert_eq!(soup.tile_uv_size, Vec2::new(0.25, 1.0));
        assert_eq!(soup.tileset, 7, "the map draws from the sheet it was built with");
        let one = build_map(Terrain::Floor, &static_sheet(), 1);
        assert_eq!(one.tile_uv_size, Vec2::new(1.0, 1.0));
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
    fn test_a_belt_row_plays_its_frames_the_way_its_carts_run() {
        // The clip's own cells, read at the clip's own rate: four cells at
        // 6.667 fps is 150 ms a cell, so a frame boundary lands on 0.15 s.
        let clip = AnimationClip::new(vec![0, 1, 2, 3], 6.667);
        assert_eq!(clip_cell(&clip, 0.0, false), Some(0));
        assert_eq!(clip_cell(&clip, 0.15, false), Some(1));
        assert_eq!(clip_cell(&clip, 0.45, false), Some(3));
        assert_eq!(clip_cell(&clip, 0.60, false), Some(0), "it loops");
        // Reversed it walks the same cells the other way — the lane runs left.
        assert_eq!(clip_cell(&clip, 0.0, true), Some(3));
        assert_eq!(clip_cell(&clip, 0.15, true), Some(2));
        assert!(lane_direction(10) < 0.0, "row 10's carts run leftward");
        assert!(lane_direction(9) > 0.0, "row 9's run rightward");
    }

    #[test]
    fn test_a_row_is_rewritten_only_when_its_frame_turns() {
        let mut map = Tilemap::new(COLS, ROWS, TILE);
        set_row(&mut map, MEDIAN_ROW, 3);
        let start = (MEDIAN_ROW * COLS) as usize;
        assert!(map.tiles[start..start + COLS as usize].iter().all(|&value| value == 3));
        // A second write of the same value is a no-op, and other rows are
        // never touched.
        set_row(&mut map, MEDIAN_ROW, 3);
        assert!(map.tiles[..start].iter().all(|&value| value == 0));
    }

    #[test]
    fn test_a_clip_with_no_frames_draws_nothing_rather_than_dividing_by_zero() {
        let empty = AnimationClip::new(Vec::new(), 6.667);
        assert_eq!(clip_cell(&empty, 1.0, false), None);
    }
}
