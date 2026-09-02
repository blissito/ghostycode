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

> ### ⚡ Novedad (0.0.19) — ACP ya viaja por red
>
> Hasta ahora el agente sólo hablaba ACP por stdio, o sea que el cliente tenía que
> lanzar a Ghosty como proceso hijo: un navegador no puede, y una caja remota
> tampoco. `ghosty serve`, sin banderas, lo publica en `ws://host:port/acp`
> —WebSocket y Streamable HTTP, la RFD oficial— y atiende a varios clientes a la
> vez, cada uno con sus propias sesiones. El transporte es el del SDK oficial de
> ACP, no uno escrito a mano.
>
> Como esa vía ejecuta herramientas, exige token de runtime y política de origen:
> sin `Origin` sólo entra quien manda cabecera `Authorization`, que una página del
> navegador no puede fijar.
>
> Lo anterior, en el [CHANGELOG](CHANGELOG.md).

## Instalación

**Recomendado** — sin Node ni Rust, baja el binario precompilado:

```bash
curl -fsSL https://formmy.app/ghosty/install.sh | sh
```

### Alternativas

```bash
# npm (baja el binario precompilado del release)
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
# Kimi K3 directo (Moonshot; 1M contexto)
ghosty auth set --provider moonshot --api-key "TU_KEY_MOONSHOT"
# GLM-5.2 directo (Z.AI Coding Plan)
ghosty auth set --provider zai --api-key "TU_TOKEN_ZAI"
# EasyBits (revendedor de DeepSeek; la misma key sirve para LLM y MCP)
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

### GLM de Z.AI — directo o vía OpenRouter

Ghosty habla con la familia **GLM de Z.AI** por dos rutas:

**Directo (Z.AI Coding Plan)** — `provider = "zai"` (`ZAI_API_KEY`), endpoint
`api.z.ai/api/coding/paas/v4`. Paridad de cache y thinking con DeepSeek:

| Modelo | Contexto | Ideal para |
|--------|----------|------------|
| `GLM-5.2` (default) | — | Modelo GLM más capaz |
| `GLM-5.1` | — | GLM estándar |
| `GLM-5-Turbo` | — | GLM rápido para explorar |

**Vía OpenRouter** — `provider = "openrouter"` (`OPENROUTER_API_KEY`):
`z-ai/glm-5.2` (1M), `z-ai/glm-5.1` (202K), `z-ai/glm-5-turbo` (202K).

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

> **¿Por qué un solo grupo y no `core,sandbox`?** EasyBits revende DeepSeek, cuya
> API tiene un **tope duro de 128 tools por request** y las cuenta todas. `core` son
> 65 y `sandbox` 47; con las ~36 built-in de Ghosty, uno solo cabe y los dos juntos
> no (146 > 128 → `Invalid 'tools': array too long`). Ghosty te avisa antes de enviar
> la petición si te pasas.
>
> Para tener **cajas, S3 y base de datos a la vez**, usa el modo híbrido: conecta
> solo `?tools=sandbox` por MCP (crear una caja no tiene endpoint REST) y usa S3 y
> DB por `curl` contra `https://www.easybits.cloud/api/v2` con la misma key. Está
> paso a paso en [`docs/TALLER.md`](docs/TALLER.md).

Gestiona otros servidores con `ghosty mcp add stdio|http <nombre> ...`,
`ghosty mcp enable|disable|remove <nombre>` y `ghosty mcp validate`.

## Más

- **Servidor**: `ghosty serve --http` (API HTTP/SSE) o `--mobile` (control desde el móvil en LAN).
- **Editores**: `ghosty acp` (stdio, para el editor que lanza a Ghosty como hijo; `ghosty acp --http`
  lo sirve por red). El handshake expone proveedor, modelo y esfuerzo de razonamiento como
  `configOptions`, y cada turno reporta consumo de contexto y costo. En una caja de EasyBits,
  `EASYBITS_API_KEY=… sh scripts/acp-serve.sh` instala, arranca por red e imprime la URL `wss://`.
- **Agente en una caja**: `ghosty serve` a secas — ACP por red en `/acp`, API en `/v1/*` y
  `GET /health` en el mismo puerto 7878. Escucha en `127.0.0.1` y sólo `--open` o `--host`
  lo abren; dentro de un contenedor te avisa que el loopback del guest no lo alcanza nadie,
  pero no decide por ti. Detalles y seguridad en
  [`docs/RUNTIME_API.md`](docs/RUNTIME_API.md).
- Otros proveedores compatibles con OpenAI vía `base_url` en la config.

## Related

- **[formmy.app/ghosty](https://formmy.app/ghosty)** — Ghosty on the web: run your agent from a browser dashboard.
- **[easybits.cloud](https://www.easybits.cloud)** — File management MCP server (100+ tools), pre-bundled with Ghosty.

## Licencia

MIT — ver [LICENSE](./LICENSE).
