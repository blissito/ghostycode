import { describe, expect, it } from "vitest";
import { faqSourceHref } from "./faq-source";

describe("FAQ source links", () => {
  it("links issue references and repository files to verifiable sources", () => {
    expect(faqSourceHref("#4674")).toBe(
      "https://github.com/blissito/ghostycode/issues/4674",
    );
    expect(faqSourceHref("README.md")).toBe(
      "https://github.com/blissito/ghostycode/blob/main/README.md",
    );
    expect(faqSourceHref("docs/PROVIDERS.md")).toBe(
      "https://github.com/blissito/ghostycode/blob/main/docs/PROVIDERS.md",
    );
    expect(faqSourceHref("release notes")).toBeNull();
  });
});
