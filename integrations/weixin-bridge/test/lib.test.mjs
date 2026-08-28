import test from "node:test";
import assert from "node:assert/strict";

import {
  activeTurnBlock,
  commandAction,
  extractText,
  MessageItemType,
  parseBool,
  parseCommand,
  parseList,
  preservedChatStateFields,
  splitMessage
} from "../src/lib.mjs";

test("extractText reads text and voice transcript items", () => {
  assert.equal(
    extractText([{ type: MessageItemType.TEXT, text_item: { text: "hello" } }]),
    "hello"
  );
  assert.equal(
    extractText([{ type: MessageItemType.VOICE, voice_item: { text: "voice text" } }]),
    "voice text"
  );
});

test("shared command helpers preserve Weixin bridge command behavior", () => {
  assert.deepEqual(parseList("u1, u2 ,, "), ["u1", "u2"]);
  assert.equal(parseBool("yes"), true);
  assert.deepEqual(parseCommand("/allow ap_1 remember"), {
    name: "allow",
    args: "ap_1 remember"
  });
  assert.deepEqual(commandAction(parseCommand("/model auto")), {
    kind: "set_model",
    modelName: "auto"
  });
  assert.deepEqual(commandAction(parseCommand("/unknown value")), {
    kind: "prompt",
    prompt: "/unknown value"
  });
});

test("shared state and runtime helpers preserve Weixin bridge behavior", () => {
  assert.deepEqual(preservedChatStateFields({ model: "m", activeTurnId: "turn-1" }), {
    model: "m"
  });
  assert.deepEqual(splitMessage("a🧪b", 2), ["a🧪", "b"]);
  assert.deepEqual(activeTurnBlock({ turns: [{ id: "turn-1", status: "in_progress" }] }), {
    turnId: "turn-1",
    message: "Thread already has active turn turn-1. Wait for it to finish or send /interrupt."
  });
});
