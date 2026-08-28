//! The first-run notice copy.
//!
//! One string, owned by the crate that owns what is collected, so the TUI and
//! the CLI cannot drift into describing two different products. Every claim
//! below is checked against [`crate::event`] by a test: if the schema grows a
//! field this text does not cover, that test fails.
//!
//! Two properties of the wording are deliberate and load-bearing:
//!
//! 1. **The default is stated plainly and the opt-out is immediate.** The
//!    native TUI starts on the yes choice and makes the opt-out equally
//!    reachable.
//! 2. **The red lines are stated as "not collected", not as "anonymized".**
//!    Sampling and hashing are not the same promise, and a notice that implies
//!    them when neither is true is worse than no notice.

/// Headline shown above [`NOTICE_BODY`].
pub const NOTICE_HEADLINE: &str = "Help improve Ghosty?";

/// The notice itself.
///
/// Wrapped at 72 columns so it renders unchanged in the native responsive
/// modal and remains readable in an 80-column terminal.
pub const NOTICE_BODY: &str = "\
Ghosty counts: which version you run, OS and CPU family, session
duration and outcome, and aggregate feature and error counters.

It never collects your conversations, code, prompts, files, repo or
branch names, model content, or credentials — and it never sends a
per-turn or per-tool timeline of agent activity.

You are identified only by a random ID stored on this machine, replaced
every 90 days. Change your mind any time:
                              ghosty config set telemetry false

Full schema, field by field:  docs/TELEMETRY.md";
