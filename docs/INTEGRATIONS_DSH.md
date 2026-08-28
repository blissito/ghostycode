# DeepSeek Harness connected through Ghosty

`ghosty integrations dsh …` connects a user's **existing** official DeepSeek
Harness installation (`dsh`, npm `@deepseek-ai/dsh`) to their Ghosty setup.
DSH stays an integrated harness surface. Ghosty remains the owner of Fleet
configuration, provider/model selection, permissions, credentials, and
lifecycle authority; DSH is not a second Fleet scheduler and never an
authority bypass.

Verified against `dsh 0.1.0-rc.6` (the latest published release at the time
of writing). DSH is a developer preview that warns of compatibility-breaking
changes; a newer `dsh` is reported as `stale-version` (launchable, unverified),
an older one or one without `--patch` as `incompatible`.

## What is (and is not) connected

Ghosty uses only DSH's documented seams:

| Seam | How Ghosty uses it |
| --- | --- |
| `dsh --version` / `dsh --help` | read-only detection (never initializes a profile) |
| `$DSH_HOME` (or `~/.dsh`) | read-only inventory: profile names, `settings.yaml` top-level namespaces, whether `.credentials.yaml` exists and is `0600`. Values are never read. |
| `--patch <file>` overlay | Ghosty writes **one** overlay under its own home and passes it at launch |
| `DSH_PERMISSION_MODE` env | mirrors the Ghosty permission posture |
| `--profile web` / `--profile headless` | the two shipped DSH profiles; DSH initializes them itself on first launch (its own documented behavior) |

Ghosty writes **only** under `$GHOSTY_HOME/integrations/dsh/` (plus,
with the opt-in plugin path below, whatever `dsh plugin` itself writes into
the dedicated `ghosty` DSH profile):

- `ghosty.patch.yml` — the overlay. Identity only: provider route, model,
  base URL, and (native DeepSeek route) `reasoningEffort`. For every
  non-native route it declares a `ghosty-<provider>` route on DSH's
  `llm-pi-ai` adapter, naming that route's own wire dialect under `api:`
  (`openai-completions`, `openai-responses`, or `anthropic-messages`) and
  `apiKeyEnv` naming the provider's canonical environment variable — the
  *name*, never the value. Keyless local routes (loopback Ollama / LM Studio
  / vLLM / SGLang) carry no credential reference.
- `receipt.json` — the current connection record plus an append-only history
  of `connect` / `update` / `disable` / `enable` / `remove` events with the
  overlay SHA-256, dsh version, `$DSH_HOME`, mapped identity, permission mode,
  and timestamps (see `docs/RECEIPTS.md`). Every event is also appended to
  `$GHOSTY_HOME/audit.log`.
- `bundle/` — only after `install-bundle`; see below. The Ghosty palette
  (skin) and the ambient ocean scene live here, in the bundle's client half —
  no stylesheet is exported.

Ghosty **never**:

- copies, prints, or embeds API keys, OAuth documents, environment secrets,
  prompts, or filesystem contents (a `--api-key`/keyring credential Ghosty
  itself materialized into the process is stripped from the launched child;
  a key the user exported in their own shell is left alone);
- writes to `$DSH_HOME` (settings, credentials, profiles, sessions);
- edits installed `@deepseek-ai/dsh` package files;
- switches to a cloud model or broadens permissions silently. Ghosty
  `read-only` → DSH `read-only`; anything else → `workspace-write`;
  `danger-full-access` only with `--allow-full-access` **and** a Ghosty
  full-access posture (`sandbox_mode = "danger-full-access"` / yolo).

## States

| State | Meaning | Launch |
| --- | --- | --- |
| `not-installed` | `dsh` not on `PATH` | refused |
| `offline` | `dsh` exists but `--version` failed | refused |
| `incompatible` | older than 0.1.0-rc.6 or no `--patch` | refused |
| `detected` | usable dsh, no Ghosty overlay | refused (`connect` first) |
| `connected` | overlay matches the current Ghosty route | allowed |
| `stale-config` | route changed, overlay edited outside Ghosty, or missing | refused (`update`) |
| `stale-version` | connected, but dsh is newer than verified | allowed, unverified |
| `disabled` | overlay kept, launches refused | refused (`enable`) |

