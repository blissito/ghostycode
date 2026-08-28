/**
 * Locale path helpers for the website router.
 *
 * Middleware, the locale switcher, and the docs-only theme toggle all have
 * to agree on what the first path segment means. `pt-BR` is one segment,
 * not two; a bare `/install` is not yet localized. One implementation keeps
 * those call sites from drifting into a regex that only matches `[a-z]{2}`.
 */
import { locales } from "./config";

const ROUTED = locales as readonly string[];

/**
 * The routed locale a first path segment names, in canonical casing.
 *
 * Matching ignores case so `/pt-br/install` resolves to the same route as
 * `/pt-BR/install`. Only the regional tag has a case to get wrong, and an
 * external link that lowercases it used to fall through to the bare-path
 * branch and land on `/en/pt-br/install` — a 404.
 */
function routedLocale(segment: string | undefined): string | null {
  if (!segment) return null;
  const lower = segment.toLowerCase();
  return ROUTED.find((l) => l.toLowerCase() === lower) ?? null;
}

/** The routed locale already in `pathname`, or null if the path is bare. */
export function pathLocale(pathname: string): string | null {
  return routedLocale(pathname.split("/")[1]);
}

/**
 * Swap or insert the locale prefix. Used by the switcher so a click on
 * `/pt-BR/docs/guide` lands on `/ja/docs/guide` rather than a nested
 * `/ja/pt-BR/docs/guide` that the compact nav then treats as a miss, and by
 * the middleware to fold a miscased prefix onto its canonical spelling.
 */
export function replacePathLocale(pathname: string, locale: string): string {
  const segments = pathname.split("/");
  if (routedLocale(segments[1])) {
    segments[1] = locale;
    return segments.join("/");
  }
  // `"/"` splits to `["", ""]`, so splicing would join to `/<locale>/` — a
  // URL Next.js only serves after a trailing-slash redirect. Every other
  // path keeps whatever trailing slash it arrived with.
  if (pathname === "" || pathname === "/") return `/${locale}`;
  segments.splice(1, 0, locale);
  return segments.join("/");
}

/** True when the path is a docs route, with or without a locale prefix. */
export function isDocsPath(pathname: string): boolean {
  const segs = pathname.split("/").filter(Boolean);
  if (routedLocale(segs[0])) {
    return segs[1] === "docs";
  }
  return segs[0] === "docs";
}
