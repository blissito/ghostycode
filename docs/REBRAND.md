# Rebrand: DeepSeek TUI → Ghosty

Starting with **v0.8.41**, this project ships under a new name: `ghosty`.

This document explains what changed, what didn't, and how to migrate. None of the
DeepSeek provider integration changed — only the local CLI / TUI brand.

## TL;DR

```bash
# 1. Uninstall the old wrapper or binaries.
npm uninstall -g deepseek-tui      # or:
cargo uninstall deepseek-tui-cli 2>/dev/null || true
cargo uninstall deepseek-tui 2>/dev/null || true
                                    # Homebrew:
                                    # brew upgrade ghosty

# 2. Install under the new name.
npm install -g ghosty            # or:
cargo install ghosty-cli --locked
                                    # Homebrew:
                                    # brew tap blissito/ghostycode
                                    # brew install ghosty

# 3. Run with the new command.
ghosty doctor
ghosty
```

Your existing `~/.deepseek/config.toml`, `~/.deepseek/sessions/`,
`~/.deepseek/skills/`, `~/.deepseek/tasks/`, and `~/.deepseek/mcp.json` are
not deleted. New Ghosty installs prefer `~/.ghosty/`, and legacy
`~/.deepseek/` state remains a read fallback while you migrate. Existing
`DEEPSEEK_*` environment variables continue to work.

## What got renamed

| Surface | Before | After |
|---|---|---|
| Installed commands | `deepseek` / `deepseek-tui` | `ghosty` / `ghosty-tui` |
| npm wrapper package | `deepseek-tui` | `ghosty` |
| Crates.io crates | `deepseek-tui-cli` / `deepseek-tui` / `deepseek-*` | `ghosty-cli` / `ghosty-tui` / `ghosty-*` |
| Release assets | `deepseek-<platform>` / `deepseek-tui-<platform>` | `ghosty-<platform>` / `ghosty-tui-<platform>`; `ghosty-tui-<platform>` remains a compatibility-only filename |
| Checksum manifest | `deepseek-artifacts-sha256.txt` | `ghosty-artifacts-sha256.txt` |

## What changed for local state

