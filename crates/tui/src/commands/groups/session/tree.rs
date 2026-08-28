use super::CommandResult;
use crate::commands::traits::{CommandInfo, RegisterCommand};
use crate::localization::MessageId;
use crate::session_tree::render_tree;
use crate::tui::app::App;
pub(in crate::commands) const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "tree",
    aliases: &[],
    usage: "/tree [interactive]",
    description_id: MessageId::CmdTreeDescription,
};
pub(in crate::commands) struct TreeCmd;
impl RegisterCommand for TreeCmd {
    fn info() -> &'static CommandInfo {
        &COMMAND_INFO
    }
    fn execute(app: &mut App, arg: Option<&str>) -> CommandResult {
        tree(app, arg)
    }
}
fn tree(app: &mut App, _arg: Option<&str>) -> CommandResult {
    let manager = match crate::session_manager::SessionManager::default_location() {
        Ok(m) => m,
        Err(e) => return CommandResult::error(format!("could not open sessions directory: {e}")),
    };
    if let Some(session_id) = app.current_session_id.clone() {
        if let Ok(mut session) = manager.load_session(&session_id) {
            session.ensure_journal();
            if let Some(journal) = session.journal.as_ref() {
                let rendered = render_tree(journal);
                let mut out = rendered;
                out.push_str("\nUse `/branch <entry_id>` to branch (moves leaf only, never rewrites history).\n");
                out.push_str("Use `/fork [session_id]` to fork this session at any node.\n");
                return CommandResult::message(out);
            }
        }
        if app.api_messages.is_empty() {
            return CommandResult::message(
                "(empty session — no entries yet)\nSend a message first, then `/tree` will show the entry journal.",
            );
        }
        let mut out = String::from("Active branch (linear — journal will be created on save):\n");
        for (i, msg) in app.api_messages.iter().enumerate() {
            let snippet: String = msg
                .content
                .iter()
                .filter_map(|b| match b {
                    crate::models::ContentBlock::Text { text, .. } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(" ");
            let short: String = snippet.chars().take(60).collect();
            let marker = if i + 1 == app.api_messages.len() {
                "*"
            } else {
                "●"
            };
            out.push_str(&format!("  {marker} [{i}] {}: {short}\n", msg.role));
        }
        out.push_str("\nUse `/branch <n>` with entry id after journal is saved.\n");
        return CommandResult::message(out);
    }
    CommandResult::message(
        "No active session. Use `/resume` to pick a session, then `/tree` to see its journal.",
    )
}
