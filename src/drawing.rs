//! UI text and HUD: menus, the top score band, the bottom timer band, and
//! the game-over overlay. World rendering is sprites + the board tilemap
//! (drawn by the engine's default render path).

use engine_core::prelude::*;

use crate::achievements::DISPLAY_SECTIONS;
use crate::constants::*;
use crate::gameplay::rules::attempt_timer;
use crate::menu::mode_hint;
use crate::types::*;

impl FroggerGame {
    fn menu_style(&self) -> MenuStyle {
        MenuStyle::from_theme(&ChaosTheme::for_mode(self.chaos_mode))
    }

    pub(crate) fn draw_ui(&self, ctx: &mut GameContext) {
        match &self.state {
            GameState::TitleScreen { selection } => self.draw_title(ctx, *selection),
            GameState::ModeSelect { selection } => self.draw_mode_select(ctx, *selection),
            GameState::Achievements => self.draw_achievements(ctx),
            _ => self.draw_gameplay(ctx),
        }
    }

    fn draw_title(&self, ctx: &mut GameContext, selection: u8) {
        let style = self.menu_style();
        let panel = MenuPanel::new("INSICULOUS FROGGER", ctx.window_size / 2.0, 380.0, 4);
        let mut y = panel.begin(ctx.ui, &style);
        let items = ["1 Player", "2 Player Co-op", "Achievements", "Exit"];
        for (i, item) in items.iter().enumerate() {
            y = panel.item(ctx.ui, y, item, i as u8 == selection, &style);
        }
        panel.hint(ctx.ui, "Navigate to move, confirm to select", &style);

        let rect = panel.panel_rect();
        let cx = ctx.window_size.x / 2.0;
        ctx.ui.label_centered("P1 WASD   -   P2 Arrows", Vec2::new(cx, rect.y + rect.height + 24.0));
        ctx.ui.label_centered("Hop to the homes. Cars kill. Water kills. Ride the logs.",
            Vec2::new(cx, rect.y + rect.height + 48.0));
    }

    fn draw_mode_select(&self, ctx: &mut GameContext, selection: u8) {
        let style = self.menu_style();
        let panel = MenuPanel::new("SELECT CHAOS MODE", ctx.window_size / 2.0, 400.0, ChaosMode::ALL.len());
        let mut y = panel.begin(ctx.ui, &style);
        for (i, &mode) in ChaosMode::ALL.iter().enumerate() {
            let c = ChaosTheme::for_mode(mode).banner_color;
            y = panel.item_colored(ctx.ui, y, mode.label(), c, i as u8 == selection, &style);
        }
        panel.hint(
            ctx.ui,
            mode_hint(ChaosMode::ALL[selection as usize % ChaosMode::ALL.len()]),
            &style,
        );
    }

    fn draw_achievements(&self, ctx: &mut GameContext) {
        let style = self.menu_style();
        let cx = ctx.window_size.x / 2.0;
        let total = ctx.achievements.total();
        let unlocked = ctx.achievements.unlocked_count();

        let panel = MenuPanel::new("ACHIEVEMENTS", ctx.window_size / 2.0, ctx.window_size.x - 120.0, 13);
        let first_y = panel.begin(ctx.ui, &style);
        let rect = panel.panel_rect();
        ctx.ui.label_centered(
            &format!("{unlocked} / {total} unlocked"),
            Vec2::new(cx, first_y - 8.0),
        );

        let left = rect.x + 28.0;
        let mut y = first_y + 18.0;

        let locked_color = Color::new(0.45, 0.45, 0.5, 1.0);
        let unlocked_color = Color::new(1.0, 0.85, 0.25, 1.0);
        let desc_color = Color::new(0.75, 0.75, 0.8, 1.0);
        let header_color = Color::new(0.6, 0.75, 1.0, 1.0);

        for (section, ids) in DISPLAY_SECTIONS {
            ctx.ui.label_styled(section, Vec2::new(left, y), header_color, 16.0);
            y += 22.0;
            for id in *ids {
                let is_unlocked = ctx.achievements.is_unlocked(id);
                let Some(ach) = ctx.achievements.get(id) else { continue };

                let (marker, name_color) = if is_unlocked {
                    ("[X]", unlocked_color)
                } else {
                    ("[ ]", locked_color)
                };

                ctx.ui.label_styled(
                    &format!("{marker} {}", ach.name),
                    Vec2::new(left + 8.0, y),
                    name_color,
                    14.0,
                );
                ctx.ui.label_styled(&ach.description, Vec2::new(left + 52.0, y + 16.0), desc_color, 12.0);
                y += 36.0;
            }
            y += 6.0;
        }

        panel.hint(ctx.ui, "ESC or SPACE to go back", &style);
    }

