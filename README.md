<div align="center">

```
   .------.
  /        \
 ──(o)⌒(o)──
 |          |
 \___/\__/\_/
```

# Ghosty Code

**Agente de código para tu terminal, sobre DeepSeek V4.** 👻

[![CI](https://github.com/blissito/ghostycode/actions/workflows/ci.yml/badge.svg)](https://github.com/blissito/ghostycode/actions/workflows/ci.yml)

</div>

Ghosty Code es una TUI (interfaz de terminal) que conversa con modelos DeepSeek V4 para
ayudarte a programar: lee y edita archivos, corre comandos, busca en tu repo y mantiene
sesiones largas. Escrito en Rust.

## Instalación

```bash
# npm (lo más fácil — baja los binarios precompilados del release)
npm install -g ghostycode

# Cargo (sin Node — requiere Rust 1.88+)
cargo install --git https://github.com/blissito/ghostycode ghosty-cli

# Descarga directa: archivos por plataforma en
# https://github.com/blissito/ghostycode/releases
```

El paquete de npm se llama **`ghostycode`**; el comando que instala es **`ghosty`**.

## Primer uso

```bash
ghosty auth set --provider deepseek --api-key "TU_DEEPSEEK_API_KEY"
ghosty doctor    # verifica setup y conexión
ghosty           # abre la TUI interactiva
```

La config vive en `~/.ghosty/config.toml`. También puedes usar la variable de entorno
`DEEPSEEK_API_KEY`.

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

## Más

- **MCP**: `ghosty mcp list` / `ghosty mcp validate` para servidores MCP.
- **Servidor**: `ghosty serve --http` (API HTTP/SSE) o `--mobile` (control desde el móvil en LAN).
- **Zed/ACP**: `ghosty serve --acp`.
- Otros proveedores compatibles con OpenAI vía `base_url` en la config.

## Licencia

MIT — ver [LICENSE](./LICENSE).
