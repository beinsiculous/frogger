use engine_core::prelude::*;

use crate::constants::*;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum GameState {
    TitleScreen { selection: u8 },
    ModeSelect { selection: u8 },
    Achievements,
    Playing,
    /// Endless rounds: the only ending is every frog running out of lives.
    GameOver,
}

/// How many frogs cross. Co-op shares the board, home slots, and score but
/// keeps per-frog lives; the match only ends once every frog is out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GameMode {
    SinglePlayer,
    TwoPlayerCoop,
}

impl GameMode {
    pub(crate) fn player_count(self) -> usize {
        match self {
            GameMode::SinglePlayer => 1,
            GameMode::TwoPlayerCoop => 2,
        }
    }
}

/// What kind of obstacle a lane carries. Road kinds kill on touch; water
/// kinds ARE the walkable platforms (the water between them kills).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LaneKind {
    Car,
    Truck,
    Log,
    /// A group of turtles moving as one segment; the Ridiculous family makes
    /// them dive on a hash-phased cycle (submerged = not a platform).
    Turtles,
}

/// Compile-time description of one lane. All ten live in `LANES`
/// (`gameplay/rules.rs`); speeds are Normal-mode base values in px/s.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LaneDef {
    pub(crate) row: u32,
    pub(crate) kind: LaneKind,
    /// +1.0 moves right, -1.0 moves left.
    pub(crate) dir: f32,
    pub(crate) speed: f32,
    /// Obstacles in the lane, evenly spaced around the torus period.
    pub(crate) count: usize,
    /// Obstacle length in tiles (trucks 2, logs 3-4, turtle groups 2-3).
    pub(crate) len_tiles: f32,
}

impl LaneDef {
    /// Half the obstacle length, world pixels — the collision half-extent.
    pub(crate) fn half_len(&self) -> f32 {
        self.len_tiles * TILE / 2.0
    }
}

/// A live lane: the def plus each obstacle's logical center x on the torus
/// (`[-LANE_PERIOD/2, LANE_PERIOD/2)`) and its sprite pair (main + ghost;
/// the ghost shows only while the obstacle straddles a window edge).
pub(crate) struct LaneState {
    pub(crate) def: LaneDef,
    pub(crate) xs: Vec<f32>,
    /// One (main, ghost) sprite pair per obstacle in `xs` order.
    pub(crate) sprites: Vec<(EntityId, EntityId)>,
}

/// One player's frog and everything private to it. Co-op frogs share homes
/// and score but keep individual lives and attempt timers.
pub(crate) struct FrogState {
    pub(crate) entity: Option<EntityId>,
    /// World x (continuous — riding a platform drifts it off-grid).
    pub(crate) x: f32,
    /// Board row (0 = home row, 12 = start row).
    pub(crate) row: u32,
    pub(crate) lives: u32,
    /// Seconds left on the current attempt; hitting 0 is a Timeout death.
    pub(crate) timer: f32,
    /// > 0 while dead and waiting to respawn (frog hidden meanwhile).
    pub(crate) respawn_timer: f32,
    /// Lowest (closest-to-home) row reached this attempt, for +10/row score.
    pub(crate) furthest_row: u32,
    /// Which column this frog respawns at.
    pub(crate) start_col: u32,
    /// Out of lives: hidden, no input, partner plays on.
    pub(crate) retired: bool,
}

impl FrogState {
    pub(crate) fn new(start_col: u32, timer: f32) -> Self {
        Self {
            entity: None,
            x: 0.0,
            row: START_ROW,
            lives: STARTING_LIVES,
            timer,
            respawn_timer: 0.0,
            furthest_row: START_ROW,
            start_col,
            retired: false,
        }
    }

    /// Alive and accepting input this frame?
    pub(crate) fn active(&self) -> bool {
        !self.retired && self.respawn_timer <= 0.0
    }
}

/// Why a frog died — drives the death particle color and (in tests) exact
/// assertions on each death path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeathCause {
    Car,
    Drown,
    /// Ridden a platform past the board edge.
    Swept,
    Timeout,
    /// Jumped into the home row outside an open slot.
    HomeMiss,
    /// Jumped into the croc-guarded slot while the croc is present.
    Croc,
}

pub(crate) struct FroggerGame {
    pub(crate) state: GameState,
    pub(crate) mode: GameMode,
    pub(crate) chaos_mode: ChaosMode,
    pub(crate) frame_count: u32,

    pub(crate) frogs: Vec<FrogState>,
    pub(crate) lanes: Vec<LaneState>,
    /// Home slots left-to-right; true = filled.
    pub(crate) homes: [bool; 5],
    /// Marker sprites shown in filled slots, index-aligned with `homes`.
    pub(crate) home_markers: Vec<EntityId>,
    /// The croc sprite (Ridiculous family only), shown while the croc is present.
    pub(crate) croc_marker: Option<EntityId>,
    /// The board tilemap entity (spawned per match, themed per chaos mode).
    pub(crate) board: Option<EntityId>,
    pub(crate) background: Option<EntityId>,

    /// Pooled score (both co-op players feed it).
    pub(crate) score: u32,
    /// 1-based round number; drives the speed ramp and croc slot pick.
    pub(crate) round: u32,
    /// Game-clock seconds since the match started (croc/dive phases).
    pub(crate) play_time: f32,
    /// Deaths this round (deathless-round achievement).
    pub(crate) deaths_this_round: u32,
    /// Home fills per player this round (co-op tag-team achievement).
    pub(crate) fills_this_round: [u32; 2],
    /// Cumulative home fills this session (milestone achievement).
    pub(crate) total_homes: u32,

    /// White 1x1 texture backing every plain sprite.
    pub(crate) tex_id: u32,
    /// The current chaos mode's procedural tileset strip, if built.
    pub(crate) tileset_tex: Option<u32>,

    /// Shared pause menu; only Playing is pausable (see the pause gate).
    pub(crate) pause: PauseMenu,
    /// Deforming spring-mass grid drawn under the board.
    pub(crate) grid: Option<GridMesh>,
    /// F1 debug toggle (kept for convention parity — no colliders here).
    pub(crate) debug_colliders: bool,
}

impl Default for FroggerGame {
    fn default() -> Self {
        Self {
            state: GameState::TitleScreen { selection: 0 },
            mode: GameMode::SinglePlayer,
            chaos_mode: ChaosMode::Normal,
            frame_count: 0,
            frogs: Vec::new(),
            lanes: Vec::new(),
            homes: [false; 5],
            home_markers: Vec::new(),
            croc_marker: None,
            board: None,
            background: None,
            score: 0,
            round: 1,
            play_time: 0.0,
            deaths_this_round: 0,
            fills_this_round: [0; 2],
            total_homes: 0,
            tex_id: 0,
            tileset_tex: None,
            pause: PauseMenu::new(),
            grid: None,
            debug_colliders: false,
        }
    }
}
