#!/bin/sh
# Ghosty Code installer — baja el binario precompilado (sin Node, sin Rust).
#   curl -fsSL https://formmy.app/ghosty/install.sh | sh
#
# Instala UN binario: `ghosty`. El runtime está consolidado —CLI y TUI en el
# mismo ejecutable—, así que ya no hay un segundo comando que mantener al lado.
#
# En taller / salón, PINEA la versión. Además de que todos terminan con el
# mismo binario, se salta la resolución remota y con ella el rate limit de la
# API de GitHub (60 req/hora por IP — y un salón entero sale por una sola IP):
#   curl -fsSL https://formmy.app/ghosty/install.sh | GHOSTY_VERSION=0.0.14 sh
#
# Overrides (mismos que el instalador de npm):
#   GHOSTY_VERSION           versión a instalar (default: último release)
#   GHOSTY_INSTALL_DIR       destino (default: $HOME/.local/bin)
#   GHOSTY_RELEASE_BASE_URL  base de descarga personalizada
#   GHOSTY_USE_CNB_MIRROR    usa el mirror CNB (China)
set -eu

REPO="blissito/ghostycode"
INSTALL_DIR="${GHOSTY_INSTALL_DIR:-$HOME/.local/bin}"
err() { echo "ghosty-install: $*" >&2; exit 1; }

# Preflight: sin curl no hay nada que hacer, y el checksum NO es opcional —
# si no hay con qué verificar, se aborta en vez de instalar a ciegas.
command -v curl >/dev/null 2>&1 || err "falta curl — instálalo y reintenta"
if command -v sha256sum >/dev/null 2>&1; then
  sha="sha256sum"
elif command -v shasum >/dev/null 2>&1; then
  sha="shasum -a 256"
else
  err "falta sha256sum y shasum — no puedo verificar la descarga"
fi

os=$(uname -s); arch=$(uname -m)
case "$os" in
  # Termux no es Linux arm64: el binario de Android es un target aparte
  # (bionic, sin glibc). Instalar el de Linux ahí falla al arrancar.
  Linux) if [ -n "${TERMUX_VERSION:-}" ]; then os=android; else os=linux; fi ;;
  Darwin) os=macos ;;
  *) err "OS no soportado: $os — usa npm o cargo (ver README)" ;;
esac
case "$arch" in
  x86_64|amd64) arch=x64 ;;
  aarch64|arm64) arch=arm64 ;;
  *) err "arquitectura no soportada: $arch" ;;
esac

# Resolución de versión. El redirect de /releases/latest NO consume cuota de la
# API; la API queda como segundo intento por si GitHub cambia el redirect.
version="${GHOSTY_VERSION:-}"
if [ -z "$version" ]; then
  final_url=$(curl -fsSLI -o /dev/null -w '%{url_effective}' \
    "https://github.com/$REPO/releases/latest" 2>/dev/null) || final_url=""
  case "$final_url" in
    */releases/tag/v*) version=${final_url##*/releases/tag/v} ;;
  esac
fi
if [ -z "$version" ]; then
  version=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null \
    | grep '"tag_name"' | head -1 | sed -E 's/.*"v?([^"]+)".*/\1/') || version=""
fi
[ -n "$version" ] || err "no pude resolver la versión (define GHOSTY_VERSION=x.y.z)"

if [ -n "${GHOSTY_RELEASE_BASE_URL:-}" ]; then
  base="${GHOSTY_RELEASE_BASE_URL%/}/"
elif [ -n "${GHOSTY_USE_CNB_MIRROR:-}" ]; then
  base="https://cnb.cool/$REPO/-/releases/v$version/"
else
  base="https://github.com/$REPO/releases/download/v$version/"
fi

cli="ghosty-$os-$arch"

if [ -x "$INSTALL_DIR/ghosty" ]; then
  prev=$("$INSTALL_DIR/ghosty" --version 2>/dev/null | head -1) || prev=""
  [ -n "$prev" ] && echo "ghosty-install: reemplazando instalación previa ($prev)"
fi

tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
echo "ghosty-install: bajando v$version ($os-$arch)…"
( cd "$tmp"
  curl -fsSLO "$base$cli"
  curl -fsSL "${base}ghosty-artifacts-sha256.txt" -o manifest.txt ) \
  || err "fallo en la descarga — revisa la red o define GHOSTY_VERSION"

# El manifiesto trae TODOS los assets del release. Se extraen las dos líneas de
# esta plataforma y se cuenta que sean exactamente 2 ANTES de verificar: un grep
# vacío haría que `$sha -c` no verificara nada y saliera 0.
( cd "$tmp"
  grep -E "  $cli\$" manifest.txt > wanted.txt || true
  lines=$(wc -l < wanted.txt | tr -d ' ')
  [ "$lines" = "1" ] || err "el manifiesto no cubre $cli (¿existe la versión $version?)"
  $sha -c wanted.txt >/dev/null ) || err "checksum inválido — descarga corrupta o manipulada"

mkdir -p "$INSTALL_DIR"
chmod +x "$tmp/$cli"
# macOS marca en cuarentena lo que baja de la red; sin esto Gatekeeper puede
# bloquear el primer arranque. Falla en silencio si no hay atributo que quitar.
if [ "$os" = "macos" ] && command -v xattr >/dev/null 2>&1; then
  xattr -d com.apple.quarantine "$tmp/$cli" 2>/dev/null || true
fi
mv "$tmp/$cli" "$INSTALL_DIR/ghosty"
echo "ghosty-install: instalado ghosty v$version en $INSTALL_DIR"

case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *) echo "ghosty-install: añade al PATH →  export PATH=\"$INSTALL_DIR:\$PATH\"" ;;
esac
echo "Listo. Arranca con:  ghosty"
echo "(Para saltarte las confirmaciones de cada tool:  ghosty --yolo)"
