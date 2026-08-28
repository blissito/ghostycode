//! `/fork` command — interactive picker (#576) + direct fork.

use crate::commands::traits::{CommandInfo, RegisterCommand};
use crate::localization::MessageId;
use crate::tui::app::App;
use crate::tui::session_picker::SessionPickerView;

use super::CommandResult;

pub(in crate::commands) const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "fork",
    aliases: &["f"],
    usage: "/fork [session_id|picker]",
    description_id: MessageId::CmdForkDescription,
};

pub(in crate::commands) struct ForkCmd;

impl RegisterCommand for ForkCmd {
    fn info() -> &'static CommandInfo {
        &COMMAND_INFO
    }

    fn execute(app: &mut App, arg: Option<&str>) -> CommandResult {
        let trimmed = arg.map(str::trim).filter(|s| !s.is_empty());
        if let Some(a) = trimmed {
            if matches!(
                a.to_ascii_lowercase().as_str(),
                "picker" | "list" | "--picker" | "pick"
            ) {
                app.view_stack
                    .push(SessionPickerView::new(&app.workspace, app.ui_locale));
                return CommandResult::message(
                    "Fork picker: select a session and then run `/fork <id>` to fork it.",
                );
            }
            return super::session::fork_from_session(app, a);
        }
        super::session::fork(app)
    }
}
