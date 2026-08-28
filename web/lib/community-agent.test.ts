import { describe, it, expect, vi, beforeEach } from "vitest";
import { validateSession } from "./community-agent";

describe("validateSession", () => {
  let mockKV: {
    get: ReturnType<typeof vi.fn>;
    put: ReturnType<typeof vi.fn>;
    list: ReturnType<typeof vi.fn>;
    delete: ReturnType<typeof vi.fn>;
  };

  beforeEach(() => {
    mockKV = {
      get: vi.fn(),
      put: vi.fn(),
      list: vi.fn(),
      delete: vi.fn(),
    };
  });

  const VALID_SID = "a".repeat(40);
  const SESSION_PREFIX = "session:admin:";

  it("should return false if kv is undefined", async () => {
    const result = await validateSession(undefined, VALID_SID);
    expect(result).toBe(false);
  });

  it("should return false if sid is undefined or null", async () => {
    const kv = mockKV as unknown as Parameters<typeof validateSession>[0];
    expect(await validateSession(kv, undefined)).toBe(false);
    expect(await validateSession(kv, null)).toBe(false);
  });

  it("should return false if sid is empty", async () => {
    const kv = mockKV as unknown as Parameters<typeof validateSession>[0];
    expect(await validateSession(kv, "")).toBe(false);
  });

  it("should return false if sid length is less than 40", async () => {
    const kv = mockKV as unknown as Parameters<typeof validateSession>[0];
    expect(await validateSession(kv, "a".repeat(39))).toBe(false);
  });

  it("should return false if sid length is greater than 64", async () => {
    const kv = mockKV as unknown as Parameters<typeof validateSession>[0];
    expect(await validateSession(kv, "a".repeat(65))).toBe(false);
  });

  it("should return false if sid contains invalid characters", async () => {
    const kv = mockKV as unknown as Parameters<typeof validateSession>[0];
    // Length 40 but contains invalid char '!'
    expect(await validateSession(kv, "a".repeat(39) + "!")).toBe(false);
  });

  it("should return false if session is not found in KV", async () => {
    const kv = mockKV as unknown as Parameters<typeof validateSession>[0];
    mockKV.get.mockResolvedValue(null);

    const result = await validateSession(kv, VALID_SID);
    expect(result).toBe(false);
    expect(mockKV.get).toHaveBeenCalledWith(SESSION_PREFIX + VALID_SID);
  });

  it("should return true if session is found in KV", async () => {
    const kv = mockKV as unknown as Parameters<typeof validateSession>[0];
    mockKV.get.mockResolvedValue(JSON.stringify({ createdAt: Date.now() }));

    const result = await validateSession(kv, VALID_SID);
    expect(result).toBe(true);
    expect(mockKV.get).toHaveBeenCalledWith(SESSION_PREFIX + VALID_SID);
  });

  it("should work with a 64 character sid", async () => {
    const kv = mockKV as unknown as Parameters<typeof validateSession>[0];
    mockKV.get.mockResolvedValue("{}");

    const sid = "b".repeat(64);
    const result = await validateSession(kv, sid);
    expect(result).toBe(true);
    expect(mockKV.get).toHaveBeenCalledWith(SESSION_PREFIX + sid);
  });
});
