//! The cracker's and the bun's drawn poses.
//!
//! Neither carries a state machine or a timer of its own: a pose is a pure
//! function of the cycle the rules already run on, so the drawn danger can
//! never drift from the real one. A lead-in ends at the flip and a lead-out
//! begins at it, which is what makes looking dangerous while safe possible —
//! and looking safe while dangerous impossible.
//!
//! The lethal window is never restated here: a pose that could kill asks the
//! rules' own predicate for it, and only the lead windows are this module's
//! own arithmetic. So at the exact threshold a cracker still shows the last
//! frame of its sink and a bun the last of its opening, which is the safe
//! direction both times.

use engine_core::prelude::*;

use crate::constants::*;
use crate::types::*;

use super::rules::{bun_cycle_time, bun_open, cracker_cycle_time, crackers_submerged};

/// A drawn pose: which clip, and which frame of that clip's own frame list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Pose {
    pub(crate) clip: &'static str,
    pub(crate) frame: usize,
}

impl Pose {
    fn new(clip: &'static str, frame: usize) -> Self {
        Self { clip, frame }
    }
}

/// Write a pose onto an entity: the clip and the frame the rules chose, held
/// still. `play` is deliberately not used — it restarts the clip and marks it
/// playing, and a machine on the entity would re-select its own state's clip
/// over the top.
pub(crate) fn write_pose(world: &mut World, entity: EntityId, pose: Pose) {
    let Some(animation) = world.get_mut::<SpriteAnimation>(entity) else {
        return;
    };
    animation.current_clip = Some(pose.clip.to_string());
    animation.current_frame = pose.frame;
    animation.playing = false;
    animation.finished = false;
}

/// Frames in each clip the poses draw from, as the sidecars declare them. A
/// pose is written as a clip and a frame rather than played, so these and the
/// windows below are the whole contract; the guard test re-derives both from
/// the synced sidecars.
pub(crate) const CRACKER_RISE_FRAMES: usize = 3;
pub(crate) const CRACKER_IDLE_FRAMES: usize = 4;
pub(crate) const CRACKER_WARNING_FRAMES: usize = 4;
pub(crate) const CRACKER_SINK_FRAMES: usize = 3;
pub(crate) const CRACKER_SUBMERGED_FRAMES: usize = 2;
pub(crate) const BUN_WARNING_FRAMES: usize = 2;
pub(crate) const BUN_OPENING_FRAMES: usize = 3;
pub(crate) const BUN_OPEN_FRAMES: usize = 2;
pub(crate) const BUN_CLOSING_FRAMES: usize = 3;

/// Which frame of a clip `elapsed` seconds into its window is showing, at the
/// rate the art was drawn at. A looping clip wraps through its frames; a
/// one-shot holds its last, which is what puts a lead-in's final frame on the
/// flip and a lead-out's first frame on it.
fn pose_frame(elapsed: f32, frames: usize, looping: bool) -> usize {
    let reached = (elapsed.max(0.0) * DANGER_POSE_FPS) as usize;
    if looping {
        reached % frames
    } else {
        reached.min(frames - 1)
    }
}

/// Write every cracker of a lane its own pose — each sprite and its seam
/// ghost, all of a group the same, because a group sinks as one.
pub(crate) fn pose_lane_crackers(
    world: &mut World,
    lane: &LaneState,
    mode: ChaosMode,
    play_time: f32,
) {
    if !lane.def.segment.sprite.is_posed() {
        return;
    }
    for (segment, pairs) in lane.sprites.iter().enumerate() {
        let pose = cracker_pose(mode, lane.def.row, segment, play_time);
        for &(main, ghost) in pairs {
            write_pose(world, main, pose);
            write_pose(world, ghost, pose);
        }
    }
}

