#!/bin/sh
# Ghosty Code installer — baja el binario precompilado (sin Node, sin Rust).
#   curl -fsSL https://formmy.app/ghosty/install.sh | sh
#
# Instala DOS binarios juntos: `ghosty` (dispatcher) y `ghosty-tui` (la
# interfaz interactiva que `ghosty` lanza). Sin el hermano `ghosty-tui` al
# lado, abrir la TUI falla con "Companion ghosty-tui binary not found".
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

os=$(uname -s); arch=$(uname -m)
case "$os" in
  Linux) os=linux ;;
  Darwin) os=macos ;;
  *) err "OS no soportado: $os — usa npm o cargo (ver README)" ;;
esac
case "$arch" in
  x86_64|amd64) arch=x64 ;;
  aarch64|arm64) arch=arm64 ;;
  *) err "arquitectura no soportada: $arch" ;;
esac

version="${GHOSTY_VERSION:-}"
[ -n "$version" ] || version=$(curl -fsSL \
  "https://api.github.com/repos/$REPO/releases/latest" \
  | grep '"tag_name"' | head -1 | sed -E 's/.*"v?([^"]+)".*/\1/')
[ -n "$version" ] || err "no pude resolver la versión (define GHOSTY_VERSION)"

if [ -n "${GHOSTY_RELEASE_BASE_URL:-}" ]; then
  base="${GHOSTY_RELEASE_BASE_URL%/}/"
elif [ -n "${GHOSTY_USE_CNB_MIRROR:-}" ]; then
  base="https://cnb.cool/$REPO/-/releases/v$version/"
else
  base="https://github.com/$REPO/releases/download/v$version/"
fi

cli="ghosty-$os-$arch"; tui="ghosty-tui-$os-$arch"
command -v sha256sum >/dev/null 2>&1 && sha="sha256sum" || sha="shasum -a 256"

tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
echo "ghosty-install: bajando v$version ($os-$arch)…"
( cd "$tmp"
  curl -fsSLO "$base$cli"
  curl -fsSLO "$base$tui"
  curl -fsSL "${base}ghosty-artifacts-sha256.txt" \
    | grep -E "  ($cli|$tui)\$" | $sha -c - ) || err "fallo en descarga o checksum"

mkdir -p "$INSTALL_DIR"
chmod +x "$tmp/$cli" "$tmp/$tui"
mv "$tmp/$cli" "$INSTALL_DIR/ghosty"
mv "$tmp/$tui" "$INSTALL_DIR/ghosty-tui"
echo "ghosty-install: instalado ghosty + ghosty-tui en $INSTALL_DIR"

case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *) echo "ghosty-install: añade al PATH →  export PATH=\"$INSTALL_DIR:\$PATH\"" ;;
esac
echo "Listo. Corre: ghosty doctor"
