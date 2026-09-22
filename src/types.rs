use engine_core::prelude::*;

use crate::constants::*;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum GameState {
    TitleScreen { selection: u8 },
    ModeSelect { selection: u8 },
    Achievements,
    Playing,
    /// Endless rounds: the only ending is every chicken running out of lives.
    GameOver,
}

/// How many chickens cross. Co-op shares the board, nests, and score but
/// keeps per-chicken lives; the match only ends once every one is out.
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
/// kinds ARE the walkable platforms (the soup between them kills).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LaneKind {
    Car,
    Truck,
    Log,
    /// A group of crackers moving as one segment; the Ridiculous family makes
    /// them sink on a hash-phased cycle (submerged = not a platform).
    Crackers,
}

// --- The sheets' own vocabulary -----------------------------------------------------
// Every name below is what the synced sidecar declares, and every one is
// checked against it: a rename in the art is a rename here or the guard test
// fails. Cells and anchors are `constants.rs`'s sheets block.

/// The two poses a chicken is drawn in, per facing. A hop moves it a whole
/// tile in one frame, so `jump` is a clip that plays once into the `idle` of
/// the same facing.
pub(crate) const IDLE_CLIP: &str = "idle";
pub(crate) const JUMP_CLIP: &str = "jump";

/// A cart's wheels and a conveyor's slats.
pub(crate) const ROLL_CLIP: &str = "roll";
/// A raft's and the soup's rest.
pub(crate) const RAFT_CLIP: &str = "idle";

pub(crate) const CRACKER_IDLE: &str = "idle";
pub(crate) const CRACKER_WARNING: &str = "warning";
pub(crate) const CRACKER_SINK: &str = "sink";
pub(crate) const CRACKER_SUBMERGED: &str = "submerged";
pub(crate) const CRACKER_RISE: &str = "rise";

pub(crate) const BUN_WARNING: &str = "warning";
pub(crate) const BUN_OPENING: &str = "opening";
pub(crate) const BUN_OPEN: &str = "open";
pub(crate) const BUN_CLOSING: &str = "closing";

pub(crate) const NEST_EMPTY: &str = "empty";
pub(crate) const NEST_ARRIVAL: &str = "arrival";
pub(crate) const NEST_OCCUPIED: &str = "occupied";

pub(crate) const POOF_CLIP: &str = "poof";
pub(crate) const CELEBRATE_CLIP: &str = "celebrate";

pub(crate) const LIFE_ICON_ROUND: &str = "round";
pub(crate) const LIFE_ICON_TALL: &str = "tall";

pub(crate) const SOUP_CLIP: &str = "idle";
pub(crate) const BELT_CLIP: &str = "run";
pub(crate) const WALL_CLIP: &str = "idle";
pub(crate) const FLOOR_CLIP: &str = "idle";

/// The opaque box of a spec's reference frame — its drawn size in art pixels.
pub(crate) fn opaque_size(spec: &SheetSpec) -> Vec2 {
    spec.bounds.1 - spec.bounds.0
}

/// Whether standing chickens back on their start tiles also brings back one
/// still in its respawn wait. A round's end does; the beat's opening does not,
/// so a partner who died just before the clear stays gone while its poof plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Revive {
    Waiting,
    Never,
}

/// The four facings a chicken is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Facing {
    North,
    East,
    South,
    West,
}

impl Facing {
    /// The suffix the sheets name this facing with.
    pub(crate) fn suffix(self) -> &'static str {
        match self {
            Facing::North => "north",
            Facing::East => "east",
            Facing::South => "south",
            Facing::West => "west",
        }
    }

    /// Every facing, in the order the sheets lay their clips out.
    pub(crate) const ALL: [Facing; 4] = [Facing::North, Facing::East, Facing::South, Facing::West];

    /// The facing a chicken wears when a life starts.
    pub(crate) const START: Facing = Facing::North;

    /// The state, and the clip, this facing idles in.
    pub(crate) fn idle_state(self) -> String {
        format!("{IDLE_CLIP}_{}", self.suffix())
    }

    /// The state, and the clip, this facing's hop plays once into its idle.
    pub(crate) fn jump_state(self) -> String {
        format!("{JUMP_CLIP}_{}", self.suffix())
    }
}

/// Which synced sheet a lane segment draws its sprites from. The sheet owns
/// the cell size, the anchor, and so the segment's drawn length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LaneSprite {
    SushiCart,
    DonutCart,
    HotDogCart,
    CeleryRaft,
    BaguetteRaft,
    Cracker,
}

