import type { DocsConfigurationDict } from "../types";

export const docsConfiguration: DocsConfigurationDict = {
  metaTitle: "Configuration · Ghosty Docs",
  metaDescription:
    "Where config.toml is read from, the per-project overlay, credential precedence, and legacy path migration.",
  bodyClassName: "text-ink-soft leading-relaxed",
  overviewTitle: "Configuration",
  overviewLead:
    "Ghosty reads its configuration from ~/.ghosty/config.toml (the legacy ~/.deepseek/config.toml is still read as a fallback). The --config flag and the GHOSTY_CONFIG_PATH environment variable can point elsewhere; --config wins when both are set, and environment variable overrides are applied after the file is loaded.",
  auditLead:
    "Inside the TUI, {auditCommand} shows which documented keys can change in the current session, which can also be persisted, and which stay file-only or restart-only — treat its “Command / reason” column as the source of truth before editing by hand.",
  overlayTitle: "Per-project overlay",
  overlayLead:
    "When a workspace contains a regular-file <workspace>/.ghosty/config.toml, the safe values it declares are merged on top of the global config (legacy <workspace>/.deepseek/config.toml files are still read when the Ghosty path is absent; symlinked project configs are rejected). This lets a repository suggest a model or tighten the local safety posture without touching the user's global config. Pass --no-project-config to skip the overlay for one launch.",
  overlayLimits:
    "The overlay is intentionally narrow: it supports model, reasoning_effort, approval_policy and sandbox_mode (tightening values only), notes_path, max_subagents (clamped to 1..=20), and allow_shell (false applies, true is ignored). Credentials, endpoints, provider selection, MCP config, hooks, skills, and instructions = [...] stay user-global — a repo-local config.toml that declares api_key, base_url, or provider is ignored, so a cloned repository cannot pick arbitrary local files into the prompt.",
  credentialsTitle: "Credential lookup",
  credentialsLead:
    "After any explicit {apiKey}, credentials resolve in config → keyring → env order. {authStatus} inspects the active provider's config file, OS keyring backend, environment variable, winning source, and last-four label without printing the key itself. Hosted, generic OpenAI-compatible, self-hosted, or native Anthropic routes are selected with {providerConfig} or {providerFlag}; the full registry lives on the Models & providers page and in docs/PROVIDERS.md.",
  legacyTitle: "Legacy .deepseek/ paths",
  legacyLead:
    "Ghosty was renamed from DeepSeek-TUI. To avoid breaking existing installs, the runtime reads state from the new ~/.ghosty/ location but falls back to ~/.deepseek/ when only the legacy directory exists, and always writes to ~/.ghosty/ — read-with-fallback, write-to-new. State-dir resolution is consolidated in resolve_state_dir / ensure_state_dir in crates/config/src/lib.rs, and every legacy path reference carries an audited keep decision.",
  sourceNote:
    "Source documents: docs/CONFIGURATION.md, docs/LEGACY_PATHS.md · Update docs-map.ts when changing.",
};
