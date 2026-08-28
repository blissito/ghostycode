import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import {
  COVERAGE_CAVEATS,
  activeInstallsSql,
  formatAge,
  formatReport,
  freshnessSql,
  newestEventFromResponse,
  parseArgs,
  rowsFromResponse,
  trendSummary,
} from "../scripts/report-active-installs.mjs";

const NOW = new Date("2026-08-09T20:00:00Z");

/** 15 complete days of fixture data ending yesterday: last-7 sums to 70, previous-7 to 35. */
function fixtureRows() {
  const rows: Array<{ day: string; active_installs: number; sessions_started: number }> = [];
  for (let offset = 1; offset <= 15; offset += 1) {
    const day = new Date(Date.UTC(2026, 7, 9) - offset * 86_400_000)
      .toISOString()
      .slice(0, 10);
    rows.push({
      day,
      active_installs: offset <= 7 ? 10 : 5,
      sessions_started: 20,
    });
  }
  return rows;
}

describe("observed active installs owner report", () => {
  it("bounds the query to Analytics Engine retention", () => {
    expect(parseArgs([])).toEqual({ days: 15, json: false });
    expect(parseArgs(["--days", "7", "--json"])).toEqual({ days: 7, json: true });
    expect(() => parseArgs(["--days", "0"])).toThrow(/1 through 90/);
    expect(() => parseArgs(["--days", "91"])).toThrow(/1 through 90/);
  });

  it("counts distinct installs only from session starts", () => {
    const sql = activeInstallsSql(15);
    expect(sql).toContain("count(DISTINCT index1) AS active_installs");
    expect(sql).toContain("sum(_sample_interval) AS sessions_started");
    expect(sql).toContain("AND blob1 = 'session_start'");
    expect(sql).toContain("INTERVAL '14' DAY");
    expect(sql).toContain("GROUP BY day");
  });

  it("labels the partial UTC day and the lower-bound definition", () => {
    const rows = rowsFromResponse({
      data: [
        {
          day: "2026-08-09",
          active_installs: "43",
          sessions_started: "67",
        },
      ],
    });
    const report = formatReport(rows, { days: 15, now: NOW });
    expect(report).toContain("2026-08-09*");
    expect(report).toContain("43");
    expect(report).toContain("Not people, not accounts, not total installs");
    expect(report).toContain("clients older than the telemetry feature");
  });

  it("never labels the count as users, people, accounts, or total installs", () => {
    const report = formatReport(fixtureRows(), { days: 15, now: NOW, newestEvent: NOW });
    expect(report).toContain("observed active installs");
    expect(report).not.toMatch(/\bDAU\b/);
    expect(report).not.toMatch(/daily active users/i);
    expect(report).not.toMatch(/unique (users|people|accounts)/i);
    // "people", "accounts", "total installs" appear only inside negations.
    for (const line of report.split("\n")) {
      if (/people|accounts|total installs/i.test(line)) {
        expect(line).toMatch(/Not people, not accounts, not total installs/);
      }
    }
  });
});

describe("7-day trend", () => {
  it("compares the last 7 complete UTC days against the previous 7", () => {
    expect(trendSummary(fixtureRows(), 15, NOW)).toEqual({
      last7: 70,
      previous7: 35,
      changePct: 100,
    });
  });

  it("excludes the partial current day from both windows", () => {
    const rows = [
      { day: "2026-08-09", active_installs: 999, sessions_started: 999 },
      ...fixtureRows(),
    ];
    expect(trendSummary(rows, 15, NOW).last7).toBe(70);
  });

  it("treats missing days as zero rather than skipping them", () => {
    const rows = fixtureRows().filter((row) => row.day !== "2026-08-05");
    expect(trendSummary(rows, 15, NOW).last7).toBe(60);
  });

  it("refuses to fabricate a comparison the window does not cover", () => {
    const short = trendSummary(fixtureRows().slice(0, 7), 8, NOW);
    expect(short.last7).toBe(70);
    expect(short.previous7).toBeNull();
    expect(short.changePct).toBeNull();
    expect(trendSummary([], 3, NOW).last7).toBeNull();
  });

  it("prints the trend with the not-a-retention-metric caveat inline", () => {
    const report = formatReport(fixtureRows(), { days: 15, now: NOW });
    expect(report).toContain("7-day trend");
    expect(report).toContain("last 7 days:      70");
    expect(report).toContain("previous 7 days:  35");
    expect(report).toContain("+100%");
    expect(report).toContain("(id rotation: not a retention metric)");
  });

  it("says so when the window is too small instead of printing zeros", () => {
    const report = formatReport(fixtureRows().slice(0, 7), { days: 8, now: NOW });
    expect(report).toContain("not covered — use --days 15 or more");
    const tiny = formatReport([], { days: 3, now: NOW });
    expect(tiny).toContain("window too small for a 7-day trend");
  });
});

