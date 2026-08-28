/**
 * The compatibility entry point stays a pure re-export of the canonical
 * report. The metric is observed active installs — the substantive tests live
 * in report-active-installs.test.ts; this file only pins that the old name
 * keeps working and adds nothing of its own.
 */

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import * as canonical from "../scripts/report-active-installs.mjs";
import * as compat from "../scripts/report-dau.mjs";

describe("report-dau compatibility entry point", () => {
  it("re-exports the canonical observed-active-installs report", () => {
    for (const name of [
      "parseArgs",
      "activeInstallsSql",
      "freshnessSql",
      "rowsFromResponse",
      "formatReport",
      "trendSummary",
      "COVERAGE_CAVEATS",
      "main",
    ] as const) {
      expect(compat[name], name).toBe(canonical[name]);
    }
  });

  it("defines no report logic of its own", () => {
    const ROOT = fileURLToPath(new URL("..", import.meta.url));
    const source = readFileSync(`${ROOT}scripts/report-dau.mjs`, "utf8");
    expect(source).toContain('export * from "./report-active-installs.mjs"');
    expect(source).not.toContain("SELECT");
  });
});
