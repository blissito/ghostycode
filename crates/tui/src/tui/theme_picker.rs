//! `/theme` picker with live preview.
//!
//! Built on [`crate::tui::settings_picker`]: navigation, filtering ownership,
//! and transactional preview/commit/rollback live in the shared controller.
//! Ocean-specific chrome (swatches, underwater surface, treatment copy) stays
//! here so the framework contract does not flatten visual character.
//!
//! Semantics preserved from the pre-framework picker:
//! - Up/Down emit a `ConfigUpdated{persist:false}` so the host swaps
//!   `app.ui_theme` immediately and the whole TUI re-paints under the modal.
//! - Enter persists (`persist:true`); Esc emits one more
//!   `ConfigUpdated{persist:false}` to restore the original theme name
//!   that was active when the picker opened.

use std::borrow::Cow;
use std::cell::RefCell;

use crossterm::event::{KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::localization::{Locale, MessageId, tr};
use crate::palette::{SELECTABLE_THEMES, ThemeId, UiTheme};
use crate::tui::menu_style;
use crate::tui::settings_picker::{
    PickerNavResult, SettingAvailability, SettingOption, SettingValues, SettingsPickerController,
    SettingsPickerLayout, handle_nav_key,
};
use crate::tui::views::{
    ActionHint, ModalKind, ModalView, ViewAction, ViewEvent, render_modal_footer,
    render_panel_scroll_rail, render_underwater_surface,
};

pub struct ThemePickerView {
    controller: SettingsPickerController,
    /// Cached UiTheme for `ThemeId::System`, captured once at construction
    /// so the per-frame render doesn't re-invoke `UiTheme::detect()` (which
    /// reads `COLORFGBG`) on every keystroke.
    system_ui_theme: UiTheme,
    /// Effective session treatment, reported separately from theme so the
    /// picker never claims an ombre is active under Terminal or Flat.
    ocean_treatment: crate::tui::ocean::OceanTreatment,
    /// User-configured background applied on top of every named-theme preview.
    /// Without carrying this into the picker, a customized Solarized Light
    /// session would render ombre behind the modal but report Flat inside it.
    background_override: Option<Color>,
    row_hitboxes: RefCell<Vec<(Rect, usize)>>,
    last_mouse_selected: Option<usize>,
    /// UI locale captured from the app at construction (#4057 wave 2).
    locale: Locale,
}

fn theme_options(original_name: &str) -> Vec<SettingOption> {
    let current = original_name.trim().to_ascii_lowercase();
    SELECTABLE_THEMES
        .iter()
        .copied()
        .map(|id| {
            let name = id.name();
            SettingOption::builder(name, id.display_name())
                .summary(id.tagline())
                .detail(id.tagline())
                .help("Pick a theme with live preview")
                .values(SettingValues::new(
                    Cow::Owned(current.clone()),
                    Cow::Borrowed("system"),
                    Cow::Borrowed(name),
                ))
                .availability(SettingAvailability::Available)
                .tab("themes")
                .prefer_list_when_narrow(true)
                .build()
        })
        .collect()
}

impl ThemePickerView {
    #[cfg(test)]
    #[must_use]
    pub fn new(original_name: String) -> Self {
        Self::new_with_treatment(
            original_name,
            crate::tui::ocean::OceanTreatment::Ombre,
            Locale::En,
        )
    }

    #[cfg(test)]
    #[must_use]
    pub fn new_with_treatment(
        original_name: String,
        ocean_treatment: crate::tui::ocean::OceanTreatment,
        locale: Locale,
    ) -> Self {
        Self::new_with_treatment_and_background(original_name, ocean_treatment, locale, None)
    }

    fn new_with_treatment_and_background(
        original_name: String,
        ocean_treatment: crate::tui::ocean::OceanTreatment,
        locale: Locale,
        background_override: Option<Color>,
    ) -> Self {
        let options = theme_options(&original_name);
        let mut controller = SettingsPickerController::new(options, original_name.clone());
        // Land on the persisted theme when it matches a selectable id.
        let normalized = original_name.trim().to_ascii_lowercase();
        if let Some(source) = SELECTABLE_THEMES
            .iter()
            .position(|id| id.name() == normalized)
        {
            let _ = controller.select_source_index(source);
        }
        Self {
            controller,
            system_ui_theme: UiTheme::detect(),
            ocean_treatment,
            background_override,
            row_hitboxes: RefCell::new(Vec::new()),
            last_mouse_selected: None,
            locale,
        }
    }

    /// Construct behind type erasure before returning to the async event loop.
    /// Keeping the concrete picker out of that already-large future prevents
    /// transient modal values from inflating the main-thread stack frame.
    #[must_use]
    pub fn boxed_with_treatment(
        original_name: String,
        ocean_treatment: crate::tui::ocean::OceanTreatment,
        locale: Locale,
        background_override: Option<Color>,
    ) -> Box<dyn ModalView> {
        Box::new(Self::new_with_treatment_and_background(
            original_name,
            ocean_treatment,
            locale,
            background_override,
        ))
    }

    fn current(&self) -> ThemeId {
        self.controller
            .selected_id()
            .and_then(|name| {
                SELECTABLE_THEMES
                    .iter()
                    .copied()
                    .find(|id| id.name() == name)
            })
            .unwrap_or(ThemeId::System)
    }

    #[cfg(test)]
    fn selected(&self) -> usize {
        self.controller.selected_source_index().unwrap_or(0)
    }

    /// Resolve a theme to a `UiTheme`, returning the cached `System`
    /// resolution to avoid repeated env-var reads inside `render`.
    fn ui_theme_for(&self, id: ThemeId) -> UiTheme {
        let theme = if matches!(id, ThemeId::System) {
            self.system_ui_theme
        } else {
            id.ui_theme()
        };
        self.background_override
            .map_or(theme, |background| theme.with_background_color(background))
    }

    fn preview_event(&self) -> ViewAction {
        ViewAction::Emit(ViewEvent::ConfigUpdated {
            key: "theme".to_string(),
            value: self.current().name().to_string(),
            persist: false,
        })
    }

    fn commit_event(&self) -> ViewAction {
        ViewAction::EmitAndClose(ViewEvent::ConfigUpdated {
            key: "theme".to_string(),
            value: self.current().name().to_string(),
            persist: true,
        })
    }

    fn revert_event(&self) -> ViewAction {
        ViewAction::EmitAndClose(ViewEvent::ConfigUpdated {
            key: "theme".to_string(),
            value: self.controller.original_id().to_string(),
            persist: false,
        })
    }

    fn action_from_nav(&self, result: PickerNavResult) -> ViewAction {
        match result {
            PickerNavResult::Preview => self.preview_event(),
            PickerNavResult::Commit => self.commit_event(),
            PickerNavResult::Cancel => self.revert_event(),
            PickerNavResult::ItemAction | PickerNavResult::None => ViewAction::None,
        }
    }

    fn move_up(&mut self) -> ViewAction {
        let result = self.controller.move_up();
        self.action_from_nav(result)
    }

    fn move_down(&mut self) -> ViewAction {
        let result = self.controller.move_down();
        self.action_from_nav(result)
    }
}

impl ModalView for ThemePickerView {
    fn kind(&self) -> ModalKind {
        ModalKind::ThemePicker
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) -> ViewAction {
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.last_mouse_selected = None;
                self.move_up()
            }
            MouseEventKind::ScrollDown => {
                self.last_mouse_selected = None;
                self.move_down()
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let clicked = self.row_hitboxes.borrow().iter().find_map(|(rect, idx)| {
                    rect.contains(ratatui::layout::Position::new(mouse.column, mouse.row))
                        .then_some(*idx)
                });
                if let Some(idx) = clicked {
                    let commit = self.last_mouse_selected == Some(idx)
                        && self.controller.selected_source_index() == Some(idx);
                    let nav = self.controller.select_source_index(idx);
                    self.last_mouse_selected = Some(idx);
                    if commit {
                        self.commit_event()
                    } else {
                        self.action_from_nav(nav)
                    }
                } else {
                    ViewAction::None
                }
            }
            _ => ViewAction::None,
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> ViewAction {
        // Theme picker keeps digit-jump / vim keys; search typing stays off so
        // `j`/`k` and `1`..=`9` retain their navigation meaning.
        let result = handle_nav_key(&mut self.controller, key, false);
        self.action_from_nav(result)
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.row_hitboxes.borrow_mut().clear();
        // The live theme has already been swapped under us via ConfigUpdated,
        // so we pull the *current* preview's UiTheme from the cursor row to
        // skin the modal chrome. That way the popup itself shifts color as
        // the cursor moves, matching what the background will look like
        // after Enter. We keep the live `surface_bg` (not the shared ink) and
        // the bare `Clear` so the preview backdrop reads as intended.
        let live = self.ui_theme_for(self.current());
        let inner =
            render_underwater_surface(area, buf, tr(self.locale, MessageId::ThemeSurfaceTitle));

        let content = render_modal_footer(
            inner,
            buf,
            &[
                ActionHint::new("↑/↓", "preview"),
                ActionHint::new("Enter", "save"),
                ActionHint::new("Esc", "revert"),
            ],
        );

        // Theme rows prefer list-when-narrow; layout still drives scroll math.
        let _layout = SettingsPickerLayout::resolve(content, 34, self.controller.selected_option());

        let mut lines: Vec<Line> = Vec::with_capacity(SELECTABLE_THEMES.len() + 3);
        let treatment = if matches!(self.current(), ThemeId::Terminal) {
            tr(self.locale, MessageId::ThemeTreatmentOmbreUnavailable)
        } else if self.ocean_treatment.is_flat()
            || crate::tui::ocean::OceanRamp::for_theme(&live).is_none()
        {
            tr(self.locale, MessageId::ThemeTreatmentFlatActive)
        } else {
            tr(self.locale, MessageId::ThemeTreatmentOmbreActive)
        };
        lines.push(Line::from(Span::styled(
            treatment,
            Style::default().fg(live.text_hint),
        )));
        lines.push(Line::from(""));

        let header_rows = lines.len();
        let visible_rows = usize::from(content.height)
            .saturating_sub(header_rows)
            .max(1);
        let source_count = self.controller.visible().len();
        let selected_visible = self.controller.selected_visible();
        let max_start = source_count.saturating_sub(visible_rows);
        let start = selected_visible
            .saturating_sub(visible_rows.saturating_sub(1))
            .min(max_start);
        let content = render_panel_scroll_rail(
            content,
            buf,
            source_count.saturating_add(header_rows),
            start,
            visible_rows,
            true,
        );

        for (visible_idx, &source_idx) in self
            .controller
            .visible()
            .iter()
            .enumerate()
            .skip(start)
            .take(visible_rows)
        {
            let row_y = content.y.saturating_add(lines.len() as u16);
            self.row_hitboxes
                .borrow_mut()
                .push((Rect::new(content.x, row_y, content.width, 1), source_idx));
            let id = SELECTABLE_THEMES
                .get(source_idx)
                .copied()
                .unwrap_or(ThemeId::System);
            let is_selected = visible_idx == selected_visible;
            let row_style = if is_selected {
                menu_style::theme_selected_row_style(&live)
            } else {
                Style::default().fg(live.text_body)
            };
            let tagline_style = if is_selected {
                Style::default().fg(live.text_muted).bg(live.selection_bg)
            } else {
                Style::default().fg(live.text_dim)
            };
            let number_style = if is_selected {
                Style::default()
                    .fg(live.status_working)
                    .bg(live.selection_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(live.text_hint)
            };
            let pointer = crate::tui::glyphs::selection_marker(is_selected);

            // 3-cell color swatch per row using the candidate theme's own
            // accent + panel + border colors so the picker doubles as a
            // legend. Use the cached resolver so `System` doesn't repeat
            // `UiTheme::detect()`.
            let row_theme = self.ui_theme_for(id);
            let swatch = vec![
                Span::styled("  ", Style::default().bg(row_theme.surface_bg)),
                Span::styled("  ", Style::default().bg(row_theme.panel_bg)),
                Span::styled("  ", Style::default().bg(row_theme.status_working)),
                Span::styled("  ", Style::default().bg(row_theme.mode_yolo)),
                Span::styled("  ", Style::default().bg(row_theme.mode_plan)),
            ];

            let mut spans: Vec<Span> = Vec::with_capacity(8);
            spans.push(Span::styled(format!(" {pointer} "), row_style));
            spans.push(Span::styled(format!("{}. ", visible_idx + 1), number_style));
            spans.push(Span::styled(
                format!("{:<22}", id.display_name()),
                row_style,
            ));
            spans.extend(swatch);
            spans.push(Span::raw("  "));

            let prefix_width = Line::from(spans.clone()).width();
            let tagline = crate::tui::ui_text::semantic_truncate(
                id.tagline(),
                usize::from(content.width).saturating_sub(prefix_width),
            );
            spans.push(Span::styled(tagline, tagline_style));

            lines.push(Line::from(spans));
        }

        Paragraph::new(lines).render(content, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn selected_name(action: &ViewAction) -> Option<&str> {
        match action {
            ViewAction::Emit(ViewEvent::ConfigUpdated { key, value, .. })
            | ViewAction::EmitAndClose(ViewEvent::ConfigUpdated { key, value, .. })
                if key == "theme" =>
            {
                Some(value.as_str())
            }
            _ => None,
        }
    }

    #[test]
    fn opens_at_persisted_theme() {
        let v = ThemePickerView::new("tokyo-night".to_string());
        assert_eq!(v.current(), ThemeId::TokyoNight);
    }

    #[test]
    fn unknown_persisted_name_falls_back_to_first_row() {
        let v = ThemePickerView::new("not-a-real-theme".to_string());
        assert_eq!(v.selected(), 0);
        assert_eq!(v.current(), ThemeId::System);
    }

    #[test]
    fn arrow_down_previews_next_theme() {
        let mut v = ThemePickerView::new("system".to_string());
        let action = v.handle_key(key(KeyCode::Down));
        assert!(matches!(action, ViewAction::Emit(_)));
        assert_eq!(selected_name(&action), Some(ThemeId::Terminal.name()));
    }

    #[test]
    fn mouse_wheel_previews_and_second_row_click_commits() {
        let mut v = ThemePickerView::new("system".to_string());
        let wheel = v.handle_mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        });
        assert!(matches!(wheel, ViewAction::Emit(_)));
        assert_eq!(selected_name(&wheel), Some(ThemeId::Terminal.name()));

        let area = Rect::new(0, 0, 100, 30);
        let mut buf = Buffer::empty(area);
        v.render(area, &mut buf);
        let (rect, idx) = v.row_hitboxes.borrow()[2];
        let click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: rect.x,
            row: rect.y,
            modifiers: KeyModifiers::NONE,
        };
        let preview = v.handle_mouse(click);
        assert!(matches!(preview, ViewAction::Emit(_)));
        assert_eq!(v.selected(), idx);
        let commit = v.handle_mouse(click);
        assert!(matches!(commit, ViewAction::EmitAndClose(_)));
    }

    #[test]
    fn arrow_navigation_wraps_at_picker_edges() {
        let mut v = ThemePickerView::new("system".to_string());
        let last = SELECTABLE_THEMES.last().unwrap();

        let action = v.handle_key(key(KeyCode::Up));
        assert_eq!(selected_name(&action), Some(last.name()));

        let action = v.handle_key(key(KeyCode::Down));
        assert_eq!(selected_name(&action), Some(SELECTABLE_THEMES[0].name()));
    }

    #[test]
    fn enter_commits_with_persist_true() {
        let mut v = ThemePickerView::new("system".to_string());
        v.handle_key(key(KeyCode::Down));
        v.handle_key(key(KeyCode::Down));
        v.handle_key(key(KeyCode::Down));
        v.handle_key(key(KeyCode::Down));
        v.handle_key(key(KeyCode::Down)); // -> CatppuccinMocha
        let action = v.handle_key(key(KeyCode::Enter));
        match action {
            ViewAction::EmitAndClose(ViewEvent::ConfigUpdated {
                key,
                value,
                persist,
            }) => {
                assert_eq!(key, "theme");
                assert_eq!(value, ThemeId::CatppuccinMocha.name());
                assert!(persist);
            }
            other => panic!("expected commit, got {other:?}"),
        }
    }

    #[test]
    fn esc_reverts_to_original() {
        let mut v = ThemePickerView::new("dracula".to_string());
        v.handle_key(key(KeyCode::Up));
        v.handle_key(key(KeyCode::Up));
        let action = v.handle_key(key(KeyCode::Esc));
        match action {
            ViewAction::EmitAndClose(ViewEvent::ConfigUpdated {
                key,
                value,
                persist,
            }) => {
                assert_eq!(key, "theme");
                assert_eq!(value, "dracula");
                assert!(!persist);
            }
            other => panic!("expected revert, got {other:?}"),
        }
    }

    #[test]
    fn digit_jumps_to_row() {
        let mut v = ThemePickerView::new("system".to_string());
        let action = v.handle_key(key(KeyCode::Char('6')));
        // Row 6 (1-indexed) -> index 5 -> CatppuccinMocha
        assert_eq!(
            selected_name(&action),
            Some(ThemeId::CatppuccinMocha.name())
        );
    }

    #[test]
    fn digit_zero_is_rejected_not_remapped_to_row_zero() {
        let mut v = ThemePickerView::new("dracula".to_string());
        let before = v.selected();
        let action = v.handle_key(key(KeyCode::Char('0')));
        assert!(matches!(action, ViewAction::None));
        assert_eq!(v.selected(), before, "'0' should not move the cursor");
    }

    #[test]
    fn render_does_not_panic_on_zero_sized_area() {
        // The picker historically panicked here via .max(W).max(H) floors
        // that produced dimensions larger than the available area, then
        // underflowed the centering arithmetic.
        let v = ThemePickerView::new("system".to_string());
        let outer = ratatui::layout::Rect::new(0, 0, 10, 10);
        let area = ratatui::layout::Rect::new(0, 0, 0, 0);
        let mut buf = ratatui::buffer::Buffer::empty(outer);
        v.render(area, &mut buf);
    }

    #[test]
    fn render_does_not_panic_on_tiny_area() {
        // 20×6 is smaller than every soft floor the picker prefers.
        let v = ThemePickerView::new("system".to_string());
        let area = ratatui::layout::Rect::new(0, 0, 20, 6);
        let mut buf = ratatui::buffer::Buffer::empty(area);
        v.render(area, &mut buf);
    }

    #[test]
    fn treatment_report_names_effective_appearance() {
        let area = ratatui::layout::Rect::new(0, 0, 100, 30);

        let flat = ThemePickerView::new_with_treatment(
            "dark".to_string(),
            crate::tui::ocean::OceanTreatment::Flat,
            Locale::En,
        );
        let mut flat_buf = ratatui::buffer::Buffer::empty(area);
        flat.render(area, &mut flat_buf);
        let flat_text = flat_buf
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(flat_text.contains("Treatment  Flat — active"));

        let terminal = ThemePickerView::new_with_treatment(
            "terminal".to_string(),
            crate::tui::ocean::OceanTreatment::Ombre,
            Locale::En,
        );
        let mut terminal_buf = ratatui::buffer::Buffer::empty(area);
        terminal.render(area, &mut terminal_buf);
        let terminal_text = terminal_buf
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(terminal_text.contains("Ombre unavailable"));
        assert!(terminal_text.contains("Terminal owns the background"));

        let solarized = ThemePickerView::new_with_treatment(
            "solarized-light".to_string(),
            crate::tui::ocean::OceanTreatment::Ombre,
            Locale::En,
        );
        let mut solarized_buf = ratatui::buffer::Buffer::empty(area);
        solarized.render(area, &mut solarized_buf);
        let solarized_text = solarized_buf
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(solarized_text.contains("Treatment  Flat — active"));
        assert!(!solarized_text.contains("Treatment  Ombre — active"));

        let solarized_custom = ThemePickerView::new_with_treatment_and_background(
            "solarized-light".to_string(),
            crate::tui::ocean::OceanTreatment::Ombre,
            Locale::En,
            Some(Color::Rgb(0x1a, 0x1b, 0x26)),
        );
        let mut solarized_custom_buf = ratatui::buffer::Buffer::empty(area);
        solarized_custom.render(area, &mut solarized_custom_buf);
        let solarized_custom_text = solarized_custom_buf
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(solarized_custom_text.contains("Treatment  Ombre — active"));
        assert!(!solarized_custom_text.contains("Treatment  Flat — active"));
    }

    #[test]
    fn every_selectable_theme_previews_and_renders_through_the_same_surface() {
        let area = ratatui::layout::Rect::new(0, 0, 100, 32);
        let mut view = ThemePickerView::new("system".to_string());

        for (index, expected) in SELECTABLE_THEMES.iter().copied().enumerate() {
            let _ = view.controller.select_source_index(index);
            assert_eq!(view.current(), expected);
            assert_eq!(selected_name(&view.preview_event()), Some(expected.name()));

            let mut buf = ratatui::buffer::Buffer::empty(area);
            view.render(area, &mut buf);
            let text = buf
                .content()
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>();
            assert!(
                text.contains(expected.display_name()),
                "{} was not represented in its live preview surface",
                expected.name()
            );
            assert!(text.contains("Treatment"));
            assert!(text.contains("Enter save"));
        }
    }

    #[test]
    fn render_semantically_truncates_taglines_at_narrow_width() {
        let v = ThemePickerView::new("system".to_string());
        let area = ratatui::layout::Rect::new(0, 0, 56, 12);
        let mut buf = ratatui::buffer::Buffer::empty(area);
        v.render(area, &mut buf);
        let rows = (0..area.height)
            .map(|y| {
                (0..area.width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>();
        let text = rows.join("\n");

        assert!(text.contains('…'), "{text}");
        for (idx, row) in rows.iter().enumerate() {
            assert!(
                crate::tui::ui_text::text_display_width(row) <= usize::from(area.width),
                "line {idx} overflows: {row:?}"
            );
        }
    }

    /// The four terminal sizes the v0.8.66 modal blocker (#3732) requires
    /// every overlay to remain readable and fully operable at.
    const BLOCKER_SIZES: [(u16, u16); 4] = [(80, 24), (100, 30), (120, 32), (160, 40)];

    #[test]
    fn theme_picker_is_usable_and_opaque_at_blocker_sizes() {
        use crate::tui::views::ViewStack;
        use ratatui::{buffer::Buffer, layout::Rect};
        use unicode_width::UnicodeWidthStr;

        for (w, h) in BLOCKER_SIZES {
            let area = Rect::new(0, 0, w, h);
            let mut buf = Buffer::empty(area);
            for y in 0..h {
                for x in 0..w {
                    buf[(x, y)].set_symbol("X");
                }
            }
            let mut stack = ViewStack::new();
            stack.push(ThemePickerView::new("system".to_string()));
            stack.render(area, &mut buf);

            let rows: Vec<String> = (0..h)
                .map(|y| {
                    (0..w)
                        .map(|x| buf[(x, y)].symbol().to_string())
                        .collect::<String>()
                })
                .collect();
            let text = rows.join("\n");

            for label in ["preview", "save", "revert"] {
                assert!(text.contains(label), "{w}x{h}: missing footer '{label}'");
            }
            assert!(
                !text.contains('X'),
                "{w}x{h}: background bleed-through into modal surface"
            );
            // The theme picker paints the *live* theme surface (not the shared
            // ink), so assert the center cell is painted (no surviving
            // sentinel) rather than checking a fixed background color.
            assert_ne!(
                buf[(w / 2, h / 2)].symbol(),
                "X",
                "{w}x{h}: modal interior must be painted"
            );
            for (y, row) in rows.iter().enumerate() {
                assert!(
                    UnicodeWidthStr::width(row.trim_end()) <= w as usize,
                    "{w}x{h}: row {y} overflows width: {row:?}"
                );
            }
        }
    }

    #[test]
    fn theme_picker_uses_shared_settings_controller() {
        let v = ThemePickerView::new("dracula".to_string());
        assert_eq!(v.controller.original_id(), "dracula");
        assert_eq!(v.controller.selected_id(), Some("dracula"));
        assert_eq!(v.controller.visible().len(), SELECTABLE_THEMES.len());
    }
}
