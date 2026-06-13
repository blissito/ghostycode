//! Welcome screen content for onboarding.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::localization::MessageId;
use crate::palette;
use crate::tui::app::App;

/// Ghosty mascot — the brand ghost with round glasses and three ruffles.
const MASCOT: &[&str] = &[
    "   .------.",
    "  /        \\",
    " ──(o)⌒(o)──",
    " |          |",
    " \\___/\\__/\\_/",
];

pub fn lines(app: &App) -> Vec<Line<'static>> {
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
            app.tr(MessageId::OnboardWelcomeTagline).to_string(),
            Style::default().fg(palette::TEXT_PRIMARY),
        )),
        Line::from(Span::styled(
            app.tr(MessageId::OnboardWelcomeFlow).to_string(),
            Style::default().fg(palette::TEXT_MUTED),
        )),
        Line::from(Span::styled(
            app.tr(MessageId::OnboardWelcomeComposerHint).to_string(),
            Style::default().fg(palette::TEXT_MUTED),
        )),
        Line::from(""),
        Line::from(Span::styled(
            app.tr(MessageId::OnboardWelcomeContinue).to_string(),
            Style::default().fg(palette::TEXT_PRIMARY),
        )),
        Line::from(Span::styled(
            app.tr(MessageId::OnboardWelcomeExit).to_string(),
            Style::default().fg(palette::TEXT_MUTED),
        )),
    ]);

    out
}