`status`, `plan`, `/setup tools` (Tools and MCP step) and `ghosty doctor`
are side-effect free.

## Commands

```bash
ghosty integrations dsh status [--json]
ghosty integrations dsh plan [--profile web|headless] [--allow-full-access] [--skin] [--json]
ghosty integrations dsh connect [--profile web|headless] [--allow-full-access] [--skin] [--yes]
ghosty integrations dsh update  [--profile …] [--allow-full-access] [--skin true|false] [--ocean true|false] [--yes]
ghosty integrations dsh launch  [--profile web|headless] [--dry-run] [-- <dsh app args>]
ghosty integrations dsh disable
ghosty integrations dsh enable
ghosty integrations dsh remove [--yes]
ghosty integrations dsh install-bundle [--app web|headless] [--yes]
ghosty integrations dsh remove-bundle [--yes]
```

`connect`, `update`, and `remove` print the exact plan (files, identity,
permission mode, disclosures, and the overlay text) and require confirmation
(`--yes` when stdin is not a terminal). `launch` runs
`DSH_PERMISSION_MODE=<mode> dsh --profile <p> --patch <overlay> …` in the
Ghosty workspace with the user's own `$DSH_HOME`, so their credentials,
sessions, and profiles remain theirs.

### Disclosures the plan makes

- DSH layers the user's `settings.yaml` sections (`agent-default-model`,
  `llm-deepseek`, `llm-pi-ai`) over the overlay per field. If those sections
  exist, DSH's saved selection can shadow the pinned identity until it is
  cleared in DSH; `status`/`plan` list them.
- Reasoning tiers are mapped only for the native DeepSeek route
  (`off|high|max`); hand-declared routes send no effort parameter.
- Wire dialects are carried, never approximated: a Chat Completions route
  declares `api: openai-completions`, an OpenAI Responses route (e.g. the
  default `deepseek/deepseek-v4-flash`) declares `api: openai-responses`,
  and an Anthropic Messages route declares `api: anthropic-messages`. This
  follows the installed adapters' own declarations (verified against
  `@deepseek-ai/dsh@0.1.0-rc.6`): `@deepseek-ai/dsh-llm-deepseek` — the
  `deepseek-official` route — speaks chat completions only (its single wire
  call posts to `<baseURL>/chat/completions`, with no protocol switch),
  while `@deepseek-ai/dsh-llm-pi-ai`'s hand-declared route schema accepts
  exactly `openai-completions | openai-responses | anthropic-messages` for
  `api:`. So DeepSeek chat routes ride the native adapter (with reasoning
  tiers), and every other dialect — including DeepSeek's own
  Responses-dialect models — rides a hand-declared `ghosty-*` pi-ai
  route in its own dialect.
- What is refused: base URLs that embed credentials (userinfo or
  query/fragment material) are never copied into the overlay; `plan` fails
  naming the current `provider/model` and the reason, and `status` shows
  carry-ability for the current route before `plan` is ever run.

## The DSH plugin path (`install-bundle`)

`--patch` is Ghosty's default because it needs nothing but the launcher.
The **documented DSH plugin mechanism** is available as an explicit opt-in:

```bash
ghosty integrations dsh install-bundle [--app web|headless] [--yes]
ghosty integrations dsh remove-bundle [--yes]
```

`install-bundle` requires an existing connection and `pnpm` on `PATH` (dsh
shells out to it); without pnpm the status reads
`plugin path: not available: pnpm missing …` and the command refuses. It:

1. materializes an npm-shaped bundle package under
   `$GHOSTY_HOME/integrations/dsh/bundle/` — `package.json`
   (`ghosty-dsh-bundle`, private, MIT, version
   `<ghosty version>+dsh.<patch sha12>`, `"dsh": {"bundle": {"patch":
   "./cordis.patch.yml"}}`), `cordis.patch.yml` (the identity overlay,
   plus one trailing skin insert row when the skin is on — see below),
   `README.md`, `NOTICE.md` (DSH MIT notice retained), and, with the skin
   on, `lib/index.js` + `lib/client.js` (the palette plugin, with the ocean
   scene spliced in unless `--ocean false`);
