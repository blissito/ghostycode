/**
 * The grep guard.
 *
 * `docs/TELEMETRY.md` publishes "Batches are IP-stripped at ingest. No IP is
 * stored, logged, or joined to `install_id`." The only thing that makes that
 * sentence true is that `src/` never asks for the address. Behavioural tests
 * cannot prove a negative here — an IP read that only fires on some code path,
 * or that goes to a log rather than to the dataset, passes every functional
 * test in the suite.
 *
 * So this file reads the shipped source as text and fails if the names appear
 * at all. It is deliberately blunt: a later edit that adds one "just for
 * debugging" cannot land quietly, and the failure names the promise it breaks.
 *
 * It scans `src/` and `wrangler.jsonc`, and never itself: this file and the
 * "Verifying no IP is stored" section of `README.md` are the only places in the
 * directory where the forbidden names are written down, and neither one ships.
 */

import { readFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const ROOT = fileURLToPath(new URL("..", import.meta.url));

const SHIPPED: Array<[string, string]> = [
  ...readdirSync(`${ROOT}src`)
    .filter((name) => name.endsWith(".ts"))
    .map(
      (name) =>
        [`src/${name}`, readFileSync(`${ROOT}src/${name}`, "utf8")] as [
          string,
          string,
        ],
    ),
  ["wrangler.jsonc", readFileSync(`${ROOT}wrangler.jsonc`, "utf8")],
];

const HANDLER = readFileSync(`${ROOT}src/index.ts`, "utf8");

/** Names that would put a client address, or a geo derived from one, in scope. */
const FORBIDDEN: Array<[string, RegExp]> = [
  ["the connecting-IP header", /cf-connecting-ip/i],
  ["the forwarded-for header", /x-forwarded-for/i],
  ["the real-IP header", /x-real-ip/i],
  ["the true-client-IP header", /true-client-ip/i],
  ["the forwarded header", /["']forwarded["']/i],
  ["request.cf geo — country", /cf\.country/i],
  ["request.cf geo — colo", /cf\.colo/i],
  ["request.cf geo — city", /cf\.city/i],
  ["request.cf geo — region", /cf\.region/i],
  ["request.cf geo — asn", /cf\.asn/i],
  ["request.cf geo — coordinates", /cf\.(latitude|longitude)/i],
  ["request.cf geo — postal code", /cf\.postalcode/i],
  ["request.cf geo — timezone", /cf\.timezone/i],
  ["the request cf property", /\brequest\.cf\b/],
  ["the cf property, however spelled", /\.cf\s*[.[]/],
];

describe("no client IP, structurally", () => {
  it.each(FORBIDDEN)("never references %s", (_label, pattern) => {
    for (const [name, source] of SHIPPED) {
      expect(
        pattern.test(source),
        `${name} references ${pattern} — docs/TELEMETRY.md promises no IP is read, stored, logged, or joined to install_id`,
      ).toBe(false);
    }
  });

  it("reads exactly two headers, ever", () => {
    const asked = new Set<string>();
    for (const [, source] of SHIPPED) {
      for (const match of source.matchAll(
        /headers\s*\.\s*(get|has)\s*\(\s*["'`]([^"'`]+)["'`]/g,
      )) {
        asked.add(match[2].toLowerCase());
      }
    }
    expect([...asked].sort()).toEqual(["content-length", "content-type"]);
  });

  it("never iterates the request headers", () => {
    for (const [name, source] of SHIPPED) {
      expect(
        /headers\s*\.\s*(entries|keys|values|forEach)/.test(source),
        `${name} enumerates headers`,
      ).toBe(false);
      expect(
        /Object\.fromEntries\s*\(\s*[a-zA-Z.]*headers/.test(source),
        `${name} snapshots headers`,
      ).toBe(false);
    }
  });

  it("logs nothing at all", () => {
    for (const [name, source] of SHIPPED) {
      expect(/\bconsole\s*\./.test(source), `${name} logs`).toBe(false);
    }
  });

  it("turns invocation logs off in the Worker config", () => {
    const config = readFileSync(`${ROOT}wrangler.jsonc`, "utf8");
    expect(config).toMatch(/"invocation_logs"\s*:\s*false/);
  });

  it("never constructs a Response with a body", () => {
    const constructions = [...HANDLER.matchAll(/new Response\(([^,)]*)/g)];
    expect(constructions.length).toBeGreaterThan(0);
    for (const construction of constructions) {
      expect(construction[1].trim()).toBe("null");
    }
  });

  it("keeps the request out of the storage path", () => {
    // `datapoint.ts` builds every Analytics Engine row. If it can see a
    // `Request`, it can see an address; it must only ever see a validated
    // batch.
    const datapoint = readFileSync(`${ROOT}src/datapoint.ts`, "utf8");
    expect(/\bRequest\b/.test(datapoint)).toBe(false);
    expect(/\brequest\b/.test(datapoint)).toBe(false);
  });

  it("declares no binding that could hold per-request state", () => {
    const config = readFileSync(`${ROOT}wrangler.jsonc`, "utf8");
    for (const binding of [
      "kv_namespaces",
      "d1_databases",
      "r2_buckets",
      "durable_objects",
      "queues",
      "hyperdrive",
      "vectorize",
    ]) {
      expect(
        new RegExp(`"${binding}"\\s*:`).test(config),
        `${binding} is not needed for write-only ingest`,
      ).toBe(false);
    }
  });
});
