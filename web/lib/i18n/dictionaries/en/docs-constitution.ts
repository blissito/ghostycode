import type { DocsConstitutionDict } from "../types";

/**
 * English reference dictionary for `app/[locale]/docs/constitution/page.tsx`.
 * Copy moved verbatim from the page's `isZh` ternaries — any wording change
 * belongs in its own commit, never mixed into a structural move.
 */
export const docsConstitution: DocsConstitutionDict = {
  metaTitle: "Constitution and /constitution · Ghosty Docs",
  metaDescription:
    "User-global constitution, repo-local law, project instructions, and runtime boundaries.",
  bodyClassName: "text-ink-soft leading-relaxed",
  overviewTitle: "Constitution and /constitution",
  overviewTitleAside: "宪章与 /constitution",
  overviewLead:
    "Ghosty gives the agent an accountable address, then a legal system for context conflicts. {constitutionCommand} is the primary personal constitution surface: guided setup stores structured user-global data in {homeConfig} and renders it as model-facing prose. Repos can still add local law via {repoConfig}; runtime policy separately encodes modes, approval, sandbox, cost, and tool boundaries.",
  principles: [
    [
      "userGlobal",
      "Use /constitution for standing personal law across projects. It is structured data rendered to prose, not a raw prompt editor.",
    ],
    [
      "repoLocal",
      ".ghosty/constitution.json is optional project policy for protected invariants, branch rules, verification, and escalation.",
    ],
    [
      "runtime",
      "Constitution text may express preferences, but approval, sandbox, shell, network, trust, and MCP permissions remain enforced config.",
    ],
  ],
  authorityNote:
    "Standard project instructions still live in AGENTS.md; memory and handoffs rank below constitutions and project instructions; the full base-prompt Markdown override is an expert escape hatch, not the normal setup path. See {configDocs}.",
  configDocsLabel: "configuration docs",
  sourceNote: "Source document: docs/ARCHITECTURE.md · Update docs-map.ts when changing.",
};
