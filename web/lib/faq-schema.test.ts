import { createElement } from "react";
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { buildFaqPageJsonLd } from "./faq-schema";
import { serializeJsonLd } from "./json-ld";
import { extractText, flattenExtractedText } from "./react-text";
import { SITE_URL } from "./page-meta";

const faqPage = readFileSync(new URL("../app/[locale]/faq/page.tsx", import.meta.url), "utf8");

describe("FAQPage structured data", () => {
  it("maps every question to an acceptedAnswer with flattened text", () => {
    const items = [
      {
        q: "What is Ghosty?",
        a: createElement(
          "p",
          null,
          "A terminal-native agent. Run ",
          createElement("code", null, "ghosty"),
          ".",
        ),
      },
      {
        q: "How do I install?",
        a: ["npm install -g ghosty", createElement("span", null, " or Cargo.")],
      },
    ];
    const schema = buildFaqPageJsonLd({
      items,
      url: `${SITE_URL}/en/faq`,
      inLanguage: "en",
    });

    expect(schema).toEqual({
      "@context": "https://schema.org",
      "@type": "FAQPage",
      url: "https://ghosty.net/en/faq",
      inLanguage: "en",
      mainEntity: [
        {
          "@type": "Question",
          name: "What is Ghosty?",
          acceptedAnswer: {
            "@type": "Answer",
            text: "A terminal-native agent. Run ghosty .",
          },
        },
        {
          "@type": "Question",
          name: "How do I install?",
          acceptedAnswer: {
            "@type": "Answer",
            text: "npm install -g ghosty or Cargo.",
          },
        },
      ],
    });
  });

  it("covers all 40 curated pairs from the FAQ page arrays", () => {
    const questions = [...faqPage.matchAll(/^\s+q: "(.+)"/gm)].map((match) => match[1]);
    expect(questions).toHaveLength(40);
    expect(new Set(questions).size).toBe(40);
    expect(faqPage).toContain("const faqEn");
    expect(faqPage).toContain("const faqZh");
    expect(faqPage).toContain("buildFaqPageJsonLd({");
    expect(faqPage).toContain("items,");
    expect(faqPage).toContain('canonicalLocaleForPath("/faq", locale)');
    expect(faqPage).toContain('type="application/ld+json"');
    expect(faqPage).toContain("serializeJsonLd(jsonLd)");
  });
});

describe("extractText", () => {
  it("flattens nested elements and ignores booleans", () => {
    const node = createElement(
      "div",
      null,
      "Hello ",
      createElement("code", null, "ghosty"),
      false,
      createElement("span", null, [" ", 2]),
    );

    expect(extractText(node)).toContain("ghosty");
    expect(flattenExtractedText(node)).toBe("Hello ghosty 2");
  });

  it("escapes angle brackets so JSON-LD cannot close the script tag", () => {
    expect(serializeJsonLd({ text: "</script>" })).toBe('{"text":"\\u003c/script>"}');
  });
});
