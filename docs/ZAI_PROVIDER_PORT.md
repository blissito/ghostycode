# Port: provider directo Z.AI (GLM) — fuente de verdad

> Objetivo: soportar el **token directo de Z.AI** (Coding Plan) con **cache
> completa** y thinking/reasoning al nivel de DeepSeek. NO improvisar: cada dato
> sale de upstream CodeWhale. También es el **playbook para agregar providers**
> (lo haremos seguido) → arquitectura de una-fila-por-provider.

## Valores exactos (de upstream, verificados)

| Dato | Valor | Fuente upstream |
|---|---|---|
| Base URL (Coding Plan) | `https://api.z.ai/api/coding/paas/v4` | `config/models.rs:145`, client test `client.rs:2383` |
| Modelo default | `GLM-5.2` | `DEFAULT_ZAI_MODEL` |
| Otros modelos | `GLM-5.1`, `GLM-5-Turbo` | `ZAI_GLM_5_1_MODEL`, `ZAI_GLM_5_TURBO_MODEL` |
| Aliases provider | `zai`, `z-ai`, `z_ai`, `z.ai`, `bigmodel` | serde aliases |
| Max concurrency | 3 | `DEFAULT_ZAI_PROVIDER_MAX_CONCURRENCY` |
| Env var key | `ZAI_API_KEY` (slot `"zai"`) | secrets slot |

### Cache + thinking (LO CRÍTICO — de `client.rs apply_reasoning_effort`)

- **enabled** (high/max/ultracode): `body["thinking"] = {"type":"enabled","clear_thinking":false}`
- **off**: `body["thinking"] = {"type":"disabled"}`

`clear_thinking:false` preserva `reasoning_content` entre turnos → prefijo
estable → **prefix-cache hits** (misma razón por la que DeepSeek rinde tanto).
Z.AI debe ir por **los mismos rieles genéricos** que DeepSeek en ghosty:
`parse_usage` (cached tokens), replay de `reasoning_content`, prompt-zones
(prefijo estable). Solo el `thinking` shape es provider-specific.

## Arquitectura objetivo (single source of truth)

Upstream ya movió esto a módulos dedicados (`crates/config/src/provider_defaults.rs`,
`crates/tui/src/config/models.rs`). Ghosty lo tiene disperso en ~40 match arms.
Dirección: **una función/tabla `provider_meta(ApiProvider) -> ProviderMeta`** con
`{ canonical, aliases, display, default_base_url, default_model, env_var,
config_slot }`. Las funciones de datos existentes delegan a esa tabla. Agregar un
provider = **1 fila** + variante de enum + (si aplica) arm de thinking en cliente.

## Etapas (build + test entre cada una; commit por etapa)

1. **Consts + enums + struct** — `ProviderKind::Zai` (config crate, +aliases +
   `zai: ProviderConfigToml`), `ApiProvider::Zai` (tui), consts de modelo/base_url,
   `DEFAULT_ZAI_PROVIDER_MAX_CONCURRENCY`.
2. **Datos (SoT)** — arms/tabla: base_url, default model, env var `ZAI_API_KEY`,
   slot `providers.zai`, display `Z.AI`, parse `"zai"`, `canonical_zai_model_id`.
3. **Cliente (cache/thinking)** — arms de `Zai` en `apply_reasoning_effort`
   (enabled/off exactos de upstream) + base_url Coding Plan. Confirmar que Zai
   reusa el path de `reasoning_content`/`parse_usage`/prompt-zones de DeepSeek.
4. **Registro de modelos** — `GLM-5.2/5.1/GLM-5-Turbo` como `ProviderKind::Zai`
   en `crates/agent/src/lib.rs` + context windows en `models.rs`.
5. **Wizard** — `submit_glm_key` escribe `provider="zai"` +
   `[providers.zai] api_key/base_url/model=GLM-5.2` (hoy escribe openrouter: BUG).
6. **Verificación** — build workspace, clippy `-D warnings`, `cargo test`,
   probar token directo end-to-end.

## Estado

- [ ] Etapa 1  · [ ] Etapa 2  · [ ] Etapa 3  · [ ] Etapa 4  · [ ] Etapa 5  · [ ] Etapa 6

## Nota

El wizard OpenRouter ya commiteado (`6004b7a`/`8b5008a`) queda **corregido** en
etapa 5 para escribir `zai` (directo), no `openrouter`. GLM vía OpenRouter sigue
disponible como modelos, pero el wizard apunta al token directo.
