import { describe, expect, it } from "vitest";
import { locales } from "./config";
import { isDocsPath, pathLocale, replacePathLocale } from "./path";

describe("pathLocale", () => {
  it("reads the first segment when it is a routed locale", () => {
    expect(pathLocale("/zh")).toBe("zh");
    expect(pathLocale("/zh/install")).toBe("zh");
    expect(pathLocale("/pt-BR/docs/guide")).toBe("pt-BR");
    expect(pathLocale("/ja/")).toBe("ja");
  });

  it("returns null for bare paths and unknown first segments", () => {
    expect(pathLocale("/")).toBeNull();
    expect(pathLocale("/install")).toBeNull();
    expect(pathLocale("/docs/guide")).toBeNull();
    expect(pathLocale("/pt/docs")).toBeNull();
  });

  it("resolves a miscased prefix to its canonical spelling", () => {
    // An external link that lowercases the one regional tag still names a
    // real route; treating it as bare lands on `/en/pt-br/...`, a 404.
    expect(pathLocale("/pt-br/install")).toBe("pt-BR");
    expect(pathLocale("/PT-BR")).toBe("pt-BR");
    expect(pathLocale("/ZH/docs")).toBe("zh");
  });
});

describe("replacePathLocale", () => {
  it("swaps an existing locale prefix, including regional tags", () => {
    expect(replacePathLocale("/zh/install", "ja")).toBe("/ja/install");
    expect(replacePathLocale("/pt-BR/docs/guide", "en")).toBe("/en/docs/guide");
    expect(replacePathLocale("/de", "zh")).toBe("/zh");
    expect(replacePathLocale("/id/", "ar")).toBe("/ar/");
  });

  it("inserts a locale prefix on a bare path", () => {
    // `/de/` would only be served after a trailing-slash redirect; every
    // other path keeps whatever trailing slash it arrived with.
    expect(replacePathLocale("/", "de")).toBe("/de");
    expect(replacePathLocale("/install", "ja")).toBe("/ja/install");
    expect(replacePathLocale("/install/", "ja")).toBe("/ja/install/");
    expect(replacePathLocale("/docs/guide", "pt-BR")).toBe("/pt-BR/docs/guide");
  });

  it("folds a miscased prefix rather than nesting behind it", () => {
    expect(replacePathLocale("/pt-br/install", "pt-BR")).toBe("/pt-BR/install");
    expect(replacePathLocale("/pt-br/docs/guide", "ja")).toBe("/ja/docs/guide");
    expect(replacePathLocale("/ZH", "de")).toBe("/de");
  });

  it("round-trips every routed locale through every other", () => {
    for (const from of locales) {
      for (const to of locales) {
        const path = `/${from}/docs/guide`;
        expect(replacePathLocale(path, to), `${from} → ${to}`).toBe(`/${to}/docs/guide`);
        expect(pathLocale(replacePathLocale(path, to))).toBe(to);
      }
    }
  });

  it("does not nest a second locale prefix on a localized path", () => {
    expect(replacePathLocale("/pt-BR/install", "pt-BR")).toBe("/pt-BR/install");
    expect(replacePathLocale("/ja/faq", "ja")).not.toContain("/ja/ja/");
  });
});

describe("isDocsPath", () => {
  it("recognizes docs routes for two-letter and regional locales", () => {
    expect(isDocsPath("/en/docs")).toBe(true);
    expect(isDocsPath("/zh/docs/guide")).toBe(true);
    expect(isDocsPath("/pt-BR/docs/hooks")).toBe(true);
    expect(isDocsPath("/docs")).toBe(true);
    expect(isDocsPath("/docs/guide")).toBe(true);
  });

  it("recognizes a miscased locale prefix on a docs route", () => {
    expect(isDocsPath("/pt-br/docs/guide")).toBe(true);
    expect(isDocsPath("/pt-br/install")).toBe(false);
  });

  it("rejects non-docs routes, including names that merely contain docs", () => {
    expect(isDocsPath("/en/install")).toBe(false);
    expect(isDocsPath("/pt-BR/community")).toBe(false);
    expect(isDocsPath("/zh")).toBe(false);
    expect(isDocsPath("/")).toBe(false);
    expect(isDocsPath("/en/community/docs-club")).toBe(false);
  });
});
