//! UI text and HUD: menus, the top score band, the bottom timer band, and
//! the game-over overlay. Every player-facing string goes through
//! `ctx.strings.tr(...)` (en + pirate locales; the title's Language item
//! cycles). World rendering is sprites + the board tilemap (drawn by the
//! engine's default render path).

use engine_core::prelude::*;

use crate::achievements::DISPLAY_SECTIONS;
use crate::constants::*;
use crate::gameplay::rules::attempt_timer;
use crate::menu::{
    achievements_panel, chaos_label_key, chaos_panel, mode_hint_key, title_panel,
    TitleItem, TITLE_ITEMS,
};
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
        let strings = &ctx.strings;
        let title = strings.tr("title.window").to_string();
        // Rows come from the same TITLE_ITEMS list the input half hit-tests
        // against, so the drawn panel and the click geometry always agree.
        let items: Vec<String> = TITLE_ITEMS
            .iter()
            .map(|item| match item {
                TitleItem::Single => strings.tr("title.single").to_string(),
                TitleItem::Coop => strings.tr("title.coop").to_string(),
                TitleItem::Achievements => strings.tr("title.achievements").to_string(),
                TitleItem::Language => format!(
                    "{}: {}",
                    strings.tr("title.language"),
                    strings.current_display_name()
                ),
                TitleItem::Exit => strings.tr("title.exit").to_string(),
            })
            .collect();
        let hint = strings.tr("title.hint").to_string();
        let controls = strings.tr("title.controls").to_string();
        let tagline = strings.tr("title.tagline").to_string();

        let panel = title_panel(&title, ctx.window_size);
        let mut y = panel.begin(ctx.ui, &style);
        for (i, item) in items.iter().enumerate() {
            y = panel.item(ctx.ui, y, item, i as u8 == selection, &style);
        }
        panel.hint(ctx.ui, &hint, &style);

        let rect = panel.panel_rect();
        let cx = ctx.window_size.x / 2.0;
        ctx.ui.label_centered(&controls, Vec2::new(cx, rect.y + rect.height + 24.0));
        ctx.ui.label_centered(&tagline, Vec2::new(cx, rect.y + rect.height + 48.0));
    }

    fn draw_mode_select(&self, ctx: &mut GameContext, selection: u8) {
        let style = self.menu_style();
        let title = ctx.strings.tr("chaos.window").to_string();
        let labels: Vec<String> = ChaosMode::ALL
            .iter()
            .map(|&m| ctx.strings.tr(chaos_label_key(m)).to_string())
            .collect();
        let hint_mode = ChaosMode::ALL[selection as usize % ChaosMode::ALL.len()];
        let hint = ctx.strings.tr(mode_hint_key(hint_mode)).to_string();

        let panel = chaos_panel(&title, ctx.window_size);
        let mut y = panel.begin(ctx.ui, &style);
        for (i, &mode) in ChaosMode::ALL.iter().enumerate() {
            let c = ChaosTheme::for_mode(mode).banner_color;
            y = panel.item_colored(ctx.ui, y, &labels[i], c, i as u8 == selection, &style);
        }
        panel.hint(ctx.ui, &hint, &style);
    }

    fn draw_achievements(&self, ctx: &mut GameContext) {
        let style = self.menu_style();
        let cx = ctx.window_size.x / 2.0;
        let total = ctx.achievements.total();
        let unlocked = ctx.achievements.unlocked_count();
        let window_title = ctx.strings.tr("ach.window").to_string();
        let unlocked_word = ctx.strings.tr("ach.unlocked").to_string();
        let hint = ctx.strings.tr("ach.hint").to_string();

        let panel = achievements_panel(&window_title, ctx.window_size);
        let first_y = panel.begin(ctx.ui, &style);
        let rect = panel.panel_rect();
        ctx.ui.label_centered(
            &format!("{unlocked} / {total} {unlocked_word}"),
            Vec2::new(cx, first_y - 8.0),
        );

        let left = rect.x + 28.0;
        let mut y = first_y + 18.0;

        let locked_color = Color::new(0.45, 0.45, 0.5, 1.0);
        let unlocked_color = Color::new(1.0, 0.85, 0.25, 1.0);
        let desc_color = Color::new(0.75, 0.75, 0.8, 1.0);
        let header_color = Color::new(0.6, 0.75, 1.0, 1.0);

        for (section_key, ids) in DISPLAY_SECTIONS {
            let section = ctx.strings.tr(section_key).to_string();
            ctx.ui.label_styled(&section, Vec2::new(left, y), header_color, 16.0);
            y += 22.0;
            for id in *ids {
                let is_unlocked = ctx.achievements.is_unlocked(id);
                let Some(ach) = ctx.achievements.get(id) else { continue };

                let (marker, name_color) = if is_unlocked {
                    ("[X]", unlocked_color)
                } else {
                    ("[ ]", locked_color)
                };

                let name_line = format!("{marker} {}", ach.name);
                let desc = ach.description.clone();
                ctx.ui.label_styled(&name_line, Vec2::new(left + 8.0, y), name_color, 14.0);
                ctx.ui.label_styled(&desc, Vec2::new(left + 52.0, y + 16.0), desc_color, 12.0);
                y += 36.0;
            }
            y += 6.0;
        }

        panel.hint(ctx.ui, &hint, &style);
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
            let title = ctx.strings.tr("over.window").to_string();
            let again = ctx.strings.tr("over.again").to_string();
            let hint = ctx.strings.tr("over.hint").to_string();
            let score_line = self.final_score_line(ctx);
            let panel = MenuPanel::new(&title, Vec2::new(cx, cy), 400.0, 2);
            let mut y = panel.begin(ctx.ui, &style);
            y = panel.line(ctx.ui, y, &score_line, &style);
            panel.line(ctx.ui, y, &again, &style);
            panel.hint(ctx.ui, &hint, &style);
        }

        if self.pause.is_active() {
            let style = self.menu_style();
            let labels = PauseMenuLabels {
                title: ctx.strings.tr("pause.title"),
                items: [
                    ctx.strings.tr("pause.resume"),
                    ctx.strings.tr("pause.restart"),
                    ctx.strings.tr("pause.quit_title"),
                    ctx.strings.tr("pause.exit_game"),
                ],
                hint: ctx.strings.tr("pause.hint"),
            };
            self.pause.draw_labeled(ctx.ui, ctx.window_size, &style, &labels);
        }
    }

    /// Top band: pooled score left, round + homes center, per-frog lives right.
    fn draw_hud(&self, ctx: &mut GameContext, cx: f32) {
        let score_word = ctx.strings.tr("hud.score").to_string();
        let round_word = ctx.strings.tr("hud.round").to_string();
        let homes_word = ctx.strings.tr("hud.homes").to_string();
        ctx.ui.label(&format!("{score_word} {}", self.score), Vec2::new(24.0, 16.0));
        ctx.ui.label_centered(
            &format!(
                "{round_word} {}  -  {homes_word} {}/5",
                self.round,
                self.homes.iter().filter(|&&h| h).count()
            ),
            Vec2::new(cx, 16.0),
        );
        match self.mode {
            GameMode::SinglePlayer => {
                let frogs_word = ctx.strings.tr("hud.frogs").to_string();
                let lives = self.frogs.first().map_or(0, |f| f.lives);
                ctx.ui.label(&format!("{frogs_word} {lives}"), Vec2::new(ctx.window_size.x - 130.0, 16.0));
            }
            GameMode::TwoPlayerCoop => {
                let p1 = ctx.strings.tr("hud.p1").to_string();
                let p2 = ctx.strings.tr("hud.p2").to_string();
                let l1 = self.frogs.first().map_or(0, |f| f.lives);
                let l2 = self.frogs.get(1).map_or(0, |f| f.lives);
                ctx.ui.label(&format!("{p1} {l1}  {p2} {l2}"), Vec2::new(ctx.window_size.x - 160.0, 16.0));
            }
        }
        if self.state == GameState::Playing {
            let pause_hint = ctx.strings.tr("hud.pause_hint").to_string();
            ctx.ui.label_styled(&pause_hint, Vec2::new(24.0, 42.0),
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
        let player_words = [
            ctx.strings.tr("hud.p1").to_string(),
            ctx.strings.tr("hud.p2").to_string(),
        ];

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
                &player_words[i.min(1)],
                Vec2::new(x - 28.0, bar_y + bar_h - 2.0),
                Color::new(0.7, 0.7, 0.75, 1.0),
                12.0,
            );
        }
    }

    fn final_score_line(&self, ctx: &GameContext) -> String {
        format!(
            "{} {}  -  {} {}  -  {} {}",
            ctx.strings.tr("over.score"), self.score,
            ctx.strings.tr("over.round"), self.round,
            ctx.strings.tr("over.homes"), self.total_homes,
        )
    }
}
