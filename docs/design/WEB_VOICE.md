# Website voice

How ghosty.net copy is written. Companion to `docs/VOICE.md` (the
terminal charter); this sheet covers the marketing and docs-portal surface
in `web/`. Reference points: opencode.ai and pi.dev — short declarative
sentences, one concrete fact each, install command above the fold, providers
and models named rather than described.

1. One idea per sentence. If a sentence has a semicolon, it is two sentences.
2. Lead with what it is and what it does: "an open-source coding agent for
   your terminal", "45 providers", "MIT". Numbers come from
   `web/lib/facts.generated.ts`; never typed by hand.
3. Verbs over nouns. "It reads your code and edits files", not "code
   inspection and file mutation under approval boundaries".
4. No self-narration about honesty. Do not write "truthful", "honest",
   "real", "evidence", "recorded", "labeled as such", "deliberately". Say the
   fact; the reader judges.
5. No internal vocabulary on marketing pages: "source candidate", "provider
   route", "permission posture", "constitution", "law", "receipt", "ledger",
   "dogfood", "dispatcher". Use the product's own control names when a name
   is needed (Plan / Work / Operate, Ask / Auto-Review / Full Access).
6. Name things by what the reader sees, not the mechanism: "runs on your
   machine", "asks before it acts", "read-only", "unreleased".
7. No stacked qualifiers. One hedge at most, and only when the fact needs it
   ("usually no API key").
8. Adjectives do not carry claims. Cut "concise", "durable", "explicit",
   "well-bounded", "practical" unless the noun cannot stand without them.
9. Install is a command, not a paragraph. Show the command, then one line on
   what it puts where.
10. Provider and model names are proper nouns; keep them exact and literal
    (DeepSeek, OpenRouter, Ollama, vLLM, SGLang). Commands, flags, paths, and
    key names stay in code style.
11. Community copy names actions a person can take today: file an issue,
    open a pull request, translate a page. No mission statements.
12. Every locale says the same thing. A copy change lands in `en` and `zh`
    together; partial locales keep key parity and fall back for page bodies.
