//! Where the HUD sits.
//!
//! Every element is laid out in **board space** — the board's own `WIN_W` x
//! `WIN_H` rectangle, its top-left at the origin — and translated to the screen
//! by [`board_origin`]. A window that is not the board's own size then crops the
//! world the same way for the HUD as for the art, instead of sliding the HUD off
//! the board it describes: at 720x720 the top band's text would otherwise sink
//! into the nest roofs while the art kept its place.
//!
//! A slot's width is always `WIN_W` divided by the player count, never the
//! window's own width, so a stretched window cannot stretch a timer bar away
//! from the row of life icons above it.

use engine_core::prelude::*;

use crate::constants::*;

/// The board's top-left corner in screen space: the window centred on it.
pub(crate) fn board_origin(window_size: Vec2) -> Vec2 {
    (window_size - Vec2::new(WIN_W, WIN_H)) / 2.0
}

/// A point in board space, in screen space.
fn on_screen(window_size: Vec2, point: Vec2) -> Vec2 {
    board_origin(window_size) + point
}

/// The pooled score's row.
pub(crate) fn score(window_size: Vec2) -> Vec2 {
    on_screen(window_size, Vec2::new(HUD_TEXT_MARGIN, TOP_BAND_TEXT_Y))
}

/// The round-and-nests line's row, centred on the board.
pub(crate) fn round_line(window_size: Vec2) -> Vec2 {
    on_screen(window_size, Vec2::new(WIN_W / 2.0, TOP_BAND_TEXT_Y))
}

/// The pause hint's row, in the bottom band beside the timer bars.
pub(crate) fn pause_hint(window_size: Vec2) -> Vec2 {
    on_screen(window_size, Vec2::new(HUD_TEXT_MARGIN, PAUSE_HINT_Y))
}

/// The chaos mode's banner, under the board's bottom edge.
pub(crate) fn chaos_banner(window_size: Vec2) -> Vec2 {
    on_screen(window_size, Vec2::new(WIN_W / 2.0, WIN_H - BANNER_INSET))
}

/// `player`'s attempt-timer bar, in screen space.
pub(crate) fn timer_bar(window_size: Vec2, player: usize, player_count: usize) -> Rect {
    let origin = board_origin(window_size);
    let (x, y, width, height) = bar_in_board_space(player, player_count);
    Rect::new(origin.x + x, origin.y + y, width, height)
}

/// The label left of `player`'s bar, in screen space.
pub(crate) fn timer_label(window_size: Vec2, player: usize, player_count: usize) -> Vec2 {
    let origin = board_origin(window_size);
    let (bar_x, bar_y, _, _) = bar_in_board_space(player, player_count);
    Vec2::new(
        origin.x + bar_x - TIMER_LABEL_GUTTER,
        origin.y + bar_y + TIMER_BAR_H - 2.0,
    )
}

/// A player's bar in board space: their slot is `WIN_W` wide and the bar is
/// inset by the same margin on each side, so the space the icons ride in
/// travels with it.
fn bar_in_board_space(player: usize, player_count: usize) -> (f32, f32, f32, f32) {
    let slot_width = WIN_W / player_count.max(1) as f32;
    (
        player as f32 * slot_width + TIMER_BAR_MARGIN,
        WIN_H - BAND + (BAND - TIMER_BAR_H) / 2.0,
        slot_width - 2.0 * TIMER_BAR_MARGIN,
        TIMER_BAR_H,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The board's own window, and three others a player can drag it to.
    const WINDOWS: [Vec2; 4] = [
        Vec2::new(WIN_W, WIN_H),
        Vec2::new(720.0, 720.0),
        Vec2::new(900.0, 768.0),
        Vec2::new(1280.0, 900.0),
    ];

    #[test]
    fn test_the_board_fills_its_own_window() {
        assert_eq!(board_origin(Vec2::new(WIN_W, WIN_H)), Vec2::ZERO);
    }

    #[test]
    fn test_every_hud_element_keeps_its_place_on_the_board() {
        // Each element, pulled back into board space, is the same at every
        // window size: the origin moves, nothing else does. An element that
        // laid itself out from the window's edges — a slot width taken from
        // the window, say — would land somewhere else here.
        let reference = Vec2::new(WIN_W, WIN_H);
        for window in WINDOWS {
            let shift = board_origin(window) - board_origin(reference);
            for (window_point, reference_point) in [
                (score(window), score(reference)),
                (round_line(window), round_line(reference)),
                (pause_hint(window), pause_hint(reference)),
                (chaos_banner(window), chaos_banner(reference)),
                (timer_label(window, 0, 1), timer_label(reference, 0, 1)),
                (timer_label(window, 1, 2), timer_label(reference, 1, 2)),
            ] {
                assert_eq!(window_point - reference_point, shift, "at {window}");
            }
            for (player, count) in [(0, 1), (0, 2), (1, 2)] {
                let bar = timer_bar(window, player, count);
                let at_reference = timer_bar(reference, player, count);
                assert_eq!(
                    Rect::new(
                        bar.x - shift.x,
                        bar.y - shift.y,
                        bar.width,
                        bar.height,
                    ),
                    at_reference,
                    "player {player} of {count} at {window}"
                );
            }
        }
    }

    #[test]
    fn test_the_bars_and_their_icons_keep_the_same_relation_at_any_window() {
        // The icons are world sprites, so they crop with the board; the bars
        // must stay the same distance from them. Board space is the meeting
        // point: the bar's own row, measured from the board's top-left, is
        // what the icons' world row already is.
        for (player, count) in [(0, 1), (0, 2), (1, 2)] {
            let bar = timer_bar(Vec2::new(WIN_W, WIN_H), player, count);
            assert_eq!(bar.y, WIN_H - BAND + (BAND - TIMER_BAR_H) / 2.0);
            assert_eq!(bar.height, TIMER_BAR_H);
            // Centred under its own slot, inset by the same margin each side.
            let slot_width = WIN_W / count as f32;
            let slot_left = player as f32 * slot_width;
            assert_eq!(bar.x - slot_left, TIMER_BAR_MARGIN);
            assert_eq!(slot_left + slot_width - (bar.x + bar.width), TIMER_BAR_MARGIN);
        }
    }

    #[test]
    fn test_a_slot_is_the_boards_width_however_wide_the_window_is() {
        // The defect this guards: a bar laid out from the window would grow
        // with it while the life icons above it stayed on the board.
        let wide = Vec2::new(1280.0, 768.0);
        let narrow = Vec2::new(720.0, 768.0);
        for (player, count) in [(0, 2), (1, 2)] {
            assert_eq!(
                timer_bar(wide, player, count).width,
                timer_bar(narrow, player, count).width,
                "player {player}'s bar stretched with the window"
            );
        }
    }
}
