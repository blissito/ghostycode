//! `Op`-in API in `crates/protocol` (issue #5261).
//!
//! The TUI engine already had an internal channel (`Op` in
//! `crates/tui/src/core/ops.rs` with `tx_op` / `rx_op` and `tx_steer`).
//! This protocol file formalizes that channel so TUI, CLI, app-server, and
//! tests share one serializable API. The wire is `OpEnvelope` + `Op`;
//! transports that already speak JSON (app-server, tests) can send the
//! envelope directly, while in-process callers continue to use the typed
//! enum.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ids::{SessionId, ThreadId};

/// Every `Op` is paired with the ids that route it. This is the
/// `Op`-in half of the `Op`-in / `EventMsg`-out contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpEnvelope {
    /// Monotonic `op:<n>` for dedup / tracing within a session.
    pub op_id: String,
    pub thread_id: ThreadId,
    pub session_id: SessionId,
    pub op: Op,
}

/// Operations that can be submitted to the core engine. This is the
/// protocol view of `crates/tui/src/core/ops::Op` — same lifecycle,
/// same provenance gate — but serializable and free of `mpsc` / `oneshot`
/// fields. In-process callers convert at the boundary; out-of-process
/// callers send the JSON directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Op {
    /// Drive one model turn: `role=user` content plus the resolved route
    /// receipt the engine will freeze at the client-freeze boundary. Headless
    /// and TUI must produce byte-identical `MessageRequest`s for identical
    /// `Op::SendMessage` payloads.
    SendMessage {
        content: String,
        /// Effective mode for this turn (`"plan" | "agent" | "operate"` etc).
        #[serde(default = "default_mode")]
        mode: String,
        /// Optional explicit route/model the caller resolved already (mirrors
        /// `ResolvedRuntimeRoute` in `crates_tui::route_runtime`). `None` means
        /// "use the thread's current route".
        #[serde(skip_serializing_if = "Option::is_none")]
        model: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        model_provider: Option<String>,
        /// Tool restriction from slash-command frontmatter.
        #[serde(default)]
        allowed_tools: Option<Vec<String>>,
        /// Runtime-supplied dynamic tools for this turn only.
        #[serde(default)]
        dynamic_tools: Vec<Value>,
        /// Structural input provenance — only `ExternalUser` may inherit
        /// YOLO/auto-approval authority (mirrors `UserInputProvenance`).
        #[serde(default = "default_provenance")]
        provenance: String,
    },

    /// Steer an in-flight turn with additional user content (drains into
    /// the turn loop's `rx_steer` channel).
    Steer {
        content: String,
    },

    /// Re-check and dispatch a goal continuation (synthetic turn that
    /// continues the same logical goal run).
    ContinueGoal,

    /// Execute a local composer shell command without a model turn.
    RunShellCommand {
        command: String,
    },

    /// Set goal status without dispatching a model turn.
    SetGoalStatus {
        status: String,
        #[serde(default)]
        clear: bool,
    },

    Cancel,
    Shutdown,

    /// Describe the exact request the next turn would send without sending it
    /// (`/dryrun` / `/preview-request`, #1004). Headless and TUI must render
    /// identical manifests for identical inputs.
    PreviewOutboundRequest {
        #[serde(default)]
        json: bool,
        #[serde(default)]
        base_prompt_only: bool,
    },
}

fn default_mode() -> String {
    "agent".to_string()
}

fn default_provenance() -> String {
    "external_user".to_string()
}

impl Op {
    #[must_use]
    pub fn is_send_message(&self) -> bool {
        matches!(self, Self::SendMessage { .. })
    }

    #[must_use]
    pub fn kind_str(&self) -> &'static str {
        match self {
            Self::SendMessage { .. } => "send_message",
            Self::Steer { .. } => "steer",
            Self::ContinueGoal => "continue_goal",
            Self::RunShellCommand { .. } => "run_shell_command",
            Self::SetGoalStatus { .. } => "set_goal_status",
            Self::Cancel => "cancel",
            Self::Shutdown => "shutdown",
            Self::PreviewOutboundRequest { .. } => "preview_outbound_request",
        }
    }
}

/// Build a headless `SendMessage` envelope with fresh ids. This is the
/// one-line helper every headless caller (CLI `exec`, app-server, tests)
/// uses so TUI and headless start a session identically.
#[must_use]
pub fn headless_send_message_op(thread_id: ThreadId, content: impl Into<String>) -> OpEnvelope {
    OpEnvelope {
        op_id: format!("op-{}", uuid::Uuid::new_v4()),
        thread_id: thread_id.clone(),
        session_id: SessionId::new(),
        op: Op::SendMessage {
            content: content.into(),
            mode: default_mode(),
            model: None,
            model_provider: None,
            allowed_tools: None,
            dynamic_tools: Vec::new(),
            provenance: default_provenance(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn op_envelope_roundtrip() {
        let env = headless_send_message_op(ThreadId::new(), "hello");
        let json = serde_json::to_string(&env).unwrap();
        let back: OpEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(back.thread_id, env.thread_id);
        assert!(back.op.is_send_message());
    }

    #[test]
    fn steer_roundtrip() {
        let op = Op::Steer {
            content: "more".into(),
        };
        let json = serde_json::to_string(&op).unwrap();
        let back: Op = serde_json::from_str(&json).unwrap();
        assert_eq!(back.kind_str(), "steer");
    }
}
