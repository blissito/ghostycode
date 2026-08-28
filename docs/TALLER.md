# Taller GhostyCode — guía de instalación

Guía para asistentes de taller presencial. Instalas GhostyCode en tu laptop, lo
conectas a EasyBits, y lo usas para todos los ejercicios.

> Si eres admin de un laboratorio Windows con SCCM/Intune, el documento que
> buscas es [CLASSROOM_INSTALL.md](CLASSROOM_INSTALL.md), no este.

---

## 1. Instalar (macOS o Linux)

Copia esta línea tal cual. **La versión va pineada a propósito**: así todo el
salón corre exactamente el mismo binario y los ejercicios dan el mismo resultado.

```bash
curl -fsSL https://formmy.app/ghosty/install.sh | GHOSTY_VERSION=0.0.15 sh
```

Baja un binario a `~/.local/bin`: `ghosty`. CLI y TUI viven en el mismo ejecutable.

Si el instalador dice que añadas el PATH:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

Verifica:

```bash
ghosty --version    # debe decir 0.0.14
```

### Goose en paralelo

En el taller comparamos los dos agentes. Goose se instala aparte y no estorba —
son binarios distintos con configuraciones distintas:

```bash
curl -fsSL https://github.com/aaif-goose/goose/releases/latest/download/download_cli.sh | sh
```

---

## 2. Tu API key

Recibiste tu key de **EasyBits** por correo antes del taller. Una sola key sirve
para el modelo y para las herramientas (cajas, S3, base de datos).

```bash
ghosty auth set --provider easybits --api-key "TU_KEY_DEL_CORREO"
ghosty doctor
```

`ghosty doctor` debe pasar todas sus verificaciones. Si falla la de conexión,
revisa que copiaste la key completa, sin espacios al final.

### Alternativa: DeepSeek directo

Si prefieres tu propia cuenta, saca una key en
[platform.deepseek.com](https://platform.deepseek.com) y usa:

```bash
ghosty auth set --provider deepseek --api-key "TU_KEY_DEEPSEEK"
```

Con esta ruta solo tienes el modelo: sin cajas, sin S3, sin base de datos.
Los ejercicios del paso 3 no te van a funcionar.

---

## 3. Conectar EasyBits (modo código)

Vas a usar EasyBits de **dos formas a la vez**, y es a propósito.

| Capacidad | Cómo | Por qué |
|---|---|---|
| Cajas (crear, ejecutar, exponer puerto, destruir) | **MCP** | Crear una caja no tiene endpoint REST: solo existe por MCP o SDK. |
| Archivos / S3, bases de datos | **API con `curl`** | Sí tienen REST completo, y así no consumen presupuesto de tools. |

El motivo de fondo: DeepSeek —que es lo que EasyBits revende— tiene un **tope duro
de 128 herramientas por petición** y las cuenta todas. Los grupos MCP de EasyBits
miden `core`=65 y `sandbox`=47; las built-in de GhostyCode son ~36. Conectar los
dos grupos da 146 y el modelo responde `Invalid 'tools': array too long` en vez de
trabajar.

Conectando **solo `sandbox`** quedas en 83 de 128, y S3 y base de datos siguen
disponibles por `curl` — en el mismo turno, sin conflicto.

### Conecta el MCP de cajas

```bash
ghosty mcp add easybits --url "https://www.easybits.cloud/api/mcp?tools=sandbox" --bearer TU_KEY_DEL_CORREO
ghosty mcp list
```

### Deja la key a mano para `curl`

```bash
echo 'export EASYBITS_API_KEY="TU_KEY_DEL_CORREO"' >> ~/.zshrc
source ~/.zshrc
```

Comprueba que responde:

```bash
curl -s https://www.easybits.cloud/api/v2/databases \
  -H "Authorization: Bearer $EASYBITS_API_KEY"
```

### El API que vas a usar

Base: `https://www.easybits.cloud/api/v2` · Auth: `Authorization: Bearer $EASYBITS_API_KEY`

**Bases de datos**

| Qué | Llamada |
|---|---|
| Listar | `GET /databases` |
| Crear | `POST /databases` → `{"name":"...","description":"..."}` |
| Consultar | `POST /databases/:dbId/query` → `{"sql":"...","args":[...]}` |
| Varias sentencias (máx 20) | `POST /databases/:dbId/exec` → `{"statements":[...]}` |
| Borrar (irreversible) | `DELETE /databases/:dbId` |

**Archivos / S3** — la subida es en dos pasos:

| Qué | Llamada |
|---|---|
| Listar | `GET /files?limit=50&cursor=...` |
| Detalle + URL de descarga | `GET /files/:fileId` |
| 1) Pedir espacio | `POST /files` → `{"fileName","contentType","size","access":"public\|private","region":"LATAM\|US\|EU"}` — devuelve `putUrl` |
| 2) Subir bytes | `PUT` a esa `putUrl` |
| 3) Confirmar | `PATCH /files/:fileId` → `{"status":"DONE"}` |
| Borrar (soft, 7 días) | `DELETE /files/:fileId` |

