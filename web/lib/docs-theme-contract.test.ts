import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const CSS = readFileSync(new URL("../app/globals.css", import.meta.url), "utf8");

function selectorBlock(selector: string): string {
  const match = CSS.match(new RegExp(`${selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\s*\\{([^}]*)\\}`, "s"));
  if (!match) throw new Error(`Missing CSS selector: ${selector}`);
  return match[1];
}

function customProperty(block: string, name: string): string {
  const match = block.match(new RegExp(`--${name}:\\s*(#[0-9a-f]{6})`, "i"));
  if (!match) throw new Error(`Missing custom property: --${name}`);
  return match[1];
}

function relativeLuminance(hex: string): number {
  const channels = hex
    .slice(1)
    .match(/.{2}/g)!
    .map((value) => Number.parseInt(value, 16) / 255)
    .map((value) => (value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4));

  return channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722;
}

function contrastRatio(foreground: string, background: string): number {
  const lighter = Math.max(relativeLuminance(foreground), relativeLuminance(background));
  const darker = Math.min(relativeLuminance(foreground), relativeLuminance(background));
  return (lighter + 0.05) / (darker + 0.05);
}

describe("docs dark-theme contrast contract", () => {
  it("keeps current and hover sidebar text at WCAG AA contrast", () => {
    const darkThemes = [
      selectorBlock('html:not([data-theme="light"]) .docs-theme'),
      selectorBlock('[data-theme="dark"] .docs-theme'),
    ];

    for (const dark of darkThemes) {
      const accent = customProperty(dark, "docs-accent");
      const background = customProperty(dark, "paper");
      expect(contrastRatio(accent, background)).toBeGreaterThanOrEqual(4.5);
    }
    expect(CSS).toMatch(/\.docs-sidebar-link:hover,\s*\.docs-sidebar-link-current\s*{[^}]*color:\s*var\(--docs-accent\)/s);
  });

  it("keeps secondary button text at WCAG AA contrast", () => {
    const darkThemes = [
      selectorBlock('html:not([data-theme="light"]) .docs-theme'),
      selectorBlock('[data-theme="dark"] .docs-theme'),
    ];

    for (const dark of darkThemes) {
      const text = customProperty(dark, "docs-button-text");
      const background = customProperty(dark, "docs-button-bg");
      expect(contrastRatio(text, background)).toBeGreaterThanOrEqual(4.5);
    }
    expect(selectorBlock(".docs-theme .portal-button-secondary")).toContain(
      "color: var(--docs-button-text)",
    );
  });
});