/// The cracker group at `index` of `lane_row` as drawn at `play_time`.
///
/// Outside the Ridiculous family nothing ever sinks, so a cracker stays in
/// its idle and only bobs; inside it, the cycle runs rise, idle, a warning
/// beat, then the sink that ends on the flip, and a submerged cracker is
/// drawn going under for as long as the rules hold it there.
pub(crate) fn cracker_pose(
    mode: ChaosMode,
    lane_row: u32,
    index: usize,
    play_time: f32,
) -> Pose {
    let cycle_time = cracker_cycle_time(lane_row, index, play_time);

    // The lead windows, laid end to end: they tile the cycle up to the flip,
    // which the rules' own predicate owns.
    let idle_start = CRACKER_RISE_SECS;
    let warning_start = idle_start + CRACKER_IDLE_SECS;
    let sink_start = warning_start + CRACKER_WARNING_SECS;
    let dive_flip = sink_start + CRACKER_SINK_SECS;

    if !mode.is_ridiculous() {
        return Pose::new(CRACKER_IDLE, pose_frame(cycle_time, CRACKER_IDLE_FRAMES, true));
    }
    if crackers_submerged(mode, lane_row, index, play_time) {
        let submerged = cycle_time - dive_flip;
        return Pose::new(
            CRACKER_SUBMERGED,
            pose_frame(submerged, CRACKER_SUBMERGED_FRAMES, true),
        );
    }

    if cycle_time < idle_start {
        Pose::new(CRACKER_RISE, pose_frame(cycle_time, CRACKER_RISE_FRAMES, false))
    } else if cycle_time < warning_start {
        let idle = cycle_time - idle_start;
        Pose::new(CRACKER_IDLE, pose_frame(idle, CRACKER_IDLE_FRAMES, true))
    } else if cycle_time < sink_start {
        let warning = cycle_time - warning_start;
        Pose::new(CRACKER_WARNING, pose_frame(warning, CRACKER_WARNING_FRAMES, true))
    } else {
        let sink = cycle_time - sink_start;
        Pose::new(CRACKER_SINK, pose_frame(sink, CRACKER_SINK_FRAMES, false))
    }
}

/// The bun in the nest it guards as drawn at `play_time`, or `None` when it
/// draws nothing at all: a filled nest hides it for the rest of the round,
/// and so does its own absent window. Only the Ridiculous family has one.
///
/// The cycle runs the closing that ends on the flip, a long absent stretch,
/// a warning beat, then the opening that ends on it; while the rules have the
/// bun open it is drawn open for as long as they hold it there.
pub(crate) fn bun_pose(
    mode: ChaosMode,
    round: u32,
    play_time: f32,
    slot_filled: bool,
) -> Option<Pose> {
    if !mode.is_ridiculous() || slot_filled {
        return None;
    }
    let cycle_time = bun_cycle_time(round, play_time);

    // The same laid-end-to-end windows: they tile the cycle up to the flip,
    // which `bun_open` owns.
    let absent_start = BUN_CLOSING_SECS;
    let warning_start = absent_start + BUN_HIDDEN_SECS;
    let opening_start = warning_start + BUN_WARNING_SECS;
    let open_flip = opening_start + BUN_OPENING_SECS;

    if bun_open(mode, round, play_time) {
        let open = cycle_time - open_flip;
        return Some(Pose::new(BUN_OPEN, pose_frame(open, BUN_OPEN_FRAMES, true)));
    }

    if cycle_time < absent_start {
        Some(Pose::new(BUN_CLOSING, pose_frame(cycle_time, BUN_CLOSING_FRAMES, false)))
    } else if cycle_time < warning_start {
        None
    } else if cycle_time < opening_start {
        let warning = cycle_time - warning_start;
        Some(Pose::new(BUN_WARNING, pose_frame(warning, BUN_WARNING_FRAMES, true)))
    } else {
        let opening = cycle_time - opening_start;
        Some(Pose::new(BUN_OPENING, pose_frame(opening, BUN_OPENING_FRAMES, false)))
    }
}
