# RFC: Output presentation filters without receipt mutation

**Issue:** #4468

**Status:** Accepted for the v0.9.2 product boundary

**Date:** 2026-07-26

## Decision

Ghosty will not run an arbitrary user script between a model response and
the canonical session record. Model-native assistant and thinking blocks remain
the durable audit, replay, cache-accounting, debugging, and provider-signature
source of truth.

Output compression belongs to an explicit **presentation/export** layer. The
first supported controls are the existing safe surfaces:

- TUI `show_thinking = false` hides thinking from the rendered transcript but
  does not delete it from the canonical message/receipt path.
- `ghosty exec --output-format text|stream-json` selects a documented output
  encoding. Structured output retains block identity so downstream tools can
  select `thinking` or `text` without Ghosty rewriting either.
- Exports may add a future `--view canonical|response-only|thinking-only`
  selector. A filtered export must label itself as a derived view and retain a
  canonical session reference; it must never overwrite the session.

This addresses the accessibility and automation need behind the proposed
CIPHER filter without turning untrusted scripts into invisible transcript
editors.

## Why

### Receipt fidelity

The session is an audit record. Replacing content before persistence would make
it impossible to prove what the provider emitted, would corrupt signed
Anthropic thinking blocks, and could make usage/cost receipts disagree with the
visible record. Storing only the transformed form is rejected. Storing both
forms by default doubles sensitive data and creates ambiguous replay authority,
so it is also rejected.

### Prompt cache and replay

Presentation filters do not change request-side token use. In particular,
`reasoning_replay_tokens` and provider-specific signed-thinking replay must use
the canonical form. A compression claim must separately measure:

1. terminal/export bytes;
2. local storage bytes;
3. request-side replay tokens;
4. provider cache-hit behavior.

Only the first is affected by the accepted v0.9.2 boundary.

### Streaming

`HookEvent::ResponseDelta` is observer-only and arrives incrementally. A
block-level transformation would require buffering until block end, adding
latency and changing cancellation semantics. Presentation consumers may buffer
for their own output, but the engine continues to emit and persist canonical
deltas.

### Trust and failure

No new arbitrary command execution is added. An external consumer may read
`stream-json` and apply its own bounded transform outside Ghosty. Its failure
cannot corrupt, delay, or replace the session. The canonical record therefore
provides the fail-open source automatically.

## Structured-output contract

`stream-json` is the accessibility and integration surface:

- events are JSON lines;
- response/thinking block identity remains explicit;
- tools may omit a block from their derived view, but must not describe that
  view as the canonical session;
- no environment map, provider credential, hidden tool payload, or unrelated
  transcript content is added for filtering;
- downstream tools should bound input, output, and processing time themselves.

Example response-only presentation:

```sh
ghosty exec --output-format stream-json "..." \
  | jq -r 'select(.type == "message_delta") | .text // empty'
```

The exact event names are versioned runtime output and callers should inspect a
fixture from their installed version rather than infer fields from this RFC.

## Rejected alternatives

1. **Pre-persistence output hook.** Rejected: mutates audit/replay authority.
2. **Mutate only thinking.** Rejected: signed thinking and request replay still
   require fidelity.
3. **Store canonical plus transformed by default.** Rejected: duplicate
   sensitive content and unclear authority.
4. **Prompt the model to abbreviate.** The reporter measured 0% adoption and it
   is not a reliable mechanical contract.
5. **A separate `[hooks.output_filter]` table.** Rejected: duplicates the hook
   schema while failing to solve the trust and receipt problems.

## Future additive work

A future derived-export API may accept a declarative, non-executable selector
and write a receipt containing the source session id, source content hash,
selector, and derived output hash. Arbitrary executable transforms remain an
external pipeline unless a later security review defines sandboxing,
disclosure, latency, and dual-form retention semantics.

## Acceptance checks

- Canonical session persistence and reasoning replay remain unchanged.
- `show_thinking` is documented as display-only.
- `stream-json` is documented as the safe machine-readable filter boundary.
- No hook stdout gains response-mutation authority.
- No new script, shell, credential, or network capability is introduced.

Credit: the CIPHER measurements and the bounded stdin/stdout/fail-open proposal
came from @eugenicum in #4468. The v0.9.2 decision preserves that integration
use case while keeping Ghosty's canonical receipts trustworthy.
