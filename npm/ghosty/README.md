# ghosty

> The terminal coding agent for supported hosted and local models — open models first.

Ghosty is a Rust TUI and CLI for many model providers — DeepSeek,
OpenRouter, Hugging Face, and local vLLM/SGLang/Ollama are supported routes,
and it speaks natively to Anthropic Claude and OpenAI when that's what you have
— with approval-gated tools, OS sandboxing, side-git snapshots, and `/restore`
rollback.

This npm package is a small launcher: it downloads the matching native
Ghosty binaries for your platform, verifies them against the release
SHA-256 manifest, and installs `ghosty` plus the `ghosty-tui` convenience name.
Both names run the same compiled runtime. The application state and credentials
still live in Ghosty's normal config files, not inside `node_modules`.

> Previously published as `deepseek-tui`. See
> [docs/REBRAND.md](https://github.com/blissito/ghostycode/blob/main/docs/REBRAND.md)
> for the migration notes; the legacy `deepseek-tui` npm package is deprecated
> and receives no further releases.

## Install

```bash
npm install -g ghosty
# or
pnpm add -g ghosty
```

For project-local usage:

```bash
npm install ghosty
npx ghosty --help
```

`postinstall` tries to download platform binaries into `bin/downloads/`. On
Linux x64 it concurrently probes the GitHub Releases and CNB first-party
checksum manifests for this package version, locks the first source that
validates, and downloads binaries only from that source. If GitHub release
assets are temporarily unreachable, install continues and the wrapper retries
the download on first run.

## First run

```bash
ghosty auth set --provider deepseek
ghosty auth status
ghosty doctor
ghosty
```

Every provider is the same one-line shape — `--provider openrouter`,
`--provider huggingface`, `--provider ollama`, or `--provider anthropic` for a
Claude key; the full registry lives in
[docs/PROVIDERS.md](https://github.com/blissito/ghostycode/blob/main/docs/PROVIDERS.md).

The single runtime reads `~/.ghosty/config.toml` for auth and default model
settings. Legacy `~/.deepseek/config.toml` installs are still read as a
compatibility fallback. Common commands are available directly, including
`ghosty doctor`, `ghosty models`, `ghosty sessions`, and
`ghosty resume --last`.

## Supported platforms

Prebuilt binaries for the GitHub release are downloaded automatically:

- Linux x64
- Linux arm64
- macOS x64 / arm64
- Windows x64 / arm64
- Android arm64 / Termux (preview; requires matching Android assets in the
  selected GitHub Release)

The source-candidate wrapper recognizes Android arm64 and resolves the
Termux-native `ghosty` and `ghosty-tui` assets. That path works only for package
versions whose matching GitHub Release publishes both assets, and remains
preview support pending real-device QA. See the support table in
[docs/INSTALL.md](https://github.com/blissito/ghostycode/blob/main/docs/INSTALL.md).

HarmonyOS PC (`openharmony`) is treated as `linux`, so it gets the Linux
binaries matching your CPU architecture (x64 or arm64). Linux riscv64 prebuilts
are temporarily paused while the locked `rquickjs-sys` dependency lacks
`riscv64gc-unknown-linux-gnu` bindings. Other platform/architecture combinations
(FreeBSD, Linux riscv64, …) aren't shipped as prebuilts. Unsupported platforms,
checksum failures, and glibc compatibility problems still fail with a clear
error pointing you at the full
[docs/INSTALL.md](https://github.com/blissito/ghostycode/blob/main/docs/INSTALL.md)
guide.

## Wrapper configuration

| Setting | What it does |
| --- | --- |
| `ghostyBinaryVersion` in `package.json` | Default native binary version. `deepseekBinaryVersion` is still read as a backward-compat fallback. |
| `GHOSTY_RELEASE_BASE_URL` | Canonical override: use an internal or mirrored release-asset directory and skip the Linux x64 GitHub/CNB race. The directory must contain `ghosty-artifacts-sha256.txt` and the platform binaries. `DEEPSEEK_TUI_RELEASE_BASE_URL` and `DEEPSEEK_RELEASE_BASE_URL` are the implemented legacy fallbacks. |
| `GHOSTY_USE_CNB_MIRROR=1` | Force the CNB (China-friendly) first-party mirror on Linux x64 and OpenHarmony x64, skipping the automatic race. Other targets fail with a clear unsupported-mirror error; use GitHub or a complete `GHOSTY_RELEASE_BASE_URL` mirror there. Without this variable, Linux x64 still probes CNB and GitHub together and uses the first valid checksum manifest. |
| `GHOSTY_VERSION` | Override the release version to download. |
| `GHOSTY_GITHUB_REPO` | Override the source repo. Defaults to `blissito/ghostycode`. |
| `GHOSTY_FORCE_DOWNLOAD=1` | Force download even when the cached binary is already present. |
| `GHOSTY_DISABLE_INSTALL=1` | Skip install-time download. |
| `GHOSTY_OPTIONAL_INSTALL=1` | Make install-time retryable download failures warn and exit `0` instead of failing `npm install`. |
| `GHOSTY_QUIET_INSTALL=1` | Suppress installer progress messages. |
| `GHOSTY_DOWNLOAD_TIMEOUT_MS` | Override the total download budget. |
| `GHOSTY_DOWNLOAD_STALL_MS` | Override the no-progress stall budget. |
| `GHOSTY_SKIP_GLIBC_CHECK=1` | Bypass the Linux glibc preflight check at your own risk. |

The corresponding `DEEPSEEK_TUI_*` and `DEEPSEEK_*` names remain accepted as
legacy aliases, after the canonical Ghosty names.

### Proxies

Downloads respect `HTTPS_PROXY` / `HTTP_PROXY` (CONNECT tunneling included)
and `NO_PROXY`, so the wrapper works behind corporate proxies. For fully
offline installs, set `GHOSTY_DISABLE_INSTALL=1` or point
`GHOSTY_RELEASE_BASE_URL` at a local mirror.

## Release integrity

- `npm publish` runs a release-asset check to ensure the required binaries,
  archives, Windows installer, and checksum manifests exist for the target
  GitHub release before publishing.
- For the default GitHub Release source, `npm run release:check` also verifies
  that those release assets were updated by a successful `release.yml` run for
  the tag commit. When `GHOSTY_RELEASE_BASE_URL` or a legacy mirror override
  is set, it checks the mirror asset URLs and checksum manifests instead.
- Install-time downloads are verified against the release checksum manifest before
  the wrapper marks them executable.

## Links

- Repository: <https://github.com/blissito/ghostycode>
- Website: <https://ghosty.net/>
- Provider registry: [docs/PROVIDERS.md](https://github.com/blissito/ghostycode/blob/main/docs/PROVIDERS.md)
- Changelog: [CHANGELOG.md](https://github.com/blissito/ghostycode/blob/main/CHANGELOG.md)