describe("event freshness", () => {
  it("asks only for the newest timestamp", () => {
    const sql = freshnessSql();
    expect(sql).toContain("max(timestamp) AS newest_event");
    expect(sql).not.toContain("blob");
    expect(sql).not.toContain("index1");
  });

  it("parses the SQL API's zone-less UTC timestamps", () => {
    const parsed = newestEventFromResponse({
      data: [{ newest_event: "2026-08-09 17:46:00" }],
    });
    expect(parsed?.toISOString()).toBe("2026-08-09T17:46:00.000Z");
    expect(newestEventFromResponse({ data: [] })).toBeNull();
    expect(newestEventFromResponse({ data: [{ newest_event: null }] })).toBeNull();
  });

  it("prints how stale the newest ingested event is", () => {
    const newestEvent = new Date("2026-08-09T17:46:00Z");
    const report = formatReport(fixtureRows(), { days: 15, now: NOW, newestEvent });
    expect(report).toContain(
      "Freshness: newest ingested event 2026-08-09T17:46:00.000Z (2h 14m ago)",
    );
  });

  it("says plainly when nothing has ever been ingested", () => {
    const report = formatReport([], { days: 15, now: NOW, newestEvent: null });
    expect(report).toContain("Freshness: no events ingested in the retention window");
  });

  it("formats ages at minute, hour, and day scale", () => {
    expect(formatAge(30_000)).toBe("0m");
    expect(formatAge(14 * 60_000)).toBe("14m");
    expect(formatAge((2 * 60 + 14) * 60_000)).toBe("2h 14m");
    expect(formatAge((3 * 1440 + 5 * 60) * 60_000)).toBe("3d 5h");
  });
});

describe("coverage caveats travel with the numbers", () => {
  it("prints every caveat in both report modes' shared source of truth", () => {
    const report = formatReport(fixtureRows(), { days: 15, now: NOW });
    for (const caveat of COVERAGE_CAVEATS) {
      expect(report).toContain(caveat);
    }
  });

  it("names the invisible populations and the rotation caveat", () => {
    const joined = COVERAGE_CAVEATS.join("\n");
    expect(joined).toContain("clients older than the telemetry feature");
    expect(joined).toContain("opted-out installs");
    expect(joined).toContain("non-emitting environments");
    expect(joined).toContain("rotate every 90 days");
    expect(joined).toContain("not a retention metric");
  });
});

describe("the report path stays inside the exclusion guarantees", () => {
  const ROOT = fileURLToPath(new URL("..", import.meta.url));
  const SOURCE = readFileSync(`${ROOT}scripts/report-active-installs.mjs`, "utf8");

  it("aggregates install ids and never selects one back out", () => {
    for (const sql of [activeInstallsSql(15), freshnessSql()]) {
      // index1 (the install id) may appear only inside count(DISTINCT …).
      expect(sql.replace(/count\(DISTINCT index1\)/g, "")).not.toContain("index1");
    }
  });

  it("touches no column that carries anything beyond the event name", () => {
    // blob1 is the event-name enum; blob2..blob17 and the doubles carry the
    // platform/counter payload and have no business in an install count.
    expect(SOURCE).not.toMatch(/blob(?!1\b)\d+/);
    expect(SOURCE).not.toMatch(/double\d+/);
  });

  it("reads nothing but the two SQL aggregates", () => {
    // No prompts, paths, repo names, account identity, or network identity
    // exist in the dataset (see no-ip.test.ts and schema-doc.test.ts); this
    // pins the report to the read side of the same contract.
    expect(SOURCE).not.toMatch(/cf-connecting-ip|x-forwarded-for|x-real-ip/i);
    expect(SOURCE).not.toMatch(/\bemail\b|\baccount_name\b|\busername\b/i);
    const selects = SOURCE.match(/SELECT[\s\S]*?FROM/g) ?? [];
    expect(selects.length).toBe(2);
  });
});
