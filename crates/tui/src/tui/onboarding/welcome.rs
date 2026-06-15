//! Welcome screen content for onboarding.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::localization::MessageId;
use crate::palette;
use crate::tui::app::App;
use crate::tui::widgets::{MASCOT_BOTTOM, MASCOT_TOP, mascot_anim_tick, mascot_eyes_row};

pub fn lines(app: &App) -> Vec<Line<'static>> {
    // Same block ghost as the chat empty state, with the same idle
    // blink/glance/smile animation so onboarding and home agree.
    let mascot = [
        MASCOT_TOP.to_string(),
        mascot_eyes_row(mascot_anim_tick()),
        MASCOT_BOTTOM.to_string(),
    ];
    let mut out: Vec<Line<'static>> = mascot
        .iter()
        .map(|row| {
            Line::from(Span::styled(
                row.clone(),
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
