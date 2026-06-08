# Rebrand: DeepSeek TUI → Ghosty Code

Starting with **v0.8.41**, this project ships under a new name: `ghosty`.

This document explains what changed, what didn't, and how to migrate. None of the
DeepSeek provider integration changed — only the local CLI / TUI brand.

## TL;DR

```bash
# 1. Uninstall the old wrapper or binaries.
npm uninstall -g deepseek-tui      # or cargo uninstall deepseek-tui-cli deepseek-tui
                                    # or brew uninstall deepseek-tui

# 2. Install under the new name.
npm install -g ghosty            # or cargo install ghosty-cli ghosty-tui --locked
                                    # legacy Homebrew installs may still use
                                    # brew install deepseek-tui until the tap
                                    # formula is renamed.

# 3. Run with the new command.
ghosty doctor
ghosty
```

Your existing `~/.deepseek/config.toml`, `~/.deepseek/sessions/`,
`~/.deepseek/skills/`, `~/.deepseek/tasks/`, and `~/.deepseek/mcp.json` are
not deleted. New Ghosty Code installs prefer `~/.ghosty/`, and legacy
`~/.deepseek/` state remains a read fallback while you migrate. Existing
`DEEPSEEK_*` environment variables continue to work.

## What got renamed

| Surface | Before | After |
|---|---|---|
| CLI dispatcher binary | `deepseek` | `ghosty` |
| TUI runtime binary | `deepseek-tui` | `ghosty-tui` |
| npm wrapper package | `deepseek-tui` | `ghosty` |
| Crates.io crates | `deepseek-tui-cli` / `deepseek-tui` / `deepseek-*` | `ghosty-cli` / `ghosty-tui` / `ghosty-*` |
| Release assets | `deepseek-<platform>` / `deepseek-tui-<platform>` | `ghosty-<platform>` / `ghosty-tui-<platform>` |
| Checksum manifest | `deepseek-artifacts-sha256.txt` | `ghosty-artifacts-sha256.txt` |

## What changed for local state

New installs write product-owned state under `~/.ghosty/`. Existing
`~/.deepseek/` config, sessions, skills, tasks, MCP config, memory, and notes
remain readable as legacy fallbacks while you migrate. Ghosty Code never deletes
the legacy directory automatically.

## What did NOT change

Anything that targets the DeepSeek provider API stays exactly as it was:

- **Environment variables**: `DEEPSEEK_API_KEY`, `DEEPSEEK_BASE_URL`,
  `DEEPSEEK_MODEL`, `DEEPSEEK_PROVIDER`, `DEEPSEEK_PROFILE`, `DEEPSEEK_YOLO`,
  `DEEPSEEK_LOG_LEVEL`, plus the existing `DEEPSEEK_TUI_*` runtime knobs
  (`DEEPSEEK_TUI_BIN`, `DEEPSEEK_TUI_RELEASE_BASE_URL`, etc.). They're kept
  for backward compatibility; renaming them would break every shell rc on
  the planet.
- **Model IDs**: `deepseek-v4-pro`, `deepseek-v4-flash`, and the legacy
  aliases `deepseek-chat` and `deepseek-reasoner`.
- **Hosts**: `api.deepseek.com` (global) and `api.deepseeki.com` (China
  fallback).
- **GitHub repository URL**: `https://github.com/blissito/ghostycode`.
  The old `blissito/DeepSeek-TUI` URL redirects there during the transition.
- **Homebrew tap and formula** (`blissito/homebrew-ghosty`): still uses
  the legacy formula name for existing installs. Treat it as compatibility-only
  until the tap is renamed; new install docs prefer `ghosty` npm, Cargo,
  Docker, or direct downloads.
- **Docker image**: `ghcr.io/hmbown/ghosty`.

## Deprecation shims (through v0.8.x)

To keep existing shell aliases, scripts, and CI working through the rename,
v0.8.41 and later v0.8.x releases ship **deprecation shims**:

- A `deepseek` binary that prints a one-line warning to stderr and forwards
  argv to `ghosty`.
- A `deepseek-tui` binary that does the same for `ghosty-tui`.
- The legacy `deepseek-tui` npm package is deprecated and no longer receives
  new releases. Install the `ghosty` npm package instead.

