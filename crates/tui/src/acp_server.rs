//! Minimal Agent Client Protocol stdio adapter.
//!
//! This starts from the ACP baseline: initialize, new session, prompt, and
//! cancel. It keeps stdout protocol-clean for editor clients and routes
//! prompts through the same configured DeepSeek client as one-shot CLI mode.
//!
//! `session/prompt` streams the provider response: each text delta is emitted
//! as a `session/update` agent_message_chunk as it arrives, instead of buffering
//! the whole turn and sending one chunk at the end. The stream is consumed
//! concurrently with the input reader so that a `session/cancel` for the same
//! session can interrupt the turn mid-stream (returning `stopReason: "cancelled"`)
//! instead of being queued behind it. A single writer task is preserved so
//! stdout stays protocol-clean.
//!
//! Each ACP session owns a [`crate::tools::ToolRegistry`] built from the same
//! file/search/git/patch/shell tools the CLI `exec` agent and the MCP server
//! adapter (`crate::mcp_server`) already use. When the model emits a tool call,
//! the turn driver executes it locally through that registry (no duplicate
//! filesystem/shell implementation), reports progress to the client as
//! `tool_call` / `tool_call_update` session updates, feeds the result back as
//! a `tool_result` content block, and re-opens the provider stream so the
//! model can keep going until it produces a final answer with no further tool
//! calls.
//!
//! ## Working directory contract
//!
//! No path in an ACP turn may depend on the process working directory. Every
//! path resolves against [`AcpSession::cwd`]: tools go through
//! `ToolContext::resolve_path`, process executors receive an explicit `cwd`,
//! hooks receive the workspace, and the system prompt composer takes
//! `workspace: &Path`. An earlier `ScopedCurrentDir` guard used to
//! `set_current_dir` per prompt round, which is process-global state: two
//! concurrent turns with different workspaces interleaved their guards and the
//! second `Drop` restored the *first* turn's directory, leaving the process
//! pointed at a foreign workspace for good. Serving more than one connection
//! from one process is what made that fatal, so the guard is gone. Do not
//! reintroduce it — pass the directory instead.

use std::collections::{HashMap, VecDeque};

use crate::mcp::{McpPool, McpServerConfig};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use anyhow::{Result, anyhow};
use futures_util::StreamExt;
use serde_json::{Value, json};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader, Lines};
use tokio_util::sync::CancellationToken;

use crate::client::DeepSeekClient;
use crate::config::{ApiProvider, Config};
use crate::core::engine::turn_loop::run_tool_call_before_hooks;
use crate::core::engine::{
    AutoReviewPlanDecision, ToolAskRuleDecision, auto_review_plan_decision_for_context,
    exec_shell_ask_rule_decision_for_policy, file_tool_ask_rule_decision_for_policy,
};
use crate::llm_client::{LlmClient, StreamEventBox};
use crate::models::Role;
use crate::models::{
    ContentBlock, ContentBlockStart, Delta, Message, MessageRequest, StreamEvent, SystemPrompt,
    Usage,
};
use crate::tools::spec::{ApprovalRequirement, PreparedToolCall, RichToolResult, ToolError};
use crate::tools::{ToolContext, ToolRegistry, ToolRegistryBuilder};
use crate::worker_profile::ShellPolicy;

const ACP_PROTOCOL_VERSION: u64 = 1;

/// `session/set_config_option` ids. `model` and `thinking_effort` carry the
/// ACP categories (`model`, `thought_level`) so a client can place them in its
/// own pickers; `provider` has no standard category.
const CONFIG_PROVIDER: &str = "provider";
const CONFIG_MODEL: &str = "model";
const CONFIG_THINKING_EFFORT: &str = "thinking_effort";
/// Effort tiers offered to the client; the route normalizes them at dispatch.
const THINKING_EFFORT_VALUES: [&str; 7] = ["auto", "off", "low", "medium", "high", "xhigh", "max"];

/// Hard cap on LLM <-> tool round-trips within a single `session/prompt`
/// turn. Guards against a model that never stops calling tools; each round is
/// one provider stream plus zero or more tool executions.
const MAX_ACP_TOOL_ROUNDS: usize = 50;

/// Maximum number of concurrent sessions kept in memory. When this limit is
/// exceeded, the oldest session with no in-flight prompt is evicted.
const MAX_ACP_SESSIONS: usize = 64;

/// A conforming ACP client answers every pending permission request with a
/// `cancelled` outcome when the prompt is cancelled. Bound that hand-off so a
/// broken client cannot strand the stdio server forever after cancellation.
const ACP_PERMISSION_CANCEL_GRACE: Duration = Duration::from_secs(2);

/// Agent-originated JSON-RPC request ids have their own namespace. Strings
/// avoid the client-specific numeric response-id compatibility shim used for
/// replies to client-originated requests.
static NEXT_ACP_PERMISSION_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

/// Content is streamed to the model in full (no truncation); this cap only
/// bounds how much of a tool's output is echoed into the `tool_call_update`
/// notification the editor renders, so a large `File`/`Bash`
/// result does not flood the client UI.
const TOOL_CALL_CONTENT_PREVIEW_CHARS: usize = 4_000;

/// Serve ACP over the process stdio pipes, the transport an editor gets when
/// it launches Ghosty as a child process.
pub async fn run_acp_server(config: Config, model: String, default_cwd: PathBuf) -> Result<()> {
    run_acp_server_over(
        config,
        model,
        default_cwd,
        BufReader::new(tokio::io::stdin()),
        tokio::io::BufWriter::new(tokio::io::stdout()),
    )
    .await
}

/// Serve one ACP connection over any byte pair.
///
/// stdio is one caller; a socket is another. The protocol above this line does
/// not know the difference, so a network transport reuses this loop verbatim
/// rather than growing a second copy of the dispatch.
pub async fn run_acp_server_over<R, W>(
    config: Config,
    model: String,
    default_cwd: PathBuf,
    reader: R,
    writer: W,
) -> Result<()>
where
    R: AsyncBufRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    let mut reader = reader.lines();
    let mut writer = writer;
    let mut server = AcpServer::new(config, model, default_cwd);

    while let Some(line) = reader.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }

        let message: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(err) => {
                write_jsonrpc_error(&mut writer, None, -32700, format!("invalid json: {err}"))
                    .await?;
                continue;
            }
        };

        if message.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
            write_jsonrpc_error(
                &mut writer,
                message
                    .get("id")
                    .cloned()
                    .map(|id| server.response_id_policy.response_id(id)),
                -32600,
                "jsonrpc version must be 2.0",
            )
            .await?;
            continue;
        }

        let id = message.get("id").cloned();
        let method = match message.get("method").and_then(Value::as_str) {
            Some(method) => method,
            None if is_jsonrpc_response(&message) => {
                // A late response to an agent-originated request (most notably
                // permission after cancellation) has no request semantics and
                // must not be answered with another JSON-RPC error.
                continue;
            }
            None => {
                write_jsonrpc_error(
                    &mut writer,
                    id.map(|id| server.response_id_policy.response_id(id)),
                    -32600,
                    "missing method",
                )
                .await?;
                continue;
            }
        };
        let params = message.get("params").cloned().unwrap_or_else(|| json!({}));

        // `session/prompt` is driven concurrently with the reader so a
        // `session/cancel` can interrupt the in-flight provider call or a
        // running tool. Every other method is request/response and handled
        // synchronously below.
        if method == "session/prompt" {
            match server.begin_prompt(params) {
                Ok(prepared) => {
                    let PreparedPrompt {
                        session_id,
                        messages,
                        cwd,
                    } = prepared;
                    let response_id_policy = server.response_id_policy;
                    let Some(tool_registry) = server.session_tool_registry(&session_id) else {
                        let id = id.map(|id| response_id_policy.response_id(id));
                        write_jsonrpc_error(&mut writer, id, -32603, "unknown sessionId").await?;
                        continue;
                    };
                    // Freeze the first round's fully composed system prompt
                    // for this entire `session/prompt`. Tool calls may edit
                    // AGENTS.md, memory, or configured instruction files, but
                    // self-authored content cannot become same-turn system
                    // authority on a later provider round.
                    let frozen_system_prompt =
                        Arc::new(std::sync::Mutex::new(None::<SystemPrompt>));
                    // Route facts captured at dispatch, so the turn's usage
                    // is priced and sized against the route that actually
                    // ran, not whatever config says once the turn is over.
                    let route_receipt = Arc::new(std::sync::Mutex::new(None::<AcpRouteReceipt>));
                    // The stream-opening closure borrows `&server` only
                    // briefly per round; each returned `StreamEventBox` is
                    // `'static`, so it can be raced against the reader
                    // without holding a borrow on the server across an
                    // await, and the main task keeps exclusive ownership of
                    // stdout.
                    let outcome = run_agentic_prompt_turn(
                        AcpTurnContext {
                            config: &server.config,
                            model: &server.model,
                            session_id: &session_id,
                            tool_registry: &tool_registry,
                            response_id_policy,
                        },
                        messages,
                        &mut reader,
                        &mut writer,
                        |msgs| {
                            // Rebind to references before the `async move`
                            // block: `async move` moves every path it
                            // touches, and these are already-`Copy`
                            // references, so only `msgs` (the per-round
                            // owned clone) is actually moved in — `server`,
                            // `cwd`, and `tool_registry` stay borrowed from
                            // the enclosing scope across every call this
                            // `FnMut` closure makes.
                            let server = &server;
                            let cwd = &cwd;
                            let tool_registry = &tool_registry;
                            let frozen_system_prompt = Arc::clone(&frozen_system_prompt);
                            let route_receipt = Arc::clone(&route_receipt);
                            async move {
                                server
                                    .open_prompt_stream(
                                        &msgs,
                                        cwd,
                                        tool_registry,
                                        &frozen_system_prompt,
                                        &route_receipt,
                                    )
                                    .await
                            }
                        },
                    )
                    .await;
                    match outcome {
                        Ok((outcome, messages, usage)) => {
                            // Chunks were already streamed; record the full
                            // conversation (including any tool rounds) for
                            // the next prompt. On cancel the turn driver keeps
                            // complete receipts for every proposed tool call,
                            // so partial side effects remain visible and no
                            // dangling tool_use block is stored.
                            server.commit_turn_messages(&session_id, messages);
                            let receipt = route_receipt.lock().ok().and_then(|slot| slot.clone());
                            let context =
                                server.record_turn_usage(&session_id, &usage, receipt.as_ref());
                            if let Some(context) = context {
                                write_usage_update(&mut writer, &session_id, &context).await?;
                            }
                            if let Some(id) = id {
                                let id = response_id_policy.response_id(id);
                                let stop_reason = match outcome {
                                    PromptOutcome::Completed(_) => "end_turn",
                                    PromptOutcome::Cancelled => "cancelled",
                                    PromptOutcome::MaxRounds(_) => "max_turn_requests",
                                };
                                let mut result = json!({ "stopReason": stop_reason });
                                if !usage.is_empty() {
                                    result["usage"] = acp_prompt_usage(&usage.total);
                                }
                                write_jsonrpc_result(&mut writer, id, result).await?;
                            }
                        }
                        Err(err) => {
                            if let Some(partial_messages) = err.partial_messages {
                                // A later provider round failed after one or
                                // more tools completed. Preserve those
                                // side-effect receipts in session history.
                                server.commit_turn_messages(&session_id, partial_messages);
                            } else {
                                // The user message was already pushed into
                                // session history by `begin_prompt`; roll it
                                // back when no tool receipt exists yet.
                                server.rollback_user_message(&session_id);
                            }
                            let id = id.map(|id| response_id_policy.response_id(id));
                            write_jsonrpc_error(&mut writer, id, -32603, err.source.to_string())
                                .await?;
                        }
                    }
                }
                Err(err) => {
                    let id = id.map(|id| server.response_id_policy.response_id(id));
                    write_jsonrpc_error(&mut writer, id, err.code, err.message).await?;
                }
            }
            continue;
        }

        match server.handle_request(method, params).await {
            Ok(AcpDispatch::Response(result)) => {
                if let Some(id) = id {
                    let id = server.response_id_policy.response_id(id);
                    write_jsonrpc_result(&mut writer, id, result).await?;
                }
            }
            Ok(AcpDispatch::ResponseThenNotify {
                result,
                session_id,
                updates,
            }) => {
                if let Some(id) = id {
                    let id = server.response_id_policy.response_id(id);
                    write_jsonrpc_result(&mut writer, id, result).await?;
                }
                for update in updates {
                    write_session_notification(&mut writer, &session_id, update).await?;
                }
            }
            Ok(AcpDispatch::Shutdown) => {
                if let Some(id) = id {
                    let id = server.response_id_policy.response_id(id);
                    write_jsonrpc_result(&mut writer, id, json!(null)).await?;
                }
                break;
            }
            Err(err) => {
                let id = id.map(|id| server.response_id_policy.response_id(id));
                write_jsonrpc_error(&mut writer, id, err.code, err.message).await?;
            }
        }
    }

    Ok(())
}

fn is_jsonrpc_response(message: &Value) -> bool {
    message.get("id").is_some()
        && (message.get("result").is_some() || message.get("error").is_some())
}

/// Outcome of a `session/prompt` turn driven against the input stream.
#[derive(Debug, PartialEq, Eq)]
enum PromptOutcome {
    /// The provider call finished first; carries the assistant text.
    Completed(String),
    /// A matching `session/cancel` arrived before the call finished.
    Cancelled,
    /// The turn reached the maximum number of tool-call round-trips.
    /// Carries whatever text the model produced in the final round.
    MaxRounds(String),
}

#[derive(Debug)]
struct AgenticPromptError {
    source: anyhow::Error,
    /// Present once at least one complete tool-result batch has been appended.
    /// Those receipts may describe real side effects and must survive a later
    /// provider failure.
    partial_messages: Option<Vec<Message>>,
}

impl std::fmt::Display for AgenticPromptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.source.fmt(f)
    }
}

impl AgenticPromptError {
    fn new(source: anyhow::Error, messages: &[Message], has_tool_receipts: bool) -> Self {
        Self {
            source,
            partial_messages: has_tool_receipts.then(|| messages.to_vec()),
        }
    }
}

/// A tool call the model requested, assembled from streamed
/// `content_block_start` / `content_block_delta` / `content_block_stop`
/// events. `parse_error` is set when the accumulated `input_json_delta`
/// bytes did not parse as JSON — the call is still surfaced (rather than
/// silently dropped) so the model gets a clear tool-result error instead of
/// the turn hanging.
#[derive(Debug, Clone)]
struct PendingToolCall {
    id: String,
    name: String,
    input: Value,
    parse_error: Option<String>,
}

/// Accumulates one streamed `tool_use` content block until its
/// `content_block_stop`.
#[derive(Debug, Default)]
struct ToolUseAccumulator {
    id: String,
    name: String,
    initial_input: Value,
    buffer: String,
}

impl ToolUseAccumulator {
    fn finalize(self) -> PendingToolCall {
        if self.buffer.trim().is_empty() {
            return PendingToolCall {
                id: self.id,
                name: self.name,
                input: self.initial_input,
                parse_error: None,
            };
        }
        match serde_json::from_str(&self.buffer) {
            Ok(input) => PendingToolCall {
                id: self.id,
                name: self.name,
                input,
                parse_error: None,
            },
            Err(_) => PendingToolCall {
                id: self.id,
                name: self.name,
                input: json!({}),
                parse_error: Some(self.buffer),
            },
        }
    }
}

/// The text payload an ACP client should see for a given stream event, if any.
/// Tool and control events carry no chunk; thinking goes through
/// [`stream_thought_chunk`] as `agent_thought_chunk`.
fn stream_text_chunk(event: &StreamEvent) -> Option<&str> {
    match event {
        StreamEvent::ContentBlockDelta {
            delta: Delta::TextDelta { text },
            ..
        } => Some(text),
        StreamEvent::ContentBlockStart {
            content_block: ContentBlockStart::Text { text },
            ..
        } => Some(text),
        _ => None,
    }
}

/// Reasoning text for a stream event, surfaced as `agent_thought_chunk` so a
/// client can show the model thinking instead of silence until the answer.
fn stream_thought_chunk(event: &StreamEvent) -> Option<&str> {
    match event {
        StreamEvent::ContentBlockDelta {
            delta: Delta::ThinkingDelta { thinking },
            ..
        } => Some(thinking),
        StreamEvent::ContentBlockStart {
            content_block: ContentBlockStart::Thinking { thinking },
            ..
        } => Some(thinking),
        _ => None,
    }
}

/// Fold a later usage report for the same provider response into `into`.
/// Providers report cumulative counts (`message_start` carries the prompt
/// side, `message_delta` the final totals), so field-wise max is the merge.
fn merge_round_usage(into: &mut Usage, later: &Usage) {
    fn max_opt(a: Option<u32>, b: Option<u32>) -> Option<u32> {
        match (a, b) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        }
    }
    into.input_tokens = into.input_tokens.max(later.input_tokens);
    into.output_tokens = into.output_tokens.max(later.output_tokens);
    into.prompt_cache_hit_tokens =
        max_opt(into.prompt_cache_hit_tokens, later.prompt_cache_hit_tokens);
    into.prompt_cache_miss_tokens = max_opt(
        into.prompt_cache_miss_tokens,
        later.prompt_cache_miss_tokens,
    );
    into.prompt_cache_write_tokens = max_opt(
        into.prompt_cache_write_tokens,
        later.prompt_cache_write_tokens,
    );
    into.reasoning_tokens = max_opt(into.reasoning_tokens, later.reasoning_tokens);
}

/// Add one provider round's usage to a running total (turn or session).
fn add_usage(total: &mut Usage, round: &Usage) {
    fn add_opt(a: Option<u32>, b: Option<u32>) -> Option<u32> {
        match (a, b) {
            (None, None) => None,
            (a, b) => Some(a.unwrap_or(0).saturating_add(b.unwrap_or(0))),
        }
    }
    total.input_tokens = total.input_tokens.saturating_add(round.input_tokens);
    total.output_tokens = total.output_tokens.saturating_add(round.output_tokens);
    total.prompt_cache_hit_tokens =
        add_opt(total.prompt_cache_hit_tokens, round.prompt_cache_hit_tokens);
    total.prompt_cache_miss_tokens = add_opt(
        total.prompt_cache_miss_tokens,
        round.prompt_cache_miss_tokens,
    );
    total.prompt_cache_write_tokens = add_opt(
        total.prompt_cache_write_tokens,
        round.prompt_cache_write_tokens,
    );
    total.reasoning_tokens = add_opt(total.reasoning_tokens, round.reasoning_tokens);
}

/// Provider usage accumulated over one `session/prompt` turn.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct AcpTurnUsage {
    /// Sum across every provider round of the turn.
    total: Usage,
    /// The final round's own report: its `input_tokens` is what the context
    /// currently holds, which is what `usage_update.used` describes.
    last_round: Option<Usage>,
}

impl AcpTurnUsage {
    fn record_round(&mut self, round: Option<Usage>) {
        if let Some(round) = round {
            add_usage(&mut self.total, &round);
            self.last_round = Some(round);
        }
    }

    fn is_empty(&self) -> bool {
        self.last_round.is_none()
    }
}