impl LaneSprite {
    pub(crate) fn spec(self) -> &'static SheetSpec {
        match self {
            LaneSprite::SushiCart => &SUSHI_CART,
            LaneSprite::DonutCart => &DONUT_CART,
            LaneSprite::HotDogCart => &HOT_DOG_CART,
            LaneSprite::CeleryRaft => &CELERY_RAFT,
            LaneSprite::BaguetteRaft => &BAGUETTE_RAFT,
            LaneSprite::Cracker => &CRACKER,
        }
    }

    /// The clip one of this sprite's sprites plays while its lane carries it.
    /// The crackers are posed by the rules frame by frame and never play a
    /// clip through a machine at all.
    pub(crate) fn travelling_clip(self) -> &'static str {
        match self {
            LaneSprite::SushiCart | LaneSprite::DonutCart | LaneSprite::HotDogCart => ROLL_CLIP,
            LaneSprite::CeleryRaft | LaneSprite::BaguetteRaft => RAFT_CLIP,
            LaneSprite::Cracker => CRACKER_IDLE,
        }
    }

    /// Whether the rules pose this sprite — the cracker's sink and rise — or
    /// the sheet's own clip carries it.
    pub(crate) fn is_posed(self) -> bool {
        self == LaneSprite::Cracker
    }

    /// Where this sprite's art sits relative to the board.
    pub(crate) fn depth(self) -> f32 {
        match self {
            LaneSprite::SushiCart | LaneSprite::DonutCart | LaneSprite::HotDogCart => CART_DEPTH,
            LaneSprite::CeleryRaft | LaneSprite::BaguetteRaft | LaneSprite::Cracker => {
                PLATFORM_DEPTH
            }
        }
    }

    /// What the editor hierarchy calls one of these.
    pub(crate) fn label(self) -> &'static str {
        match self {
            LaneSprite::SushiCart | LaneSprite::DonutCart | LaneSprite::HotDogCart => "Cart",
            LaneSprite::CeleryRaft | LaneSprite::BaguetteRaft => "Raft",
            LaneSprite::Cracker => "Cracker",
        }
    }
}

/// One lane's obstacle as drawn: the sheet it is made of and how many of that
/// sheet's cells it is — a car one cart, a truck one hot dog cart, a log two
/// rafts abutted, a group of crackers its two or three.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Segment {
    pub(crate) sprite: LaneSprite,
    pub(crate) sprites: usize,
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
    /// Segments in the lane, evenly spaced around the torus period.
    pub(crate) count: usize,
    pub(crate) segment: Segment,
}

impl LaneDef {
    /// Half the segment's drawn length, world pixels — the collision
    /// half-extent. Its sprites stand one cell apart with the sheet's anchor
    /// on each, so the segment reaches from the first sprite's opaque left to
    /// the last sprite's opaque right.
    pub(crate) fn half_len(&self) -> f32 {
        let spec = self.segment.sprite.spec();
        let between_sprites = (self.segment.sprites as f32 - 1.0) * spec.cell.x;
        (between_sprites + opaque_size(spec).x) / 2.0
    }

    /// Where sprite `sprite` of the segment stands, relative to the segment's
    /// own centre.
    pub(crate) fn sprite_offset(&self, sprite: usize) -> f32 {
        let first = (self.segment.sprites as f32 - 1.0) / 2.0;
        (sprite as f32 - first) * self.segment.sprite.spec().cell.x
    }
}

/// One of the board's four tile maps: which terrain it holds and the entity
/// carrying its `Tilemap`. The terrain rides along so the animator can reach
/// the right sheet without asking the map what it is.
pub(crate) struct BoardMap {
    pub(crate) terrain: crate::board::Terrain,
    pub(crate) entity: EntityId,
}

/// A live lane: the def plus each segment's logical centre x on the torus
/// (`[-LANE_PERIOD/2, LANE_PERIOD/2)`) and its sprite pairs — one (main,
/// ghost) pair per sprite of the segment. The ghost shows only while the
/// segment straddles a window edge.
pub(crate) struct LaneState {
    pub(crate) def: LaneDef,
    pub(crate) xs: Vec<f32>,
    /// One `Vec` of (main, ghost) pairs per segment in `xs` order.
    pub(crate) sprites: Vec<Vec<(EntityId, EntityId)>>,
}

