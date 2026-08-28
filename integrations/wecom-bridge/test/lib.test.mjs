import assert from "node:assert/strict";
import { mkdtemp, rm, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import { ThreadStore, validateBridgeConfig, isApprovalResponse, isDenyResponse } from "../src/lib.mjs";

// ─── ThreadStore ─────────────────────────────────────────────────

test("ThreadStore writes private state files", async () => {
  const dir = await mkdtemp(path.join(tmpdir(), "ghosty-wecom-"));
  try {
    const statePath = path.join(dir, "nested", "thread-map.json");
    const store = await ThreadStore.open(statePath);

    await store.setChat("single:user-a", {
      threadId: "thread-a",
      lastSeq: 1,
      activeTurnId: null
    });

    const saved = await ThreadStore.open(statePath);
    assert.equal((await saved.getChat("single:user-a")).threadId, "thread-a");

    if (process.platform !== "win32") {
      assert.equal((await stat(path.dirname(statePath))).mode & 0o777, 0o700);
      assert.equal((await stat(statePath)).mode & 0o777, 0o600);
    }
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
});

// ─── Config validation ──────────────────────────────────────────

test("validateBridgeConfig rejects placeholder secrets", () => {
  const result = validateBridgeConfig({
    WECOM_BOT_ID: "your-bot-id",
    WECOM_BOT_SECRET: "your-bot-secret",
    GHOSTY_RUNTIME_TOKEN: "replace-with-long-random-token",
    GHOSTY_RUNTIME_URL: "http://127.0.0.1:7878"
  });

  assert.equal(result.ok, false);
  assert.deepEqual(
    result.errors.map((item) => item.code),
    ["placeholder_runtime_token"]
  );
});

// ─── isApprovalResponse ─────────────────────────────────────────

test("isApprovalResponse: single-word Chinese keywords", () => {
  for (const kw of ["允许", "可以", "好", "同意", "批准"]) {
    assert.equal(isApprovalResponse(kw), true, `"${kw}" should be approval`);
  }
});

test("isApprovalResponse: single-word English keywords", () => {
  for (const kw of ["yes", "ok", "y", "approve", "allow"]) {
    assert.equal(isApprovalResponse(kw), true, `"${kw}" should be approval`);
  }
});

test("isApprovalResponse: two-character Chinese keywords", () => {
  for (const kw of ["好的", "可以", "没问题", "批准了", "同意"]) {
    assert.equal(isApprovalResponse(kw), true, `"${kw}" should be approval`);
  }
});

test("isApprovalResponse: case-insensitive", () => {
  assert.equal(isApprovalResponse("YES"), true);
  assert.equal(isApprovalResponse("Ok"), true);
  assert.equal(isApprovalResponse("Allow"), true);
});

test("isApprovalResponse: trims whitespace", () => {
  assert.equal(isApprovalResponse("  允许  "), true);
  assert.equal(isApprovalResponse("  yes  "), true);
});

test("isApprovalResponse: rejects non-approval text", () => {
  assert.equal(isApprovalResponse("帮我查一下"), false);
  assert.equal(isApprovalResponse("/allow abc123"), false);
  assert.equal(isApprovalResponse("run prompt"), false);
});

test("isApprovalResponse: handles null/empty/undefined", () => {
  assert.equal(isApprovalResponse(null), false);
  assert.equal(isApprovalResponse(""), false);
  assert.equal(isApprovalResponse(undefined), false);
});

// ─── isDenyResponse ─────────────────────────────────────────────

test("isDenyResponse: single-word Chinese keywords", () => {
  for (const kw of ["拒绝", "不行", "不要", "取消", "否"]) {
    assert.equal(isDenyResponse(kw), true, `"${kw}" should be denial`);
  }
});

test("isDenyResponse: single-word English keywords", () => {
  for (const kw of ["no", "n", "deny", "reject", "stop"]) {
    assert.equal(isDenyResponse(kw), true, `"${kw}" should be denial`);
  }
});

test("isDenyResponse: two-character Chinese keywords", () => {
  for (const kw of ["不可以", "不同意", "不要执行"]) {
    assert.equal(isDenyResponse(kw), true, `"${kw}" should be denial`);
  }
});

test("isDenyResponse: case-insensitive and trims whitespace", () => {
  assert.equal(isDenyResponse("  NO  "), true);
  assert.equal(isDenyResponse("No"), true);
});

test("isDenyResponse: rejects non-denial text", () => {
  assert.equal(isDenyResponse("让我想想"), false);
  assert.equal(isDenyResponse("/deny abc123"), false);
  assert.equal(isDenyResponse("继续"), false);
});

test("isDenyResponse: handles null/empty/undefined", () => {
  assert.equal(isDenyResponse(null), false);
  assert.equal(isDenyResponse(""), false);
  assert.equal(isDenyResponse(undefined), false);
});

// ─── Mutual exclusivity ─────────────────────────────────────────

test("no keyword is both approval and denial", () => {
  const all = [
    "允许", "可以", "好", "同意", "批准",
    "好的", "没问题", "批准了",
    "yes", "ok", "y", "approve", "allow",
    "拒绝", "不行", "不要", "取消", "否",
    "不可以", "不同意", "不要执行",
    "no", "n", "deny", "reject", "stop"
  ];
  for (const kw of all) {
    assert.notEqual(
      isApprovalResponse(kw), isDenyResponse(kw),
      `"${kw}" should not be both approval and denial`
    );
  }
});
