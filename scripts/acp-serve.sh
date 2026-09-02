#!/bin/sh
# Levanta Ghosty como agente ACP por red con sólo la llave de EasyBits.
#   EASYBITS_API_KEY=eb_... sh scripts/acp-serve.sh
#   curl -fsSL https://raw.githubusercontent.com/blissito/ghostycode/main/scripts/acp-serve.sh | EASYBITS_API_KEY=eb_... sh
#
# Instala `ghosty` si no está, fija el proveedor en easybits y sirve ACP en
# `/acp` (WebSocket + Streamable HTTP). Pensado para una caja de EasyBits: el
# host de cajas publica el puerto en https://sb-<id>-<puerto>.sandboxes.easybits.cloud
# y termina TLS ahí, así que el cliente entra por `wss://` sin que Ghosty
# necesite certificado (no uses --tls-cert detrás de ese proxy).
#
# Variables:
#   EASYBITS_API_KEY        obligatoria
#   GHOSTY_RUNTIME_TOKEN    token del listener (default: se genera y se imprime)
#   GHOSTY_ACP_PORT         puerto (default: 7878)
#   GHOSTY_ACP_LOCAL=1      quedarse en 127.0.0.1 en vez de --open
#   GHOSTY_ACP_ALLOW_ORIGIN origen de navegador permitido (opcional)
#   GHOSTY_MODEL            modelo inicial (default: el de easybits)
set -eu

err() { echo "ghosty-acp: $*" >&2; exit 1; }

[ -n "${EASYBITS_API_KEY:-}" ] || err "falta EASYBITS_API_KEY"

INSTALL_DIR="${GHOSTY_INSTALL_DIR:-$HOME/.local/bin}"
PATH="$INSTALL_DIR:$PATH"
if ! command -v ghosty >/dev/null 2>&1; then
  command -v curl >/dev/null 2>&1 || err "falta curl para instalar ghosty"
  curl -fsSL https://formmy.app/ghosty/install.sh | sh
  command -v ghosty >/dev/null 2>&1 || err "ghosty no quedó en PATH tras instalar"
fi

if [ -z "${GHOSTY_RUNTIME_TOKEN:-}" ]; then
  if command -v openssl >/dev/null 2>&1; then
    GHOSTY_RUNTIME_TOKEN=$(openssl rand -hex 24)
  else
    GHOSTY_RUNTIME_TOKEN=$(od -An -N24 -tx1 /dev/urandom | tr -d ' \n')
  fi
fi
export GHOSTY_RUNTIME_TOKEN
export GHOSTY_PROVIDER=easybits
export EASYBITS_API_KEY

PORT="${GHOSTY_ACP_PORT:-7878}"
set -- acp --http --port "$PORT"
[ "${GHOSTY_ACP_LOCAL:-0}" = "1" ] || set -- "$@" --open
[ -z "${GHOSTY_ACP_ALLOW_ORIGIN:-}" ] || set -- "$@" --allow-origin "$GHOSTY_ACP_ALLOW_ORIGIN"

echo "ghosty-acp: token  $GHOSTY_RUNTIME_TOKEN"
echo "ghosty-acp: local  ws://127.0.0.1:$PORT/acp?token=$GHOSTY_RUNTIME_TOKEN"
if [ -n "${EASYBITS_SANDBOX_ID:-}" ]; then
  echo "ghosty-acp: caja   wss://sb-$EASYBITS_SANDBOX_ID-$PORT.sandboxes.easybits.cloud/acp?token=$GHOSTY_RUNTIME_TOKEN"
  echo "ghosty-acp:        (expón el puerto $PORT de la caja para que esa URL exista)"
fi
echo "ghosty-acp: salud  GET /health en el mismo puerto, sin token"

exec ghosty "$@"
