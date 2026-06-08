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
  "  │    npm install -g ghostycode                                       │",
  "  │                                                                   │",
  "  │  ghosty ships the same `ghosty` and `ghosty-tui`         │",
  "  │  binaries plus deprecation shims under the old names. See:        │",
  "  │  https://github.com/blissito/ghostycode/blob/main/docs/REBRAND.md │",
  "  │                                                                   │",
  "  ╰───────────────────────────────────────────────────────────────────╯",
  "",
].join("\n");

process.stderr.write(notice);
