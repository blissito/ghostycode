#!/usr/bin/env bash
set -euo pipefail

if [[ "${EUID}" -ne 0 ]]; then
  echo "Run as root: sudo bash scripts/tencent-lighthouse/install-services.sh" >&2
  exit 1
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
GHOSTY_USER="${GHOSTY_USER:-${DEEPSEEK_USER:-ghosty}}"
GHOSTY_ROOT="${GHOSTY_ROOT:-${DEEPSEEK_ROOT:-/opt/ghosty}}"
BRIDGE_KIND="${GHOSTY_BRIDGE:-${DEEPSEEK_BRIDGE:-feishu}}"

case "${BRIDGE_KIND}" in
  feishu|lark)
    BRIDGE_SRC="integrations/feishu-bridge"
    BRIDGE_DST="${GHOSTY_ROOT}/bridge"
    BRIDGE_UNIT="ghosty-feishu-bridge.service"
    BRIDGE_ENV="/etc/ghosty/feishu-bridge.env"
    BRIDGE_ENV_EXAMPLE="deploy/tencent-lighthouse/examples/feishu-bridge.env.example"
    BRIDGE_STATE_DIR="/var/lib/ghosty-feishu-bridge"
    VALIDATOR="integrations/feishu-bridge/scripts/validate-config.mjs"
    ;;
  telegram)
    BRIDGE_SRC="integrations/telegram-bridge"
    BRIDGE_DST="${GHOSTY_ROOT}/telegram-bridge"
    BRIDGE_UNIT="ghosty-telegram-bridge.service"
    BRIDGE_ENV="/etc/ghosty/telegram-bridge.env"
    BRIDGE_ENV_EXAMPLE="deploy/tencent-lighthouse/examples/telegram-bridge.env.example"
    BRIDGE_STATE_DIR="/var/lib/ghosty-telegram-bridge"
    VALIDATOR="integrations/telegram-bridge/scripts/validate-config.mjs"
    ;;
  *)
    echo "Unknown bridge '${BRIDGE_KIND}'. Use GHOSTY_BRIDGE=feishu or GHOSTY_BRIDGE=telegram." >&2
    exit 1
    ;;
esac

install -d -m 0750 -o root -g "${GHOSTY_USER}" /etc/ghosty
install -d -m 0700 -o "${GHOSTY_USER}" -g "${GHOSTY_USER}" "${BRIDGE_STATE_DIR}"
install -d -o "${GHOSTY_USER}" -g "${GHOSTY_USER}" "${BRIDGE_DST}"

if [[ ! -f /etc/ghosty/runtime.env && -f "${REPO_ROOT}/deploy/tencent-lighthouse/examples/runtime.env.example" ]]; then
  install -m 0640 -o root -g "${GHOSTY_USER}" \
    "${REPO_ROOT}/deploy/tencent-lighthouse/examples/runtime.env.example" \
    /etc/ghosty/runtime.env
fi

if [[ ! -f "${BRIDGE_ENV}" && -f "${REPO_ROOT}/${BRIDGE_ENV_EXAMPLE}" ]]; then
  install -m 0640 -o root -g "${GHOSTY_USER}" \
    "${REPO_ROOT}/${BRIDGE_ENV_EXAMPLE}" \
    "${BRIDGE_ENV}"
fi
rsync -a --delete \
  --exclude node_modules \
  "${REPO_ROOT}/${BRIDGE_SRC}/" \
  "${BRIDGE_DST}/"
chown -R "${GHOSTY_USER}:${GHOSTY_USER}" "${BRIDGE_DST}"

if [[ -f "${BRIDGE_DST}/package-lock.json" ]]; then
  sudo -u "${GHOSTY_USER}" npm --prefix "${BRIDGE_DST}" ci --omit=dev
else
  sudo -u "${GHOSTY_USER}" npm --prefix "${BRIDGE_DST}" install --omit=dev
fi

install -m 0644 "${REPO_ROOT}/deploy/tencent-lighthouse/systemd/ghosty-runtime.service" /etc/systemd/system/ghosty-runtime.service
install -m 0644 "${REPO_ROOT}/deploy/tencent-lighthouse/systemd/${BRIDGE_UNIT}" "/etc/systemd/system/${BRIDGE_UNIT}"

systemctl daemon-reload
systemctl enable ghosty-runtime "${BRIDGE_UNIT}"

cat <<'EOF'
Services installed but not started.

Before starting, verify:
  /etc/ghosty/runtime.env
EOF
cat <<EOF
  ${BRIDGE_ENV}
  sudo -u ${GHOSTY_USER} node ${REPO_ROOT}/${VALIDATOR} --env ${BRIDGE_ENV} --runtime-env /etc/ghosty/runtime.env --workspace-root /opt/whalebro --check-filesystem
Then run:
  sudo systemctl start ghosty-runtime
  sudo systemctl start ${BRIDGE_UNIT}
  sudo GHOSTY_BRIDGE=${BRIDGE_KIND} bash /opt/whalebro/ghosty/scripts/tencent-lighthouse/doctor.sh
  sudo journalctl -u ${BRIDGE_UNIT} -f
EOF