/// Consume a provider response `stream`, emitting each text delta as a
/// `session/update` chunk, while concurrently watching `reader` for a
/// `session/cancel` targeting `session_id`.
///
/// This is the streaming + cancellation control point. It is generic over the
/// reader/writer and takes the boxed stream, so it is unit-tested with canned
/// in-memory streams and readers — no real provider call required. The caller
/// keeps the only writer, so streamed chunks and acknowledgements all stay on
/// the single protocol-clean stdout stream.
///
/// Returns [`PromptOutcome::Completed`] with the full accumulated text once the
/// stream ends (or emits `message_stop`), plus any `tool_use` blocks the model
/// emitted during the round, so the caller can execute them and continue the
/// turn. A matching `session/cancel` (request or notification form) ends it
/// early with [`PromptOutcome::Cancelled`] — dropping the stream aborts the
/// underlying provider connection. The turn is single-flight: a cancel for a
/// different session is acknowledged and ignored; any other concurrent *request*
/// is rejected with a clear error so the client is not left waiting;
/// notifications without an id are ignored.
async fn drive_prompt_stream<R, W>(
    mut stream: StreamEventBox,
    session_id: &str,
    response_id_policy: JsonRpcResponseIdPolicy,
    reader: &mut Lines<R>,
    writer: &mut W,
) -> Result<(PromptOutcome, Vec<PendingToolCall>, Option<Usage>)>
where
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut accumulated = String::new();
    let mut tool_calls: Vec<PendingToolCall> = Vec::new();
    let mut pending_tool_uses: HashMap<u32, ToolUseAccumulator> = HashMap::new();
    let mut round_usage: Option<Usage> = None;
    // Once input closes mid-turn we stop selecting on the reader and just drain
    // the stream to completion, rather than spinning on repeated EOFs.
    let mut reader_open = true;
    loop {
        tokio::select! {
            event = stream.next() => {
                match event {
                    // Stream exhausted without an explicit stop: turn is done.
                    None => return Ok((PromptOutcome::Completed(accumulated), tool_calls, round_usage)),
                    Some(Ok(event)) => {
                        if let Some(text) = stream_text_chunk(&event)
                            && !text.is_empty() {
                                accumulated.push_str(text);
                                write_session_update(writer, session_id, text.to_string()).await?;
                            }
                        if let Some(thought) = stream_thought_chunk(&event)
                            && !thought.is_empty() {
                                write_session_chunk(
                                    writer,
                                    session_id,
                                    "agent_thought_chunk",
                                    thought.to_string(),
                                )
                                .await?;
                            }
                        match event {
                            StreamEvent::MessageStart { message } => {
                                match round_usage.as_mut() {
                                    Some(usage) => merge_round_usage(usage, &message.usage),
                                    None => round_usage = Some(message.usage),
                                }
                            }
                            StreamEvent::MessageDelta { usage: Some(usage), .. } => {
                                match round_usage.as_mut() {
                                    Some(current) => merge_round_usage(current, &usage),
                                    None => round_usage = Some(usage),
                                }
                            }
                            StreamEvent::ContentBlockStart {
                                index,
                                content_block: ContentBlockStart::ToolUse { id, name, input, ..},
                            } => {
                                pending_tool_uses.insert(
                                    index,
                                    ToolUseAccumulator {
                                        id,
                                        name,
                                        initial_input: input,
                                        buffer: String::new(),
                                    },
                                );
                            }
                            StreamEvent::ContentBlockDelta {
                                index,
                                delta: Delta::InputJsonDelta { partial_json },
                            } => {
                                if let Some(acc) = pending_tool_uses.get_mut(&index) {
                                    acc.buffer.push_str(&partial_json);
                                }
                            }
                            StreamEvent::ContentBlockStop { index } => {
                                if let Some(acc) = pending_tool_uses.remove(&index) {
                                    tool_calls.push(acc.finalize());
                                }
                            }
                            StreamEvent::MessageStop => {
                                return Ok((PromptOutcome::Completed(accumulated), tool_calls, round_usage));
                            }
                            StreamEvent::Error { error } => {
                                return Err(anyhow!("provider stream error: {error}"));
                            }
                            _ => {}
                        }
                    }
                    Some(Err(err)) => return Err(err),
                }
            }
            line = reader.next_line(), if reader_open => {
                let line = match line? {
                    Some(line) => line,
                    // Input closed mid-turn: stop watching it, keep draining.
                    None => {
                        reader_open = false;
                        continue;
                    }
                };
                if line.trim().is_empty() {
                    continue;
                }
                let message: Value = match serde_json::from_str(&line) {
                    Ok(value) => value,
                    Err(err) => {
                        write_jsonrpc_error(writer, None, -32700, format!("invalid json: {err}"))
                            .await?;
                        continue;
                    }
                };
                let id = message.get("id").cloned();
                match message.get("method").and_then(Value::as_str) {
                    Some("session/cancel") => {
                        let target = message.pointer("/params/sessionId").and_then(Value::as_str);
                        // A cancel with no sessionId is treated as targeting the
                        // single in-flight turn.
                        if target.is_none() || target == Some(session_id) {
                            if let Some(id) = id {
                                let id = response_id_policy.response_id(id);
                                write_jsonrpc_result(writer, id, json!(null)).await?;
                            }
                            // Dropping `stream` on return aborts the provider call.
                            return Ok((PromptOutcome::Cancelled, tool_calls, round_usage));
                        }
                        // Cancel for some other session: acknowledge, keep going.
                        if let Some(id) = id {
                            let id = response_id_policy.response_id(id);
                            write_jsonrpc_result(writer, id, json!(null)).await?;
                        }
                    }
                    _ => {
                        // The turn is single-flight; do not silently drop a
                        // request the client expects a response to.
                        if let Some(id) = id {
                            let id = response_id_policy.response_id(id);
                            write_jsonrpc_error(
                                writer,
                                Some(id),
                                -32603,
                                "a session/prompt turn is already in progress",
                            )
                            .await?;
                        }
                    }
                }
            }
        }
    }
}

/// Outcome of executing one batch of tool calls from a single round.
enum ToolBatchOutcome {
    /// Every tool call ran to completion; carries the `tool_result` messages
    /// to append to the conversation, in call order.
    Completed(Vec<Message>),
    /// A matching `session/cancel` arrived while a tool was awaiting approval
    /// or running. Carries receipts for every proposed call so completed or
    /// partially completed side effects never disappear from history.
    Cancelled(Vec<Message>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AcpToolAdmission {
    Auto,
    RequestPermission(String),
    Block(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AcpPermissionDecision {
    Allow,
    Reject(String),
    Cancelled,
}

fn acp_shell_command_requests_detach(command: &str) -> bool {
    let mut single_quoted = false;
    let mut double_quoted = false;
    let mut escaped = false;
    let chars = command.chars().collect::<Vec<_>>();
    for (index, ch) in chars.iter().copied().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' && !single_quoted {
            escaped = true;
            continue;
        }
        if ch == '\'' && !double_quoted {
            single_quoted = !single_quoted;
            continue;
        }
        if ch == '"' && !single_quoted {
            double_quoted = !double_quoted;
            continue;
        }
        if ch != '&' || single_quoted || double_quoted {
            continue;
        }
        let previous = index.checked_sub(1).and_then(|i| chars.get(i)).copied();
        let next = chars.get(index + 1).copied();
        // `&&`, `&>`/`&>>`, and `>&` are chaining/redirection rather than a
        // detached child. Any other unquoted ampersand is a background
        // control operator and is unavailable in ACP.
        if previous != Some('&') && next != Some('&') && next != Some('>') && previous != Some('>')
        {
            return true;
        }
    }

    shell_words::split(command).is_ok_and(|words| {
        words.iter().any(|word| {
            matches!(
                word.to_ascii_lowercase().as_str(),
                "nohup" | "disown" | "setsid" | "daemonize"
            )
        })
    })
}

#[derive(Debug)]
struct PreparedAcpTool {
    call: PreparedToolCall,
    admission: AcpToolAdmission,
    additional_context: Option<String>,
}

/// Prepare one registered call and fold every policy layer that can tighten
/// its admission. ACP deliberately does not use the TUI's workspace-write
/// carve-out: a remembered exact allow rule may clear the ordinary tool hold,
/// but the built-in safety floor and repository law can always re-add a prompt
/// or hard block afterwards.
fn prepare_acp_tool_admission(
    config: &Config,
    registry: &ToolRegistry,
    call: &PendingToolCall,
) -> std::result::Result<(PreparedToolCall, AcpToolAdmission), ToolError> {
    let spec = registry.get(&call.name).ok_or_else(|| {
        ToolError::not_available(format!("tool '{}' is not registered", call.name))
    })?;
    let prepared = spec.prepare(call.input.clone(), registry.context())?;
    let canonical_name =
        crate::tools::canonical_action::canonical_action_alias(&call.name, &prepared.input);
    if matches!(call.name.as_str(), "bash" | "Bash" | "exec_shell")
        || canonical_name.starts_with("exec_shell")
    {
        let action = prepared
            .input
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or("run");
        let requests_stateful_shell = action != "run"
            || prepared.starts_detached
            || prepared.input.get("interactive").and_then(Value::as_bool) == Some(true)
            || prepared.input.get("persist").and_then(Value::as_bool) == Some(true)
            || prepared
                .input
                .get("command")
                .and_then(Value::as_str)
                .is_some_and(acp_shell_command_requests_detach);
        if requests_stateful_shell {
            return Ok((
                prepared,
                AcpToolAdmission::Block(
                    "ACP v0.9.6 exposes foreground Bash runs only; background, TTY, interactive, persistent, and background-task control actions are unavailable."
                        .to_string(),
                ),
            ));
        }
    }
    let mut permission_reason =
        (prepared.approval != ApprovalRequirement::Auto).then(|| prepared.description.clone());
    let approval_mode = crate::tui::approval::ApprovalMode::Suggest;
    let workspace = registry.context().workspace.as_path();

    let typed_rule = exec_shell_ask_rule_decision_for_policy(
        &config.exec_policy_engine,
        &call.name,
        &prepared.input,
        workspace,
        approval_mode,
    )
    .or_else(|| {
        file_tool_ask_rule_decision_for_policy(
            &config.exec_policy_engine,
            &call.name,
            &prepared.input,
            workspace,
            approval_mode,
        )
    });
    match typed_rule {
        Some(ToolAskRuleDecision::Allow) => permission_reason = None,
        Some(ToolAskRuleDecision::Prompt(reason)) => permission_reason = Some(reason),
        Some(ToolAskRuleDecision::Block(reason)) => {
            return Ok((prepared, AcpToolAdmission::Block(reason)));
        }
        None => {}
    }

    let run_origin = if prepared.starts_detached {
        crate::tui::auto_review::RunOrigin::Background
    } else {
        crate::tui::auto_review::RunOrigin::Headless
    };
    let review_context = crate::tui::auto_review::AutoReviewContext::from_tool_call(
        &call.name,
        &prepared.input,
        run_origin,
        approval_mode,
        crate::config::is_workspace_trusted(workspace),
        Some(workspace),
    );
    let (auto_review, _audit) =
        auto_review_plan_decision_for_context(&config.auto_review_policy(), &review_context);
    match auto_review {
        AutoReviewPlanDecision::NoChange | AutoReviewPlanDecision::Allow => {}
        AutoReviewPlanDecision::ForcePrompt(reason) => permission_reason = Some(reason),
        // Headless adapters keep the deterministic-only tier: a fallback hold
        // that interactive Auto posture would send to the model guardian is
        // a hard block here (the reviewer is an interactive-session feature).
        AutoReviewPlanDecision::ConsultReviewer(reason) | AutoReviewPlanDecision::Block(reason) => {
            return Ok((prepared, AcpToolAdmission::Block(reason)));
        }
    }

    if let Some(repo_law) =
        crate::repo_law::repo_law_plan_decision(workspace, &call.name, &prepared.input)
    {
        match repo_law {
            crate::repo_law::RepoLawPlanDecision::ForcePrompt(reason) => {
                permission_reason = Some(reason);
            }
            crate::repo_law::RepoLawPlanDecision::Block(reason) => {
                return Ok((prepared, AcpToolAdmission::Block(reason)));
            }
        }
    }

    let admission = permission_reason
        .map(AcpToolAdmission::RequestPermission)
        .unwrap_or(AcpToolAdmission::Auto);
    Ok((prepared, admission))
}

/// Run the same strict pre-tool hook gate as the native turn loop, then
/// prepare and evaluate policy from the hook's final input. The initial
/// preparation is deliberately side-effect free and catches malformed input
/// before an operator hook is asked to reason about it; any rewrite is fully
/// re-prepared and all policy layers run again from that rewritten value.
async fn prepare_acp_tool_with_hooks(
    config: &Config,
    model: &str,
    registry: &ToolRegistry,
    call: &PendingToolCall,
) -> std::result::Result<PreparedAcpTool, ToolError> {
    let spec = registry.get(&call.name).ok_or_else(|| {
        ToolError::not_available(format!("tool '{}' is not registered", call.name))
    })?;
    // Initial validation mirrors the native prepare-before-hooks contract.
    spec.prepare(call.input.clone(), registry.context())?;

    let hook_outcome = run_tool_call_before_hooks(
        registry.context().runtime.hook_executor.as_ref(),
        &call.name,
        &call.id,
        &call.input,
        crate::tui::app::AppMode::Agent,
        registry.context().workspace.as_path(),
        model,
    )
    .await?;

    let mut final_call = call.clone();
    if let Some(updated_input) = hook_outcome.updated_input {
        final_call.input = updated_input;
    }
    let (prepared, mut admission) = prepare_acp_tool_admission(config, registry, &final_call)?;
    if hook_outcome.requires_approval && matches!(admission, AcpToolAdmission::Auto) {
        admission = AcpToolAdmission::RequestPermission(
            "A ToolCallBefore hook requires explicit approval for this call.".to_string(),
        );
    }

    Ok(PreparedAcpTool {
        call: prepared,
        admission,
        additional_context: hook_outcome.additional_context,
    })
}

fn next_acp_permission_request_id() -> Value {
    Value::String(format!(
        "ghosty-permission-{}",
        NEXT_ACP_PERMISSION_REQUEST_ID.fetch_add(1, Ordering::Relaxed)
    ))
}

async fn write_tool_permission_request<W>(
    writer: &mut W,
    request_id: &Value,
    session_id: &str,
    call: &PendingToolCall,
    reason: &str,
) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    write_json_line(
        writer,
        json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": "session/request_permission",
            "params": {
                "sessionId": session_id,
                "toolCall": {
                    "toolCallId": call.id,
                    "title": tool_call_title(call),
                    "kind": tool_call_kind(call),
                    "status": "pending",
                    "rawInput": call.input,
                    "content": [{
                        "type": "content",
                        "content": { "type": "text", "text": reason }
                    }]
                },
                "options": [
                    {
                        "optionId": "allow-once",
                        "name": "Allow once",
                        "kind": "allow_once"
                    },
                    {
                        "optionId": "reject-once",
                        "name": "Reject",
                        "kind": "reject_once"
                    }
                ]
            }
        }),
    )
    .await
}

/// Ask the ACP client to approve one sensitive call. The client owns the UI
/// and ACP v1 requires it to answer a pending request with `cancelled` when the
/// prompt turn is cancelled. Unknown, malformed, or errored responses all fail
/// closed as rejection; only the exact offered `allow-once` id authorizes work.
async fn request_tool_permission<R, W>(
    reader: &mut Lines<R>,
    writer: &mut W,
    response_id_policy: JsonRpcResponseIdPolicy,
    session_id: &str,
    call: &PendingToolCall,
    reason: &str,
) -> Result<AcpPermissionDecision>
where
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let request_id = next_acp_permission_request_id();
    write_tool_permission_request(writer, &request_id, session_id, call, reason).await?;
    let mut cancel_deadline = None;

    loop {
        let line = if let Some(deadline) = cancel_deadline {
            match tokio::time::timeout_at(deadline, reader.next_line()).await {
                Ok(line) => line?,
                Err(_) => return Ok(AcpPermissionDecision::Cancelled),
            }
        } else {
            reader.next_line().await?
        };
        let Some(line) = line else {
            return Ok(AcpPermissionDecision::Reject(
                "Permission denied: ACP client disconnected before answering.".to_string(),
            ));
        };
        if line.trim().is_empty() {
            continue;
        }
        let message: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(err) => {
                write_jsonrpc_error(writer, None, -32700, format!("invalid json: {err}")).await?;
                continue;
            }
        };

        if let Some(method) = message.get("method").and_then(Value::as_str) {
            let message_id = message.get("id").cloned();
            if method == "session/cancel" {
                let target = message.pointer("/params/sessionId").and_then(Value::as_str);
                if target.is_none() || target == Some(session_id) {
                    if let Some(message_id) = message_id {
                        let message_id = response_id_policy.response_id(message_id);
                        write_jsonrpc_result(writer, message_id, json!(null)).await?;
                    }
                    cancel_deadline.get_or_insert_with(|| {
                        tokio::time::Instant::now() + ACP_PERMISSION_CANCEL_GRACE
                    });
                    continue;
                }
                if let Some(message_id) = message_id {
                    let message_id = response_id_policy.response_id(message_id);
                    write_jsonrpc_result(writer, message_id, json!(null)).await?;
                }
                continue;
            }

            if let Some(message_id) = message_id {
                let message_id = response_id_policy.response_id(message_id);
                write_jsonrpc_error(
                    writer,
                    Some(message_id),
                    -32603,
                    "a session/prompt turn is already in progress",
                )
                .await?;
            }
            continue;
        }

        if message.get("id") != Some(&request_id) {
            // This is a response, not a request; there is nothing valid to send
            // back. Ignore stale/unrelated agent-response traffic without
            // allowing it to satisfy the permission gate.
            continue;
        }
        if cancel_deadline.is_some() {
            return Ok(AcpPermissionDecision::Cancelled);
        }
        if message.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
            return Ok(AcpPermissionDecision::Reject(
                "Permission denied: malformed ACP response.".to_string(),
            ));
        }
        if message.get("error").is_some() {
            return Ok(AcpPermissionDecision::Reject(
                "Permission denied: ACP client returned an error.".to_string(),
            ));
        }
        match message
            .pointer("/result/outcome/outcome")
            .and_then(Value::as_str)
        {
            Some("cancelled") => return Ok(AcpPermissionDecision::Cancelled),
            Some("selected") => {
                let option_id = message
                    .pointer("/result/outcome/optionId")
                    .and_then(Value::as_str);
                return Ok(match option_id {
                    Some("allow-once") => AcpPermissionDecision::Allow,
                    Some("reject-once") => AcpPermissionDecision::Reject(
                        "Permission denied by the user; the tool was not executed.".to_string(),
                    ),
                    _ => AcpPermissionDecision::Reject(
                        "Permission denied: ACP client selected an unknown option.".to_string(),
                    ),
                });
            }
            _ => {
                return Ok(AcpPermissionDecision::Reject(
                    "Permission denied: malformed ACP response.".to_string(),
                ));
            }
        }
    }
}

async fn record_tool_execution_result<W>(
    writer: &mut W,
    session_id: &str,
    call: &PendingToolCall,
    result: std::result::Result<RichToolResult, ToolError>,
) -> Result<Message>
where
    W: AsyncWrite + Unpin,
{
    let (content, is_error, rich_blocks) = match result {
        Ok(tool_result) => (
            tool_result.result.content,
            !tool_result.result.success,
            tool_result.content_blocks,
        ),
        Err(err) => (format!("Error: {err}"), true, Vec::new()),
    };
    let status = if is_error { "failed" } else { "completed" };
    write_tool_call_update_with_blocks(
        writer,
        session_id,
        call,
        status,
        Some(&content),
        &rich_blocks,
    )
    .await?;
    Ok(tool_result_message_with_blocks(
        &call.id,
        content,
        is_error,
        rich_blocks
            .iter()
            .filter_map(|block| serde_json::to_value(block).ok())
            .collect(),
    ))
}

async fn record_unstarted_cancelled_calls<W, I>(
    writer: &mut W,
    session_id: &str,
    calls: I,
) -> Result<Vec<Message>>
where
    W: AsyncWrite + Unpin,
    I: IntoIterator<Item = PendingToolCall>,
{
    let mut messages = Vec::new();
    for call in calls {
        write_tool_call_start(writer, session_id, &call).await?;
        let content = "Cancelled before execution; the tool was not run.";
        write_tool_call_update(writer, session_id, &call, "failed", Some(content)).await?;
        messages.push(tool_result_message(&call.id, content.to_string(), true));
    }
    Ok(messages)
}

