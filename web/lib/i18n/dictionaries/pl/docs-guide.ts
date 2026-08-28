import type { DocsGuideDict } from "../types";

/**
 * Polish dictionary for the docs "Getting started" page. Latin script —
 * the reference body typography is kept.
 */
export const docsGuide: DocsGuideDict = {
  metaTitle: "Pierwsze kroki · Dokumentacja Ghosty",
  metaDescription:
    "Pełna droga od instalacji do idealnej Floty: instalacja, pierwsza sesja bez kluczy, podpięcie providera i konfiguracja Floty.",
  bodyClassName: "text-ink-soft leading-relaxed",
  overviewTitle: "Pierwsze kroki",
  overviewLead:
    "Cztery kroki od jednej komendy instalacji do Floty gotowej do twojej pracy.",
  sessionTitle: "Zobacz prawdziwą sesję",
  sessionLead:
    "Tu pojawi się nagranie prawdziwej sesji. Nagrania jeszcze nie ma, więc nic nie jest wyświetlane.",
  nextTitle: "Co dalej",
  sourceNote:
    "Dokumenty źródłowe: docs/GUIDE.md, docs/KEYBINDINGS.md · Treść kroków żyje w web/lib/content/getting-started.ts; przy zmianie zaktualizuj docs-map.ts.",
};
