import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const CSS = readFileSync(new URL("../app/globals.css", import.meta.url), "utf8");
const TUI_TOKENS = readFileSync(
  new URL("../../crates/tui/src/palette/tokens.rs", import.meta.url),
  "utf8",
);

function rustRgb(name: string): string {
  const match = TUI_TOKENS.match(
    new RegExp(`pub const ${name}_RGB:[^=]+\\= \\((\\d+), (\\d+), (\\d+)\\)`),
  );
  if (!match) throw new Error(`Missing Rust RGB token: ${name}_RGB`);
  return `#${match
    .slice(1)
    .map((channel) => Number(channel).toString(16).padStart(2, "0"))
    .join("")}`;
}

function cssHex(name: string): string {
  const match = CSS.match(new RegExp(`--${name}:\\s*(#[0-9a-f]{6})`, "i"));
  if (!match) throw new Error(`Missing CSS token: --${name}`);
  return match[1].toLowerCase();
}

describe("Blue Stage public-surface contract", () => {
  it("shares the TUI stage, action, human, and structural dark tokens", () => {
    expect(cssHex("ocean-deep")).toBe(rustRgb("WHALE_BG"));
    expect(cssHex("action-on-dark")).toBe(rustRgb("WHALE_ACTION"));
    expect(cssHex("signal-gold")).toBe(rustRgb("WHALE_HUMAN"));
    expect(cssHex("ocean-current")).toBe(rustRgb("WHALE_ICE"));
  });

  it("shares the TUI light surface, text, and interaction tokens", () => {
    expect(cssHex("paper")).toBe(rustRgb("LIGHT_SURFACE"));
    expect(cssHex("paper-deep")).toBe(rustRgb("LIGHT_ELEVATED"));
    expect(cssHex("ink")).toBe(rustRgb("LIGHT_TEXT_BODY"));
    expect(cssHex("indigo")).toBe(rustRgb("LIGHT_ACTION"));
  });

  it("reserves Signal Gold for the whale while controls use action blue", () => {
    expect(CSS).toMatch(/\.ghosty-mark-primary \{ fill: var\(--signal-gold\); \}/);
    expect(CSS).toMatch(/\.portal-button-primary[\s\S]*background: var\(--indigo-deep\)/);
    expect(CSS).toMatch(/\.nav-link::after[\s\S]*background: var\(--indigo\)/);
  });

  it("keeps localized navigation controls inside compact viewports", () => {
    const mobile = CSS.split("@media (max-width: 520px)")[1];

    expect(mobile).toMatch(/\.site-nav-inner\s*\{\s*gap:\s*0\.5rem/);
    expect(mobile).toMatch(/\.site-nav-actions\s*\{[\s\S]*?min-width:\s*0/);
    expect(mobile).toMatch(
      /\.site-nav-actions select\s*\{[\s\S]*?width:\s*6\.75rem;[\s\S]*?min-width:\s*0/,
    );
    expect(mobile).toMatch(/\.paper-wordmark-tag\s*\{\s*display:\s*none/);
    expect(CSS).toMatch(/\.site-nav-actions\s*\{[\s\S]*?flex-shrink:\s*0/);
    expect(CSS).toMatch(/\.site-nav-actions\s*>\s*\*\s*\{\s*flex-shrink:\s*0/);
    expect(CSS).toMatch(
      /@media \(max-width: 900px\)[\s\S]*?\.site-github-link,[\s\S]*?\.site-discord-link\s*\{\s*display:\s*none/,
    );
    expect(mobile).not.toMatch(/body:has\(\.product-home\) \.site-nav-actions select/);
    // The locale <select> and the home wordmark must keep a usable hit
    // target on every viewport, not only below 520px. Long native option
    // labels and 2xl companion text used to collapse the wordmark to 0.
    expect(CSS).toMatch(
      /\.site-nav-actions select\s*\{\s*width:\s*6\.75rem;\s*max-width:\s*6\.75rem;\s*min-width:\s*0;/,
    );
    // `min-width` is the floor that keeps the wordmark clickable; the shrink
    // factor stays at 1 so the compact controls are never the ones pushed
    // past `overflow-x: clip` when the row is over budget.
    expect(CSS).toMatch(/\.paper-wordmark\s*\{[\s\S]*?flex:\s*0 1 auto;[\s\S]*?min-width:\s*8\.75rem;/);
    expect(CSS).not.toMatch(/\.paper-wordmark\s*\{[\s\S]*?flex:\s*0 0 auto;/);
  });
});
