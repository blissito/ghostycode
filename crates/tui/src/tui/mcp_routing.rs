//! MCP manager formatting and UI action helpers.

use crate::mcp::{
    EASYBITS_DOCS_URL, EASYBITS_MCP_NAME, EASYBITS_MCP_URL, McpManagerSnapshot, McpServerSnapshot,
};
use crate::tui::app::App;
use crate::tui::history::HistoryCell;
use crate::tui::pager::PagerView;

pub(super) fn format_mcp_manager(snapshot: &McpManagerSnapshot) -> String {
    let mut lines = vec![
        format!("MCP config: {}", snapshot.config_path.display()),
        format!("Config exists: {}", snapshot.config_exists),
    ];
    if snapshot.restart_required {
        lines.push(
            "Restart required: MCP config changed; the current model-visible MCP tool pool is not hot-reloaded."
                .to_string(),
        );
    }
    lines.push(String::new());

    if snapshot.servers.is_empty() {
        lines.push("No MCP servers configured.".to_string());
    } else {
        for server in &snapshot.servers {
            push_server(lines.as_mut(), server);
        }
    }

    lines.push(String::new());
    lines.push("/mcp add | enable | disable | remove | reload".to_string());
    lines.join("\n")
}

fn push_server(lines: &mut Vec<String>, server: &McpServerSnapshot) {
    let state = if server.enabled {
        if server.connected {
            "connected"
        } else if server.error.is_some() {
            "failed"
        } else {
            "enabled"
        }
    } else {
        "disabled"
    };
    lines.push(format!(
        "- {} [{state}] {} {}",
        server.name, server.transport, server.command_or_url
    ));
    lines.push(format!(
        "  {} tools, {} resources, {} prompts",
        server.tools.len(),
        server.resources.len(),
        server.prompts.len()
    ));
    // The bundled EasyBits server ships disabled until the user supplies a key;
    // point them at the one-command setup and where to get the key.
    if server.name == EASYBITS_MCP_NAME && !server.enabled {
        lines.push(format!(
            "  /mcp add http {EASYBITS_MCP_NAME} {EASYBITS_MCP_URL} --bearer <KEY>"
        ));
        lines.push(format!("  key: {EASYBITS_DOCS_URL}"));
    }
    if let Some(error) = server.error.as_ref() {
        lines.push(format!("  error: {error}"));
    }
}

pub(super) fn open_mcp_manager_pager(app: &mut App, snapshot: &McpManagerSnapshot) {
    let width = app
        .viewport
        .last_transcript_area
        .map(|area| area.width)
        .unwrap_or(100)
        .saturating_sub(4);
    app.view_stack.push(PagerView::from_text(
        "MCP Manager".to_string(),
        &format_mcp_manager(snapshot),
        width.max(60),
    ));
}

pub(super) fn add_mcp_message(app: &mut App, content: String) {
    app.add_message(HistoryCell::System { content });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::McpDiscoveredItem;
    use std::path::PathBuf;

    #[test]
    fn manager_text_shows_failed_disabled_and_runtime_names() {
        let snapshot = McpManagerSnapshot {
            config_path: PathBuf::from("/tmp/mcp.json"),
            config_exists: true,
            restart_required: true,
            servers: vec![
                McpServerSnapshot {
                    name: "fs".to_string(),
                    enabled: true,
                    required: false,
                    transport: "stdio".to_string(),
                    command_or_url: "node server.js".to_string(),
                    connect_timeout: 10,
                    execute_timeout: 60,
                    read_timeout: 120,
                    connected: true,
                    error: None,
                    tools: vec![McpDiscoveredItem {
                        name: "read".to_string(),
                        model_name: "mcp_fs_read".to_string(),
                        description: Some("Read a file".to_string()),
                    }],
                    resources: Vec::new(),
                    prompts: Vec::new(),
                },
                McpServerSnapshot {
                    name: "bad".to_string(),
                    enabled: true,
                    required: false,
                    transport: "http/sse".to_string(),
                    command_or_url: "https://example.invalid/mcp".to_string(),
                    connect_timeout: 10,
                    execute_timeout: 60,
                    read_timeout: 120,
                    connected: false,
                    error: Some("boom".to_string()),
                    tools: Vec::new(),
                    resources: Vec::new(),
                    prompts: Vec::new(),
                },
            ],
        };
        let text = format_mcp_manager(&snapshot);
        assert!(text.contains("Restart required"));
        assert!(text.contains("[connected]"));
        assert!(text.contains("[failed]"));
        assert!(text.contains("boom"));
        assert!(text.contains("1 tools, 0 resources, 0 prompts"));
    }
}
