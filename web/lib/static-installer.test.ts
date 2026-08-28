import { beforeEach, describe, expect, it, vi } from "vitest";
import { fetchWithStaticInstaller } from "./static-installer";

function ctx(): unknown {
  return {
    waitUntil: vi.fn(),
    passThroughOnException: vi.fn(),
  };
}

describe("static installer route", () => {
  const fallbackFetch = vi.fn(async () => new Response("fallback", { status: 200 }));

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("serves /install.sh from the static asset binding before OpenNext fallback", async () => {
    const assetFetch = vi.fn(async () =>
      new Response("#!/bin/sh\necho ghosty\n", {
        headers: { "content-type": "application/octet-stream" },
      }),
    );

    const response = await fetchWithStaticInstaller(
      new Request("https://ghosty.net/install.sh"),
      { ASSETS: { fetch: assetFetch } },
      ctx(),
      fallbackFetch,
    );

    expect(assetFetch).toHaveBeenCalledOnce();
    expect(fallbackFetch).not.toHaveBeenCalled();
    expect(response.headers.get("content-type")).toBe("text/x-shellscript; charset=utf-8");
    expect(response.headers.get("cache-control")).toBe("public, max-age=300");
    expect(await response.text()).toContain("echo ghosty");
  });

  it("delegates non-installer paths to the OpenNext handler", async () => {
    const assetFetch = vi.fn();

    const response = await fetchWithStaticInstaller(
      new Request("https://ghosty.net/install"),
      { ASSETS: { fetch: assetFetch } },
      ctx(),
      fallbackFetch,
    );

    expect(assetFetch).not.toHaveBeenCalled();
    expect(fallbackFetch).toHaveBeenCalledOnce();
    expect(await response.text()).toBe("fallback");
  });
});