#[derive(Clone, Copy)]
struct AcpTurnContext<'a> {
    config: &'a Config,
    model: &'a str,
    session_id: &'a str,
    tool_registry: &'a ToolRegistry,
    response_id_policy: JsonRpcResponseIdPolicy,
}

/// Execute `tool_calls` in order against `registry`, reporting each one to
/// the client as `tool_call` / `tool_call_update` session updates, while
/// racing every execution against the reader for a `session/cancel`
/// targeting `session_id`. On cancel, the tool's [`CancellationToken`] is
/// signalled and the in-flight call is awaited to completion (so a
/// cancel-aware tool like `Bash` gets a chance to kill its child
/// process) before returning [`ToolBatchOutcome::Cancelled`].
async fn execute_tool_calls_with_cancellation<R, W>(
    context: AcpTurnContext<'_>,
    tool_calls: Vec<PendingToolCall>,
    reader: &mut Lines<R>,
    writer: &mut W,
) -> Result<ToolBatchOutcome>
where
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let AcpTurnContext {
        config,
        model,
        session_id,
        tool_registry: registry,
        response_id_policy,
    } = context;
    let mut result_messages = Vec::with_capacity(tool_calls.len());
    let mut reader_open = true;
    let mut calls = tool_calls.into_iter();

    while let Some(mut call) = calls.next() {
        write_tool_call_start(writer, session_id, &call).await?;

        if let Some(parse_error) = call.parse_error.clone() {
            let content = format!("Error: tool arguments were not valid JSON: {parse_error}");
            write_tool_call_update(writer, session_id, &call, "failed", Some(&content)).await?;
            result_messages.push(tool_result_message(&call.id, content, true));
            continue;
        }

        let prepared = match prepare_acp_tool_with_hooks(config, model, registry, &call).await {
            Ok(prepared) => prepared,
            Err(err) => {
                let content = format!("Error: {err}");
                write_tool_call_update(writer, session_id, &call, "failed", Some(&content)).await?;
                result_messages.push(tool_result_message(&call.id, content, true));
                continue;
            }
        };
        call.input = prepared.call.input;

        match prepared.admission {
            AcpToolAdmission::Auto => {}
            AcpToolAdmission::Block(reason) => {
                let content = format!("Blocked by Ghosty policy: {reason}");
                write_tool_call_update(writer, session_id, &call, "failed", Some(&content)).await?;
                result_messages.push(tool_result_message(&call.id, content, true));
                continue;
            }
            AcpToolAdmission::RequestPermission(reason) => {
                match request_tool_permission(
                    reader,
                    writer,
                    response_id_policy,
                    session_id,
                    &call,
                    &reason,
                )
                .await?
                {
                    AcpPermissionDecision::Allow => {}
                    AcpPermissionDecision::Reject(content) => {
                        write_tool_call_update(writer, session_id, &call, "failed", Some(&content))
                            .await?;
                        result_messages.push(tool_result_message(&call.id, content, true));
                        continue;
                    }
                    AcpPermissionDecision::Cancelled => {
                        let content = "Cancelled while awaiting permission; the tool was not run.";
                        write_tool_call_update(writer, session_id, &call, "failed", Some(content))
                            .await?;
                        result_messages.push(tool_result_message(
                            &call.id,
                            content.to_string(),
                            true,
                        ));
                        result_messages.extend(
                            record_unstarted_cancelled_calls(writer, session_id, calls).await?,
                        );
                        return Ok(ToolBatchOutcome::Cancelled(result_messages));
                    }
                }
            }
        }

        write_tool_call_update(writer, session_id, &call, "in_progress", None).await?;

        let cancel_token = CancellationToken::new();
        let mut turn_context = registry.context().clone();
        turn_context.cancel_token = Some(cancel_token.clone());
        let exec_fut = registry.execute_rich_full_with_context(
            &call.name,
            call.input.clone(),
            Some(&turn_context),
        );
        tokio::pin!(exec_fut);

        let mut cancelled = false;
        let exec_result = loop {
            tokio::select! {
                result = &mut exec_fut => break result,
                line = reader.next_line(), if reader_open => {
                    let line = match line? {
                        Some(line) => line,
                        None => {
                            reader_open = false;
                            continue;
                        }
                    };
                    if line.trim().is_empty() {
                        continue;
                    }
                    let message: Value = match serde_json::from_str(&line) {
                        Ok(value) => value,
                        Err(err) => {
                            write_jsonrpc_error(writer, None, -32700, format!("invalid json: {err}"))
                                .await?;
                            continue;
                        }
                    };
                    let msg_id = message.get("id").cloned();
                    match message.get("method").and_then(Value::as_str) {
                        Some("session/cancel") => {
                            let target = message.pointer("/params/sessionId").and_then(Value::as_str);
                            if target.is_none() || target == Some(session_id) {
                                if let Some(msg_id) = msg_id {
                                    let msg_id = response_id_policy.response_id(msg_id);
                                    write_jsonrpc_result(writer, msg_id, json!(null)).await?;
                                }
                                cancel_token.cancel();
                                // Give the tool a chance to observe the token and
                                // wind down (e.g. kill a running child process)
                                // before we drop it.
                                cancelled = true;
                                break (&mut exec_fut).await;
                            }
                            if let Some(msg_id) = msg_id {
                                let msg_id = response_id_policy.response_id(msg_id);
                                write_jsonrpc_result(writer, msg_id, json!(null)).await?;
                            }
                        }
                        _ => {
                            if let Some(msg_id) = msg_id {
                                let msg_id = response_id_policy.response_id(msg_id);
                                write_jsonrpc_error(
                                    writer,
                                    Some(msg_id),
                                    -32603,
                                    "a session/prompt turn is already in progress",
                                )
                                .await?;
                            }
                        }
                    }
                }
            }
        };

        let exec_result = exec_result.map(|mut result| {
            if let Some(context) = prepared.additional_context.as_deref() {
                result.result.content =
                    format!("{}\n\n[hook context] {context}", result.result.content);
            }
            result
        });
        result_messages
            .push(record_tool_execution_result(writer, session_id, &call, exec_result).await?);
        if cancelled {
            result_messages
                .extend(record_unstarted_cancelled_calls(writer, session_id, calls).await?);
            return Ok(ToolBatchOutcome::Cancelled(result_messages));
        }
    }

    Ok(ToolBatchOutcome::Completed(result_messages))
}

fn tool_result_message(tool_use_id: &str, content: String, is_error: bool) -> Message {
    tool_result_message_with_blocks(tool_use_id, content, is_error, Vec::new())
}

fn tool_result_message_with_blocks(
    tool_use_id: &str,
    content: String,
    is_error: bool,
    content_blocks: Vec<Value>,
) -> Message {
    Message {
        role: Role::User,
        content: vec![ContentBlock::ToolResult {
            tool_use_id: tool_use_id.to_string(),
            content,
            is_error: Some(is_error),
            content_blocks: (!content_blocks.is_empty()).then_some(content_blocks),
        }],
    }
}

/// Drive one `session/prompt` turn to completion, looping through as many
/// LLM <-> tool round-trips as the model requests (bounded by
/// [`MAX_ACP_TOOL_ROUNDS`]).
///
/// `open_stream` opens a fresh provider stream for the given message
/// history; production callers wire it to [`AcpServer::open_prompt_stream`],
/// while tests supply canned per-round streams so the loop can be exercised
/// without a real provider. Returns the outcome of the final round plus the
/// full message history (including any tool-call/tool-result rounds), which
/// the caller commits to session history only when the turn completed
/// normally.
///
/// `open_stream` takes the message history *by value* (a clone per round)
/// rather than `&[Message]`: an `async fn`'s returned future captures the
/// lifetime of every reference parameter, so a borrowed slice here would
/// force `Fut` to depend on each call's borrow lifetime — which `FnMut`'s
/// single associated `Fut` type cannot express. Taking ownership sidesteps
/// that; production callers move the clone into an `async move` block.
async fn run_agentic_prompt_turn<R, W, F, Fut>(
    context: AcpTurnContext<'_>,
    mut messages: Vec<Message>,
    reader: &mut Lines<R>,
    writer: &mut W,
    mut open_stream: F,
) -> std::result::Result<(PromptOutcome, Vec<Message>, AcpTurnUsage), AgenticPromptError>
where
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin,
    F: FnMut(Vec<Message>) -> Fut,
    Fut: Future<Output = Result<StreamEventBox>>,
{
    let AcpTurnContext {
        session_id,
        response_id_policy,
        ..
    } = context;
    let mut has_tool_receipts = false;
    let mut usage = AcpTurnUsage::default();
    for _round in 0..MAX_ACP_TOOL_ROUNDS {
        let stream = open_stream(messages.clone())
            .await
            .map_err(|error| AgenticPromptError::new(error, &messages, has_tool_receipts))?;
        let (outcome, tool_calls, round_usage) =
            drive_prompt_stream(stream, session_id, response_id_policy, reader, writer)
                .await
                .map_err(|error| AgenticPromptError::new(error, &messages, has_tool_receipts))?;
        usage.record_round(round_usage);

        let text = match outcome {
            PromptOutcome::Cancelled => return Ok((PromptOutcome::Cancelled, messages, usage)),
            PromptOutcome::Completed(text) => text,
            PromptOutcome::MaxRounds(text) => text,
        };

        let mut assistant_content = Vec::new();
        if !text.is_empty() {
            assistant_content.push(ContentBlock::Text {
                text: text.clone(),
                cache_control: None,
            });
        }
        for call in &tool_calls {
            assistant_content.push(ContentBlock::ToolUse {
                id: call.id.clone(),
                name: call.name.clone(),
                input: call.input.clone(),
                caller: None,
                thought_signature: None,
            });
        }
        if !assistant_content.is_empty() {
            messages.push(Message {
                role: Role::Assistant,
                content: assistant_content,
            });
        }

        if tool_calls.is_empty() {
            return Ok((PromptOutcome::Completed(text), messages, usage));
        }

        let batch = execute_tool_calls_with_cancellation(context, tool_calls, reader, writer)
            .await
            .map_err(|error| AgenticPromptError::new(error, &messages, has_tool_receipts))?;
        match batch {
            ToolBatchOutcome::Cancelled(tool_result_messages) => {
                messages.extend(tool_result_messages);
                return Ok((PromptOutcome::Cancelled, messages, usage));
            }
            ToolBatchOutcome::Completed(tool_result_messages) => {
                messages.extend(tool_result_messages);
                has_tool_receipts = true;
            }
        }
    }

    // Max rounds reached: return the text accumulated in the final round
    // rather than an error, so the client gets a structured completion
    // with a clear stop reason.
    let final_text = messages
        .iter()
        .rev()
        .find(|m| m.role == "assistant")
        .and_then(|m| {
            m.content.iter().find_map(|b| match b {
                ContentBlock::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
        })
        .unwrap_or_default();
    Ok((PromptOutcome::MaxRounds(final_text), messages, usage))
}

struct AcpServer {
    config: Config,
    model: String,
    default_cwd: PathBuf,
    sessions: HashMap<String, AcpSession>,
    /// Insertion-order tracking of session ids. Used to evict the *oldest*
    /// session (by insertion order, not arbitrary HashMap iteration) when
    /// the session cap is reached.
    insertion_order: VecDeque<String>,
    response_id_policy: JsonRpcResponseIdPolicy,
}

struct AcpSession {
    cwd: PathBuf,
    messages: Vec<Message>,
    /// Tokens across every turn of the session, for `session/prompt`'s
    /// cumulative `usage` and the running cost behind `usage_update`.
    usage: Usage,
    /// Priced spend so far, per currency. A turn whose route cannot be priced
    /// adds nothing rather than a fake zero.
    cost_usd: f64,
    cost_cny: f64,
    /// The session's MCP pool: the user's configured servers plus anything the
    /// client handed us in `session/new`. Kept so the session can shut its
    /// child processes down; see [`AcpServer::retire_session`].
    mcp_pool: Option<Arc<tokio::sync::Mutex<McpPool>>>,
    /// Built once per session over the session `cwd`, then reused for every
    /// prompt turn: `to_api_tools()` memoises the serialised catalog, and
    /// `file_read_tracker` / the shell manager need to persist across turns.
    tool_registry: Arc<ToolRegistry>,
}

/// Route facts captured when a prompt round is dispatched: enough to price the
/// turn's usage and size the context without re-reading mutable config after
/// the fact (the same rule the interactive turn loop follows).
#[derive(Debug, Clone)]
struct AcpRouteReceipt {
    envelope: crate::cost_status::EffectiveRouteEnvelope,
    context_window_tokens: u32,
}

/// What one `usage_update` notification carries after a turn.
#[derive(Debug, Clone, PartialEq)]
struct AcpUsageContext {
    /// Tokens the context currently holds: the final round's prompt plus its
    /// answer (`input_tokens` is the authoritative prompt total; cache
    /// classes partition it, they do not add to it).
    used: u64,
    /// Context window of the route that ran.
    size: u64,
    /// Session spend so far, when the route is money-metered and priced.
    cost: Option<(f64, &'static str)>,
}

/// The `&mut self` result of validating a `session/prompt`: the user turn is
/// already recorded, and the cloned conversation + cwd are ready for the
/// borrow-free provider call that the prompt driver races against cancellation.
struct PreparedPrompt {
    session_id: String,
    messages: Vec<Message>,
    cwd: PathBuf,
}

enum AcpDispatch {
    Response(Value),
    /// A response followed by `session/update` notifications for the same
    /// session, written after the response so the client sees them in the
    /// order the protocol describes.
    ResponseThenNotify {
        result: Value,
        session_id: String,
        updates: Vec<Value>,
    },
    Shutdown,
}

#[derive(Debug)]
struct AcpError {
    code: i32,
    message: String,
}

impl AcpServer {
    fn new(config: Config, model: String, default_cwd: PathBuf) -> Self {
        Self {
            config,
            model,
            default_cwd,
            sessions: HashMap::new(),
            insertion_order: VecDeque::new(),
            response_id_policy: JsonRpcResponseIdPolicy::Preserve,
        }
    }

    // `session/prompt` is handled in the main loop (it needs to run concurrently
    // with the reader for cancellation); every other method is request/response.
    async fn handle_request(
        &mut self,
        method: &str,
        params: Value,
    ) -> std::result::Result<AcpDispatch, AcpError> {
        match method {
            "initialize" => {
                self.response_id_policy = JsonRpcResponseIdPolicy::from_initialize_params(&params);
                Ok(AcpDispatch::Response(initialize_result(
                    params.get("protocolVersion").and_then(Value::as_u64),
                    &self.config,
                )))
            }
            "session/new" => {
                let result = self.new_session(params).await?;
                let session_id = result["sessionId"].as_str().unwrap_or_default().to_string();
                // Goose sends the (empty) command list at session setup and
                // clients such as Zed wait for it before enabling slash input.
                Ok(AcpDispatch::ResponseThenNotify {
                    result,
                    session_id,
                    updates: vec![json!({
                        "sessionUpdate": "available_commands_update",
                        "availableCommands": []
                    })],
                })
            }
            "session/set_config_option" => {
                let (session_id, result) = self.set_config_option(params)?;
                let update = json!({
                    "sessionUpdate": "config_option_update",
                    "configOptions": result["configOptions"].clone()
                });
                Ok(AcpDispatch::ResponseThenNotify {
                    result,
                    session_id,
                    updates: vec![update],
                })
            }
            // A cancel that arrives with no prompt in flight is an idempotent
            // no-op (the in-flight case is handled by the prompt driver).
            "session/cancel" => Ok(AcpDispatch::Response(json!(null))),
            "shutdown" => {
                self.retire_all_sessions().await;
                Ok(AcpDispatch::Shutdown)
            }
            _ => Err(AcpError::method_not_found(method)),
        }
    }

    async fn new_session(&mut self, params: Value) -> std::result::Result<Value, AcpError> {
        let cwd = params
            .get("cwd")
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .unwrap_or_else(|| self.default_cwd.clone());

        let mcp_pool = self.build_session_mcp_pool(&params, &cwd).await?;
        let session_id = format!("ghosty-{}", uuid::Uuid::new_v4());
        let tool_registry =
            Arc::new(build_acp_tool_registry(&self.config, &cwd, mcp_pool.as_ref()).await);

        // The tool array goes to the provider whole. Over the cap the request
        // is rejected outright, and until now that surfaced as an opaque
        // provider error on every turn — so refuse the session instead, while
        // the client can still act on it.
        let tool_count = tool_registry.to_api_tools().len();
        if tool_count > crate::core::engine::tool_catalog::DEEPSEEK_MAX_TOOLS {
            if let Some(pool) = mcp_pool {
                pool.lock().await.shutdown_all().await;
            }
            return Err(AcpError::invalid_params(format!(
                "this session would expose {tool_count} tools, over the provider cap of {}. \
                 Reduce the MCP servers passed in mcpServers or configured in mcp.json.",
                crate::core::engine::tool_catalog::DEEPSEEK_MAX_TOOLS
            )));
        }

        // Evict oldest session when at capacity.
        if self.sessions.len() >= MAX_ACP_SESSIONS {
            // `VecDeque` preserves true insertion order; HashMap iteration
            // does not. Pop from the front to evict the session created
            // earliest.
            if let Some(oldest) = self.insertion_order.pop_front() {
                self.retire_session(&oldest).await;
            }
        }

        self.insertion_order.push_back(session_id.clone());
        self.sessions.insert(
            session_id.clone(),
            AcpSession {
                cwd,
                messages: Vec::new(),
                usage: Usage::default(),
                cost_usd: 0.0,
                cost_cny: 0.0,
                tool_registry,
                mcp_pool,
            },
        );
        Ok(json!({
            "sessionId": session_id,
            "configOptions": self.config_options()
        }))
    }

    /// Assemble the session's MCP pool: the user's own `mcp.json` servers plus
    /// whatever the client passed in `mcpServers`. Goose merges the two the
    /// same way rather than letting one replace the other, and it is the right
    /// call: the operator's servers are why the agent works at all, and the
    /// client's are what this particular conversation needs.
    async fn build_session_mcp_pool(
        &self,
        params: &Value,
        cwd: &std::path::Path,
    ) -> std::result::Result<Option<Arc<tokio::sync::Mutex<McpPool>>>, AcpError> {
        let requested = params
            .get("mcpServers")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        let path = self.config.mcp_config_path();
        let plugins = Arc::new(crate::plugins::PluginRegistry::empty(cwd));
        let pool = McpPool::from_config_path_with_workspace_and_plugins(&path, cwd, plugins)
            .map_err(|error| {
                AcpError::invalid_params(format!(
                    "could not load the MCP config at {}: {error}",
                    path.display()
                ))
            })?;

        for entry in &requested {
            let (name, config) = parse_acp_mcp_server(entry).map_err(AcpError::invalid_params)?;
            pool.add_runtime_server_config(name, config)
                .map_err(AcpError::invalid_params)?;
        }

        Ok(Some(Arc::new(tokio::sync::Mutex::new(pool))))
    }

    /// Drop a session and stop anything it owns. Eviction used to be a bare
    /// `remove`, which is fine for a struct of plain data and a leak once the
    /// session owns child processes.
    async fn retire_session(&mut self, session_id: &str) {
        if let Some(session) = self.sessions.remove(session_id)
            && let Some(pool) = session.mcp_pool
        {
            pool.lock().await.shutdown_all().await;
        }
    }

    /// Stop every session's MCP children. Called on `shutdown`.
    async fn retire_all_sessions(&mut self) {
        let ids = self.insertion_order.drain(..).collect::<Vec<_>>();
        for id in ids {
            self.retire_session(&id).await;
        }
    }

    fn session_tool_registry(&self, session_id: &str) -> Option<Arc<ToolRegistry>> {
        self.sessions
            .get(session_id)
            .map(|session| session.tool_registry.clone())
    }

    /// Provider ids a client may select: built-ins plus the user's own
    /// `[providers.<name>]` tables, which keep their raw key so routing can
    /// still find the configured base URL / auth / model (#1519).
    fn provider_choices(&self) -> Vec<(String, String)> {
        let mut providers = ApiProvider::sorted_for_display()
            .into_iter()
            .map(|provider| {
                (
                    provider.as_str().to_string(),
                    provider.display_name().to_string(),
                )
            })
            .collect::<Vec<_>>();
        if let Some(custom) = self.config.providers.as_ref().map(|p| &p.custom) {
            let mut names = custom.keys().collect::<Vec<_>>();
            names.sort();
            providers.extend(names.into_iter().map(|name| (name.clone(), name.clone())));
        }
        providers
    }

    /// The configured provider key as the client should see it: a custom
    /// `[providers.<name>]` entry round-trips by name instead of canonicalizing
    /// to "custom".
    fn current_provider_key(&self) -> String {
        match self.config.provider.as_deref() {
            Some(name) if !name.trim().is_empty() => name.to_string(),
            _ => self.config.api_provider().as_str().to_string(),
        }
    }

    /// Catalog models for the active provider, with the active model first
    /// so the current value is always one of the options.
    fn model_choices(&self) -> Vec<String> {
        let active = self.config.api_provider();
        let mut models = vec![self.model.clone()];
        for model in crate::provider_lake::models_for_provider(&self.config, active, active) {
            if !models.contains(&model) {
                models.push(model);
            }
        }
        models
    }

    fn current_thinking_effort(&self) -> &'static str {
        self.config
            .reasoning_effort()
            .filter(|_| self.config.reasoning_effort_is_explicit())
            .map_or("auto", |value| {
                crate::tui::app::ReasoningEffort::from_setting(value).as_setting()
            })
    }

    /// The session config options every ACP client understands: provider,
    /// model, and thinking effort as `select` options. This replaces the
    /// former `session/listProviders` / `session/currentModel` /
    /// `session/selectModel` methods, which no ACP client called.
    fn config_options(&self) -> Value {
        let select = |value: &str, name: &str| json!({ "value": value, "name": name });
        let providers = self
            .provider_choices()
            .iter()
            .map(|(id, name)| select(id, name))
            .collect::<Vec<_>>();
        let models = self
            .model_choices()
            .iter()
            .map(|model| select(model, model))
            .collect::<Vec<_>>();
        let efforts = THINKING_EFFORT_VALUES
            .iter()
            .map(|value| select(value, value))
            .collect::<Vec<_>>();
        json!([
            {
                "id": CONFIG_PROVIDER,
                "name": "Provider",
                "type": "select",
                "currentValue": self.current_provider_key(),
                "options": providers
            },
            {
                "id": CONFIG_MODEL,
                "name": "Model",
                "category": "model",
                "type": "select",
                "currentValue": self.model,
                "options": models
            },
            {
                "id": CONFIG_THINKING_EFFORT,
                "name": "Thinking effort",
                "category": "thought_level",
                "type": "select",
                "currentValue": self.current_thinking_effort(),
                "options": efforts
            }
        ])
    }

    /// `session/set_config_option`: apply one option and return the session
    /// id plus the refreshed option list, which the caller also sends as a
    /// `config_option_update` (goose does the same).
    fn set_config_option(
        &mut self,
        params: Value,
    ) -> std::result::Result<(String, Value), AcpError> {
        let session_id = params
            .get("sessionId")
            .and_then(Value::as_str)
            .ok_or_else(|| AcpError::invalid_params("sessionId is required"))?
            .to_string();
        if !self.sessions.contains_key(&session_id) {
            return Err(AcpError::invalid_params("unknown sessionId"));
        }
        let config_id = params
            .get("configId")
            .and_then(Value::as_str)
            .ok_or_else(|| AcpError::invalid_params("configId is required"))?;
        let value = params
            .get("value")
            .and_then(Value::as_str)
            .ok_or_else(|| AcpError::invalid_params("value must be a string"))?
            .trim();
        if value.is_empty() {
            return Err(AcpError::invalid_params("value must not be empty"));
        }

        match config_id {
            CONFIG_PROVIDER => self.set_provider(value)?,
            CONFIG_MODEL => self.model = value.to_string(),
            CONFIG_THINKING_EFFORT => {
                let effort = crate::tui::app::ReasoningEffort::parse_strict(value)
                    .map_err(AcpError::invalid_params)?;
                self.config.reasoning_effort = Some(effort.as_setting().to_string());
                self.config.reasoning_effort_inferred_from_legacy_alias = false;
            }
            other => {
                return Err(AcpError::invalid_params(format!(
                    "unknown configId: {other}"
                )));
            }
        }
        Ok((
            session_id,
            json!({ "configOptions": self.config_options() }),
        ))
    }

    /// Switch the active provider. Accepts a built-in id/alias or a custom
    /// `[providers.<name>]` table name. The model follows the provider's
    /// default so the pair stays coherent; the client can pick another model
    /// from the refreshed option list.
    fn set_provider(&mut self, provider_name: &str) -> std::result::Result<(), AcpError> {
        let is_custom = self
            .config
            .providers
            .as_ref()
            .and_then(|providers| providers.custom_provider_config(provider_name))
            .is_some();
        let builtin = ApiProvider::parse(provider_name);
        if !is_custom && builtin.is_none() {
            return Err(AcpError::invalid_params(format!(
                "unknown provider: {provider_name}"
            )));
        }
        self.config.provider = Some(provider_name.to_string());
        let default_model = if is_custom {
            self.config
                .providers
                .as_ref()
                .and_then(|providers| providers.custom_provider_config(provider_name))
                .and_then(|cfg| cfg.model.clone())
        } else {
            builtin
                .and_then(ApiProvider::metadata)
                .map(|metadata| metadata.default_model().to_string())
        };
        if let Some(model) = default_model {
            self.model = model;
        }
        Ok(())
    }

    /// Validate a `session/prompt` request and append the user turn to history,
    /// returning the cloned conversation for the (borrow-free) provider call.
    ///
    /// This is the `&mut self` half of a prompt turn; the streaming provider
    /// call lives in [`AcpServer::open_prompt_stream`] (which borrows `&self`
    /// only and returns a `'static` stream) so it can be raced against the
    /// reader for cancellation.
    fn begin_prompt(&mut self, params: Value) -> std::result::Result<PreparedPrompt, AcpError> {
        let session_id = params
            .get("sessionId")
            .and_then(Value::as_str)
            .ok_or_else(|| AcpError::invalid_params("sessionId is required"))?
            .to_string();
        let prompt = extract_prompt_text(params.get("prompt"))
            .filter(|text| !text.trim().is_empty())
            .ok_or_else(|| AcpError::invalid_params("prompt must include text content"))?;

        let (messages, cwd) = {
            let session = self
                .sessions
                .get_mut(&session_id)
                .ok_or_else(|| AcpError::invalid_params("unknown sessionId"))?;
            session.messages.push(Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: prompt,
                    cache_control: None,
                }],
            });
            (session.messages.clone(), session.cwd.clone())
        };

        Ok(PreparedPrompt {
            session_id,
            messages,
            cwd,
        })
    }

    /// Fold a finished turn's usage into the session and describe the
    /// `usage_update` to send, if the turn reported any usage at all.
    fn record_turn_usage(
        &mut self,
        session_id: &str,
        usage: &AcpTurnUsage,
        receipt: Option<&AcpRouteReceipt>,
    ) -> Option<AcpUsageContext> {
        let last_round = usage.last_round.as_ref()?;
        let session = self.sessions.get_mut(session_id)?;
        add_usage(&mut session.usage, &usage.total);
        if let Some(receipt) = receipt
            && let Some(estimate) = receipt.envelope.audit(&usage.total).estimate
        {
            session.cost_usd += estimate.usd;
            session.cost_cny += estimate.cny;
        }
        let cost = if session.cost_usd > 0.0 {
            Some((session.cost_usd, "USD"))
        } else if session.cost_cny > 0.0 {
            Some((session.cost_cny, "CNY"))
        } else {
            None
        };
        Some(AcpUsageContext {
            used: u64::from(last_round.input_tokens) + u64::from(last_round.output_tokens),
            size: receipt.map_or(0, |receipt| u64::from(receipt.context_window_tokens)),
            cost,
        })
    }

    /// Commit the full message list produced by a completed turn into the
    /// session's history — the original history plus every assistant/tool-
    /// call/tool-result round the turn drove.
    ///
    /// Called on **all** outcomes: normal completion, max-rounds, AND cancel.
    /// (The caller strips dangling assistant tool_use blocks on cancel before
    /// committing, which produces a clean partial history the next prompt can
    /// continue from instead of leaving the pre-turn state untouched.)
    fn commit_turn_messages(&mut self, session_id: &str, messages: Vec<Message>) {
        if let Some(session) = self.sessions.get_mut(session_id) {
            session.messages = messages;
        }
    }

    /// Remove the last user message from the session history. Used to unwind
    /// the `begin_prompt` push when the turn itself fails (e.g. provider
    /// stream error), so the next prompt doesn't start with two consecutive
    /// `user` messages.
    fn rollback_user_message(&mut self, session_id: &str) {
        if let Some(session) = self.sessions.get_mut(session_id)
            && session.messages.last().map(|m| m.role.as_str()) == Some("user")
        {
            session.messages.pop();
        }
    }

    /// Resolve the route, build the streaming request, and open the provider
    /// response stream. Borrows `&self` only to read config/model; the returned
    /// [`StreamEventBox`] is `'static`, so the caller can race it against the
    /// reader without holding any borrow on the server.
    ///
    /// Every path below resolves against the `cwd` argument, never against the
    /// process working directory — see the module contract.
    async fn open_prompt_stream(
        &self,
        messages: &[Message],
        cwd: &Path,
        tool_registry: &ToolRegistry,
        frozen_system_prompt: &std::sync::Mutex<Option<SystemPrompt>>,
        route_receipt: &std::sync::Mutex<Option<AcpRouteReceipt>>,
    ) -> Result<StreamEventBox> {
        let last_user_text = messages
            .iter()
            .rev()
            .find_map(|m| {
                if m.role == "user" {
                    m.content.iter().find_map(|b| match b {
                        ContentBlock::Text { text, .. } => Some(text.as_str()),
                        _ => None,
                    })
                } else {
                    None
                }
            })
            .unwrap_or("");
        let route =
            crate::resolve_cli_auto_route(&self.config, &self.model, last_user_text).await?;
        let execution_config = crate::config_for_cli_route(&self.config, &route);
        let client = DeepSeekClient::new(&execution_config)?;
        let model = route.model;
        let request_route = client.effective_route_envelope(&model, chrono::Utc::now());
        let reasoning_effort = route
            .reasoning_effort
            .and_then(|effort| {
                effort.api_value_for_route(
                    execution_config.api_provider(),
                    &execution_config.deepseek_base_url(),
                    &model,
                )
            })
            .map(str::to_string);

        let tools = tool_registry.to_api_tools();
        let (route_limits, image_input) =
            resolve_acp_route_facts(&execution_config, request_route.provider, &model);
        if let Ok(mut slot) = route_receipt.lock() {
            *slot = Some(AcpRouteReceipt {
                context_window_tokens: crate::route_budget::route_context_window_tokens(
                    request_route.provider,
                    &request_route.model,
                    route_limits,
                ),
                envelope: request_route.clone(),
            });
        }
        let system = frozen_acp_system_prompt(
            frozen_system_prompt,
            &execution_config,
            cwd,
            request_route.provider,
            &request_route.model,
            route_limits,
        );

        let mut outbound_messages = messages.to_vec();
        crate::image_attach::strip_images_when_unsupported(
            &mut outbound_messages,
            image_input,
            &request_route.model,
        );
        let request = MessageRequest {
            model,
            messages: outbound_messages,
            max_tokens: crate::route_budget::effective_max_output_tokens_for_route(
                request_route.provider,
                &request_route.model,
                route_limits,
            ),
            system: Some(system),
            tools: Some(tools.clone()),
            tool_choice: if tools.is_empty() {
                None
            } else {
                Some(json!({ "type": "auto" }))
            },
            metadata: None,
            thinking: None,
            reasoning_effort,
            stream: Some(true),
            temperature: None,
            top_p: None,
        };

        client.create_message_stream(request).await
    }
}