These shims will be removed in **v0.9.0**. Please migrate before then.

## Migrating in practice

### npm

```bash
npm uninstall -g deepseek-tui
npm install -g ghosty
```

### Cargo

```bash
cargo uninstall deepseek-tui-cli deepseek-tui 2>/dev/null || true
cargo install ghosty-cli ghosty-tui --locked
```

Or in a checkout:

```bash
cargo install --path crates/cli --locked --force
cargo install --path crates/tui --locked --force
```

### Homebrew

The tap formula still installs through the legacy `deepseek-tui` name for
existing Homebrew users. Keep using `brew upgrade deepseek-tui` only for that
compatibility path. New installs should prefer npm, Cargo, Docker, or direct
downloads until the formula and tap repo are renamed.

### Manual / GitHub Releases

`v0.8.41` Releases attach **both** the canonical `ghosty-*` /
`ghosty-tui-*` assets and the legacy `deepseek-*` / `deepseek-tui-*`
shim assets. Existing `deepseek update` invocations on v0.8.40 keep working;
they land you on the deprecation shim, which then prompts the install of
`ghosty`.

A second checksum manifest, `deepseek-artifacts-sha256.txt`, is attached as
an alias of `ghosty-artifacts-sha256.txt` so v0.8.40's hardcoded lookup
still verifies.

### Sessions, skills, and manual workspaces

Renaming the binary does not require starting over:

- **Config**: on first launch, Ghosty Code copies `~/.deepseek/config.toml` to
  `~/.ghosty/config.toml` if the Ghosty Code file does not already exist.
  It never overwrites a newer Ghosty Code config. You can inspect the active path
  with `ghosty doctor`.
- **Sessions and tasks**: managed state is read from `~/.ghosty/...` when
  present, with `~/.deepseek/...` used as the legacy fallback when only the old
  directory exists. Existing saved sessions still appear in `ghosty sessions`
  and the TUI resume picker.
- **Skills**: Ghosty Code discovers workspace skills first, then global skills,
  including both `~/.ghosty/skills` and legacy `~/.deepseek/skills`. Existing
  skill directories with `SKILL.md` do not need to be rewritten.
- **MCP config**: the default path is `~/.ghosty/mcp.json`. If that file is
  absent, Ghosty Code still reads legacy `~/.deepseek/mcp.json`. To use a custom
  MCP config file, set `mcp_config_path` in `config.toml` or
  `DEEPSEEK_MCP_CONFIG`.
- **Manual binary installs**: keep the dispatcher and TUI binaries as siblings
  on your `PATH`: `ghosty` plus `ghosty-tui`. On Windows, the recommended
  user-local location is `%LOCALAPPDATA%\Programs\Ghosty Code\bin`. On Unix-like
  systems, any user-writable `PATH` directory is fine as long as both binaries
  are present.
- **Specified work directories**: running `ghosty` from a project directory,
  or launching it with a specific workspace path, does not move project files.
  Ghosty Code reads `<workspace>/.ghosty/config.toml` first and falls back to
  legacy `<workspace>/.deepseek/config.toml` when the new path is absent.

If both `~/.ghosty/...` and `~/.deepseek/...` copies exist, the Ghosty Code
path wins. Keep the legacy directory until you have confirmed `ghosty
doctor`, `ghosty sessions`, and your expected skills all show the same state.

## Why the name change

Ghosty Code is a shorter, terminal-friendlier handle for the same terminal
coding agent and the longer-term product direction: a DeepSeek-first agentic
terminal for open source and open-weight coding models. The project name,
command names, package names, release assets, Docker image, and CNB mirror move
to Ghosty Code; the official DeepSeek provider, model IDs, env vars, and
`~/.deepseek/` config surface remain first-class.

## Reporting issues with the rename

If your install broke during the migration, please open an issue at
<https://github.com/blissito/ghostycode/issues> and include:

- The output of `ghosty --version` (or `deepseek --version` if you're
  still on the shim).
- Which install path you used (npm, cargo, brew, manual).
- The exact command you ran and the full error output.

We'll prioritize migration regressions.
