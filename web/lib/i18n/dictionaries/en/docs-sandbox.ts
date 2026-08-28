import type { DocsSandboxDict } from "../types";

/**
 * English reference dictionary for `app/[locale]/docs/sandbox/page.tsx`.
 * Copy moved verbatim from the page's `isZh` ternaries — any wording change
 * belongs in its own commit, never mixed into a structural move.
 */
export const docsSandbox: DocsSandboxDict = {
  metaTitle: "Sandbox & Approval · Ghosty Docs",
  metaDescription:
    "The honest boundary: macOS Seatbelt, opt-in Linux bubblewrap, platform gaps, and approval policy.",
  bodyClassName: "text-ink-soft leading-relaxed",
  overviewTitle: "Sandbox & Approval",
  overviewLead:
    "Ghosty can launch shell commands proposed by a model. Approval policy, workspace-aware tools, and an operating-system command wrapper are separate controls: an approval is not a sandbox, and selecting workspace-write does not prove the current platform has an OS wrapper available. This page describes only behavior wired into the command execution path.",
  platforms: [
    [
      "macOS · Seatbelt",
      "Ghosty probes /usr/bin/sandbox-exec; when the probe succeeds and the policy requests a sandbox, the child command is wrapped in a generated Seatbelt profile: broad filesystem reads, policy-limited writes, and network only when the policy enables it. A failed probe is reported honestly as no OS sandbox.",
    ],
    [
      "Linux · opt-in bubblewrap",
      "Linux command sandboxing is opt-in: set prefer_bwrap = true and keep /usr/bin/bwrap executable. The child gets a read-only root view with writable mounts derived from the resolved policy; the network namespace is isolated by default and --share-net is added only when the policy enables network access. Without the opt-in, Ghosty reports none.",
    ],
    [
      "Windows · no OS sandbox",
      "The Windows command path currently reports no OS sandbox. Host permissions and approval policy still apply, but they are not a Ghosty OS command sandbox.",
    ],
    [
      "External OpenSandbox execution",
      'With sandbox_backend = "opensandbox", shell execution is sent to the configured OpenSandbox-compatible HTTP endpoint instead of starting a local child. Isolation guarantees belong to the configured service and its operator.',
    ],
  ],
  policiesTitle: "Policies and fallbacks",
  policiesLead:
    "The local {sandboxMode} values are {readOnly}, {workspaceWrite}, {dangerFullAccess}, and {externalSandbox}. The first two are enforced by Seatbelt or bubblewrap only when that wrapper is selected and available; {dangerFullAccess} deliberately bypasses the local OS wrapper; {externalSandbox} declares that execution is already externally isolated. When no wrapper is selected, the shell command runs without Ghosty OS isolation — approval rules and workspace-aware native file tools remain separate controls.",
  diagnosticsTitle: "Diagnostics and limits",
  diagnosticsLead:
    "ghosty setup --status, ghosty doctor, ghosty doctor --json, and the diagnostics tool report the locally available wrapper after applying the resolved bubblewrap preference. Denial attribution is intentionally conservative: a child command's generic Permission denied is not by itself proof that Ghosty's sandbox blocked it, and unsandboxed command failures are never labeled sandbox denials.",
  diagnosticsLimits:
    "The limitations are stated just as plainly: availability is checked before launch, yet the selected wrapper can still fail because of host policy, container restrictions, or a race after the probe; bubblewrap ignores a configured writable root that is missing or not a directory; and no sandbox protects against kernel vulnerabilities or all resource-exhaustion and side-channel attacks.",
  sourceNote: "Source document: docs/SANDBOX.md · Update docs-map.ts when changing.",
};
