//! Federated plugin marketplace catalog parsing (#5311).
//!
//! Parses real published catalog schemas — Kimi, Claude, Codex, and the
//! native Ghosty format — into one normalized candidate model with
//! per-entry fault isolation, provenance that never grants trust, and
//! install plans that say honestly whether Ghosty can fetch a source.
//!
//! This layer is parser-only: no network, no filesystem reads, no process
//! execution. Every fetch happens through the existing reviewed installer
//! when an operator explicitly installs a candidate.

pub mod parsers;
pub mod store;
#[cfg(test)]
mod tests;
pub mod types;