fn resolve_acp_route_facts(
    config: &Config,
    provider: ApiProvider,
    model: &str,
) -> (
    Option<ghosty_config::route::RouteLimits>,
    crate::model_profile::SupportState,
) {
    let Ok(route) = crate::route_runtime::resolve_runtime_route(config, provider, Some(model))
    else {
        return (None, crate::model_profile::SupportState::Unknown);
    };
    (
        crate::route_budget::known_route_limits(route.candidate.limits()),
        route.candidate.capabilities().image_input,
    )
}

/// Return the first fully composed ACP system prompt for this user turn.
/// Later tool rounds clone that exact value instead of re-reading mutable
/// instruction sources from disk.
fn frozen_acp_system_prompt(
    slot: &std::sync::Mutex<Option<SystemPrompt>>,
    config: &Config,
    workspace: &std::path::Path,
    provider: ApiProvider,
    model: &str,
    route_limits: Option<ghosty_config::route::RouteLimits>,
) -> SystemPrompt {
    let mut slot = match slot.lock() {
        Ok(slot) => slot,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some(system) = slot.as_ref() {
        return system.clone();
    }
    let system = build_acp_system_prompt(config, workspace, provider, model, route_limits);
    *slot = Some(system.clone());
    system
}

/// Compose ACP's stable prompt through the same headless host seam as
/// `ghosty exec`. Tool availability remains owned by the request catalog;
/// this function supplies the shared constitution, project instructions,
/// configured instruction files, memory, locale, and route context.
fn build_acp_system_prompt(
    config: &Config,
    workspace: &std::path::Path,
    provider: ApiProvider,
    model: &str,
    route_limits: Option<ghosty_config::route::RouteLimits>,
) -> SystemPrompt {
    let settings = crate::settings::Settings::load().unwrap_or_default();
    let locale_tag = crate::localization::resolve_locale(&settings.locale)
        .tag()
        .to_string();
    let instructions = config
        .instructions_paths()
        .into_iter()
        .map(crate::prompts::InstructionSource::from)
        .collect::<Vec<_>>();
    let skills_dir = config.skills_dir();
    let user_memory_block = crate::native_memory::native_prompt_block(
        config.memory_enabled(),
        &config.memory_path(),
        workspace,
    );

    crate::prompts::system_prompt_for_mode_with_context_skills_session_and_approval_for_host(
        workspace,
        None,
        Some(&skills_dir),
        Some(&instructions),
        crate::prompts::PromptSessionContext {
            user_memory_block: user_memory_block.as_deref(),
            goal_objective: None,
            project_context_pack_enabled: config.project_context_pack_enabled(),
            locale_tag: &locale_tag,
            translation_enabled: false,
            model_id: model,
            context_window_override: Some(crate::route_budget::route_context_window_tokens(
                provider,
                model,
                route_limits,
            )),
            verbosity: config.verbosity.as_deref(),
            skills_scan_ghosty_only: config.skills_config().scan_ghosty_only(),
            plugin_registry: None,
            mode: crate::tui::app::AppMode::Agent,
        },
        crate::prompts::PromptHost::Headless,
    )
}

/// One entry of `session/new`'s `mcpServers`, mapped onto the config shape the
/// rest of Ghosty already speaks (`crate::mcp::McpServerConfig`).
///
/// The ACP schema tags `http` and `sse` with `"type"` and leaves **stdio
/// untagged** as the fallback every agent must support, so the discriminator
/// is "does `type` say http/sse, else treat it as stdio". `headers` and `env`
/// arrive as arrays of `{name, value}` pairs rather than maps.
fn parse_acp_mcp_server(value: &Value) -> std::result::Result<(String, McpServerConfig), String> {
    let name = value
        .get("name")
        .and_then(Value::as_str)
        .filter(|name| !name.trim().is_empty())
        .ok_or_else(|| "each entry of mcpServers needs a non-empty `name`".to_string())?
        .to_string();

    let pairs = |field: &str| -> HashMap<String, String> {
        value
            .get(field)
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| {
                        Some((
                            item.get("name").and_then(Value::as_str)?.to_string(),
                            item.get("value").and_then(Value::as_str)?.to_string(),
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default()
    };

    // `McpServerConfig` has no `Default`, and hand-listing its twenty fields
    // here would rot the moment one is added. Go through its own serde
    // contract: every field it does not require carries `#[serde(default)]`.
    let mut fields = serde_json::Map::new();
    fields.insert("command".to_string(), Value::Null);
    fields.insert("url".to_string(), Value::Null);

    match value.get("type").and_then(Value::as_str) {
        Some(transport @ ("http" | "sse")) => {
            let url = value.get("url").and_then(Value::as_str).ok_or_else(|| {
                format!("mcpServers entry `{name}` of type {transport} needs `url`")
            })?;
            fields.insert("url".to_string(), Value::String(url.to_string()));
            fields.insert("headers".to_string(), json!(pairs("headers")));
            // `validate_mcp_transport` accepts only "sse" as an override;
            // Streamable HTTP is the default when none is given.
            if transport == "sse" {
                fields.insert("transport".to_string(), Value::String("sse".to_string()));
            }
        }
        Some(other) => {
            return Err(format!(
                "mcpServers entry `{name}` has unsupported type `{other}`; use http, sse, or omit it for stdio"
            ));
        }
        None => {
            let command = value
                .get("command")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("mcpServers entry `{name}` needs `command` for stdio"))?;
            fields.insert("command".to_string(), Value::String(command.to_string()));
            let args = value
                .get("args")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(str::to_string))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            fields.insert("args".to_string(), json!(args));
            fields.insert("env".to_string(), json!(pairs("env")));
        }
    }

    let config: McpServerConfig = serde_json::from_value(Value::Object(fields))
        .map_err(|error| format!("mcpServers entry `{name}` is not a usable server: {error}"))?;
    Ok((name, config))
}

/// Build the tool registry for one ACP session, rooted at the session's
/// `cwd`. Reuses the shared registry builders used by headless `exec` and the
/// MCP adapter — no ACP-specific tool implementations.
///
/// `Bash` is registered only when all three independent gates allow it:
/// headless shell access is explicitly enabled in config, the stable shell
/// feature is enabled, and any requested sandbox boundary was actually built.
/// Omitting any gate fails closed. These are the same gates headless `exec`
/// and the MCP adapter use, so ACP does not invent its own shell policy.
///
/// Deliberately *not* a gate: `clientCapabilities.terminal`. In ACP that
/// capability means "I, the client, lend you a terminal" — it describes what
/// the client offers, not whether a shell exists. `Bash` here is the agent's
/// own shell, run in this process against its own sandbox, so a client that
/// lends nothing is irrelevant to it. Reading the capability as permission
/// left the remote case — an agent alone in a microVM, whose client has no
/// terminal to lend and wants the agent to use its own — with a whole machine
/// and no way to run `ls`. The client-side `terminal/*` methods stay
/// unexposed either way.
/// `ToolContext::new` leaves `auto_approve` at its default (`false`), so the
/// shell's own last-line safety check remains active after ACP's shared
/// prepared-call, typed-policy, auto-review, repository-law, and explicit
/// `session/request_permission` gates have admitted the call.
async fn build_acp_tool_registry(
    config: &Config,
    workspace: &std::path::Path,
    mcp_pool: Option<&Arc<tokio::sync::Mutex<McpPool>>>,
) -> ToolRegistry {
    let features = config.features();
    let external_sandbox_requested = config.sandbox_backend.as_deref().is_some_and(|kind| {
        let kind = kind.trim();
        !kind.is_empty() && !kind.eq_ignore_ascii_case("none")
    });
    let sandbox_backend = match crate::sandbox::backend::create_backend(config) {
        Ok(backend) => backend.map(std::sync::Arc::from),
        Err(error) => {
            tracing::warn!("Failed to create ACP sandbox backend: {error}");
            None
        }
    };
    // A requested external sandbox is an execution boundary, not a hint. If
    // it cannot be constructed, omit Bash instead of silently running the
    // command on the local host.
    let sandbox_backend_ready = !external_sandbox_requested || sandbox_backend.is_some();
    let allow_shell = config.allow_shell()
        && features.enabled(crate::features::Feature::ShellTool)
        && sandbox_backend_ready;
    let shell_policy = if allow_shell {
        ShellPolicy::Full
    } else {
        ShellPolicy::None
    };
    let sandbox_policy = crate::core::authority::sandbox_policy_for_turn(
        crate::tui::app::AppMode::Agent,
        crate::tui::approval::ApprovalMode::Suggest,
        config.sandbox_mode.as_deref(),
        workspace,
        crate::core::authority::SandboxNetworkAccess::from_config(config.sandbox_network_access),
    );
    let mut context = ToolContext::new(workspace)
        .with_shell_policy(shell_policy)
        .with_elevated_sandbox_policy(sandbox_policy);
    match context.shell_manager.lock() {
        Ok(mut manager) => manager.set_prefer_bwrap(config.prefer_bwrap.unwrap_or(false)),
        Err(poisoned) => poisoned
            .into_inner()
            .set_prefer_bwrap(config.prefer_bwrap.unwrap_or(false)),
    }
    if let Some(backend) = sandbox_backend {
        context = context.with_sandbox_backend(backend);
    }
    let hooks_config =
        crate::hooks::HooksConfig::load_with_project(config.hooks_config(), workspace);
    context.runtime.hook_executor = Some(Arc::new(crate::hooks::HookExecutor::new(
        hooks_config,
        workspace.to_path_buf(),
    )));

    let mut builder = ToolRegistryBuilder::new()
        .with_file_tools()
        .with_search_tools()
        .with_git_tools();
    if features.enabled(crate::features::Feature::ApplyPatch) {
        builder = builder.with_patch_tools();
    }
    if allow_shell {
        builder = builder.with_foreground_shell_tools();
    }

    // MCP tools, both the user's configured servers and any the ACP client
    // supplied. The snapshot is taken here, holding the lock across an await,
    // because the builder must not guess: a registry that quietly omits every
    // MCP tool looks exactly like a server that has none.
    if let Some(pool) = mcp_pool {
        let mut locked = pool.lock().await;
        locked.connect_all().await;
        let snapshot = locked
            .all_tools()
            .into_iter()
            .map(|(name, tool)| (name, tool.clone()))
            .collect::<Vec<_>>();
        drop(locked);
        builder = builder.with_mcp_tools(Arc::clone(pool), snapshot);
    }

    let mut registry = builder.build(context);
    // ACP does not load arbitrary plugin replacements in v0.9.6, but it must
    // never fall through to a built-in the operator disabled or replaced.
    if let Some(overrides) = config
        .tools
        .as_ref()
        .and_then(|tools| tools.overrides.as_ref())
    {
        for tool_name in overrides.keys() {
            remove_acp_overridden_builtin(&mut registry, tool_name);
        }
    }
    registry
}

/// ACP does not load executable tool replacements in v0.9.6. Remove the
/// built-in compatibility family for every configured override so neither a
/// hidden legacy alias nor a newly canonical lowercase name can fall through
/// to the original implementation.
fn remove_acp_overridden_builtin(registry: &mut ToolRegistry, tool_name: &str) {
    let aliases: &[&str] = match tool_name {
        "bash" | "Bash" | "exec_shell" => &["bash", "Bash", "exec_shell"],
        "read" | "write" | "edit" | "File" | "read_file" | "write_file" | "edit_file" => &[
            "read",
            "write",
            "edit",
            "File",
            "read_file",
            "write_file",
            "edit_file",
        ],
        "apply_patch" => &["apply_patch"],
        _ => std::slice::from_ref(&tool_name),
    };
    for alias in aliases {
        registry.remove_tool(alias);
    }
}

/// ACP `kind` hint for a tool call, used by the client to pick an icon/label.
/// Falls back to `"other"` for tools without an obvious category.
///
/// `File` is a single canonical tool covering read/list/search/write/edit/
/// patch (#4625), so its kind depends on the `action` argument rather than
/// the tool name alone.
fn tool_call_kind(call: &PendingToolCall) -> &'static str {
    match call.name.as_str() {
        "File" => match call.input.get("action").and_then(Value::as_str) {
            Some("write" | "edit" | "patch") => "edit",
            _ => "read",
        },
        "apply_patch" => "edit",
        "Git" => "read",
        "bash" | "Bash" | "terminal/run" | "terminal/send" | "terminal/wait"
        | "terminal/cancel" | "terminal/reset" => "execute",
        _ => "other",
    }
}

/// Human-readable title for a tool call: the tool name plus its primary
/// argument (path/command/pattern) when present, so the client's tool-call
/// card is legible without expanding raw input.
fn tool_call_title(call: &PendingToolCall) -> String {
    let detail = call
        .input
        .get("path")
        .or_else(|| call.input.get("command"))
        .or_else(|| call.input.get("pattern"))
        .or_else(|| call.input.get("task_id"))
        .and_then(Value::as_str);
    match detail {
        Some(detail) => format!("{}: {}", call.name, detail),
        None => call.name.clone(),
    }
}

fn truncate_for_acp(content: &str) -> String {
    if content.chars().count() <= TOOL_CALL_CONTENT_PREVIEW_CHARS {
        return content.to_string();
    }
    let truncated: String = content
        .chars()
        .take(TOOL_CALL_CONTENT_PREVIEW_CHARS)
        .collect();
    format!("{truncated}\n… [truncated for display; the full result was sent to the model]")
}

async fn write_tool_call_start<W>(
    writer: &mut W,
    session_id: &str,
    call: &PendingToolCall,
) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    let notification = json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "sessionId": session_id,
            "update": {
                "sessionUpdate": "tool_call",
                "toolCallId": call.id,
                "title": tool_call_title(call),
                "kind": tool_call_kind(call),
                "status": "pending",
                "rawInput": call.input,
            }
        }
    });
    write_json_line(writer, notification).await
}

