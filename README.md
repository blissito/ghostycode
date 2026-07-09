<div align="center">

<img src="https://easybits-public.fly.storage.tigris.dev/699f35cbc8ad86037eda62b1/HGF" alt="Ghosty" width="160" />

# Ghosty Code

**DeepSeek V4 terminal coding agent &amp; constitutional harness.** 👻

[![CI](https://github.com/blissito/ghostycode/actions/workflows/ci.yml/badge.svg)](https://github.com/blissito/ghostycode/actions/workflows/ci.yml)

</div>

Ghosty Code is a **DeepSeek V4 terminal coding agent** and **constitutional harness** —
a Rust TUI that reads, edits, runs shell commands, searches your repo, and coordinates
sub-agents through long tool-using sessions with evidence-driven verification.
Built for developers who want a keyboard-first coding agent with MCP support,
session persistence, and zero vendor lock-in. Open source (MIT).

## Instalación

**Recomendado** — sin Node ni Rust, baja el binario precompilado:

```bash
curl -fsSL https://formmy.app/ghosty/install.sh | sh
```

### Alternativas

```bash
# npm (baja los binarios precompilados del release)
npm install -g ghostycode

# Cargo (requiere Rust 1.88+)
cargo install --git https://github.com/blissito/ghostycode ghosty-cli

# Descarga directa: archivos por plataforma en
# https://github.com/blissito/ghostycode/releases
```

El paquete de npm se llama **`ghostycode`**; el comando que instala es **`ghosty`**.

## Primer uso

```bash
ghosty auth set --provider deepseek --api-key "TU_DEEPSEEK_API_KEY"
# Alternativa: EasyBits (revendedor de DeepSeek; la misma key sirve para LLM y MCP)
ghosty auth set --provider easybits --api-key "TU_EASYBITS_API_KEY"
ghosty doctor    # verifica setup y conexión
ghosty           # abre la TUI interactiva
```

La config vive en `~/.ghosty/config.toml`. También puedes usar la variable de entorno
`DEEPSEEK_API_KEY`. Más providers y las notas de EasyBits en
[`docs/PROVIDERS.md`](docs/PROVIDERS.md).

## Comandos básicos

```bash
ghosty                                # TUI interactiva
ghosty "explica esta función"         # prompt de una sola vez
ghosty --model auto "arregla el bug"  # auto-selecciona modelo + thinking
ghosty --yolo                         # auto-aprueba herramientas
ghosty sessions                       # lista sesiones guardadas
ghosty resume --last                  # retoma la última sesión
ghosty models                         # lista modelos disponibles
ghosty update                         # actualiza el binario
```

## Modos

- **Agent** — ejecuta herramientas (editar, correr, buscar) con tu aprobación.
- **Plan** — propone un plan antes de tocar nada.
- **Yolo** (`--yolo`) — auto-aprueba todo. Úsalo con cuidado.

Cambia de modo dentro de la TUI o con flags al arrancar.

## Modelos DeepSeek V4

| Modelo | Thinking | Ideal para |
|--------|----------|------------|
| `deepseek-v4-pro` | ✅ | Razonamiento complejo, código, mates |
| `deepseek-v4-flash` | ❌ | Tareas rápidas y económicas |
| `auto` | — | Elige modelo + thinking según el turno |

Override con `--model <nombre>` o `/model` dentro de la TUI.

### Modelos abiertos vía OpenRouter

Además de DeepSeek, puedes rutear modelos abiertos con `provider = "openrouter"`
(`OPENROUTER_API_KEY`). Incluye la familia **GLM de Z.AI**:

| Modelo | Contexto | Ideal para |
|--------|----------|------------|
| `z-ai/glm-5.2` (alias `glm-5.2`) | 1M | Modelo GLM más capaz |
| `z-ai/glm-5.1` (alias `glm-5.1`) | 202K | GLM estándar |
| `z-ai/glm-5-turbo` (alias `glm-5-turbo`) | 202K | GLM rápido para explorar |

La lista completa de modelos OpenRouter (Qwen, Kimi, MiniMax, Gemma, etc.) está
en [`docs/PROVIDERS.md`](docs/PROVIDERS.md).

## MCP — easybits incluido por defecto

Ghosty trae preconfigurado el servidor MCP de **easybits** (gestión de archivos
desde el agente, 100+ herramientas). Viene **desactivado** de fábrica hasta que
añadas tu llave, así que una instalación nueva nunca falla por falta de credencial.

1. Consigue tu API key en el panel de desarrollador de easybits:
   **https://www.easybits.cloud/dash/developer**
2. Añádela (esto la activa):

   ```bash
   ghosty mcp add easybits --url "https://www.easybits.cloud/api/mcp?tools=core" --bearer TU_EASYBITS_API_KEY
   ```

3. Verifica: `ghosty mcp list`

> **¿Por qué `tools=core` y no `core,sandbox` ni `tools=all`?** EasyBits revende
> DeepSeek, cuya API (compatible con OpenAI) tiene un **tope duro de 128 tools por
> request**. EasyBits expone `core` = 65 tools y `sandbox` = 47; sumadas a las ~36
> built-in de Ghosty, `core` solo cabe (101 total) pero `core,sandbox` se pasa (146 >
> 128) y verás el error `Invalid 'tools': array too long ... maximum length 128`. Usa
> **un solo grupo**: `tools=core` (o `tools=sandbox` si solo necesitas ejecutar código).
> Con Anthropic no pasa porque sus tools MCP van como `defer_loading` (ToolSearch) y no
> cuentan en el tope; DeepSeek ignora ese flag y cuenta todas.

Gestiona otros servidores con `ghosty mcp add stdio|http <nombre> ...`,
`ghosty mcp enable|disable|remove <nombre>` y `ghosty mcp validate`.

## Más

- **Servidor**: `ghosty serve --http` (API HTTP/SSE) o `--mobile` (control desde el móvil en LAN).
- **Zed/ACP**: `ghosty serve --acp`.
- Otros proveedores compatibles con OpenAI vía `base_url` en la config.

## Related

- **[formmy.app/ghosty](https://formmy.app/ghosty)** — Ghosty on the web: run your agent from a browser dashboard.
- **[easybits.cloud](https://www.easybits.cloud)** — File management MCP server (100+ tools), pre-bundled with Ghosty.

## Licencia

MIT — ver [LICENSE](./LICENSE).
