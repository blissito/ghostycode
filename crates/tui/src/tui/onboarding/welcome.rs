//! Welcome screen content for onboarding.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::palette;

/// Ghosty mascot — the brand ghost with round glasses and three ruffles.
const MASCOT: &[&str] = &[
    "   .------.",
    "  /        \\",
    " ──(o)⌒(o)──",
    " |          |",
    " \\___/\\__/\\_/",
];

pub fn lines() -> Vec<Line<'static>> {
    let mut out: Vec<Line<'static>> = MASCOT
        .iter()
        .map(|row| {
            Line::from(Span::styled(
                (*row).to_string(),
                Style::default().fg(palette::GHOSTY_PURPLE),
            ))
        })
        .collect();
    out.push(Line::from(""));
    out.extend(vec![
        Line::from(Span::styled(
            "Ghosty Code",
            Style::default()
                .fg(palette::GHOSTY_PURPLE)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("Version {}", env!("CARGO_PKG_VERSION")),
            Style::default().fg(palette::TEXT_MUTED),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "A focused terminal workspace for longer model sessions.",
            Style::default().fg(palette::TEXT_PRIMARY),
        )),
        Line::from(Span::styled(
            "You'll add an API key, review trust for this directory, and then land in the chat.",
            Style::default().fg(palette::TEXT_MUTED),
        )),
        Line::from(Span::styled(
            "The main composer is multi-line, so you can write full prompts instead of squeezing everything into one line.",
            Style::default().fg(palette::TEXT_MUTED),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press Enter to continue.",
            Style::default().fg(palette::TEXT_PRIMARY),
        )),
        Line::from(Span::styled(
            "Ctrl+C exits at any point.",
            Style::default().fg(palette::TEXT_MUTED),
        )),
    ]);

    out
}
