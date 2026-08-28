//! `/lsp` command — enable/disable LSP integration.
//!
//! Bridges to config::config::lsp_command for actual execution.

use crate::commands::traits::{CommandInfo, RegisterCommand};
use crate::localization::MessageId;
use crate::tui::app::App;

use crate::commands::CommandResult;

pub(in crate::commands) const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "lsp",
    aliases: &[],
    usage: "/lsp [on|off|status]",
    description_id: MessageId::CmdLspDescription,
};

pub(in crate::commands) struct LspCmd;

impl RegisterCommand for LspCmd {
    fn info() -> &'static CommandInfo {
        &COMMAND_INFO
    }

    fn execute(app: &mut App, arg: Option<&str>) -> CommandResult {
        super::super::config::config::lsp_command(app, arg)
    }
}
