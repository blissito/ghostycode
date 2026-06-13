//! Optional EasyBits MCP API key entry screen for onboarding.
//!
//! The user can paste their EasyBits API key or press Enter with an empty
//! field to skip. When a key is entered, it is written to `mcp.json` via
//! `crate::mcp::add_server_config`, enabling the bundled-but-disabled
//! EasyBits MCP server.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::localization::MessageId;
use crate::palette;
use crate::tui::app::App;

pub fn lines(app: &App) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(Span::styled(
            app.tr(MessageId::OnboardEasybitsTitle).to_string(),
            Style::default()
                .fg(palette::DEEPSEEK_SKY)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            app.tr(MessageId::OnboardEasybitsBlurb).to_string(),
            Style::default().fg(palette::TEXT_PRIMARY),
        )),
        Line::from(""),
        Line::from(Span::styled(
            app.tr(MessageId::OnboardEasybitsStep1).to_string(),
            Style::default().fg(palette::TEXT_PRIMARY),
        )),
        Line::from(Span::styled(
            app.tr(MessageId::OnboardEasybitsStep2).to_string(),
            Style::default().fg(palette::TEXT_PRIMARY),
        )),
        Line::from(""),
        Line::from(Span::styled(
            app.tr(MessageId::OnboardEasybitsSkipHint).to_string(),
            Style::default().fg(palette::TEXT_MUTED),
        )),
        Line::from(""),
    ];

    let masked = mask_key(&app.easybits_key_input);
    let placeholder = app.tr(MessageId::OnboardEasybitsPlaceholder).to_string();
    let display = if masked.is_empty() {
        placeholder
    } else {
        masked
    };
    lines.push(Line::from(vec![
        Span::styled(
            app.tr(MessageId::OnboardEasybitsLabel).to_string(),
            Style::default().fg(palette::TEXT_MUTED),
        ),
        Span::styled(
            display,
            Style::default()
                .fg(palette::TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(""));

    if let Some(message) = app.status_message.as_deref() {
        lines.push(Line::from(Span::styled(
            message.to_string(),
            Style::default().fg(palette::STATUS_WARNING),
        )));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        app.tr(MessageId::OnboardEasybitsFooter).to_string(),
        Style::default().fg(palette::TEXT_MUTED),
    )));

    lines
}

fn mask_key(input: &str) -> String {
    let trimmed = input.trim();
    let len = trimmed.chars().count();
    if len == 0 {
        return String::new();
    }
    if len <= 4 {
        return "*".repeat(len);
    }
    let visible: String = trimmed
        .chars()
        .rev()
        .take(4)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("{}{}", "*".repeat(len - 4), visible)
}
