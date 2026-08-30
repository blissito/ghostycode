# ghostycode

**Ghosty Code** — agente de código para tu terminal. Corre sobre **DeepSeek V4**,
**Kimi K3** (Moonshot) o **GLM-5.2** (Z.AI Coding Plan). 👻

Instala y corre Ghosty Code desde los binarios precompilados (Rust) publicados en GitHub Releases.

> **Novedad (0.0.19)** — **ACP ya viaja por red.** Hasta ahora el agente sólo hablaba ACP
> por stdio, o sea que el cliente tenía que lanzar a Ghosty como proceso hijo: un navegador
> no puede, y una caja remota tampoco. `ghosty serve --acp --acp-http` lo publica en
> `ws://host:port/acp` —WebSocket y Streamable HTTP, la RFD oficial— y atiende a varios
> clientes a la vez, cada uno con sus propias sesiones. El transporte es el del SDK oficial
> de ACP, no uno escrito a mano. Como esa vía ejecuta herramientas, exige token de runtime
> y política de origen: sin `Origin` sólo entra quien manda cabecera `Authorization`, que
> una página del navegador no puede fijar. De paso cierra ese mismo agujero para quien ya
> usaba el servidor HTTP.
>
> **Novedad (0.0.15)** — **la sesión ya no se te olvida.** Desde 0.0.13, los bytes de
> las capturas que pegabas se quedaban en el historial y se reenviaban cada turno, pero
> el medidor de contexto los contaba como cero tokens: el proveedor terminaba rechazando
> la petición y la conversación se resumía de emergencia, en silencio. Ahora las imágenes
> conservan sus bytes sólo en los 2 mensajes más recientes (las viejas dejan su ruta y el
> modelo puede releerlas con `image_analyze`), el estimador sí las cobra, y si el contexto
> se desborda te avisa antes de resumir. Cambiar de modelo tampoco vuelve a apagarte
> `auto_compact` sin decírtelo.
>
> Sigue vigente **Kimi K3** de Moonshot: 1M de contexto, razonamiento siempre activo y
> visión nativa. Modelo por defecto del provider Moonshot; Kimi K2.6 (256K) se queda para
> cambiar al vuelo con `/model`. Un comando:
> `ghosty auth set --provider moonshot --api-key "TU_KEY_MOONSHOT"`.
> También vía OpenRouter (`moonshotai/kimi-k3`).

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

¿Sin Node ni Rust? Baja el binario precompilado directo:

```bash
curl -fsSL https://formmy.app/ghosty/install.sh | sh
```

El `postinstall` baja el binario de tu plataforma a `bin/downloads/` y expone el
comando **`ghosty`**. Si los assets del release no están disponibles
en ese momento, la instalación continúa y el wrapper reintenta la descarga al primer uso.

## Primer uso

```bash
# DeepSeek V4
ghosty auth set --provider deepseek --api-key "TU_DEEPSEEK_API_KEY"
# Kimi K3 directo (Moonshot; 1M contexto)
ghosty auth set --provider moonshot --api-key "TU_KEY_MOONSHOT"
# GLM-5.2 directo (Z.AI Coding Plan)
ghosty auth set --provider zai --api-key "TU_TOKEN_ZAI"
ghosty doctor
ghosty
```

La config vive en `~/.ghosty/config.toml` (también lee la variable `DEEPSEEK_API_KEY`).
O elige tu provider en el wizard de arranque. Kimi K3 (`moonshotai/kimi-k3`) y GLM-5.2
(`z-ai/glm-5.2`) también están disponibles vía OpenRouter.

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
