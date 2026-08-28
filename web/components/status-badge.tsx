/**
 * <StatusBadge> — honest availability labels for public surfaces.
 *
 * Used wherever the site shows something that is not a shipped, stable
 * feature: experimental bridges, preview platforms, pending media, and
 * unavailable states. The label is always visible text — never color alone —
 * so the state is conveyed accessibly in both locales.
 */

import type { LocalizedText } from "@/lib/content/vocabulary";
import { pickText } from "@/lib/i18n/dictionaries";

export type StatusKind = "experimental" | "preview" | "pending" | "unavailable";

const DEFAULT_LABELS: Record<StatusKind, LocalizedText> = {
  experimental: { en: "Experimental", zh: "实验性" },
  preview: { en: "Preview", zh: "预览" },
  pending: { en: "Pending", zh: "待就绪" },
  unavailable: { en: "Unavailable", zh: "暂不可用" },
};

export function StatusBadge({
  kind,
  locale = "en",
  label,
}: {
  kind: StatusKind;
  locale?: string;
  /** Overrides the default per-kind label (e.g. a media entry's pendingLabel). */
  label?: LocalizedText;
}) {
  const text = pickText(label ?? DEFAULT_LABELS[kind], locale);

  return (
    <span className={`status-badge status-badge-${kind}`}>
      <span className="status-badge-dot" aria-hidden="true" />
      {text}
    </span>
  );
}