2. runs the documented `dsh plugin --profile ghosty add <path>` twice: first
   for DSH's own shipped app bundle (`@deepseek-ai/dsh-web-app` or
   `dsh-headless`, linked from the installed launcher so the profile can boot;
   no network), then for the Ghosty bundle so its rows patch last. DSH
   creates the **dedicated** profile `$DSH_HOME/profiles/ghosty`
   (`package.json` with `link:` dependencies, `pnpm-lock.yaml`,
   `node_modules` links). The user's `web`/`headless` profiles are never
   touched;
3. records an `install_bundle` receipt (profile dir, bundle dir, package
   version, patch SHA-256, app bundle source, pnpm version, SHA-256 digest of
   the `dsh plugin` output — the output text itself is not stored).

Afterwards `dsh --profile ghosty` alone carries the identity (verified with
`dsh --profile ghosty --dump-config`), and `launch` prefers that profile
without `--patch`; `launch --profile web|headless` still uses the overlay.
Because the profile dependency is a `link:` to the Ghosty-owned directory,
`update` regenerates `cordis.patch.yml` (and the skin files) in place — no
pnpm run. Stale detection covers the bundle: a modified or missing bundle
patch, a bundle that no longer matches the overlay, a `lib/client.js` that
is missing, modified, present while the receipt says the skin is off, or
carrying/lacking the ocean scene against the receipt's `ocean` decision, or
a profile manifest that stopped listing `ghosty-dsh-bundle` all report
`stale-config`.

`remove-bundle` runs `dsh plugin --profile ghosty remove
ghosty-dsh-bundle` and deletes only the Ghosty-owned bundle files. The
profile directory itself (and the app bundle link dsh recorded there) is
DSH-owned and is left in place; the receipt says so. `remove` refuses while a
bundle is installed.

## Skin (bundle profile, `overrideTokens`)

DSH 0.1.0-rc.6 has one documented token-level theming seam:
`ThemeService.overrideTokens(source, tokens)` in
`@deepseek-ai/dsh-client-ui-theme`, which stacks a partial `--dsw-alias-*`
layer over the active theme (per-token, later layers win) and returns a
disposer. That is the mechanism the Ghosty skin uses. It is **applied only
through the bundle profile** (`dsh --profile ghosty`); the `--patch`
overlay never carries skin code, so `launch --profile web|headless` stays
overlay-only and stock-themed.

`install-bundle` turns the skin **on by default**. With the skin on, the
bundle is a dual-face DSH plugin:

- `package.json` gains `"dsh": {"client": {"platform": "web", "immediately":
  true, "inject": ["@deepseek-ai/dsh-client-ui-theme"]}}` and
  `"exports": {".": …, "./client": …, "./package.json": …}` (Node exports maps are exhaustive; the loader imports the bare name and dsh-client-modules resolves `<name>/package.json`);
- `lib/index.js` is a no-op Node cordis entry (so the row mounts) and
  `lib/client.js` is a plain `window.__ModuleLoader__.load({ id, factory })`
  script whose factory calls
  `ctx.theme.overrideTokens("ghosty-dsh-bundle", TOKENS)` inside
  `ctx.effect` and returns the disposer (`inject: ["theme"]` defers it until
  the theme service exists);
- `cordis.patch.yml` ends with
  `- insert: [{ id: ghosty-skin, name: ghosty-dsh-bundle }]` after the
  identity rows.

`TOKENS` is a bounded map of `--dsw-alias-*` names (backgrounds, borders,
brand, buttons, labels, error/success/warn states, code blocks, scrollbar,
toast, tooltip) onto light/dark values rendered from the TUI's real palette
(`crates/tui/src/palette`, Blue Stage dark and light) — palette constants
only, no user data or environment. The receipt records `skin: true|false`
and `skin_sha256` (SHA-256 of the rendered `TOKENS` JSON); `package.json`
carries the same hash under `ghosty.skin_sha256`.

### Whale Brothers / Ghosty identity

