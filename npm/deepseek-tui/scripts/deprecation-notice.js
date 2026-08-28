#!/usr/bin/env node

const notice = [
  "",
  "  ╭───────────────────────────────────────────────────────────────────╮",
  "  │                                                                   │",
  "  │  deepseek-tui has been renamed to `ghosty`.                    │",
  "  │                                                                   │",
  "  │  Please uninstall this package and install ghosty instead:     │",
  "  │                                                                   │",
  "  │    npm uninstall -g deepseek-tui                                  │",
  "  │    npm install -g ghosty                                       │",
  "  │                                                                   │",
  "  │  ghosty installs the `ghosty` and `ghosty-tui` commands.         │",
  "  │  Historical old-name shims ended with v0.8.x. See:                │",
  "  │  https://github.com/blissito/ghostycode/blob/main/docs/REBRAND.md │",
  "  │                                                                   │",
  "  ╰───────────────────────────────────────────────────────────────────╯",
  "",
].join("\n");

process.stderr.write(notice);
