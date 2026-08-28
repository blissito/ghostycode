#!/usr/bin/env bash
set -euo pipefail

if [[ "${EUID}" -ne 0 ]]; then
  echo "Run as root: sudo bash scripts/tencent-lighthouse/bootstrap-ubuntu.sh" >&2
  exit 1
fi

GHOSTY_USER="${GHOSTY_USER:-${DEEPSEEK_USER:-ghosty}}"
GHOSTY_ROOT="${GHOSTY_ROOT:-${DEEPSEEK_ROOT:-/opt/ghosty}}"
WHALEBRO_ROOT="${WHALEBRO_ROOT:-/opt/whalebro}"
REPO_URL="${GHOSTY_REPO_URL:-${DEEPSEEK_REPO_URL:-https://github.com/blissito/ghostycode.git}}"
WHALEBRO_EXTRA_REPOS="${WHALEBRO_EXTRA_REPOS:-}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOURCE_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
SOURCE_BRANCH="$(git -C "${SOURCE_ROOT}" branch --show-current 2>/dev/null || true)"
REPO_BRANCH="${GHOSTY_REPO_BRANCH:-${DEEPSEEK_REPO_BRANCH:-${SOURCE_BRANCH:-main}}}"

apt-get update
apt-get install -y \
  ca-certificates \
  curl \
  git \
  iproute2 \
  openssh-client \
  build-essential \
  pkg-config \
  libdbus-1-dev \
  libssl-dev \
  nodejs \
  npm \
  rsync \
  tmux \
  fail2ban \
  ufw

node_major="$(node -p "Number(process.versions.node.split('.')[0])")"
if (( node_major < 18 )); then
  echo "Node.js 18+ is required for the phone bridges; install a newer Node.js before running install-services.sh." >&2
fi

if ! id -u "${GHOSTY_USER}" >/dev/null 2>&1; then
  useradd --create-home --shell /bin/bash "${GHOSTY_USER}"
fi

install -d -o "${GHOSTY_USER}" -g "${GHOSTY_USER}" "${GHOSTY_ROOT}"
install -d -o "${GHOSTY_USER}" -g "${GHOSTY_USER}" "${GHOSTY_ROOT}/bridge"
install -d -o "${GHOSTY_USER}" -g "${GHOSTY_USER}" "${GHOSTY_ROOT}/telegram-bridge"
install -d -o "${GHOSTY_USER}" -g "${GHOSTY_USER}" "${WHALEBRO_ROOT}"
install -d -o "${GHOSTY_USER}" -g "${GHOSTY_USER}" "${WHALEBRO_ROOT}/worktrees"
install -d -m 0750 -o root -g "${GHOSTY_USER}" /etc/ghosty
install -d -m 0700 -o "${GHOSTY_USER}" -g "${GHOSTY_USER}" /var/lib/ghosty-feishu-bridge
install -d -m 0700 -o "${GHOSTY_USER}" -g "${GHOSTY_USER}" /var/lib/ghosty-telegram-bridge

if [[ ! -d "${WHALEBRO_ROOT}/ghosty/.git" ]]; then
  sudo -u "${GHOSTY_USER}" git clone --branch "${REPO_BRANCH}" "${REPO_URL}" "${WHALEBRO_ROOT}/ghosty"
fi

for repo_spec in ${WHALEBRO_EXTRA_REPOS}; do
  repo_name="${repo_spec%%=*}"
  repo_url="${repo_spec#*=}"
  if [[ -z "${repo_name}" || -z "${repo_url}" || "${repo_name}" == "${repo_url}" ]]; then
    echo "Skipping malformed WHALEBRO_EXTRA_REPOS entry: ${repo_spec}" >&2
    continue
  fi
  if [[ ! -d "${WHALEBRO_ROOT}/${repo_name}/.git" ]]; then
    sudo -u "${GHOSTY_USER}" git clone "${repo_url}" "${WHALEBRO_ROOT}/${repo_name}" || {
      echo "Warning: failed to clone optional repo ${repo_name} from ${repo_url}" >&2
    }
  fi
done

if [[ ! -f /etc/ghosty/runtime.env ]]; then
  cat >/etc/ghosty/runtime.env <<'EOF'
GHOSTY_RUNTIME_TOKEN=replace-with-long-random-token
GHOSTY_RUNTIME_PORT=7878
GHOSTY_RUNTIME_WORKERS=2
GHOSTY_PROVIDER=deepseek
DEEPSEEK_API_KEY=replace-with-provider-key
RUST_LOG=info
EOF
  chown root:"${GHOSTY_USER}" /etc/ghosty/runtime.env
  chmod 0640 /etc/ghosty/runtime.env
fi

if [[ ! -f /etc/ghosty/feishu-bridge.env ]]; then
  cat >/etc/ghosty/feishu-bridge.env <<'EOF'
FEISHU_APP_ID=cli_xxxxxxxxxxxxxxxx
FEISHU_APP_SECRET=replace-with-app-secret
FEISHU_DOMAIN=feishu
GHOSTY_RUNTIME_URL=http://127.0.0.1:7878
GHOSTY_RUNTIME_TOKEN=replace-with-same-token-as-runtime-env
GHOSTY_WORKSPACE=/opt/whalebro
GHOSTY_MODEL=auto
GHOSTY_MODE=agent
GHOSTY_ALLOW_SHELL=true
GHOSTY_TRUST_MODE=false
GHOSTY_AUTO_APPROVE=false
GHOSTY_CHAT_ALLOWLIST=
GHOSTY_ALLOW_UNLISTED=false
FEISHU_THREAD_MAP_PATH=/var/lib/ghosty-feishu-bridge/thread-map.json
FEISHU_ALLOW_GROUPS=false
FEISHU_REQUIRE_PREFIX_IN_GROUP=true
FEISHU_GROUP_PREFIX=/cw
FEISHU_MAX_REPLY_CHARS=3500
GHOSTY_TURN_TIMEOUT_MS=900000
EOF
  chown root:"${GHOSTY_USER}" /etc/ghosty/feishu-bridge.env
  chmod 0640 /etc/ghosty/feishu-bridge.env
fi

ufw allow OpenSSH
ufw --force enable

cat <<EOF

Base server setup complete.

Next:
1. Install Rust 1.88+ for ${GHOSTY_USER}; rustup is the usual path.
2. Build/install both binaries:
   sudo -iu ${GHOSTY_USER}
   cd ${WHALEBRO_ROOT}/ghosty
   cargo install --path crates/cli --locked --force
   cargo install --path crates/tui --locked --force
3. Copy integrations/feishu-bridge or integrations/telegram-bridge to ${GHOSTY_ROOT} and run npm install.
4. Edit /etc/ghosty/runtime.env and the selected bridge env file.
5. Install systemd units with scripts/tencent-lighthouse/install-services.sh.
6. After the env files are edited and services are started, run:
   sudo bash scripts/tencent-lighthouse/doctor.sh

EOF
