#!/usr/bin/env node

/**
 * Compatibility entry point. The canonical report is
 * `report-active-installs.mjs` — the metric is observed active installs, not
 * daily active users, and the canonical name says so. This file exists only so
 * `npm run report:dau` and any muscle memory pointing at the old path keep
 * working; it adds nothing.
 */

import { pathToFileURL } from "node:url";

import { runCli } from "./report-active-installs.mjs";

export * from "./report-active-installs.mjs";

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  runCli("report:dau");
}
