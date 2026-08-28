import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { DISCORD_URL, REPO_URL } from "./i18n/links";
import { IDENTITY_PHRASE, SITE_NAME, SITE_URL } from "./page-meta";
import {
  ORGANIZATION_ID,
  ORGANIZATION_LOGO_URL,
  WEBSITE_ID,
  buildSiteJsonLd,
} from "./site-schema";

describe("Organization and WebSite structured data", () => {
  it("emits a stable Organization @id with logo and sameAs", () => {
    const schema = buildSiteJsonLd("en");
    const organization = schema["@graph"].find((node) => node["@type"] === "Organization");
    const website = schema["@graph"].find((node) => node["@type"] === "WebSite");

    expect(organization).toEqual({
      "@type": "Organization",
      "@id": ORGANIZATION_ID,
      name: SITE_NAME,
      url: SITE_URL,
      logo: { "@type": "ImageObject", url: ORGANIZATION_LOGO_URL },
      sameAs: [REPO_URL, DISCORD_URL],
    });
    expect(organization?.["@id"]).toBe("https://ghosty.net/#organization");
    expect(website).toMatchObject({
      "@type": "WebSite",
      "@id": WEBSITE_ID,
      name: SITE_NAME,
      url: SITE_URL,
      description: IDENTITY_PHRASE,
      inLanguage: "en",
      publisher: { "@id": ORGANIZATION_ID },
    });
  });

  it("sets WebSite inLanguage from the routed locale", () => {
    expect(buildSiteJsonLd("zh")["@graph"].find((node) => node["@type"] === "WebSite")?.inLanguage).toBe(
      "zh",
    );
    expect(buildSiteJsonLd("ja")["@graph"].find((node) => node["@type"] === "WebSite")?.inLanguage).toBe(
      "ja",
    );
  });

  it("embeds the graph from the locale layout, not a second homepage-only tag", () => {
    const layout = readFileSync(new URL("../app/[locale]/layout.tsx", import.meta.url), "utf8");
    expect(layout).toContain("buildSiteJsonLd(locale)");
    expect(layout).toContain("type=\"application/ld+json\"");
    expect(layout).toContain("serializeJsonLd(siteJsonLd)");
  });
});
