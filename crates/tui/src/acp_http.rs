//! ACP over the RFD network transports: WebSocket and Streamable HTTP.
//!
//! This module owns *policy* only. The transport itself is
//! [`agent_client_protocol_http::AcpHttpServer`], the reference implementation
//! that ships with the official ACP SDK — the same one goose adopted after
//! deleting the transport it had written by hand. Framing, connection identity,
//! the SSE fan-out, and `$/cancel_request` propagation all live there, so the
//! bugs that transport code keeps re-learning (messages emitted before a client
//! subscribes, deadlocking a write behind its own response, JSON-RPC ids that
//! collide because `1` and `"1"` hash the same) are paid for once, upstream.
//!
//! ## How our adapter reaches that transport
//!
//! [`crate::acp_server`] speaks newline-delimited JSON-RPC over a byte pair,
//! and the SDK's [`ByteStreams`] transport speaks newline-delimited JSON-RPC
//! over a byte pair. So the two are joined with an in-memory duplex rather than
//! by rewriting the adapter against the SDK's typed handler API:
//!
//! ```text
//!   socket ── AcpHttpServer ── ByteStreams ─┐
//!                                           ├─ tokio::io::duplex
//!   run_acp_server_over (our dispatch) ─────┘
//! ```
//!
//! One connection is one duplex is one task running our existing loop, which is
//! why concurrent connections need nothing new: each carries its own
//! `AcpServer` and its own sessions. What this shape deliberately does *not*
//! buy is two prompts in flight inside a single connection — our loop
//! serializes a turn against its reader. One client is one connection, so that
//! limit is not felt; lifting it means moving the dispatch onto the SDK's
//! handler builder, which is a separate decision.

use std::path::PathBuf;
use std::sync::Arc;

use agent_client_protocol::ByteStreams;
use agent_client_protocol_http::{AcpHttpServer, CorsOptions, ServerOptions};
use axum::Router;
use tokio::io::BufReader;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use crate::config::Config;

/// Buffer for the in-memory pipe joining the SDK transport to our dispatch.
///
/// It only has to absorb the gap between one JSON-RPC message being written
/// and the peer task reading it; a tool result carrying a large file preview is
/// the biggest realistic frame.
const DUPLEX_BUFFER_BYTES: usize = 64 * 1024;

/// Everything an accepted connection needs to start a fresh ACP session tree.
///
/// Cloned per connection: the config is a snapshot, never shared mutable state,
/// because two connections may resolve different routes and must not see each
/// other's edits mid-turn.
#[derive(Clone)]
pub struct AcpHttpFactory {
    config: Config,
    model: String,
    default_cwd: PathBuf,
}

impl AcpHttpFactory {
    #[must_use]
    pub fn new(config: Config, model: String, default_cwd: PathBuf) -> Self {
        Self {
            config,
            model,
            default_cwd,
        }
    }

    /// Build the `axum` router serving ACP at `path`.
    ///
    /// `health_endpoint` is forced off: the crate would register its own
    /// `GET /health`, and the runtime API already owns that route — `axum`
    /// panics when a merge finds the path twice, so leaving this on turns a
    /// configuration detail into a failure to boot.
    ///
    /// `allowed_origins` is the caller's ACP allow-list, handed to the crate as
    /// well as enforced in [`crate::runtime_api`]'s own middleware. Both layers
    /// are needed and neither is redundant:
    ///
    /// * `CorsOptions::Disabled` rejects **every** request carrying an `Origin`
    ///   (its own tests assert `!disabled().allows_origin(Some(_))`), so
    ///   leaving it off meant no browser could ever connect, with or without a
    ///   valid token — the exact client this transport exists for.
    /// * The crate's allow-list, on the other hand, treats a *missing* `Origin`
    ///   as permitted, which is the cross-site WebSocket hole our middleware
    ///   closes by demanding a header credential there.
    ///
    /// So the crate screens listed origins and we screen the rest. With no
    /// operator origins configured the old `Disabled` behaviour stands.
    ///
    /// `health_endpoint` is forced off: the crate would register its own
    /// `GET /health`, and the runtime API already owns that route — `axum`
    /// panics when a merge finds the path twice, so leaving this on turns a
    /// configuration detail into a failure to boot.
    pub fn into_router(self, path: &str, allowed_origins: &[String]) -> Router {
        let cors = if allowed_origins.is_empty() {
            CorsOptions::Disabled
        } else {
            CorsOptions::allow_origins(allowed_origins).unwrap_or_else(|error| {
                tracing::warn!(
                    "ignoring unparsable ACP origin allow-list ({error}); \
                     browser clients will be refused by the transport"
                );
                CorsOptions::Disabled
            })
        };
        AcpHttpServer::new(move || self.clone().connect())
            .with_options(ServerOptions {
                path: path.to_string(),
                cors,
                health_endpoint: false,
            })
            .into_router()
    }

    /// Spawn one ACP connection and hand the SDK the far end of its pipe.
    fn connect(self) -> ByteStreams<impl futures_util::AsyncWrite, impl futures_util::AsyncRead> {
        let (ours, theirs) = tokio::io::duplex(DUPLEX_BUFFER_BYTES);
        let (ours_read, ours_write) = tokio::io::split(ours);
        let (theirs_read, theirs_write) = tokio::io::split(theirs);

        tokio::spawn(async move {
            // A connection ending is ordinary: the peer closed the socket, or
            // the idle reaper cut it. Log and let the task retire; the duplex
            // halves drop with it and the session's tools are released.
            if let Err(error) = crate::acp_server::run_acp_server_over(
                self.config,
                self.model,
                self.default_cwd,
                BufReader::new(ours_read),
                ours_write,
            )
            .await
            {
                tracing::debug!(target: "acp", "ACP connection ended: {error:#}");
            }
        });

        ByteStreams::new(theirs_write.compat_write(), theirs_read.compat())
    }
}

/// Shared handle stored on the runtime API state.
pub type SharedAcpHttpFactory = Arc<AcpHttpFactory>;
