//! `EventMsg`-out API in `crates/protocol` (issue #5261).
//!
//! Mirrors `crates/tui/src/core/events::Event` but as a serializable
//! protocol. The TUI's `rx_event` / `Event` channel, the app-server's SSE
//! stream, and the CLI's `stream-json` output all speak this one type so
//! headless and TUI observe byte-identical event shapes for the same `Op`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ids::{SessionId, ThreadId};

/// One event emitted by the core engine to every consumer (TUI, CLI,
/// app-server, tests). This is the `EventMsg`-out half of the `Op`-in /
/// `EventMsg`-out contract. It is a straight projection of the existing
/// internal `Event` variants (streaming deltas, tool lifecycle, turn
/// lifecycle, approvals) plus the thread/session ids that `ThreadId` /
/// `SessionId` now make explicit.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum EventMsg {
    TurnStarted {
        thread_id: ThreadId,
        session_id: SessionId,
        turn_id: String,
    },
    ResponseDelta {
        thread_id: ThreadId,
        session_id: SessionId,
        delta: String,
        #[serde(default)]
        channel: String,
    },
    ToolCallStarted {
        thread_id: ThreadId,
        session_id: SessionId,
        tool_call_id: String,
        tool_name: String,
        input: Value,
    },
    ToolCallComplete {
        thread_id: ThreadId,
        session_id: SessionId,
        tool_call_id: String,
        tool_name: String,
        result: Value,
    },
    TurnComplete {
        thread_id: ThreadId,
        session_id: SessionId,
        turn_id: String,
        status: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    TurnUsage {
        thread_id: ThreadId,
        session_id: SessionId,
        input_tokens: u32,
        output_tokens: u32,
    },
    CompactionStarted {
        thread_id: ThreadId,
        session_id: SessionId,
        message: String,
    },
    CompactionCompleted {
        thread_id: ThreadId,
        session_id: SessionId,
        message: String,
    },
    Error {
        thread_id: ThreadId,
        session_id: SessionId,
        message: String,
    },
}

/// Envelope that carries an `EventMsg` over the wire / channel with a
/// monotonic seq so consumers can detect drops. Mirrors the existing
/// `RuntimeEventEnvelope` but typed to `EventMsg`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub seq: u64,
    pub thread_id: ThreadId,
    pub session_id: SessionId,
    pub turn_id: Option<String>,
    pub event: EventMsg,
}

impl EventMsg {
    #[must_use]
    pub fn kind_str(&self) -> &'static str {
        match self {
            Self::TurnStarted { .. } => "turn_started",
            Self::ResponseDelta { .. } => "response_delta",
            Self::ToolCallStarted { .. } => "tool_call_started",
            Self::ToolCallComplete { .. } => "tool_call_complete",
            Self::TurnComplete { .. } => "turn_complete",
            Self::TurnUsage { .. } => "turn_usage",
            Self::CompactionStarted { .. } => "compaction_started",
            Self::CompactionCompleted { .. } => "compaction_completed",
            Self::Error { .. } => "error",
        }
    }

    #[must_use]
    pub fn thread_id(&self) -> &ThreadId {
        match self {
            Self::TurnStarted { thread_id, .. }
            | Self::ResponseDelta { thread_id, .. }
            | Self::ToolCallStarted { thread_id, .. }
            | Self::ToolCallComplete { thread_id, .. }
            | Self::TurnComplete { thread_id, .. }
            | Self::TurnUsage { thread_id, .. }
            | Self::CompactionStarted { thread_id, .. }
            | Self::CompactionCompleted { thread_id, .. }
            | Self::Error { thread_id, .. } => thread_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_msg_roundtrip() {
        let msg = EventMsg::TurnComplete {
            thread_id: ThreadId::new(),
            session_id: SessionId::new(),
            turn_id: "turn-1".into(),
            status: "completed".into(),
            error: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: EventMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(back.kind_str(), "turn_complete");
    }
}
