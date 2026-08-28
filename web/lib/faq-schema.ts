import { flattenExtractedText } from "./react-text";

export interface FaqSchemaItem {
  q: string;
  a: React.ReactNode;
}

/**
 * FAQPage structured data from the same Q&A pairs the FAQ page renders.
 * `url` must be the canonical FAQ URL (en or zh), not a partial-locale fallback.
 */
export function buildFaqPageJsonLd({
  items,
  url,
  inLanguage,
}: {
  items: readonly FaqSchemaItem[];
  url: string;
  inLanguage: string;
}) {
  return {
    "@context": "https://schema.org",
    "@type": "FAQPage",
    url,
    inLanguage,
    mainEntity: items.map((item) => ({
      "@type": "Question",
      name: item.q,
      acceptedAnswer: {
        "@type": "Answer",
        text: flattenExtractedText(item.a),
      },
    })),
  };
}