The skin mounts a small plugin-owned lockup in the top-right corner that says
`WHALE BROTHERS`, `GHOSTY`, and `× DEEPSEEK HARNESS`. It is additive: it
registers through DSH's frame-wide `shell.overlay` slot and does not replace or
rewrite DeepSeek Harness branding or controls. The lockup uses the active skin
tokens, ignores pointer input, collapses to a compact whale mark below 760 px,
and is removed with the client plugin.
`package.json` records the generated fragment as `ghosty.brand_sha256`.

### Ocean scene (whales and glyph fish)

With the skin on, `lib/client.js` also carries an ambient ocean: a
full-viewport `<canvas>` (`position: fixed; inset: 0; z-index: -1;
pointer-events: none`, painted below `#root` and above the body background)
with a visible depth gradient, one near and one far whale silhouette (blunt
head, low dorsal hump, long pectoral flipper, horizontal fluke flexing ±10°)
gliding slowly across on a gentle sine, biased to the lower half and the top
edge so they never cross the composer card, an occasional short spout of
bubbles from the head, a small school of Ghosty glyph fish (`><>` /
`><o>` in the code font, flocking-lite behind a wandering leader) and faint
rising bubbles. The
palette is the skin's own (`surface_bg`, `accent_primary`, `text_body`,
`text_dim` for light and dark); the scene follows DSH's `theme/change` event
so it flips with the app.

To let the canvas show through, the client re-issues two background tokens
as translucent rgba over the opaque table while the scene is on:
`--dsw-alias-bg-base` (α 0.42; the frame and the centre column both paint
it) and `--dsw-specific-sidebar-fill` (α 0.78, keeping navigation distinct).
Panels,
composer, code blocks and every other layer stay opaque. Verified live on
dsh 0.1.0-rc.6 in both schemes: no console errors, frames differ, and text
stays legible (see `docs/design/assets/dsh-ocean-{light,dark}.png`).

Budget: `requestAnimationFrame` capped at ~30 fps, paused while
`document.hidden`, one static frame under `prefers-reduced-motion: reduce`,
device-pixel-ratio aware, no per-frame allocations (typed arrays reused).
The scene ships inside `client.js` because dsh-client-modules serves exactly
one file per client plugin (`/plugins/<id>/client.js`); there is no
`lib/scene.js`. `package.json` records `ghosty.ocean` and
`ghosty.ocean_scene_sha256`; the receipt records `ocean: true|false`.

Off switches, smallest first: in the browser `localStorage["ghosty.ocean"]
= "off"` (or body class `ghosty-ocean-off`) skips both the canvas and the
translucent tokens on that machine; `window.__ghostyOcean.stop()` /
`.start()` / `.setIntensity(0..1)` are exposed for the console; and
`ghosty integrations dsh update --ocean false` regenerates `client.js`
without the scene (default on; a bare `update` keeps the previous choice;
`--skin false` implies no scene).

Escape hatch: `ghosty integrations dsh update --skin false` regenerates
the bundle without the client half and without the insert row (no pnpm run;
the `link:` dependency picks the files up in place); `update --skin true`
turns it back on, and a bare `update` keeps the previous choice.
`install-bundle` itself takes no `--skin` flag. `connect --skin` / `plan
--skin` record the same decision ahead of a later bundle install and write
no extra files. `remove-bundle` deletes the client half with the rest of the
Ghosty-owned bundle files, and the `overrideTokens` layer is disposed
with the plugin, so stock DSH theming returns.

The 0.9.8 `--skin` CSS/preview export (`ghosty-dsh-skin.css`,
`ghosty-dsh-skin-preview.html`) is gone: `dsh-client-ui-layout` writes
the alias tokens as inline `body.style` properties, so any stylesheet rule
lost to them by construction. `connect`/`update` delete those leftover files
if present.

## Removal

`remove` deletes only the overlay (and any 0.9.8 skin/preview leftovers)
under `$GHOSTY_HOME/integrations/dsh/`, appends a `remove` receipt, and
never touches `$DSH_HOME` or the installed package. DSH keeps working exactly as
before the connection.

## Attribution

DeepSeek Harness is © 2026 DeepSeek, MIT licensed; the integration invokes the
installed launcher and does not redistribute it. This is not native Ghosty
functionality: every surface labels it "DeepSeek Harness connected through
Ghosty".
