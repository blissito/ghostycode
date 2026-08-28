#!/usr/bin/env node

const assert = require("node:assert/strict");
const test = require("node:test");

const {
  CLEANUP_MAX_RETRIES,
  CLEANUP_RETRY_DELAY_MS,
  removeSmokeWorkspace,
} = require("./npm-wrapper-smoke");

test("npm wrapper cleanup bounds retries for transient Windows removal races", async () => {
  const calls = [];
  await removeSmokeWorkspace("C:\\Temp\\ghosty-smoke", async (...args) => {
    calls.push(args);
  });

  assert.deepEqual(calls, [
    [
      "C:\\Temp\\ghosty-smoke",
      {
        force: true,
        recursive: true,
        maxRetries: CLEANUP_MAX_RETRIES,
        retryDelay: CLEANUP_RETRY_DELAY_MS,
      },
    ],
  ]);
  assert.ok(Number.isInteger(CLEANUP_MAX_RETRIES));
  assert.ok(CLEANUP_MAX_RETRIES > 0);
  assert.ok(Number.isFinite(CLEANUP_RETRY_DELAY_MS));
  assert.ok(CLEANUP_RETRY_DELAY_MS > 0);
});

test("npm wrapper cleanup surfaces persistent removal failures", async () => {
  const failure = Object.assign(new Error("directory remains non-empty"), {
    code: "ENOTEMPTY",
  });

  await assert.rejects(
    removeSmokeWorkspace("C:\\Temp\\ghosty-smoke", async () => {
      throw failure;
    }),
    (error) => error === failure,
  );
});