New installs write product-owned state under `~/.ghosty/`. Existing
`~/.deepseek/` config, sessions, skills, tasks, MCP config, memory, and notes
remain readable as legacy fallbacks while you migrate. Ghosty never deletes
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
- **Hosts**: `api.deepseek.com` (global). The legacy typo host
  `api.deepseeki.com` is not an official DeepSeek endpoint; it is only
  still accepted in URL heuristics for existing configs and is not
  offered as a fallback (#1079).
- **GitHub repository URL**: `https://github.com/blissito/ghostycode`.
  The old `Hmbown/DeepSeek-TUI` URL redirects there during the transition.
- **Homebrew tap and formula**: the formula is `ghosty`. The tap GitHub
  repo is still `blissito/homebrew-ghosty` until it is renamed;
  `brew tap blissito/ghostycode && brew install ghosty` is the current
  path. The legacy `deepseek-tui` formula remains a deprecated alias for
  one overlap release.
- **Docker image**: `ghcr.io/blissito/ghostycode`.

## Deprecation shims (removed in v0.9.0)

To keep existing shell aliases, scripts, and CI working through the rename,
v0.8.41 and later v0.8.x releases shipped **deprecation shims**:

- A `deepseek` binary that prints a one-line warning to stderr and forwards
  argv to `ghosty`.
- A `deepseek-tui` binary that does the same for `ghosty-tui`.
- The legacy `deepseek-tui` npm package is deprecated and no longer receives
  new releases. Install the `ghosty` npm package instead.

These binary shims are removed in **v0.9.0**. DeepSeek provider support, model
IDs, `DEEPSEEK_*` environment variables, and legacy `~/.deepseek/` state
fallbacks remain supported.

## Migrating in practice

### npm

```bash
npm uninstall -g deepseek-tui
npm install -g ghosty
```

### Cargo

```bash
cargo uninstall deepseek-tui-cli 2>/dev/null || true
cargo uninstall deepseek-tui 2>/dev/null || true
cargo install ghosty-cli --locked
```

Or in a checkout:

```bash
cargo install --path crates/cli --locked --force
```

Cargo installs the canonical `ghosty` command. Release/npm/Homebrew
installers also provide the byte-identical `ghosty-tui` short name; Cargo users can
add an optional `ghosty-tui` symlink beside `ghosty`.

### Legacy `deepseek update`

Current v0.8.x compatibility binaries recognize when they are running under a
legacy `deepseek` or `deepseek-tui` filename. In that case, `deepseek update`
or `deepseek-tui update` downloads the canonical Ghosty release assets and
installs them beside the legacy binary as `ghosty` and `ghosty-tui` when
the install directory is writable. That describes the historical v0.8
compatibility updater, not the current install surface; after upgrading, use
`ghosty` or `ghosty-tui`.

If that update path cannot write to the install directory, use the npm, Cargo,
Homebrew, or manual reinstall commands above. The legacy npm package
`deepseek-tui` remains deprecated and is not republished; npm users should move
to `npm install -g ghosty`.

### Homebrew

**Current published state (v0.9.10; workspace source candidate v0.9.11):** The
formula is `ghosty`. New installs:

```bash
brew tap blissito/ghostycode
brew install ghosty
brew upgrade ghosty
```

The tap GitHub repo is still `blissito/homebrew-ghosty` until it is
renamed to `blissito/homebrew-ghosty` (then `brew tap Hmbown/ghosty`
works; the old tap name keeps working through GitHub's redirect). The
legacy `deepseek-tui` formula remains a deprecated alias for this overlap
release so existing `brew upgrade deepseek-tui` crontabs keep working.

**Remaining rollout:**

1. Rename the tap repo to `blissito/homebrew-ghosty` when adding
   `HOMEBREW_TAP_PAT`, then tell Ghostybot.
2. After one more minor release, remove the `deepseek-tui` alias.

### Manual / GitHub Releases

`v0.8.41` through `v0.8.x` Releases attached the canonical `ghosty-*` /
`ghosty-tui-*` assets (plus `ghosty-tui-*` from v0.8.66 onward) and
compatibility-only `deepseek-*` / `deepseek-tui-*` shim assets. Starting in
v0.9.0, Releases attach the current `ghosty-*` / `ghosty-tui-*` assets, the
`ghosty-artifacts-sha256.txt` checksum manifest, and byte-identical
`ghosty-tui-*` compatibility filenames required by legacy update clients.
Those compatibility filenames are not a third installed command. Install or
update through `ghosty` before moving to v0.9.0.

### Sessions, skills, and manual workspaces

Renaming the binary does not require starting over:

- **Config**: on first launch, Ghosty copies `~/.deepseek/config.toml` to
  `~/.ghosty/config.toml` if the Ghosty file does not already exist.
  It never overwrites a newer Ghosty config. You can inspect the active path
  with `ghosty doctor`.
- **Sessions and tasks**: managed state is read from `~/.ghosty/...` when
  present, with `~/.deepseek/...` used as the legacy fallback when only the old
  directory exists. Existing saved sessions still appear in `ghosty sessions`
  and the TUI resume picker.
- **Skills**: Ghosty discovers workspace skills first, then global skills,
  including both `~/.ghosty/skills` and legacy `~/.deepseek/skills`. Existing
  skill directories with `SKILL.md` do not need to be rewritten.
- **MCP config**: the default path is `~/.ghosty/mcp.json`. If that file is
  absent, Ghosty still reads legacy `~/.deepseek/mcp.json`. To use a custom
  MCP config file, set `mcp_config_path` in `config.toml` or
  `DEEPSEEK_MCP_CONFIG`.
- **Manual binary installs**: keep the two current command files together on
  your `PATH`: `ghosty` and `ghosty-tui`. On Windows, the
  recommended user-local location is `%LOCALAPPDATA%\Programs\GhostyCode\bin`.
  On Unix-like systems, any user-writable `PATH` directory is fine as long as
  both commands are present. Do not install a compatibility-only
  `ghosty-tui-*` release filename as a third command.
- **Specified work directories**: running `ghosty` from a project directory,
  or launching it with a specific workspace path, does not move project files.
  Ghosty reads `<workspace>/.ghosty/config.toml` first and falls back to
  legacy `<workspace>/.deepseek/config.toml` when the new path is absent.

If both `~/.ghosty/...` and `~/.deepseek/...` copies exist, the Ghosty
path wins. Keep the legacy directory until you have confirmed `ghosty
doctor`, `ghosty sessions`, and your expected skills all show the same state.

### If sessions appear missing after an upgrade

Run `ghosty doctor` before copying or deleting anything. Doctor compares
top-level session JSON **filenames and filesystem metadata only** between
`~/.deepseek/sessions/` and `~/.ghosty/sessions/`. It does not read chat
contents, traverse `checkpoints/`, or modify either directory. The JSON form
exposes the same result at `legacy_state.session_recovery`.

If doctor lists recoverable filenames:

1. Back up both session directories (if present) and close other Ghosty
   processes.
2. Run `ghosty sessions`. This invokes the existing additive migration,
   which creates only missing destination files, never overwrites a file that
   already exists under `~/.ghosty/sessions/`, skips checkpoint internals,
   and leaves every legacy original in place.
3. Rerun `ghosty doctor`, then confirm the sessions appear with `ghosty
   sessions`. If any filenames remain listed, keep both backups and report the
   listed source/destination filenames without sharing chat contents.

An explicit `GHOSTY_HOME` intentionally isolates that home and disables the
ambient `~/.deepseek` fallback. Doctor will not inspect the ambient legacy home
in that mode. To diagnose the default home without changing the isolated one,
use a separate shell with `GHOSTY_HOME` unset and rerun `ghosty doctor`.

## Why the name change

Ghosty is a shorter, terminal-friendlier handle for the same terminal
coding agent and the longer-term product direction: an agentic terminal for
open source and open-weight coding models, with DeepSeek — the provider the
project started with — remaining first-class alongside every other provider. The project name,
command names, package names, release assets, Docker image, and CNB mirror move
to Ghosty; the official DeepSeek provider, model IDs, env vars, and
`~/.deepseek/` config surface remain first-class.

## Reporting issues with the rename

If your install broke during the migration, please open an issue at
<https://github.com/blissito/ghostycode/issues> and include:

- The output of `ghosty --version` (or `deepseek --version` if you're
  still on the shim).
- Which install path you used (npm, cargo, brew, manual).
- The exact command you ran and the full error output.

We'll prioritize migration regressions.
