//! Shared hover-hit abstraction for OSC-8 terminal links.

// Public API surface; hover_layer + mouse_ui consume these primitives.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
};

use crate::tui::ocean;

/// Kind of interactive surface under the pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoverTargetKind {
    Link,
    /// A compact row that omitted part of its full source label.
    TruncatedText,
}

/// Result of a hover hit-test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoverHit {
    pub kind: HoverTargetKind,
    pub area: Rect,
    /// Optional label for tooltips / copy affordances.
    pub label: String,
    /// Whether a hover-only `copy` chip should be shown.
    pub copyable: bool,
}

/// Underline + glow for OSC-8 / file links under the pointer.
#[must_use]
pub fn link_hover_style(fg: Color, reduced_motion: bool, elapsed_ms: u128) -> Style {
    let scale = if reduced_motion {
        1.15
    } else {
        let phase = (elapsed_ms % 1_200) as f32 / 1_200.0;
        1.10 + (phase * std::f32::consts::TAU).sin().abs() * 0.12
    };
    Style::default()
        .fg(ocean::scale_color(fg, scale))
        .add_modifier(Modifier::UNDERLINED | Modifier::BOLD)
}

/// Hover-only `copy` chip text (display width fixed).
#[must_use]
pub fn copy_affordance() -> &'static str {
    "⧉ copy"
}

/// Whether `column,row` hits `area`.
#[must_use]
pub fn point_in_rect(column: u16, row: u16, area: Option<Rect>) -> bool {
    let Some(area) = area else {
        return false;
    };
    column >= area.x
        && column < area.x.saturating_add(area.width)
        && row >= area.y
        && row < area.y.saturating_add(area.height)
}

/// Hit-test a list of rectangular targets; returns the topmost match.
#[must_use]
pub fn hit_test(column: u16, row: u16, targets: &[HoverHit]) -> Option<&HoverHit> {
    targets
        .iter()
        .rev()
        .find(|t| point_in_rect(column, row, Some(t.area)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_test_returns_topmost() {
        let targets = vec![
            HoverHit {
                kind: HoverTargetKind::Link,
                area: Rect::new(0, 0, 10, 1),
                label: "a".into(),
                copyable: false,
            },
            HoverHit {
                kind: HoverTargetKind::Link,
                area: Rect::new(2, 0, 4, 1),
                label: "b".into(),
                copyable: true,
            },
        ];
        let hit = hit_test(3, 0, &targets).expect("hit");
        assert_eq!(hit.kind, HoverTargetKind::Link);
    }

    #[test]
    fn copy_affordance_is_stable() {
        assert_eq!(copy_affordance(), "⧉ copy");
    }
}
