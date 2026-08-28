import type { DocsHooksDict } from "../types";

/**
 * English reference dictionary for `app/[locale]/docs/hooks/page.tsx`.
 * Copy moved verbatim from the page's `isZh` ternaries — any wording change
 * belongs in its own commit, never mixed into a structural move.
 */
export const docsHooks: DocsHooksDict = {
  metaTitle: "Hooks · Ghosty Docs",
  metaDescription:
    "The shipped lifecycle hooks: mutable message_submit, tool_call_before decisions, turn_end, and sub-agent observer events.",
  bodyClassName: "text-ink-soft leading-relaxed",
  overviewTitle: "Hooks",
  overviewLead:
    "Hooks attach your own commands to Ghosty's lifecycle: inject context before a message is submitted, enforce policy before a tool call, and audit turns or sub-agent activity. This page describes what currently ships; docs/rfcs/1364-hooks-lifecycle.md is the design RFC for this surface, and docs/CONFIGURATION.md carries the full configuration schema.",
  configIntro:
    "Hooks are configured under {hooksTable} entries in config.toml; run {hooksCommand} in the TUI to see every configured hook grouped by event — name, command preview, timeout, and condition — plus the global {enabledKey} state.",
  events: [
    [
      "message_submit (mutable)",
      "Runs before a submitted message is added to history or sent to the model. The hook receives JSON on stdin; exit 0 with stdout JSON carrying a non-empty text field replaces the submitted text, and exit 2 blocks the submission before the turn starts. Multiple hooks run serially in config order, each receiving the previous hook's output. Hooks marked background = true are observer-only and cannot transform or block.",
    ],
    [
      "tool_call_before (decision)",
      "Runs before each tool call executes. Beyond the exit-2 hard deny (which always wins), a foreground hook may print a JSON decision on stdout with exit 0: allow / deny / ask, plus updatedInput to rewrite the tool input and additionalContext appended to the tool result the model sees. When several hooks match, precedence is deny > ask > allow; tool_name conditions support * globs (mcp__* matches every MCP tool). Full Access does not open tool-approval prompts, so ask does not downgrade that posture.",
    ],
    [
      "turn_end (observer)",
      "Fires after each model turn ends, once usage, cost, notifications, receipts, and queue-recovery state have settled. The stdin JSON carries fields such as status, duration_ms, usage, totals, and queued_message_count. Stdout is ignored and failures are warn-only — the hook cannot block input, mutate the transcript, or change the next queued follow-up.",
    ],
    [
      "subagent_spawn / subagent_complete (observer)",
      "Observe sub-agent start and completion with bounded JSON metadata on stdin (agent_id, status, truncated prompt/result previews). Failures are warn-only and never block scheduling or change prompts or results; use the transcript handle returned by agent when full detail is needed.",
    ],
  ],
  projectTitle: "Project-local hooks",
  projectLead:
    'Repositories can ship policy in <workspace>/.ghosty/hooks.toml. Because project hooks are executable shell configuration, Ghosty loads them only after the workspace is trusted through the trust prompt or a trust_level = "trusted" entry in user-owned config — session /trust on and legacy .deepseek/trusted markers do not enable project hooks by themselves. Once trusted, project hooks are appended after the global hooks from config.toml, so they run last and win updatedInput ties. A malformed trusted project file logs a warning and startup falls back to global hooks only.',
  sourceNote:
    "Source documents: docs/rfcs/1364-hooks-lifecycle.md (design RFC), docs/CONFIGURATION.md (configuration schema) · Update docs-map.ts when changing.",
};
