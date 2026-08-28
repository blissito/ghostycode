import { describe, expect, it } from "vitest";
import { NextRequest } from "next/server";
import { middleware } from "../middleware";

/**
 * `www.ghosty.net` and `ghosty.net` are both bound to this worker as
 * custom domains, so without a canonical-host redirect the whole site is
 * reachable — and indexable — twice.
 */
function request(url: string, host: string, headers: Record<string, string> = {}) {
  return new NextRequest(new URL(url), {
    headers: new Headers({ host, ...headers }),
  });
}

describe("canonical host", () => {
  it("301s www to the apex, preserving path and query", () => {
    const res = middleware(
      request("https://www.ghosty.net/en/docs/hooks?x=1", "www.ghosty.net"),
    );
    expect(res.status).toBe(301);
    expect(res.headers.get("location")).toBe("https://ghosty.net/en/docs/hooks?x=1");
  });

  it("redirects assets and API routes too, so a moved document stops pulling from www", () => {
    for (const path of ["/_next/static/chunk.js", "/api/curated", "/opengraph-image"]) {
      const res = middleware(request(`https://www.ghosty.net${path}`, "www.ghosty.net"));
      expect(res.status, path).toBe(301);
      expect(res.headers.get("location"), path).toBe(`https://ghosty.net${path}`);
    }
  });

  it("leaves the apex host alone", () => {
    const res = middleware(request("https://ghosty.net/en", "ghosty.net"));
    expect(res.status).not.toBe(301);
  });

  it("leaves localhost and preview hosts alone", () => {
    for (const host of ["localhost:3000", "ghosty-web.pages.dev"]) {
      const res = middleware(request("https://example.test/en", host));
      expect(res.status, host).not.toBe(301);
    }
  });

  it("still applies security headers to the redirect", () => {
    const res = middleware(request("https://www.ghosty.net/en", "www.ghosty.net"));
    expect(res.headers.get("X-Content-Type-Options")).toBe("nosniff");
    expect(res.headers.get("Strict-Transport-Security")).toContain("max-age=");
  });
});

describe("dotted well-known paths", () => {
  it("does not locale-prefix /llms.txt", () => {
    const res = middleware(request("https://ghosty.net/llms.txt", "ghosty.net"));
    expect(res.status).not.toBe(307);
    expect(res.status).not.toBe(308);
    expect(res.headers.get("location")).toBeNull();
  });
});

describe("locale prefix", () => {
  it("prefixes a bare path with the detected locale", () => {
    const res = middleware(
      request("https://ghosty.net/install", "ghosty.net", {
        "accept-language": "ja,en;q=0.8",
      }),
    );
    expect(res.status).toBe(307);
    expect(res.headers.get("location")).toBe("https://ghosty.net/ja/install");
  });

  it("leaves an already-localized path, including pt-BR, unprefixed", () => {
    for (const path of ["/zh/install", "/pt-BR/docs/guide", "/de"]) {
      const res = middleware(request(`https://ghosty.net${path}`, "ghosty.net"));
      expect(res.status, path).not.toBe(307);
      expect(res.headers.get("location"), path).toBeNull();
    }
  });

  it("folds a miscased locale prefix onto its canonical spelling", () => {
    // Without this the segment reads as bare and the request lands on
    // `/en/pt-br/install`, which is a 404 rather than the Portuguese page.
    const res = middleware(
      request("https://ghosty.net/pt-br/install?x=1", "ghosty.net"),
    );
    expect(res.status).toBe(308);
    expect(res.headers.get("location")).toBe("https://ghosty.net/pt-BR/install?x=1");
  });

  it("settles after one canonicalizing redirect", () => {
    const res = middleware(request("https://ghosty.net/pt-BR/install", "ghosty.net"));
    expect(res.status).not.toBe(308);
    expect(res.headers.get("location")).toBeNull();
  });
});