async fn write_tool_call_update<W>(
    writer: &mut W,
    session_id: &str,
    call: &PendingToolCall,
    status: &str,
    content: Option<&str>,
) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    write_tool_call_update_with_blocks(writer, session_id, call, status, content, &[]).await
}

async fn write_tool_call_update_with_blocks<W>(
    writer: &mut W,
    session_id: &str,
    call: &PendingToolCall,
    status: &str,
    content: Option<&str>,
    rich_blocks: &[ghosty_tools::ToolResultContentBlock],
) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    let mut update = json!({
        "sessionUpdate": "tool_call_update",
        "toolCallId": call.id,
        "status": status,
    });
    if content.is_some() || !rich_blocks.is_empty() {
        let mut blocks = Vec::with_capacity(rich_blocks.len() + usize::from(content.is_some()));
        if let Some(content) = content {
            blocks.push(json!({
                "type": "content",
                "content": { "type": "text", "text": truncate_for_acp(content) }
            }));
        }
        blocks.extend(rich_blocks.iter().map(|block| match block {
            ghosty_tools::ToolResultContentBlock::Image { mime_type, data } => json!({
                "type": "content",
                "content": { "type": "image", "data": data, "mimeType": mime_type }
            }),
        }));
        update["content"] = json!(blocks);
    }
    let notification = json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "sessionId": session_id,
            "update": update
        }
    });
    write_json_line(writer, notification).await
}

impl AcpError {
    fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: message.into(),
        }
    }

    fn method_not_found(method: &str) -> Self {
        Self {
            code: -32601,
            message: format!("method not found: {method}"),
        }
    }
}

fn initialize_result(client_protocol_version: Option<u64>, config: &Config) -> Value {
    json!({
        "protocolVersion": client_protocol_version
            .map(|version| version.min(ACP_PROTOCOL_VERSION))
            .unwrap_or(ACP_PROTOCOL_VERSION),
        "agentCapabilities": {
            "loadSession": false,
            "promptCapabilities": {
                "image": false,
                "audio": false,
                "embeddedContext": true
            },
            "mcpCapabilities": {
                "http": true,
                "sse": true
            },
            "sessionCapabilities": {}
        },
        "agentInfo": {
            "name": "ghosty",
            "title": "ghosty",
            "version": env!("CARGO_PKG_VERSION")
        },
        "authMethods": acp_auth_methods(config)
    })
}

fn acp_auth_methods(config: &Config) -> Value {
    let provider = config.api_provider().as_str();
    json!([
        {
            "id": "ghosty-terminal-auth",
            "name": "Set Ghosty API key",
            "description": format!("Run Ghosty's terminal credential setup for the {provider} provider."),
            "type": "terminal",
            "args": ["auth", "set", "--provider", provider],
            "env": {}
        }
    ])
}

fn extract_prompt_text(prompt: Option<&Value>) -> Option<String> {
    match prompt? {
        Value::String(text) => Some(text.clone()),
        Value::Array(blocks) => {
            let parts = blocks
                .iter()
                .filter_map(content_block_text)
                .collect::<Vec<_>>();
            (!parts.is_empty()).then(|| parts.join("\n\n"))
        }
        _ => None,
    }
}

fn content_block_text(block: &Value) -> Option<String> {
    match block.get("type").and_then(Value::as_str)? {
        "text" => block
            .get("text")
            .and_then(Value::as_str)
            .map(str::to_string),
        "resource" => resource_text(block),
        "resource_link" | "resourceLink" => resource_link_text(block),
        _ => None,
    }
}

fn resource_text(block: &Value) -> Option<String> {
    let resource = block.get("resource").unwrap_or(block);
    if let Some(text) = resource.get("text").and_then(Value::as_str) {
        return Some(text.to_string());
    }
    resource_link_text(resource)
}

fn resource_link_text(block: &Value) -> Option<String> {
    let uri = block
        .get("uri")
        .or_else(|| block.pointer("/resource/uri"))
        .and_then(Value::as_str)?;
    Some(format!("@{uri}"))
}

async fn write_session_update<W>(writer: &mut W, session_id: &str, text: String) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    write_session_chunk(writer, session_id, "agent_message_chunk", text).await
}

/// One text content chunk: `agent_message_chunk` or `agent_thought_chunk`.
async fn write_session_chunk<W>(
    writer: &mut W,
    session_id: &str,
    session_update: &str,
    text: String,
) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    write_session_notification(
        writer,
        session_id,
        json!({
            "sessionUpdate": session_update,
            "content": {
                "type": "text",
                "text": text
            }
        }),
    )
    .await
}

/// Any `session/update` notification; `update` is the tagged update object.
async fn write_session_notification<W>(
    writer: &mut W,
    session_id: &str,
    update: Value,
) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    write_json_line(
        writer,
        json!({
            "jsonrpc": "2.0",
            "method": "session/update",
            "params": {
                "sessionId": session_id,
                "update": update
            }
        }),
    )
    .await
}

async fn write_usage_update<W>(
    writer: &mut W,
    session_id: &str,
    context: &AcpUsageContext,
) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    let mut update = json!({
        "sessionUpdate": "usage_update",
        "used": context.used,
        "size": context.size
    });
    if let Some((amount, currency)) = context.cost {
        update["cost"] = json!({ "amount": amount, "currency": currency });
    }
    write_session_notification(writer, session_id, update).await
}

/// ACP `Usage` for a `session/prompt` response, from a provider usage total.
fn acp_prompt_usage(usage: &Usage) -> Value {
    let input = u64::from(usage.input_tokens);
    let output = u64::from(usage.output_tokens);
    let mut value = json!({
        "totalTokens": input + output,
        "inputTokens": input,
        "outputTokens": output
    });
    if let Some(thought) = usage.reasoning_tokens {
        value["thoughtTokens"] = json!(thought);
    }
    if let Some(read) = usage.prompt_cache_hit_tokens {
        value["cachedReadTokens"] = json!(read);
    }
    if let Some(write) = usage.prompt_cache_write_tokens {
        value["cachedWriteTokens"] = json!(write);
    }
    value
}

async fn write_jsonrpc_result<W>(writer: &mut W, id: Value, result: Value) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    write_json_line(
        writer,
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        }),
    )
    .await
}

async fn write_jsonrpc_error<W>(
    writer: &mut W,
    id: Option<Value>,
    code: i32,
    message: impl Into<String>,
) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    write_json_line(
        writer,
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": code,
                "message": message.into()
            }
        }),
    )
    .await
}