/// One player's chicken and everything private to it. Co-op chickens share
/// nests and score but keep individual lives and attempt timers.
pub(crate) struct ChickenState {
    pub(crate) entity: Option<EntityId>,
    /// World x (continuous — riding a platform drifts it off-grid).
    pub(crate) x: f32,
    /// Board row (0 = home row, 12 = start row).
    pub(crate) row: u32,
    /// The one source of the drawn pose: a hop sets it, and nothing else can.
    pub(crate) facing: Facing,
    pub(crate) lives: u32,
    /// Seconds left on the current attempt; hitting 0 is a Timeout death.
    pub(crate) timer: f32,
    /// > 0 while dead and waiting to respawn (chicken hidden meanwhile).
    pub(crate) respawn_timer: f32,
    /// Lowest (closest-to-home) row reached this attempt, for +10/row score.
    pub(crate) furthest_row: u32,
    /// Which column this chicken respawns at.
    pub(crate) start_col: u32,
    /// Out of lives: hidden, no input, partner plays on.
    pub(crate) retired: bool,
}

impl ChickenState {
    pub(crate) fn new(start_col: u32, timer: f32) -> Self {
        Self {
            entity: None,
            x: 0.0,
            row: START_ROW,
            facing: Facing::START,
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

/// Why a chicken died — drives the splash a water death leaves under its poof
/// and (in tests) exact assertions on each death path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeathCause {
    Cart,
    Drown,
    /// Ridden a platform past the board edge.
    Swept,
    Timeout,
    /// Jumped into the home row outside an open nest.
    HomeMiss,
    /// Jumped into the bun's nest while the bun is open.
    Bun,
}

impl DeathCause {
    /// Whether this death happened in the soup, and so leaves a splash.
    pub(crate) fn is_soup(self) -> bool {
        matches!(self, DeathCause::Drown | DeathCause::Swept)
    }
}

/// The game's art: one loaded `SpriteSheet` per synced sheet, plus the 1x1
/// white texture the particle effects draw with.
///
/// Every sheet's path, cell and measured bounds live in `constants.rs`'s
/// sheets block; each PNG and its `.sheet.ron` sidecar is a synced copy of the
/// deion_assets master (`assets/sprites/sync.list`), so no art here is
/// hand-authored and none is loaded from anywhere else.
pub(crate) struct Sheets {
    pub(crate) white: u32,
    pub(crate) chicken_round: SpriteSheet,
    pub(crate) chicken_tall: SpriteSheet,
    pub(crate) sushi_cart: SpriteSheet,
    pub(crate) donut_cart: SpriteSheet,
    pub(crate) hot_dog_cart: SpriteSheet,
    pub(crate) celery_raft: SpriteSheet,
    pub(crate) baguette_raft: SpriteSheet,
    pub(crate) cracker: SpriteSheet,
    pub(crate) bun: SpriteSheet,
    pub(crate) nest: SpriteSheet,
    pub(crate) arrival_burst: SpriteSheet,
    pub(crate) feather_poof: SpriteSheet,
    pub(crate) life_icon: SpriteSheet,
    pub(crate) conveyor_belt: SpriteSheet,
    pub(crate) tomato_soup: SpriteSheet,
    pub(crate) coop_floor: SpriteSheet,
    pub(crate) coop_wall: SpriteSheet,
}

impl Sheets {
    /// The sheet a lane sprite draws from.
    pub(crate) fn lane(&self, sprite: LaneSprite) -> &SpriteSheet {
        match sprite {
            LaneSprite::SushiCart => &self.sushi_cart,
            LaneSprite::DonutCart => &self.donut_cart,
            LaneSprite::HotDogCart => &self.hot_dog_cart,
            LaneSprite::CeleryRaft => &self.celery_raft,
            LaneSprite::BaguetteRaft => &self.baguette_raft,
            LaneSprite::Cracker => &self.cracker,
        }
    }

    /// The sheet a player's chicken draws from: P1 the round one, P2 the tall.
    pub(crate) fn player(&self, index: usize) -> &SpriteSheet {
        if index == 0 {
            &self.chicken_round
        } else {
            &self.chicken_tall
        }
    }

    /// The sheet a player's life icon draws from — the same head, off the
    /// icon sheet rather than the character's.
    pub(crate) fn life_icon_clip(index: usize) -> &'static str {
        if index == 0 {
            LIFE_ICON_ROUND
        } else {
            LIFE_ICON_TALL
        }
    }
}

/// A sheet with no texture, one cell and no clips. `Sheets::default` holds
/// these until `init()` loads the real ones: the engine builds the game with
/// `Default` and calls `init` on the first frame, before any entity that could
/// draw exists.
fn placeholder_sheet() -> SpriteSheet {
    SpriteSheet {
        texture: TextureHandle { id: 0 },
        grid: SheetGrid::new(1, 1),
        clips: Vec::new(),
        path: String::new(),
    }
}

impl Default for Sheets {
    fn default() -> Self {
        Self {
            white: 0,
            chicken_round: placeholder_sheet(),
            chicken_tall: placeholder_sheet(),
            sushi_cart: placeholder_sheet(),
            donut_cart: placeholder_sheet(),
            hot_dog_cart: placeholder_sheet(),
            celery_raft: placeholder_sheet(),
            baguette_raft: placeholder_sheet(),
            cracker: placeholder_sheet(),
            bun: placeholder_sheet(),
            nest: placeholder_sheet(),
            arrival_burst: placeholder_sheet(),
            feather_poof: placeholder_sheet(),
            life_icon: placeholder_sheet(),
            conveyor_belt: placeholder_sheet(),
            tomato_soup: placeholder_sheet(),
            coop_floor: placeholder_sheet(),
            coop_wall: placeholder_sheet(),
        }
    }
}

pub struct FroggerGame {
    pub(crate) state: GameState,
    pub(crate) mode: GameMode,
    pub(crate) chaos_mode: ChaosMode,
    /// Every loaded sheet, plus the white texture the particles draw with.
    pub(crate) sheets: Sheets,

