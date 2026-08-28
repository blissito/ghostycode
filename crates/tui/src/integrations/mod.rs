//! Third-party harness integrations owned by Ghosty.
//!
//! Every integration here is an adapter around another tool's *documented*
//! seams. Ghosty keeps ownership of provider/model selection, permissions,
//! credentials, and lifecycle authority; the integrated surface never becomes
//! a second scheduler or an authority bypass.

pub(crate) mod cli;
pub(crate) mod dsh;
