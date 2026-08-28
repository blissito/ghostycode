use super::CommandResult;
use crate::commands::traits::{CommandInfo, RegisterCommand};
use crate::localization::MessageId;
use crate::tui::app::App;
use crate::tui::session_picker::SessionPickerView;
use std::path::PathBuf;
pub(in crate::commands) const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "resume",
    aliases: &["r"],
    usage: "/resume [session_id|path/to/export.json]",
    description_id: MessageId::CmdResumeDescription,
};
pub(in crate::commands) struct ResumeCmd;
impl RegisterCommand for ResumeCmd {
    fn info() -> &'static CommandInfo {
        &COMMAND_INFO
    }
    fn execute(app: &mut App, arg: Option<&str>) -> CommandResult {
        resume(app, arg)
    }
}
fn resume(app: &mut App, arg: Option<&str>) -> CommandResult {
    if app.session_transition_blocked() {
        return CommandResult::error(
            "Cannot resume while runtime work is active. Wait for the turn to finish, or cancel it first.",
        );
    }
    let Some(raw) = arg.map(str::trim).filter(|s| !s.is_empty()) else {
        app.view_stack
            .push(SessionPickerView::new(&app.workspace, app.ui_locale));
        return CommandResult::ok();
    };
    let path = PathBuf::from(raw);
    if path.is_file() || raw.ends_with(".json") && std::path::Path::new(raw).exists() {
        return import_foreign(app, &path);
    }
    let ws_path = app.workspace.join(raw);
    if ws_path.is_file() {
        return import_foreign(app, &ws_path);
    }
    let manager = match crate::session_manager::SessionManager::default_location() {
        Ok(m) => m,
        Err(e) => return CommandResult::error(format!("could not open sessions directory: {e}")),
    };
    let session = manager
        .load_session(raw)
        .or_else(|_| manager.load_session_by_prefix(raw));
    match session {
        Ok(sess) => {
            let path = manager
                .sessions_dir()
                .join(format!("{}.json", sess.metadata.id));
            if path.exists() {
                return CommandResult::action(crate::tui::app::AppAction::LoadSession(path));
            }
            CommandResult::message(format!(
                "Resuming session {} ({})",
                crate::session_manager::truncate_id(&sess.metadata.id),
                sess.metadata.title
            ))
        }
        Err(e) => {
            if let Ok(container) = crate::session_tree::SessionImportContainer::from_json(raw) {
                return import_container(app, container);
            }
            CommandResult::error(format!(
                "Cannot resume '{raw}': {e}\nUse `/resume` without args to pick, or pass a session id, or a path to an exported session JSON."
            ))
        }
    }
}
fn import_foreign(app: &mut App, path: &PathBuf) -> CommandResult {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            return CommandResult::error(format!(
                "failed to read import file {}: {e}",
                path.display()
            ));
        }
    };
    if let Ok(container) = crate::session_tree::SessionImportContainer::from_json(&content) {
        return import_container(app, container);
    }
    if let Ok(foreign) = serde_json::from_str::<crate::session_manager::SavedSession>(&content) {
        let container = foreign.export_container("foreign");
        return import_container(app, container);
    }
    CommandResult::error(format!(
        "File {} is not a recognized session export",
        path.display()
    ))
}
fn import_container(
    app: &mut App,
    container: crate::session_tree::SessionImportContainer,
) -> CommandResult {
    let manager = match crate::session_manager::SessionManager::default_location() {
        Ok(m) => m,
        Err(e) => return CommandResult::error(format!("could not open sessions directory: {e}")),
    };
    let model = app.model.clone();
    let workspace = app.workspace.clone();
    let imported =
        match crate::session_manager::SavedSession::import_foreign(container, workspace, model) {
            Ok(s) => s,
            Err(e) => return CommandResult::error(format!("foreign import failed: {e}")),
        };
    let new_id = imported.metadata.id.clone();
    if let Err(e) = manager.save_session(&imported) {
        return CommandResult::error(format!("imported session could not be saved: {e}"));
    }
    app.current_session_id = Some(new_id.clone());
    app.current_session_metadata = Some(imported.metadata.clone());
    app.api_messages = imported.messages.clone();
    app.view_stack.push(SessionPickerView::new_selecting(
        &app.workspace,
        app.ui_locale,
        &new_id,
    ));
    CommandResult::message(format!(
        "Imported foreign session as {} ({} entries, leaf {})",
        crate::session_manager::truncate_id(&new_id),
        imported
            .journal
            .as_ref()
            .map(|j| j.entries.len())
            .unwrap_or(0),
        imported.leaf_id.as_deref().unwrap_or("(none)")
    ))
}