Ejemplo completo — crear una base y consultarla:

```bash
db=$(curl -s -X POST https://www.easybits.cloud/api/v2/databases \
  -H "Authorization: Bearer $EASYBITS_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"name":"taller"}' | python3 -c 'import sys,json;print(json.load(sys.stdin)["id"])')

curl -s -X POST "https://www.easybits.cloud/api/v2/databases/$db/query" \
  -H "Authorization: Bearer $EASYBITS_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"sql":"select 1 as ok"}'
```

No tienes que memorizar nada de esto: dile a GhostyCode qué quieres y él escribe
el `curl`. Para eso está el modo código.

> **Cuidado con el límite de peticiones: 100 cada 15 minutos por key.** Si le pides
> al agente que recorra cientos de archivos en un bucle, se te acaba la cuota a
> media clase. Pídele que trabaje por lotes.

---

## 4. Arrancar

```bash
ghosty
```

Abre la interfaz interactiva. GhostyCode te va a pedir confirmación antes de
cada comando que toque tu máquina — eso es a propósito.

Cuando ya le tengas confianza y quieras que corra sin preguntar:

```bash
ghosty --yolo
```

> Úsalo solo dentro del directorio del taller. `--yolo` significa que ejecuta
> sin pedirte permiso.

---

## Si algo falla

| Síntoma | Qué hacer |
|---|---|
| `ghosty: command not found` | El PATH. Corre `export PATH="$HOME/.local/bin:$PATH"` y abre una terminal nueva. |
| `no pude resolver la versión` | Estás sin la variable pineada. Usa la línea del paso 1 tal cual, con `GHOSTY_VERSION=0.0.15`. |
| `Invalid 'tools': array too long` | Conectaste `core` además de `sandbox`. Ve al paso 3: solo va `?tools=sandbox`. |
| `429` o "rate limit" en un `curl` | Son 100 peticiones cada 15 min por key. Espera y pide lotes más chicos. |
| `DeepSeek API key not found` | La key no quedó guardada. Repite `ghosty auth set` y luego `ghosty doctor`. |
| El instalador se cuelga o da 403 | La red del lugar. Pide la ruta de respaldo (abajo). |
| macOS bloquea el binario | `xattr -d com.apple.quarantine ~/.local/bin/ghosty` |

### Ruta de respaldo: caja remota

Si la instalación local no sale, no pierdas el taller. Te damos acceso a una
microVM que ya trae GhostyCode instalado y configurado. Pide la URL y el token
al instructor; se entra por navegador y no requiere instalar nada.

---

## Referencia rápida

| Qué | Dónde |
|---|---|
| Binario | `~/.local/bin/ghosty` |
| Configuración | `~/.ghosty/config.toml` |
| Servidores MCP | `~/.ghosty/mcp.json` |
| Sesiones | `~/.ghosty/` |

Documentación completa: [INSTALL.md](INSTALL.md) · [CONFIGURATION.md](CONFIGURATION.md) · [MCP.md](MCP.md)