    fn draw_gameplay(&self, ctx: &mut GameContext) {
        let cx = ctx.window_size.x / 2.0;
        let cy = ctx.window_size.y / 2.0;

        self.draw_hud(ctx, cx);
        self.draw_timer_bars(ctx);

        let theme = ChaosTheme::for_mode(self.chaos_mode);
        if let Some(banner) = theme.banner_text {
            let color = Color::new(
                theme.banner_color.x, theme.banner_color.y, theme.banner_color.z, theme.banner_color.w,
            );
            ctx.ui.label_centered_styled(banner, Vec2::new(cx, ctx.window_size.y - 14.0), color, 14.0);
        }

        if self.state == GameState::GameOver {
            let style = self.menu_style();
            let panel = MenuPanel::new("OUT OF FROGS", Vec2::new(cx, cy), 400.0, 2);
            let mut y = panel.begin(ctx.ui, &style);
            y = panel.line(ctx.ui, y, &self.final_score_line(), &style);
            panel.line(ctx.ui, y, "SPACE / ENTER to play again", &style);
            panel.hint(ctx.ui, "ESC for title screen", &style);
        }

        if self.pause.is_active() {
            self.pause.draw(ctx.ui, ctx.window_size, &self.menu_style());
        }
    }

    /// Top band: pooled score left, round center, per-frog lives right.
    fn draw_hud(&self, ctx: &mut GameContext, cx: f32) {
        ctx.ui.label(&format!("SCORE {}", self.score), Vec2::new(24.0, 16.0));
        ctx.ui.label_centered(
            &format!("ROUND {}  -  HOMES {}/5", self.round, self.homes.iter().filter(|&&h| h).count()),
            Vec2::new(cx, 16.0),
        );
        match self.mode {
            GameMode::SinglePlayer => {
                let lives = self.frogs.first().map_or(0, |f| f.lives);
                ctx.ui.label(&format!("FROGS {lives}"), Vec2::new(ctx.window_size.x - 130.0, 16.0));
            }
            GameMode::TwoPlayerCoop => {
                let l1 = self.frogs.first().map_or(0, |f| f.lives);
                let l2 = self.frogs.get(1).map_or(0, |f| f.lives);
                ctx.ui.label(&format!("P1 {l1}  P2 {l2}"), Vec2::new(ctx.window_size.x - 160.0, 16.0));
            }
        }
        if self.state == GameState::Playing {
            ctx.ui.label_styled("ESC to pause", Vec2::new(24.0, 42.0),
                Color::new(0.6, 0.6, 0.65, 1.0), 12.0);
        }
    }

    /// Bottom band: one attempt-timer bar per frog (P1 left, P2 right).
    fn draw_timer_bars(&self, ctx: &mut GameContext) {
        let full = attempt_timer(self.chaos_mode);
        let band_top = ctx.window_size.y - BAND;
        let bar_h = 14.0;
        let bar_y = band_top + (BAND - bar_h) / 2.0;
        let count = self.frogs.len().max(1) as f32;
        let slot_w = ctx.window_size.x / count;

        for (i, frog) in self.frogs.iter().enumerate() {
            let margin = 60.0;
            let max_w = slot_w - 2.0 * margin;
            let x = i as f32 * slot_w + margin;
            let frac = (frog.timer / full).clamp(0.0, 1.0);
            let color = if frog.retired {
                Color::new(0.3, 0.3, 0.35, 1.0)
            } else if frac < 0.25 {
                Color::new(1.0, 0.3, 0.25, 1.0)
            } else {
                Color::new(0.4, 0.95, 0.5, 1.0)
            };
            ctx.ui.rect_border(
                Rect::new(x, bar_y, max_w, bar_h),
                Color::new(0.5, 0.5, 0.55, 1.0), 1.0, 2.0,
            );
            if !frog.retired && frac > 0.0 {
                ctx.ui.rect(Rect::new(x + 2.0, bar_y + 2.0, (max_w - 4.0) * frac, bar_h - 4.0), color);
            }
            ctx.ui.label_styled(
                &format!("P{}", i + 1),
                Vec2::new(x - 28.0, bar_y + bar_h - 2.0),
                Color::new(0.7, 0.7, 0.75, 1.0),
                12.0,
            );
        }
    }

    fn final_score_line(&self) -> String {
        format!("Score {}  -  round {}  -  {} homes", self.score, self.round, self.total_homes)
    }
}