async fn write_json_line<W>(writer: &mut W, value: Value) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    writer.write_all(value.to_string().as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JsonRpcResponseIdPolicy {
    /// JSON-RPC's normal contract: echo the request id without changing type.
    Preserve,
    /// Zed's ACP client currently decodes response ids as strings even when it
    /// sent a number. Keep this narrow compatibility mode client-identified.
    StringifyNumeric,
}

impl JsonRpcResponseIdPolicy {
    fn from_initialize_params(params: &Value) -> Self {
        let client_name = params
            .pointer("/clientInfo/name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if client_name.eq_ignore_ascii_case("zed") {
            Self::StringifyNumeric
        } else {
            Self::Preserve
        }
    }

    fn response_id(self, id: Value) -> Value {
        match (self, id) {
            (Self::StringifyNumeric, Value::Number(number)) => Value::String(number.to_string()),
            (_, id) => id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::VecDeque;

    #[tokio::test]
    async fn tool_update_emits_typed_acp_image_content() {
        let mut output = Vec::new();
        let call = PendingToolCall {
            id: "call_image_1".to_string(),
            name: "read".to_string(),
            input: json!({"path": "shot.png"}),
            parse_error: None,
        };
        write_tool_call_update_with_blocks(
            &mut output,
            "session_1",
            &call,
            "completed",
            Some("screenshot captured"),
            &[ghosty_tools::ToolResultContentBlock::Image {
                mime_type: "image/png".to_string(),
                data: "QUJD".to_string(),
            }],
        )
        .await
        .expect("ACP update");

        let lines = parse_lines(output);
        let content = lines[0]["params"]["update"]["content"]
            .as_array()
            .expect("ACP content blocks");
        assert_eq!(content[0]["content"]["type"], "text");
        assert_eq!(content[1]["content"]["type"], "image");
        assert_eq!(content[1]["content"]["mimeType"], "image/png");
        assert_eq!(content[1]["content"]["data"], "QUJD");
    }

    #[test]
    fn initialize_advertises_baseline_acp_agent() {
        let result = initialize_result(Some(1), &Config::default());

        assert_eq!(result["protocolVersion"], 1);
        assert_eq!(result["agentInfo"]["name"], "ghosty");
        assert_eq!(result["agentCapabilities"]["loadSession"], false);
        assert_eq!(
            result["agentCapabilities"]["promptCapabilities"]["embeddedContext"],
            true
        );
        assert_eq!(result["authMethods"][0]["type"], "terminal");
        assert_eq!(
            result["authMethods"][0]["args"],
            json!(["auth", "set", "--provider", "deepseek"])
        );
    }

    #[test]
    fn initialize_does_not_advertise_nonstandard_capabilities() {
        let result = initialize_result(Some(1), &Config::default());

        assert!(result["agentCapabilities"].get("modelSelection").is_none());
    }

    #[test]
    fn config_options_expose_provider_model_and_effort() {
        let config = Config::default();
        let expected_provider = config.api_provider().as_str().to_string();
        let server = AcpServer::new(config, "deepseek-reasoner".into(), PathBuf::from("/tmp"));

        let options = server.config_options();
        let options = options.as_array().expect("options array");
        let by_id = |id: &str| {
            options
                .iter()
                .find(|option| option["id"] == id)
                .unwrap_or_else(|| panic!("option {id}"))
        };

        let provider = by_id(CONFIG_PROVIDER);
        assert_eq!(provider["type"], "select");
        assert_eq!(provider["currentValue"], expected_provider);
        assert!(
            provider["options"]
                .as_array()
                .expect("provider options")
                .iter()
                .any(|option| option["value"] == "deepseek")
        );

        let model = by_id(CONFIG_MODEL);
        assert_eq!(model["category"], "model");
        assert_eq!(model["currentValue"], "deepseek-reasoner");
        assert_eq!(model["options"][0]["value"], "deepseek-reasoner");

        let effort = by_id(CONFIG_THINKING_EFFORT);
        assert_eq!(effort["category"], "thought_level");
        assert_eq!(effort["currentValue"], "auto");
    }

    fn server_with_session() -> (AcpServer, String) {
        let mut server = AcpServer::new(
            Config::default(),
            "deepseek-chat".into(),
            PathBuf::from("/tmp"),
        );
        let session_id = "ghosty-test".to_string();
        server.insertion_order.push_back(session_id.clone());
        server.sessions.insert(
            session_id.clone(),
            AcpSession {
                cwd: PathBuf::from("/tmp"),
                messages: Vec::new(),
                usage: Usage::default(),
                cost_usd: 0.0,
                cost_cny: 0.0,
                mcp_pool: None,
                tool_registry: Arc::new(
                    ToolRegistryBuilder::new().build(ToolContext::new(PathBuf::from("/tmp"))),
                ),
            },
        );
        (server, session_id)
    }

    #[test]
    fn set_config_option_switches_provider_model_and_effort() {
        let (mut server, session_id) = server_with_session();

        let (returned_id, result) = server
            .set_config_option(json!({
                "sessionId": session_id,
                "configId": CONFIG_PROVIDER,
                "value": "openai"
            }))
            .expect("set provider");
        assert_eq!(returned_id, session_id);
        assert_eq!(server.current_provider_key(), "openai");
        // The model follows the provider so the pair stays coherent.
        assert_ne!(server.model, "deepseek-chat");
        assert_eq!(result["configOptions"][0]["currentValue"], "openai");

        server
            .set_config_option(json!({
                "sessionId": session_id,
                "configId": CONFIG_MODEL,
                "value": "gpt-4o"
            }))
            .expect("set model");
        assert_eq!(server.model, "gpt-4o");

        server
            .set_config_option(json!({
                "sessionId": session_id,
                "configId": CONFIG_THINKING_EFFORT,
                "value": "high"
            }))
            .expect("set effort");
        assert_eq!(server.config.reasoning_effort(), Some("high"));
        assert_eq!(server.current_thinking_effort(), "high");
    }

    #[test]
    fn set_config_option_rejects_bad_input() {
        let (mut server, session_id) = server_with_session();
        let before = server.config_options();

        for params in [
            json!({ "sessionId": "nope", "configId": CONFIG_MODEL, "value": "x" }),
            json!({ "sessionId": session_id, "configId": CONFIG_PROVIDER, "value": "unknown-provider" }),
            json!({ "sessionId": session_id, "configId": CONFIG_THINKING_EFFORT, "value": "turbo" }),
            json!({ "sessionId": session_id, "configId": "colour", "value": "blue" }),
            json!({ "sessionId": session_id, "configId": CONFIG_MODEL, "value": 7 }),
        ] {
            let err = server.set_config_option(params).expect_err("rejected");
            assert_eq!(err.code, -32602);
        }
        assert_eq!(server.config_options(), before);
    }

    #[test]
    fn prompt_usage_maps_provider_usage_to_acp_fields() {
        let usage = Usage {
            input_tokens: 100,
            output_tokens: 40,
            prompt_cache_hit_tokens: Some(70),
            prompt_cache_miss_tokens: Some(30),
            prompt_cache_write_tokens: None,
            reasoning_tokens: Some(12),
            reasoning_replay_tokens: None,
            server_tool_use: None,
        };
        assert_eq!(
            acp_prompt_usage(&usage),
            json!({
                "totalTokens": 140,
                "inputTokens": 100,
                "outputTokens": 40,
                "thoughtTokens": 12,
                "cachedReadTokens": 70
            })
        );
    }

    #[tokio::test]
    async fn usage_update_carries_context_and_cost() {
        let mut out = Vec::new();
        write_usage_update(
            &mut out,
            "sess_1",
            &AcpUsageContext {
                used: 1_500,
                size: 128_000,
                cost: Some((0.0042, "USD")),
            },
        )
        .await
        .expect("write usage update");

        let update = &parse_lines(out)[0]["params"]["update"];
        assert_eq!(update["sessionUpdate"], "usage_update");
        assert_eq!(update["used"], 1_500);
        assert_eq!(update["size"], 128_000);
        assert_eq!(update["cost"]["currency"], "USD");
    }

    #[test]
    fn extract_prompt_text_accepts_text_and_resource_blocks() {
        let prompt = json!([
            { "type": "text", "text": "Review this file" },
            {
                "type": "resource",
                "resource": {
                    "uri": "file:///tmp/app.rs",
                    "mimeType": "text/rust",
                    "text": "fn main() {}"
                }
            },
            { "type": "resource_link", "uri": "file:///tmp/lib.rs" }
        ]);

        let text = extract_prompt_text(Some(&prompt)).expect("prompt text");

        assert!(text.contains("Review this file"));
        assert!(text.contains("fn main() {}"));
        assert!(text.contains("@file:///tmp/lib.rs"));
    }

    #[tokio::test]
    async fn session_update_is_protocol_clean_single_line_json() {
        let mut out = Vec::new();

        write_session_update(&mut out, "sess_1", "hello\nworld".to_string())
            .await
            .expect("write update");

        let line = String::from_utf8(out).expect("utf8");
        assert_eq!(line.lines().count(), 1);
        let value: Value = serde_json::from_str(line.trim()).expect("json");
        assert_eq!(value["method"], "session/update");
        assert_eq!(value["params"]["sessionId"], "sess_1");
        assert_eq!(value["params"]["update"]["content"]["text"], "hello\nworld");
    }

    #[tokio::test]
    async fn jsonrpc_result_preserves_numeric_ids_for_avante_acp() {
        let mut out = Vec::new();

        let params = json!({
            "protocolVersion": 1,
            "clientCapabilities": {}
        });
        let id = JsonRpcResponseIdPolicy::from_initialize_params(&params).response_id(json!(1));
        write_jsonrpc_result(&mut out, id, json!({"ok": true}))
            .await
            .expect("write result");

        let line = String::from_utf8(out).expect("utf8");
        let value: Value = serde_json::from_str(line.trim()).expect("json");
        // Numeric ID must stay numeric — avante.nvim's Lua client uses
        // strict table keys (callbacks[1] ≠ callbacks["1"]).
        assert!(
            value["id"].is_number(),
            "numeric id must stay numeric, got {:?}",
            value["id"]
        );
        assert_eq!(value["result"], json!({"ok": true}));
    }

    #[tokio::test]
    async fn jsonrpc_result_stringifies_numeric_ids_for_zed_acp() {
        let mut out = Vec::new();

        let params = json!({
            "protocolVersion": 1,
            "clientCapabilities": {},
            "clientInfo": {
                "name": "zed",
                "version": "1.2.6"
            }
        });
        let id = JsonRpcResponseIdPolicy::from_initialize_params(&params).response_id(json!(1));
        write_jsonrpc_result(&mut out, id, json!({"ok": true}))
            .await
            .expect("write result");

        let line = String::from_utf8(out).expect("utf8");
        let value: Value = serde_json::from_str(line.trim()).expect("json");
        assert_eq!(value["id"], "1");
        assert_eq!(value["result"], json!({"ok": true}));
    }

    #[tokio::test]
    async fn jsonrpc_error_keeps_absent_id_null() {
        let mut out = Vec::new();

        write_jsonrpc_error(&mut out, None, -32700, "invalid json")
            .await
            .expect("write error");

        let line = String::from_utf8(out).expect("utf8");
        let value: Value = serde_json::from_str(line.trim()).expect("json");
        assert_eq!(value["id"], Value::Null);
        assert_eq!(value["error"]["code"], -32700);
    }

    #[tokio::test]
    async fn new_session_starts_with_empty_messages() {
        let mut server = AcpServer::new(
            Config::default(),
            "test-model".to_string(),
            PathBuf::from("/tmp"),
        );
        let result = server
            .new_session(json!({ "cwd": "/tmp" }))
            .await
            .expect("new session");
        let session_id = result["sessionId"].as_str().expect("session id");
        let session = server.sessions.get(session_id).expect("session exists");
        assert!(session.messages.is_empty());
    }

    #[tokio::test]
    async fn prompt_appends_user_and_assistant_messages_to_history() {
        let mut server = AcpServer::new(
            Config::default(),
            "test-model".to_string(),
            PathBuf::from("/tmp"),
        );
        let result = server
            .new_session(json!({ "cwd": "/tmp" }))
            .await
            .expect("new session");
        let session_id = result["sessionId"].as_str().unwrap().to_string();

        // Simulate adding a user message (same logic as prompt() but without LLM call)
        {
            let session = server.sessions.get_mut(&session_id).unwrap();
            session.messages.push(Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: "1+1".to_string(),
                    cache_control: None,
                }],
            });
        }

        // Simulate assistant response
        {
            let session = server.sessions.get_mut(&session_id).unwrap();
            session.messages.push(Message {
                role: Role::Assistant,
                content: vec![ContentBlock::Text {
                    text: "2".to_string(),
                    cache_control: None,
                }],
            });
        }

        // Second user message
        {
            let session = server.sessions.get_mut(&session_id).unwrap();
            session.messages.push(Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: "add one more".to_string(),
                    cache_control: None,
                }],
            });
        }

        // Verify full conversation history
        let session = server.sessions.get(&session_id).unwrap();
        assert_eq!(session.messages.len(), 3);
        assert_eq!(session.messages[0].role, "user");
        assert_eq!(session.messages[1].role, "assistant");
        assert_eq!(session.messages[2].role, "user");

        // Verify text content
        assert_eq!(
            match &session.messages[0].content[0] {
                ContentBlock::Text { text, .. } => text.clone(),
                _ => String::new(),
            },
            "1+1"
        );
        assert_eq!(
            match &session.messages[1].content[0] {
                ContentBlock::Text { text, .. } => text.clone(),
                _ => String::new(),
            },
            "2"
        );
        assert_eq!(
            match &session.messages[2].content[0] {
                ContentBlock::Text { text, .. } => text.clone(),
                _ => String::new(),
            },
            "add one more"
        );
    }

    fn lines_from(input: &'static str) -> Lines<BufReader<&'static [u8]>> {
        BufReader::new(input.as_bytes()).lines()
    }

    fn text_delta(text: &str) -> StreamEvent {
        StreamEvent::ContentBlockDelta {
            index: 0,
            delta: Delta::TextDelta {
                text: text.to_string(),
            },
        }
    }

    /// Simulate one streamed `tool_use` content block at `index`: a start
    /// event carrying the id/name, an `input_json_delta` with the full
    /// arguments JSON, and the closing stop event — matching the real
    /// provider's per-index streaming shape closely enough to exercise
    /// [`drive_prompt_stream`]'s accumulator.
    fn tool_use_events(index: u32, id: &str, name: &str, input_json: &str) -> Vec<StreamEvent> {
        vec![
            StreamEvent::ContentBlockStart {
                index,
                content_block: ContentBlockStart::ToolUse {
                    id: id.to_string(),
                    name: name.to_string(),
                    input: json!({}),
                    caller: None,
                    thought_signature: None,
                },
            },
            StreamEvent::ContentBlockDelta {
                index,
                delta: Delta::InputJsonDelta {
                    partial_json: input_json.to_string(),
                },
            },
            StreamEvent::ContentBlockStop { index },
        ]
    }

    /// A stream that yields the given events immediately, then ends.
    fn ready_stream(events: Vec<StreamEvent>) -> StreamEventBox {
        Box::pin(futures_util::stream::iter(
            events.into_iter().map(Ok::<_, anyhow::Error>),
        ))
    }

    fn error_stream(message: &'static str) -> StreamEventBox {
        Box::pin(futures_util::stream::iter(vec![Err(anyhow!(message))]))
    }

    /// A stream that never yields, so a concurrent cancel always wins.
    fn pending_stream() -> StreamEventBox {
        Box::pin(futures_util::stream::pending::<Result<StreamEvent>>())
    }

    /// A stream that yields `events` immediately, then emits `message_stop`
    /// after a short delay — long enough that an already-buffered reader line is
    /// processed first, making the ordering deterministic in tests.
    fn events_then_delayed_stop(events: Vec<StreamEvent>) -> StreamEventBox {
        let head = futures_util::stream::iter(events.into_iter().map(Ok::<_, anyhow::Error>));
        let tail = futures_util::stream::once(async {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            Ok(StreamEvent::MessageStop)
        });
        Box::pin(head.chain(tail))
    }

    fn parse_lines(out: Vec<u8>) -> Vec<Value> {
        String::from_utf8(out)
            .expect("utf8")
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).expect("json"))
            .collect()
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum PermissionClientScript {
        Allow,
        Reject,
        WrongIdThenReject,
        CancelThenLateAllow,
        AllowThenCancelRunning,
    }

    async fn write_client_message(writer: &mut tokio::io::DuplexStream, message: Value) {
        writer
            .write_all(format!("{message}\n").as_bytes())
            .await
            .expect("write simulated ACP client message");
    }

    async fn drive_permission_client(
        output: tokio::io::DuplexStream,
        mut input: tokio::io::DuplexStream,
        script: PermissionClientScript,
        must_not_exist_before_response: Option<PathBuf>,
    ) -> Vec<Value> {
        let mut output = BufReader::new(output).lines();
        let mut seen = Vec::new();
        let mut sent_running_cancel = false;
        while let Some(line) = output.next_line().await.expect("read agent output") {
            let message: Value = serde_json::from_str(&line).expect("agent output json");
            seen.push(message.clone());

            if message.get("method").and_then(Value::as_str) == Some("session/request_permission") {
                if let Some(path) = must_not_exist_before_response.as_ref() {
                    assert!(
                        !path.exists(),
                        "sensitive tool ran before the permission response: {}",
                        path.display()
                    );
                }
                let request_id = message["id"].clone();
                let selected = |option_id: &str| {
                    json!({
                        "jsonrpc": "2.0",
                        "id": request_id.clone(),
                        "result": {
                            "outcome": {
                                "outcome": "selected",
                                "optionId": option_id
                            }
                        }
                    })
                };
                match script {
                    PermissionClientScript::Allow
                    | PermissionClientScript::AllowThenCancelRunning => {
                        write_client_message(&mut input, selected("allow-once")).await;
                    }
                    PermissionClientScript::Reject => {
                        write_client_message(&mut input, selected("reject-once")).await;
                    }
                    PermissionClientScript::WrongIdThenReject => {
                        write_client_message(
                            &mut input,
                            json!({
                                "jsonrpc": "2.0",
                                "id": "wrong-agent-request-id",
                                "result": {
                                    "outcome": {
                                        "outcome": "selected",
                                        "optionId": "allow-once"
                                    }
                                }
                            }),
                        )
                        .await;
                        write_client_message(&mut input, selected("reject-once")).await;
                    }
                    PermissionClientScript::CancelThenLateAllow => {
                        write_client_message(
                            &mut input,
                            json!({
                                "jsonrpc": "2.0",
                                "method": "session/cancel",
                                "params": { "sessionId": "sess_1" }
                            }),
                        )
                        .await;
                        write_client_message(
                            &mut input,
                            json!({
                                "jsonrpc": "2.0",
                                "id": request_id.clone(),
                                "result": { "outcome": { "outcome": "cancelled" } }
                            }),
                        )
                        .await;
                        write_client_message(&mut input, selected("allow-once")).await;
                    }
                }
            }

            let update_status = message
                .pointer("/params/update/status")
                .and_then(Value::as_str);
            if script == PermissionClientScript::AllowThenCancelRunning
                && update_status == Some("in_progress")
                && !sent_running_cancel
            {
                sent_running_cancel = true;
                tokio::time::sleep(Duration::from_millis(100)).await;
                write_client_message(
                    &mut input,
                    json!({
                        "jsonrpc": "2.0",
                        "id": 7,
                        "method": "session/cancel",
                        "params": { "sessionId": "sess_1" }
                    }),
                )
                .await;
            }
            if matches!(update_status, Some("completed" | "failed")) {
                break;
            }
        }
        seen
    }

    async fn execute_one_with_permission_client(
        config: &Config,
        registry: &ToolRegistry,
        call: PendingToolCall,
        script: PermissionClientScript,
        response_id_policy: JsonRpcResponseIdPolicy,
        must_not_exist_before_response: Option<PathBuf>,
    ) -> (ToolBatchOutcome, Vec<Value>, Option<Value>) {
        let (client_input, agent_input) = tokio::io::duplex(64 * 1024);
        let (agent_output, client_output) = tokio::io::duplex(64 * 1024);
        let client = tokio::spawn(drive_permission_client(
            client_output,
            client_input,
            script,
            must_not_exist_before_response,
        ));
        let mut reader = BufReader::new(agent_input).lines();
        let mut writer = agent_output;

        let outcome = execute_tool_calls_with_cancellation(
            AcpTurnContext {
                config,
                model: "test-model",
                session_id: "sess_1",
                tool_registry: registry,
                response_id_policy,
            },
            vec![call],
            &mut reader,
            &mut writer,
        )
        .await
        .expect("execute ACP tool batch");
        let late_response = if script == PermissionClientScript::CancelThenLateAllow {
            let line = tokio::time::timeout(Duration::from_secs(1), reader.next_line())
                .await
                .expect("late permission response arrived")
                .expect("read late permission response")
                .expect("late permission response line");
            Some(serde_json::from_str(&line).expect("late response json"))
        } else {
            None
        };
        drop(writer);
        let seen = client.await.expect("simulated ACP client joins");
        (outcome, seen, late_response)
    }

    #[tokio::test]
    async fn drive_prompt_streams_each_delta_as_a_chunk_then_completes() {
        let stream = ready_stream(vec![
            StreamEvent::ContentBlockDelta {
                index: 0,
                delta: Delta::ThinkingDelta {
                    thinking: "hmm".to_string(),
                },
            },
            text_delta("hello"),
            text_delta(" world"),
            StreamEvent::MessageDelta {
                delta: crate::models::MessageDelta {
                    stop_reason: None,
                    stop_sequence: None,
                },
                usage: Some(Usage {
                    input_tokens: 20,
                    output_tokens: 5,
                    ..Usage::default()
                }),
            },
            StreamEvent::MessageStop,
        ]);
        let mut reader = lines_from("");
        let mut out = Vec::new();

        let (outcome, tool_calls, usage) = drive_prompt_stream(
            stream,
            "sess_1",
            JsonRpcResponseIdPolicy::Preserve,
            &mut reader,
            &mut out,
        )
        .await
        .expect("driver ok");

        // Full text is accumulated for history (thinking is not)...
        assert_eq!(outcome, PromptOutcome::Completed("hello world".to_string()));
        assert!(tool_calls.is_empty());
        // ...the provider's usage report is returned for the turn total...
        let usage = usage.expect("usage captured");
        assert_eq!((usage.input_tokens, usage.output_tokens), (20, 5));
        // ...and each delta was emitted as its own session/update chunk,
        // thinking as `agent_thought_chunk`.
        let updates = parse_lines(out);
        assert_eq!(updates.len(), 3);
        assert!(updates.iter().all(|u| u["method"] == "session/update"));
        assert_eq!(
            updates[0]["params"]["update"]["sessionUpdate"],
            "agent_thought_chunk"
        );
        assert_eq!(updates[0]["params"]["update"]["content"]["text"], "hmm");
        assert_eq!(
            updates[1]["params"]["update"]["sessionUpdate"],
            "agent_message_chunk"
        );
        assert_eq!(updates[1]["params"]["update"]["content"]["text"], "hello");
        assert_eq!(updates[2]["params"]["update"]["content"]["text"], " world");
    }

    #[tokio::test]
    async fn drive_prompt_cancels_when_matching_cancel_arrives() {
        // A provider stream that never finishes within the test.
        let stream = pending_stream();
        let mut reader = lines_from(
            r#"{"jsonrpc":"2.0","method":"session/cancel","params":{"sessionId":"sess_1"}}"#,
        );
        let mut out = Vec::new();

        let (outcome, tool_calls, _usage) = drive_prompt_stream(
            stream,
            "sess_1",
            JsonRpcResponseIdPolicy::Preserve,
            &mut reader,
            &mut out,
        )
        .await
        .expect("driver ok");

        assert_eq!(outcome, PromptOutcome::Cancelled);
        assert!(tool_calls.is_empty());
        // Notification-form cancel (no id) is acknowledged by acting, not writing.
        assert!(out.is_empty());
    }

    #[tokio::test]
    async fn drive_prompt_ignores_cancel_for_a_different_session() {
        // The unrelated cancel line is buffered and ready; the delayed stop makes
        // it process first, proving it does not abort the turn.
        let stream = events_then_delayed_stop(vec![text_delta("kept")]);
        let mut reader = lines_from(
            r#"{"jsonrpc":"2.0","id":7,"method":"session/cancel","params":{"sessionId":"other"}}"#,
        );
        let mut out = Vec::new();

        let (outcome, _tool_calls, _usage) = drive_prompt_stream(
            stream,
            "sess_1",
            JsonRpcResponseIdPolicy::StringifyNumeric,
            &mut reader,
            &mut out,
        )
        .await
        .expect("driver ok");

        assert_eq!(outcome, PromptOutcome::Completed("kept".to_string()));
        // The other-session cancel carried an id, so it was acknowledged with null.
        let lines = parse_lines(out);
        assert!(
            lines
                .iter()
                .any(|v| v["id"] == "7" && v["result"] == Value::Null),
            "expected a null ack for the other-session cancel, got {lines:?}"
        );
    }

    #[tokio::test]
    async fn drive_prompt_rejects_a_concurrent_request_but_keeps_running() {
        let stream = events_then_delayed_stop(vec![text_delta("done")]);
        // A non-cancel request arrives mid-turn.
        let mut reader =
            lines_from(r#"{"jsonrpc":"2.0","id":9,"method":"session/new","params":{}}"#);
        let mut out = Vec::new();

        let (outcome, _tool_calls, _usage) = drive_prompt_stream(
            stream,
            "sess_1",
            JsonRpcResponseIdPolicy::StringifyNumeric,
            &mut reader,
            &mut out,
        )
        .await
        .expect("driver ok");

        assert_eq!(outcome, PromptOutcome::Completed("done".to_string()));
        let lines = parse_lines(out);
        assert!(
            lines
                .iter()
                .any(|v| v["id"] == "9" && v["error"]["code"] == -32603),
            "expected a prompt-in-progress error for the concurrent request, got {lines:?}"
        );
    }

    #[tokio::test]
    async fn drive_prompt_assembles_a_single_streamed_tool_call() {
        let mut events = tool_use_events(0, "call_1", "read_file", r#"{"path":"src/lib.rs"}"#);
        events.push(StreamEvent::MessageStop);
        let stream = ready_stream(events);
        let mut reader = lines_from("");
        let mut out = Vec::new();

        let (outcome, tool_calls, _usage) = drive_prompt_stream(
            stream,
            "sess_1",
            JsonRpcResponseIdPolicy::Preserve,
            &mut reader,
            &mut out,
        )
        .await
        .expect("driver ok");

        assert_eq!(outcome, PromptOutcome::Completed(String::new()));
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, "call_1");
        assert_eq!(tool_calls[0].name, "read_file");
        assert_eq!(tool_calls[0].input, json!({"path": "src/lib.rs"}));
        assert!(tool_calls[0].parse_error.is_none());
    }

    #[tokio::test]
    async fn drive_prompt_assembles_multiple_parallel_tool_calls_in_order() {
        let mut events = tool_use_events(0, "call_1", "read_file", r#"{"path":"a.rs"}"#);
        events.extend(tool_use_events(
            1,
            "call_2",
            "read_file",
            r#"{"path":"b.rs"}"#,
        ));
        events.push(StreamEvent::MessageStop);
        let stream = ready_stream(events);
        let mut reader = lines_from("");
        let mut out = Vec::new();

        let (_outcome, tool_calls, _usage) = drive_prompt_stream(
            stream,
            "sess_1",
            JsonRpcResponseIdPolicy::Preserve,
            &mut reader,
            &mut out,
        )
        .await
        .expect("driver ok");

        assert_eq!(tool_calls.len(), 2);
        assert_eq!(tool_calls[0].id, "call_1");
        assert_eq!(tool_calls[1].id, "call_2");
    }

    #[tokio::test]
    async fn drive_prompt_reports_malformed_tool_arguments_instead_of_dropping_the_call() {
        let mut events = tool_use_events(0, "call_1", "read_file", "{not json");
        events.push(StreamEvent::MessageStop);
        let stream = ready_stream(events);
        let mut reader = lines_from("");
        let mut out = Vec::new();

        let (_outcome, tool_calls, _usage) = drive_prompt_stream(
            stream,
            "sess_1",
            JsonRpcResponseIdPolicy::Preserve,
            &mut reader,
            &mut out,
        )
        .await
        .expect("driver ok");

        assert_eq!(tool_calls.len(), 1);
        assert!(tool_calls[0].parse_error.is_some());
    }

    #[tokio::test]
    async fn different_sessions_have_independent_history() {
        let mut server = AcpServer::new(
            Config::default(),
            "test-model".to_string(),
            PathBuf::from("/tmp"),
        );
        let result1 = server
            .new_session(json!({ "cwd": "/tmp" }))
            .await
            .expect("session 1");
        let result2 = server
            .new_session(json!({ "cwd": "/tmp" }))
            .await
            .expect("session 2");
        let sid1 = result1["sessionId"].as_str().unwrap().to_string();
        let sid2 = result2["sessionId"].as_str().unwrap().to_string();

        // Add messages to session 1
        {
            let session = server.sessions.get_mut(&sid1).unwrap();
            session.messages.push(Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: "hello".to_string(),
                    cache_control: None,
                }],
            });
        }

        // Session 2 should remain empty
        let session2 = server.sessions.get(&sid2).unwrap();
        assert!(session2.messages.is_empty());

        // Session 1 should have the message
        let session1 = server.sessions.get(&sid1).unwrap();
        assert_eq!(session1.messages.len(), 1);
    }

    #[tokio::test]
    async fn concurrent_sessions_each_get_their_own_tool_registry() {
        let mut server = AcpServer::new(
            Config {
                allow_shell: Some(true),
                ..Config::default()
            },
            "test-model".to_string(),
            PathBuf::from("/tmp"),
        );
        let s1 = server.new_session(json!({ "cwd": "/tmp" })).await.unwrap();
        let s2 = server.new_session(json!({ "cwd": "/tmp" })).await.unwrap();
        let id1 = s1["sessionId"].as_str().unwrap();
        let id2 = s2["sessionId"].as_str().unwrap();

        let reg1 = server.session_tool_registry(id1).expect("registry 1");
        let reg2 = server.session_tool_registry(id2).expect("registry 2");
        assert!(!Arc::ptr_eq(&reg1, &reg2));
        // Both sessions expose the same reusable tool surface: the
        // canonical `File` action tool (read/list/search/write/edit),
        // `Git`, the `apply_patch` back-compat alias, and `Bash` (#4625
        // consolidated the old per-action tool names).
        assert!(reg1.contains("File"));
        assert!(reg1.contains("Git"));
        assert!(reg1.contains("apply_patch"));
        assert!(reg1.contains("bash"));
        assert!(reg1.contains("Bash"));
        assert!(
            reg1.names()
                .into_iter()
                .all(|name| !name.starts_with("terminal/")),
            "ACP must not expose stateful terminal tools"
        );
        assert!(reg1.context().runtime.hook_executor.is_some());
    }

    /// The three shapes of the ACP `McpServer` enum. `stdio` is the untagged
    /// fallback, so "no `type`" must not be read as an error.
    #[test]
    fn acp_mcp_server_shapes_map_onto_the_config() {
        let (name, stdio) = parse_acp_mcp_server(&json!({
            "name": "voice",
            "command": "/usr/bin/voice-mcp",
            "args": ["--stdio"],
            "env": [{"name": "TOKEN", "value": "s3cret"}]
        }))
        .expect("stdio");
        assert_eq!(name, "voice");
        assert_eq!(stdio.command.as_deref(), Some("/usr/bin/voice-mcp"));
        assert_eq!(stdio.args, vec!["--stdio".to_string()]);
        assert_eq!(stdio.env.get("TOKEN").map(String::as_str), Some("s3cret"));
        assert!(stdio.url.is_none());

        let (_, http) = parse_acp_mcp_server(&json!({
            "type": "http",
            "name": "art",
            "url": "https://mcp.example/mcp",
            "headers": [{"name": "Authorization", "value": "Bearer t"}]
        }))
        .expect("http");
        assert_eq!(http.url.as_deref(), Some("https://mcp.example/mcp"));
        assert_eq!(
            http.headers.get("Authorization").map(String::as_str),
            Some("Bearer t")
        );
        // Streamable HTTP is the default; only sse is an explicit override.
        assert_eq!(http.transport, None);

        let (_, sse) = parse_acp_mcp_server(&json!({
            "type": "sse", "name": "old", "url": "https://mcp.example/sse"
        }))
        .expect("sse");
        assert_eq!(sse.transport.as_deref(), Some("sse"));
    }

    /// A malformed entry has to say which one and why: `session/new` is the
    /// only place the client can still act on it.
    #[test]
    fn acp_mcp_server_rejects_unusable_entries() {
        assert!(parse_acp_mcp_server(&json!({ "command": "/bin/x" })).is_err());
        assert!(parse_acp_mcp_server(&json!({ "name": "a" })).is_err());
        assert!(parse_acp_mcp_server(&json!({ "type": "http", "name": "a" })).is_err());
        let err = parse_acp_mcp_server(&json!({ "type": "carrier-pigeon", "name": "a" }))
            .expect_err("unknown transport");
        assert!(err.contains("carrier-pigeon"), "{err}");
    }

    /// The regression: a client that lends no terminal must still get an
    /// agent that can use its own shell. `clientCapabilities.terminal` is an
    /// offer from the client, not permission for the agent.
    #[tokio::test]
    async fn shell_tool_survives_a_client_that_declares_no_terminal_support() {
        let workspace = std::env::temp_dir();
        let config = Config {
            allow_shell: Some(true),
            ..Config::default()
        };
        let registry = build_acp_tool_registry(&config, &workspace, None).await;
        assert!(registry.contains("Bash"));
        assert!(
            registry
                .names()
                .into_iter()
                .all(|name| !name.starts_with("terminal/")),
            "the client-side terminal methods stay unexposed"
        );
        assert!(registry.contains("File"));
    }

    #[tokio::test]
    async fn shell_tool_omitted_without_headless_config_opt_in() {
        let workspace = std::env::temp_dir();
        let registry = build_acp_tool_registry(&Config::default(), &workspace, None).await;
        assert!(!registry.contains("Bash"));
        assert_eq!(registry.context().shell_policy, ShellPolicy::None);
        assert!(!registry.context().auto_approve);
    }

    #[tokio::test]
    async fn acp_shell_uses_configured_external_sandbox_or_fails_closed() {
        let workspace = std::env::temp_dir();
        let configured = Config {
            allow_shell: Some(true),
            sandbox_backend: Some("opensandbox".to_string()),
            sandbox_url: Some("http://127.0.0.1:8080".to_string()),
            ..Config::default()
        };
        let registry = build_acp_tool_registry(&configured, &workspace, None).await;
        assert!(registry.contains("bash"));
        assert!(registry.context().sandbox_backend.is_some());

        let unsupported = Config {
            allow_shell: Some(true),
            sandbox_backend: Some("unsupported-backend".to_string()),
            ..Config::default()
        };
        let registry = build_acp_tool_registry(&unsupported, &workspace, None).await;
        assert!(!registry.contains("bash"));
        assert!(!registry.contains("Bash"));
        assert!(registry.context().sandbox_backend.is_none());
    }

    #[tokio::test]
    async fn acp_tool_override_removes_every_builtin_compatibility_alias() {
        let mut overrides = std::collections::HashMap::new();
        overrides.insert("Bash".to_string(), crate::config::ToolOverride::Disabled);
        let config = Config {
            allow_shell: Some(true),
            tools: Some(crate::config::ToolsConfig {
                overrides: Some(overrides),
                ..crate::config::ToolsConfig::default()
            }),
            ..Config::default()
        };
        let registry = build_acp_tool_registry(&config, &std::env::temp_dir(), None).await;
        assert!(!registry.contains("bash"));
        assert!(!registry.contains("Bash"));
        assert!(
            registry
                .names()
                .into_iter()
                .all(|name| !name.starts_with("terminal/"))
        );
    }

    #[test]
    fn acp_prompt_uses_the_stable_headless_composer() {
        let workspace = tempfile::tempdir().expect("workspace");
        std::fs::write(
            workspace.path().join("AGENTS.md"),
            "# ACP project law\n\nKeep the acp-project-marker visible.",
        )
        .expect("write project instructions");
        let extra = workspace.path().join("maintainer-instructions.md");
        std::fs::write(&extra, "Keep the acp-config-marker visible.")
            .expect("write configured instructions");
        let config = Config {
            instructions: Some(vec![extra.to_string_lossy().into_owned()]),
            ..Config::default()
        };

        let prompt = build_acp_system_prompt(
            &config,
            workspace.path(),
            ApiProvider::Deepseek,
            "deepseek-v4-pro",
            None,
        );
        let text = crate::prompts::system_prompt_flat_text(&prompt);

        assert!(text.contains(crate::prompts::text::HEADLESS_BASE_PROMPT.trim()));
        assert!(text.contains("acp-project-marker"));
        assert!(text.contains("acp-config-marker"));
        assert!(!text.contains("You are a coding assistant inside an ACP-compatible editor."));
    }

    #[test]
    fn acp_system_prompt_is_byte_stable_after_round_one_agents_write() {
        let workspace = tempfile::tempdir().expect("workspace");
        let agents = workspace.path().join("AGENTS.md");
        std::fs::write(&agents, "round-one-authority").expect("write initial AGENTS");
        let config = Config::default();
        let slot = std::sync::Mutex::new(None);

        let round_one = frozen_acp_system_prompt(
            &slot,
            &config,
            workspace.path(),
            ApiProvider::Deepseek,
            "deepseek-v4-pro",
            None,
        );
        // Simulate a model tool changing project instructions during round 1.
        std::fs::write(&agents, "round-two-self-authored-authority")
            .expect("mutate AGENTS between rounds");
        let round_two = frozen_acp_system_prompt(
            &slot,
            &config,
            workspace.path(),
            ApiProvider::Deepseek,
            "deepseek-v4-pro",
            None,
        );

        assert_eq!(
            serde_json::to_vec(&round_one).unwrap(),
            serde_json::to_vec(&round_two).unwrap(),
            "later rounds must receive the byte-identical first-round system prompt"
        );
        let freshly_composed = build_acp_system_prompt(
            &config,
            workspace.path(),
            ApiProvider::Deepseek,
            "deepseek-v4-pro",
            None,
        );
        assert!(
            crate::prompts::system_prompt_flat_text(&freshly_composed)
                .contains("round-two-self-authored-authority"),
            "fixture must prove the mutable source really changed"
        );
        assert!(
            !crate::prompts::system_prompt_flat_text(&round_two)
                .contains("round-two-self-authored-authority")
        );
    }

    async fn workspace_registry() -> (tempfile::TempDir, ToolRegistry) {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = Config {
            allow_shell: Some(true),
            ..Config::default()
        };
        let registry = build_acp_tool_registry(&config, dir.path(), None).await;
        (dir, registry)
    }

    fn pending_call(name: &str, input: Value) -> PendingToolCall {
        PendingToolCall {
            id: "call_1".to_string(),
            name: name.to_string(),
            input,
            parse_error: None,
        }
    }

    fn config_with_policy_rule(rule: ghosty_execpolicy::ToolAskRule) -> Config {
        Config {
            exec_policy_engine: ghosty_execpolicy::ExecPolicyEngine::with_rulesets(vec![
                ghosty_execpolicy::Ruleset::user(vec![], vec![]).with_ask_rules(vec![rule]),
            ]),
            ..Config::default()
        }
    }

    fn tool_call_hook_command(payload: &Value) -> String {
        let payload = payload.to_string();
        if cfg!(windows) {
            format!("echo {payload}")
        } else {
            format!("printf '%s\\n' '{payload}'")
        }
    }

    fn config_with_tool_call_hook(mut config: Config, payload: Value, strict: bool) -> Config {
        let mut hook = crate::hooks::Hook::new(
            crate::hooks::HookEvent::ToolCallBefore,
            &tool_call_hook_command(&payload),
        );
        hook.continue_on_error = !strict;
        config.hooks = Some(crate::hooks::HooksConfig {
            enabled: true,
            hooks: vec![hook],
            ..crate::hooks::HooksConfig::default()
        });
        config
    }

    #[tokio::test]
    async fn acp_strict_tool_call_before_deny_blocks_before_execution() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = config_with_tool_call_hook(
            Config::default(),
            json!({"decision": "deny", "reason": "release gate"}),
            true,
        );
        let registry = build_acp_tool_registry(&config, dir.path(), None).await;
        let error = prepare_acp_tool_with_hooks(
            &config,
            "test-model",
            &registry,
            &pending_call("File", json!({"action": "read", "path": "safe.txt"})),
        )
        .await
        .expect_err("strict hook must deny");

        assert!(error.to_string().contains("release gate"));
    }

    #[tokio::test]
    async fn acp_hook_rewrite_is_reprepared_and_policy_is_re_evaluated() {
        let dir = tempfile::tempdir().expect("tempdir");
        let deny_rewritten_write = ghosty_execpolicy::ToolAskRule {
            action: ghosty_execpolicy::PermissionAction::Deny,
            ..ghosty_execpolicy::ToolAskRule::file_path("write_file", "rewritten.txt")
        };
        let config = config_with_tool_call_hook(
            config_with_policy_rule(deny_rewritten_write),
            json!({
                "updatedInput": {
                    "action": "write",
                    "path": "rewritten.txt",
                    "content": "rewritten by hook"
                }
            }),
            true,
        );
        let registry = build_acp_tool_registry(&config, dir.path(), None).await;
        let raw = pending_call("File", json!({"action": "read", "path": "safe.txt"}));
        let (_, raw_admission) = prepare_acp_tool_admission(&config, &registry, &raw).unwrap();
        assert_eq!(raw_admission, AcpToolAdmission::Auto);

        let prepared = prepare_acp_tool_with_hooks(&config, "test-model", &registry, &raw)
            .await
            .expect("hook rewrite prepares");
        assert_eq!(
            prepared.call.input.get("action").and_then(Value::as_str),
            Some("write")
        );
        assert!(matches!(prepared.admission, AcpToolAdmission::Block(_)));
        assert!(!dir.path().join("rewritten.txt").exists());
    }

    #[tokio::test]
    async fn acp_admission_is_input_specific_and_has_no_workspace_write_carve_out() {
        let (dir, registry) = workspace_registry().await;
        let config = Config::default();
        let read = pending_call("File", json!({"action": "read", "path": "src/lib.rs"}));
        let write = pending_call(
            "File",
            json!({"action": "write", "path": "src/lib.rs", "content": "new"}),
        );

        let (_, read_admission) = prepare_acp_tool_admission(&config, &registry, &read).unwrap();
        let (_, write_admission) = prepare_acp_tool_admission(&config, &registry, &write).unwrap();

        assert_eq!(read_admission, AcpToolAdmission::Auto);
        assert!(matches!(
            write_admission,
            AcpToolAdmission::RequestPermission(_)
        ));
        assert_eq!(registry.context().workspace, dir.path());
    }

    #[tokio::test]
    async fn acp_admission_folds_typed_rules_then_headless_safety_floor() {
        let (dir, registry) = workspace_registry().await;
        let workspace = dir.path().to_string_lossy().into_owned();
        let input = json!({"action": "write", "path": "allowed.txt", "content": "new"});
        let call = pending_call("File", input.clone());

        let allow = ghosty_execpolicy::ToolAskRule::file_path("write_file", "allowed.txt")
            .into_exact_workspace_allow(workspace.clone());
        let (_, admission) =
            prepare_acp_tool_admission(&config_with_policy_rule(allow), &registry, &call).unwrap();
        assert_eq!(admission, AcpToolAdmission::Auto);

        let ask = ghosty_execpolicy::ToolAskRule::file_path("write_file", "allowed.txt");
        let (_, admission) =
            prepare_acp_tool_admission(&config_with_policy_rule(ask), &registry, &call).unwrap();
        assert!(matches!(
            admission,
            AcpToolAdmission::RequestPermission(reason) if reason.contains("requires approval")
        ));

        let deny = ghosty_execpolicy::ToolAskRule {
            action: ghosty_execpolicy::PermissionAction::Deny,
            ..ghosty_execpolicy::ToolAskRule::file_path("write_file", "allowed.txt")
        };
        let (_, admission) =
            prepare_acp_tool_admission(&config_with_policy_rule(deny), &registry, &call).unwrap();
        assert!(matches!(admission, AcpToolAdmission::Block(_)));

        let command = "rm -rf ~/";
        let shell_allow = ghosty_execpolicy::ToolAskRule::exec_shell(command)
            .into_exact_workspace_allow(workspace);
        let shell_call = pending_call("Bash", json!({"command": command}));
        let (_, admission) = prepare_acp_tool_admission(
            &config_with_policy_rule(shell_allow),
            &registry,
            &shell_call,
        )
        .unwrap();
        assert!(matches!(
            admission,
            AcpToolAdmission::RequestPermission(reason)
                if reason.contains("Built-in safety gate")
        ));
    }

    #[tokio::test]
    async fn acp_admission_blocks_detached_and_stateful_bash_inputs() {
        let (_dir, registry) = workspace_registry().await;
        for input in [
            json!({"command": "sleep 30", "background": true}),
            json!({"command": "sleep 30", "tty": true}),
            json!({"command": "echo hi", "interactive": true}),
            json!({"command": "serve", "background": true, "persist": true}),
            json!({"command": "sleep 30 &"}),
            json!({"command": "nohup sleep 30"}),
            json!({"action": "wait", "task_id": "shell-1"}),
            json!({"action": "cancel", "task_id": "shell-1"}),
        ] {
            let call = pending_call("Bash", input);
            let (_, admission) =
                prepare_acp_tool_admission(&Config::default(), &registry, &call).unwrap();
            assert!(matches!(
                admission,
                AcpToolAdmission::Block(reason)
                    if reason.contains("foreground Bash runs only")
            ));
        }
        assert!(!acp_shell_command_requests_detach("echo '&' && echo done"));
    }

    #[tokio::test]
    async fn acp_admission_auto_review_and_repo_law_override_typed_allow() {
        let (dir, registry) = workspace_registry().await;
        let workspace = dir.path().to_string_lossy().into_owned();
        let command = "cargo test";
        let shell_allow = ghosty_execpolicy::ToolAskRule::exec_shell(command)
            .into_exact_workspace_allow(workspace.clone());
        let mut auto_review_block = config_with_policy_rule(shell_allow);
        auto_review_block.auto_review = Some(crate::config::AutoReviewConfig {
            block: vec![crate::config::AutoReviewRuleConfig {
                id: Some("acp-shell-block".to_string()),
                action_kind: Some("shell".to_string()),
                reason: Some("ACP shell is disabled by policy".to_string()),
                ..Default::default()
            }],
            ..Default::default()
        });
        let (_, admission) = prepare_acp_tool_admission(
            &auto_review_block,
            &registry,
            &pending_call("Bash", json!({"command": command})),
        )
        .unwrap();
        assert!(matches!(
            admission,
            AcpToolAdmission::Block(reason) if reason.contains("ACP shell is disabled by policy")
        ));

        let law_dir = dir.path().join(".ghosty");
        std::fs::create_dir_all(&law_dir).unwrap();
        std::fs::write(
            law_dir.join("constitution.json"),
            r#"{
                "protected_invariants": [
                    { "text": "Never rewrite the wire", "paths": ["wire.rs"], "action": "block" },
                    { "text": "Review release notes", "paths": ["CHANGELOG.md"] }
                ]
            }"#,
        )
        .unwrap();

        for (path, expected_block) in [("wire.rs", true), ("CHANGELOG.md", false)] {
            let allow = ghosty_execpolicy::ToolAskRule::file_path("write_file", path)
                .into_exact_workspace_allow(workspace.clone());
            let config = config_with_policy_rule(allow);
            let call = pending_call(
                "File",
                json!({"action": "write", "path": path, "content": "new"}),
            );
            let (_, admission) = prepare_acp_tool_admission(&config, &registry, &call).unwrap();
            if expected_block {
                assert!(matches!(
                    admission,
                    AcpToolAdmission::Block(reason) if reason.contains("Never rewrite the wire")
                ));
            } else {
                assert!(matches!(
                    admission,
                    AcpToolAdmission::RequestPermission(reason)
                        if reason.contains("Review release notes")
                ));
            }
        }
    }

    #[tokio::test]
    async fn acp_read_runs_without_permission_but_reports_pending_before_in_progress() {
        let (dir, registry) = workspace_registry().await;
        std::fs::write(dir.path().join("read.txt"), "safe").unwrap();
        let mut reader = lines_from("");
        let mut out = Vec::new();

        let outcome = execute_tool_calls_with_cancellation(
            AcpTurnContext {
                config: &Config::default(),
                model: "test-model",
                session_id: "sess_1",
                tool_registry: &registry,
                response_id_policy: JsonRpcResponseIdPolicy::Preserve,
            },
            vec![pending_call(
                "File",
                json!({"action": "read", "path": "read.txt"}),
            )],
            &mut reader,
            &mut out,
        )
        .await
        .unwrap();

        assert!(matches!(outcome, ToolBatchOutcome::Completed(_)));
        let messages = parse_lines(out);
        assert!(!messages.iter().any(|message| {
            message.get("method").and_then(Value::as_str) == Some("session/request_permission")
        }));
        let statuses = messages
            .iter()
            .filter_map(|message| {
                message
                    .pointer("/params/update/status")
                    .and_then(Value::as_str)
            })
            .collect::<Vec<_>>();
        assert_eq!(statuses, vec!["pending", "in_progress", "completed"]);
    }

    #[tokio::test]
    async fn acp_permission_allow_executes_write_once_after_response() {
        let (dir, registry) = workspace_registry().await;
        let target = dir.path().join("allowed.txt");
        let (outcome, messages, late) = execute_one_with_permission_client(
            &Config::default(),
            &registry,
            pending_call(
                "File",
                json!({"action": "write", "path": "allowed.txt", "content": "written once"}),
            ),
            PermissionClientScript::Allow,
            JsonRpcResponseIdPolicy::Preserve,
            Some(target.clone()),
        )
        .await;

        assert!(late.is_none());
        assert!(matches!(outcome, ToolBatchOutcome::Completed(_)));
        assert_eq!(std::fs::read_to_string(target).unwrap(), "written once");
        let request = messages
            .iter()
            .find(|message| message["method"] == "session/request_permission")
            .expect("permission request");
        assert_eq!(request["params"]["toolCall"]["status"], "pending");
        assert_eq!(request["params"]["options"][0]["kind"], "allow_once");
        assert_eq!(request["params"]["options"][1]["kind"], "reject_once");
        let statuses = messages
            .iter()
            .filter_map(|message| {
                message
                    .pointer("/params/update/status")
                    .and_then(Value::as_str)
            })
            .collect::<Vec<_>>();
        assert_eq!(statuses, vec!["pending", "in_progress", "completed"]);
    }

    #[tokio::test]
    async fn acp_permission_reject_and_wrong_id_fail_closed_without_write() {
        for script in [
            PermissionClientScript::Reject,
            PermissionClientScript::WrongIdThenReject,
        ] {
            let (dir, registry) = workspace_registry().await;
            let target = dir.path().join("denied.txt");
            let (outcome, messages, _) = execute_one_with_permission_client(
                &Config::default(),
                &registry,
                pending_call(
                    "File",
                    json!({"action": "write", "path": "denied.txt", "content": "forbidden"}),
                ),
                script,
                JsonRpcResponseIdPolicy::Preserve,
                Some(target.clone()),
            )
            .await;

            assert!(!target.exists(), "{script:?} must not authorize the write");
            let ToolBatchOutcome::Completed(results) = outcome else {
                panic!("rejection should complete with a failed tool result");
            };
            assert_eq!(results.len(), 1);
            let ContentBlock::ToolResult { is_error, .. } = &results[0].content[0] else {
                panic!("expected tool result");
            };
            assert_eq!(*is_error, Some(true));
            assert!(!messages.iter().any(|message| {
                message
                    .pointer("/params/update/status")
                    .and_then(Value::as_str)
                    == Some("in_progress")
            }));
        }
    }

    #[tokio::test]
    async fn acp_permission_cancel_ignores_late_allow_and_never_runs_tool() {
        let (dir, registry) = workspace_registry().await;
        let target = dir.path().join("cancelled.txt");
        let (outcome, messages, late) = execute_one_with_permission_client(
            &Config::default(),
            &registry,
            pending_call(
                "File",
                json!({"action": "write", "path": "cancelled.txt", "content": "forbidden"}),
            ),
            PermissionClientScript::CancelThenLateAllow,
            JsonRpcResponseIdPolicy::Preserve,
            Some(target.clone()),
        )
        .await;

        assert!(!target.exists());
        assert!(matches!(outcome, ToolBatchOutcome::Cancelled(_)));
        assert!(messages.iter().any(|message| {
            message
                .pointer("/params/update/status")
                .and_then(Value::as_str)
                == Some("failed")
        }));
        let late = late.expect("late allow remains queued for the outer dispatcher");
        assert!(is_jsonrpc_response(&late));
        assert_eq!(
            late.pointer("/result/outcome/optionId")
                .and_then(Value::as_str),
            Some("allow-once")
        );
    }

    #[tokio::test]
    async fn tool_registry_read_file_returns_real_contents() {
        let (dir, registry) = workspace_registry().await;
        std::fs::write(dir.path().join("hello.txt"), "hi there").unwrap();

        let result = registry
            .execute_full("File", json!({"action": "read", "path": "hello.txt"}))
            .await
            .expect("read_file succeeds");

        assert!(result.success);
        assert!(result.content.contains("hi there"));
    }

    #[tokio::test]
    async fn tool_registry_write_file_creates_a_real_file() {
        let (dir, registry) = workspace_registry().await;

        let result = registry
            .execute_full(
                "File",
                json!({"action": "write", "path": "created.txt", "content": "new content"}),
            )
            .await
            .expect("write_file succeeds");

        assert!(result.success);
        let on_disk = std::fs::read_to_string(dir.path().join("created.txt")).unwrap();
        assert_eq!(on_disk, "new content");
    }

    #[tokio::test]
    async fn tool_registry_list_dir_reports_real_directory_contents() {
        let (dir, registry) = workspace_registry().await;
        std::fs::write(dir.path().join("a.txt"), "a").unwrap();
        std::fs::write(dir.path().join("b.txt"), "b").unwrap();

        let result = registry
            .execute_full("File", json!({"action": "list", "path": "."}))
            .await
            .expect("list_dir succeeds");

        assert!(result.success);
        assert!(result.content.contains("a.txt"));
        assert!(result.content.contains("b.txt"));
    }

    #[tokio::test]
    async fn tool_registry_bash_runs_a_real_command() {
        let (_dir, registry) = workspace_registry().await;

        let result = registry
            .execute_full("Bash", json!({"command": "echo acp-terminal-check"}))
            .await
            .expect("Bash succeeds");

        assert!(result.content.contains("acp-terminal-check"));
    }

    #[tokio::test]
    async fn tool_registry_read_file_reports_failure_for_missing_path() {
        let (_dir, registry) = workspace_registry().await;

        let err = registry
            .execute_full(
                "File",
                json!({"action": "read", "path": "does-not-exist.txt"}),
            )
            .await
            .expect_err("missing file is a tool error");

        assert!(!err.to_string().is_empty());
    }

    /// Feeds [`run_agentic_prompt_turn`] a fixed sequence of canned per-round
    /// streams (no real provider), so the multi-round tool loop — including
    /// nested tool calls across several rounds — is exercised end-to-end
    /// against the real file-tool registry. A plain struct + inherent async
    /// method (rather than a boxed closure) lets each test's `|msgs| scripted.next()`
    /// closure return the async method's own anonymous future type directly,
    /// so `Fut` is inferred without needing a `dyn Future` trait object.
    struct ScriptedStreams(RefCell<VecDeque<StreamEventBox>>);

    impl ScriptedStreams {
        fn new(streams: Vec<StreamEventBox>) -> Self {
            Self(RefCell::new(VecDeque::from(streams)))
        }

        async fn next(&self) -> Result<StreamEventBox> {
            Ok(self
                .0
                .borrow_mut()
                .pop_front()
                .expect("test provided enough scripted rounds"))
        }
    }

    #[tokio::test]
    async fn agentic_turn_executes_a_tool_call_then_streams_the_final_answer() {
        let (dir, registry) = workspace_registry().await;
        std::fs::write(dir.path().join("VERSION"), "9.9.9").unwrap();

        let round1 = ready_stream({
            let mut events =
                tool_use_events(0, "call_1", "File", r#"{"action":"read","path":"VERSION"}"#);
            events.push(StreamEvent::MessageStop);
            events
        });
        let round2 = ready_stream(vec![
            text_delta("The version is 9.9.9"),
            StreamEvent::MessageStop,
        ]);

        let scripted = ScriptedStreams::new(vec![round1, round2]);
        let mut reader = lines_from("");
        let mut out = Vec::new();

        let (outcome, messages, _usage) = run_agentic_prompt_turn(
            AcpTurnContext {
                config: &Config::default(),
                model: "test-model",
                session_id: "sess_1",
                tool_registry: &registry,
                response_id_policy: JsonRpcResponseIdPolicy::Preserve,
            },
            vec![Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: "What version is this?".to_string(),
                    cache_control: None,
                }],
            }],
            &mut reader,
            &mut out,
            |_msgs| scripted.next(),
        )
        .await
        .expect("turn completes");

        assert_eq!(
            outcome,
            PromptOutcome::Completed("The version is 9.9.9".to_string())
        );
        // user -> assistant(tool_use) -> user(tool_result) -> assistant(text)
        assert_eq!(messages.len(), 4);
        assert!(matches!(
            messages[1].content[0],
            ContentBlock::ToolUse { .. }
        ));
        let ContentBlock::ToolResult {
            content, is_error, ..
        } = &messages[2].content[0]
        else {
            panic!("expected a tool_result message");
        };
        assert!(content.contains("9.9.9"));
        assert_eq!(*is_error, Some(false));

        // The client saw a tool_call start, a completed update, and the
        // streamed final-answer chunk.
        let lines = parse_lines(out);
        assert!(
            lines
                .iter()
                .any(|v| v["params"]["update"]["sessionUpdate"] == "tool_call")
        );
        assert!(lines.iter().any(
            |v| v["params"]["update"]["sessionUpdate"] == "tool_call_update"
                && v["params"]["update"]["status"] == "completed"
        ));
        assert!(lines.iter().any(|v| v["params"]["update"]["sessionUpdate"]
            == "agent_message_chunk"
            && v["params"]["update"]["content"]["text"] == "The version is 9.9.9"));
    }

    #[tokio::test]
    async fn agentic_turn_preserves_tool_receipts_when_later_provider_round_fails() {
        let (dir, registry) = workspace_registry().await;
        std::fs::write(dir.path().join("receipt.txt"), "observed").unwrap();
        let round1 = ready_stream({
            let mut events = tool_use_events(
                0,
                "call_receipt",
                "File",
                r#"{"action":"read","path":"receipt.txt"}"#,
            );
            events.push(StreamEvent::MessageStop);
            events
        });
        let scripted = ScriptedStreams::new(vec![round1, error_stream("provider unavailable")]);
        let mut reader = lines_from("");
        let mut out = Vec::new();

        let error = run_agentic_prompt_turn(
            AcpTurnContext {
                config: &Config::default(),
                model: "test-model",
                session_id: "sess_1",
                tool_registry: &registry,
                response_id_policy: JsonRpcResponseIdPolicy::Preserve,
            },
            vec![Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: "Read receipt.txt".to_string(),
                    cache_control: None,
                }],
            }],
            &mut reader,
            &mut out,
            |_msgs| scripted.next(),
        )
        .await
        .expect_err("second provider round fails");

        assert!(error.source.to_string().contains("provider unavailable"));
        let messages = error
            .partial_messages
            .expect("completed tool receipt must be returned for commit");
        assert_eq!(messages.len(), 3);
        assert!(matches!(
            &messages[2].content[0],
            ContentBlock::ToolResult { tool_use_id, .. } if tool_use_id == "call_receipt"
        ));
    }

    #[tokio::test]
    async fn agentic_turn_chains_nested_tool_calls_across_rounds() {
        let (dir, registry) = workspace_registry().await;
        std::fs::write(dir.path().join("a.txt"), "contents-of-a").unwrap();
        std::fs::write(dir.path().join("b.txt"), "contents-of-b").unwrap();

        let round1 = ready_stream({
            let mut events =
                tool_use_events(0, "call_1", "File", r#"{"action":"read","path":"a.txt"}"#);
            events.push(StreamEvent::MessageStop);
            events
        });
        // After seeing a.txt's contents, the model asks for b.txt too.
        let round2 = ready_stream({
            let mut events =
                tool_use_events(0, "call_2", "File", r#"{"action":"read","path":"b.txt"}"#);
            events.push(StreamEvent::MessageStop);
            events
        });
        let round3 = ready_stream(vec![
            text_delta("Both files read"),
            StreamEvent::MessageStop,
        ]);

        let scripted = ScriptedStreams::new(vec![round1, round2, round3]);
        let mut reader = lines_from("");
        let mut out = Vec::new();

        let (outcome, messages, _usage) = run_agentic_prompt_turn(
            AcpTurnContext {
                config: &Config::default(),
                model: "test-model",
                session_id: "sess_1",
                tool_registry: &registry,
                response_id_policy: JsonRpcResponseIdPolicy::Preserve,
            },
            vec![Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: "Read both files".to_string(),
                    cache_control: None,
                }],
            }],
            &mut reader,
            &mut out,
            |_msgs| scripted.next(),
        )
        .await
        .expect("turn completes");

        assert_eq!(
            outcome,
            PromptOutcome::Completed("Both files read".to_string())
        );
        // user, assistant(tool_use a), user(result a), assistant(tool_use b),
        // user(result b), assistant(text)
        assert_eq!(messages.len(), 6);
        let ContentBlock::ToolResult {
            content: a_content, ..
        } = &messages[2].content[0]
        else {
            panic!("expected tool_result for a.txt");
        };
        assert!(a_content.contains("contents-of-a"));
        let ContentBlock::ToolResult {
            content: b_content, ..
        } = &messages[4].content[0]
        else {
            panic!("expected tool_result for b.txt");
        };
        assert!(b_content.contains("contents-of-b"));
    }

    #[tokio::test]
    async fn agentic_turn_reports_a_tool_failure_back_to_the_model_and_keeps_going() {
        let (_dir, registry) = workspace_registry().await;

        let round1 = ready_stream({
            let mut events = tool_use_events(
                0,
                "call_1",
                "File",
                r#"{"action":"read","path":"missing.txt"}"#,
            );
            events.push(StreamEvent::MessageStop);
            events
        });
        let round2 = ready_stream(vec![
            text_delta("That file does not exist"),
            StreamEvent::MessageStop,
        ]);

        let scripted = ScriptedStreams::new(vec![round1, round2]);
        let mut reader = lines_from("");
        let mut out = Vec::new();

        let (outcome, messages, _usage) = run_agentic_prompt_turn(
            AcpTurnContext {
                config: &Config::default(),
                model: "test-model",
                session_id: "sess_1",
                tool_registry: &registry,
                response_id_policy: JsonRpcResponseIdPolicy::Preserve,
            },
            vec![Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: "Read missing.txt".to_string(),
                    cache_control: None,
                }],
            }],
            &mut reader,
            &mut out,
            |_msgs| scripted.next(),
        )
        .await
        .expect("turn completes even though the tool failed");

        assert_eq!(
            outcome,
            PromptOutcome::Completed("That file does not exist".to_string())
        );
        let ContentBlock::ToolResult { is_error, .. } = &messages[2].content[0] else {
            panic!("expected a tool_result message");
        };
        assert_eq!(*is_error, Some(true));

        let lines = parse_lines(out);
        assert!(lines.iter().any(
            |v| v["params"]["update"]["sessionUpdate"] == "tool_call_update"
                && v["params"]["update"]["status"] == "failed"
        ));
    }

    #[cfg(windows)]
    const SLOW_SHELL_COMMAND: &str = "ping -n 6 127.0.0.1 >NUL";
    #[cfg(not(windows))]
    const SLOW_SHELL_COMMAND: &str = "sleep 5";

    #[tokio::test]
    async fn tool_batch_cancels_a_running_bash_and_applies_response_id_policy() {
        let (_dir, registry) = workspace_registry().await;
        let started = std::time::Instant::now();

        let (outcome, messages, _) = execute_one_with_permission_client(
            &Config::default(),
            &registry,
            pending_call("Bash", json!({ "command": SLOW_SHELL_COMMAND })),
            PermissionClientScript::AllowThenCancelRunning,
            JsonRpcResponseIdPolicy::StringifyNumeric,
            None,
        )
        .await;

        assert!(matches!(outcome, ToolBatchOutcome::Cancelled(_)));
        assert!(
            started.elapsed() < std::time::Duration::from_secs(4),
            "Bash cancellation must preempt the five-second command"
        );
        assert!(
            messages
                .iter()
                .any(|value| value["id"] == "7" && value["result"].is_null()),
            "Zed-compatible cancellation response id was not stringified: {messages:?}"
        );
    }

    /// The process working directory is shared by every connection this
    /// process serves, so an ACP turn may not move it. A `ScopedCurrentDir`
    /// guard used to `set_current_dir` per prompt round; with two concurrent
    /// turns its `Drop` restored the wrong directory and left the process
    /// pointed at a foreign workspace permanently. This pins the contract:
    /// building registries and system prompts for two different workspaces
    /// leaves the process directory untouched.
    #[tokio::test]
    async fn acp_session_setup_never_moves_the_process_working_directory() {
        let before = std::env::current_dir().expect("cwd before");

        let (dir1, _registry1) = workspace_registry().await;
        let (dir2, _registry2) = workspace_registry().await;
        assert_ne!(dir1.path(), dir2.path());

        for workspace in [dir1.path(), dir2.path()] {
            let _ = build_acp_system_prompt(
                &Config::default(),
                workspace,
                ApiProvider::Deepseek,
                "deepseek-v4-pro",
                None,
            );
        }

        assert_eq!(
            std::env::current_dir().expect("cwd after"),
            before,
            "an ACP session must resolve paths against its own cwd, never by moving the process"
        );
    }

    #[tokio::test]
    async fn concurrent_acp_sessions_execute_tools_independently() {
        let (dir1, registry1) = workspace_registry().await;
        let (dir2, registry2) = workspace_registry().await;
        std::fs::write(dir1.path().join("f.txt"), "session-one").unwrap();
        std::fs::write(dir2.path().join("f.txt"), "session-two").unwrap();

        let (result1, result2) = tokio::join!(
            registry1.execute_full("File", json!({"action": "read", "path": "f.txt"})),
            registry2.execute_full("File", json!({"action": "read", "path": "f.txt"})),
        );

        assert!(
            result1
                .expect("session 1 read")
                .content
                .contains("session-one")
        );
        assert!(
            result2
                .expect("session 2 read")
                .content
                .contains("session-two")
        );
    }
}
