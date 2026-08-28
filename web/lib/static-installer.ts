type AssetBinding = {
  fetch(input: Request | string, init?: RequestInit): Promise<Response>;
};

type FallbackFetch = (
  request: Request,
  env: Record<string, unknown>,
  ctx: unknown,
) => Response | Promise<Response>;

const INSTALL_SCRIPT_PATH = "/install.sh";

export async function fetchWithStaticInstaller(
  request: Request,
  env: Record<string, unknown>,
  ctx: unknown,
  fallbackFetch: FallbackFetch,
): Promise<Response> {
  const url = new URL(request.url);
  const assets = (env as { ASSETS?: AssetBinding }).ASSETS;

  if (url.pathname === INSTALL_SCRIPT_PATH && assets) {
    const assetResponse = await assets.fetch(request);
    if (assetResponse.status !== 404) {
      const headers = new Headers(assetResponse.headers);
      headers.set("content-type", "text/x-shellscript; charset=utf-8");
      headers.set("cache-control", "public, max-age=300");
      return new Response(assetResponse.body, {
        status: assetResponse.status,
        statusText: assetResponse.statusText,
        headers,
      });
    }
  }

  return fallbackFetch(request, env, ctx);
}