    pub(crate) chickens: Vec<ChickenState>,
    pub(crate) lanes: Vec<LaneState>,
    /// Nests left-to-right; true = filled.
    pub(crate) homes: [bool; 5],
    /// The five nest entities, index-aligned with `homes`; always visible, and
    /// their own machine moves `empty` to `arrival` to `occupied`.
    pub(crate) nests: [Option<EntityId>; 5],
    /// The chicken seated in each filled nest, drawn on that nest's straw.
    pub(crate) seated: [Option<EntityId>; 5],
    /// The snapping bun (Ridiculous family only), surfaced inside the nest it
    /// guards while its slot is open.
    pub(crate) bun: Option<EntityId>,
    /// The board's four tile maps, one per sheet: floor, wall, soup, belt.
    pub(crate) board_maps: Vec<BoardMap>,
    /// One icon per life per player, in the bottom band.
    pub(crate) life_icons: Vec<EntityId>,
    /// The deforming grid: the engine simulates and draws it, the game queues
    /// impulses into it.
    pub(crate) backdrop: Option<EntityId>,
    /// Every detached one-shot (poofs, bursts). An id may already have
    /// despawned itself, so draining them is tolerant.
    pub(crate) transient_visuals: Vec<EntityId>,
    pub(crate) background: Option<EntityId>,

    /// Pooled score (both co-op players feed it).
    pub(crate) score: u32,
    /// 1-based round number; drives the speed ramp and the bun's nest pick.
    pub(crate) round: u32,
    /// Game-clock seconds since the match started (the dive and bun phases).
    pub(crate) play_time: f32,
    /// Seconds left of the round-clear beat, `None` outside one. While it runs
    /// the match is still `Playing` but no chicken is simulated.
    pub(crate) round_clear_in: Option<f32>,
    /// Deaths this round (deathless-round achievement).
    pub(crate) deaths_this_round: u32,
    /// Nest fills per player this round (co-op tag-team achievement).
    pub(crate) fills_this_round: [u32; 2],
    /// Cumulative nest fills this session (milestone achievement).
    pub(crate) total_homes: u32,

    /// Shared pause menu; only Playing is pausable (see the pause gate).
    pub(crate) pause: PauseMenu,
}

impl Default for FroggerGame {
    fn default() -> Self {
        Self {
            state: GameState::TitleScreen { selection: 0 },
            mode: GameMode::SinglePlayer,
            chaos_mode: ChaosMode::Normal,
            sheets: Sheets::default(),
            chickens: Vec::new(),
            lanes: Vec::new(),
            homes: [false; 5],
            nests: [None; 5],
            seated: [None; 5],
            bun: None,
            board_maps: Vec::new(),
            life_icons: Vec::new(),
            backdrop: None,
            transient_visuals: Vec::new(),
            background: None,
            score: 0,
            round: 1,
            play_time: 0.0,
            round_clear_in: None,
            deaths_this_round: 0,
            fills_this_round: [0; 2],
            total_homes: 0,
            pause: PauseMenu::new(),
        }
    }
}
