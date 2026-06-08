# US Remote VM Quickstart

This is the default remote-phone setup for US-based users who want the least
cloud ceremony. Use this path unless you specifically need the Tencent/CNB/
Feishu workflow in `docs/TENCENT_CLOUD_REMOTE_FIRST.md`.

## Recommendation

Use a small persistent Ubuntu VPS/VM, not a serverless/container platform, for
the main Ghosty Code host.

Good defaults:

- DigitalOcean Droplet: 2 vCPU / 4 GB RAM / 80 GB SSD, Ubuntu.
- AWS Lightsail: 2 vCPU / 4 GB RAM / 80 GB SSD, Ubuntu.

Minimum for a smoke test:

- 2 vCPU / 2 GB RAM / 60 GB SSD.

Better for Rust builds, subagents, and longer sessions:

- 4 vCPU / 8 GB RAM / 160 GB SSD.

## Why Not Railway First

Railway is fine for a tiny relay service, but the main Ghosty Code runtime wants:

- a persistent checkout and worktrees
- shell access
- systemd or equivalent long-running service supervision
- predictable local disk paths
- direct SSH recovery when the agent or bridge is unhealthy

That maps more cleanly to a VM. Use Railway later only if you want a public web
status page or a small bridge relay in front of a VM-hosted runtime.

## Provider Choice

- Choose DigitalOcean if you want the simplest VPS control panel and predictable
  developer workflow.
- Choose AWS Lightsail if you already use AWS billing or want the AWS free-trial
  path while staying in a simple VPS product.
- Avoid raw EC2 for the first setup unless you already know AWS networking,
  IAM, security groups, and EBS.
- Avoid Lambda/ECR/Load Balancers for the first setup; they are not the
  persistent interactive host Ghosty Code needs.

## Target Architecture

```text
Laptop
  -> git push / SSH

DigitalOcean or AWS Lightsail Ubuntu VM
  -> /opt/whalebro/ghosty
  -> /opt/whalebro/worktrees
  -> ghosty-runtime.service on 127.0.0.1:7878
  -> ghosty-telegram-bridge.service

Telegram phone DM
  -> Telegram Bot API long polling
  -> local runtime API with GHOSTY_RUNTIME_TOKEN
```

The runtime API must stay on `127.0.0.1`. Telegram long polling does not need
an inbound public webhook port.

## Setup Shape

Create an Ubuntu VM with SSH-key login. Open SSH only. Then:

```bash
sudo apt-get update
sudo apt-get install -y git

export GHOSTY_BRANCH=codex/v0.8.53
export GHOSTY_REPO_URL=https://github.com/blissito/ghostycode.git

git clone --branch "$GHOSTY_BRANCH" "$GHOSTY_REPO_URL" /tmp/ghosty
cd /tmp/ghosty
sudo GHOSTY_REPO_URL="$GHOSTY_REPO_URL" \
  GHOSTY_REPO_BRANCH="$GHOSTY_BRANCH" \
  bash scripts/tencent-lighthouse/bootstrap-ubuntu.sh
```

The bootstrap script is named for the older Tencent runbook, but it now creates
Ghosty Code-primary paths and env files:

- `/etc/ghosty/runtime.env`
- `/etc/ghosty/feishu-bridge.env`
- `/opt/whalebro`
- `/opt/ghosty`

Install Rust for the `ghosty` user, then build:

```bash
sudo -iu ghosty
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/rustup-init.sh
sed -n '1,120p' /tmp/rustup-init.sh
sh /tmp/rustup-init.sh -y --profile minimal
. "$HOME/.cargo/env"
rustup default stable
cd /opt/whalebro/ghosty
cargo install --path crates/cli --locked --force
cargo install --path crates/tui --locked --force
exit
```

Install the Telegram bridge service:

```bash
cd /opt/whalebro/ghosty
sudo GHOSTY_BRIDGE=telegram bash scripts/tencent-lighthouse/install-services.sh
```

Create a bot with Telegram's `@BotFather`, then edit:

```bash
sudoedit /etc/ghosty/runtime.env
sudoedit /etc/ghosty/telegram-bridge.env
```

Required values:

- `/etc/ghosty/runtime.env`
  - `GHOSTY_RUNTIME_TOKEN`
  - `GHOSTY_RUNTIME_PORT=7878`
  - `GHOSTY_PROVIDER=<provider>`
  - provider API key such as `ARCEE_API_KEY`, `DEEPSEEK_API_KEY`, or
    `XIAOMI_MIMO_API_KEY`
- `/etc/ghosty/telegram-bridge.env`
  - `TELEGRAM_BOT_TOKEN`
  - `GHOSTY_RUNTIME_TOKEN` matching runtime.env
  - `GHOSTY_WORKSPACE=/opt/whalebro`

For first pairing, temporarily set `TELEGRAM_ALLOW_UNLISTED=true`, DM the bot
`/status`, copy the returned `chat_id` into `TELEGRAM_CHAT_ALLOWLIST`, then set
`TELEGRAM_ALLOW_UNLISTED=false`.

Validate and start:

```bash
sudo -u ghosty node /opt/ghosty/telegram-bridge/scripts/validate-config.mjs \
  --env /etc/ghosty/telegram-bridge.env \
  --runtime-env /etc/ghosty/runtime.env \
  --workspace-root /opt/whalebro \
  --check-filesystem

sudo systemctl start ghosty-runtime
sudo systemctl start ghosty-telegram-bridge
sudo GHOSTY_BRIDGE=telegram bash /opt/whalebro/ghosty/scripts/tencent-lighthouse/doctor.sh
```

Useful logs:

```bash
sudo journalctl -u ghosty-runtime -f
sudo journalctl -u ghosty-telegram-bridge -f
```

## First Smoke Test

From Telegram:

1. Send `/status`.
2. Send `/menu` and confirm the tappable control panel appears.
3. Send `summarize git status in /opt/whalebro/ghosty`.
4. Send `/threads` and test a `Resume` button.
5. Start a prompt that requires shell approval, then test both approval buttons
   and the text fallback `/allow <approval_id>` / `/deny <approval_id>`.
6. Restart the VM and confirm both services come back.
