# ghostycode

**Ghosty Code** — agente de código para tu terminal, sobre DeepSeek V4. 👻

Instala y corre Ghosty Code desde los binarios precompilados (Rust) publicados en GitHub Releases.

## Instalación

```bash
npm install -g ghostycode
# o
pnpm add -g ghostycode
```

Uso local en un proyecto:

```bash
npm install ghostycode
npx ghostycode --help
```

El `postinstall` baja los binarios de tu plataforma a `bin/downloads/` y expone los
comandos **`ghosty`** y **`ghosty-tui`**. Si los assets del release no están disponibles
en ese momento, la instalación continúa y el wrapper reintenta la descarga al primer uso.

## Primer uso

```bash
ghosty auth set --provider deepseek --api-key "TU_DEEPSEEK_API_KEY"
ghosty doctor
ghosty
```

La config vive en `~/.ghosty/config.toml` (también lee la variable `DEEPSEEK_API_KEY`).

## Comandos básicos

```bash
ghosty                                # TUI interactiva
ghosty "explica esta función"         # prompt de una sola vez
ghosty --model auto "arregla el bug"  # auto-selecciona modelo + thinking
ghosty --yolo                         # auto-aprueba herramientas
ghosty sessions / resume --last       # sesiones guardadas
ghosty update                         # actualiza el binario
```

## MCP — easybits incluido por defecto

Ghosty trae preconfigurado el servidor MCP de **easybits** (gestión de archivos
desde el agente, 100+ herramientas), **desactivado** hasta que añadas tu llave.

1. Consigue tu API key: **https://www.easybits.cloud/dash/developer**
2. Añádela (esto la activa):

   ```bash
   ghosty mcp add easybits --url "https://www.easybits.cloud/api/mcp?tools=all" --bearer TU_EASYBITS_API_KEY
   ```

3. Verifica con `ghosty mcp list`.

Código y documentación: https://github.com/blissito/ghostycode

## Licencia

MIT
