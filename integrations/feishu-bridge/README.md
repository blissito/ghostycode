# Feishu / Lark Bridge

This bridge lets a Feishu or Lark chat control a local `ghosty serve --http`
runtime from a phone. It uses the official Lark/Feishu Node SDK long-connection
mode, so the first version does not need a public webhook URL.

Security model:

- `ghosty serve --http` stays bound to `127.0.0.1`.
- `/v1/*` runtime calls use `GHOSTY_RUNTIME_TOKEN`.
- Feishu/Lark chats must be allowlisted in `GHOSTY_CHAT_ALLOWLIST` unless
  `GHOSTY_ALLOW_UNLISTED=true`
  is set for first pairing.
- Direct messages are the intended MVP control surface. Group chat control is
  disabled unless `FEISHU_ALLOW_GROUPS=true`.
- Tool approvals are text commands: `/allow <approval_id>` or `/deny <approval_id>`.
- Feishu/Lark only sees the prompts, status, thread summaries, and approval
  messages the bridge sends. The workspace, shell, and runtime HTTP listener
  stay local behind the Ghosty runtime token.

## Setup

```bash
cd /opt/ghosty/feishu-bridge
npm install --omit=dev
cp .env.example /etc/ghosty/feishu-bridge.env
sudoedit /etc/ghosty/feishu-bridge.env
node src/index.mjs
```

Validate the env files before starting the service:

```bash
npm run validate:config -- \
  --env /etc/ghosty/feishu-bridge.env \
  --runtime-env /etc/ghosty/runtime.env \
  --workspace-root /opt/whalebro \
  --check-filesystem
```

For first pairing, temporarily set `GHOSTY_ALLOW_UNLISTED=true`, send the
bot `/status`, copy the returned `chat_id`, `open_id`, or `union_id` into
`GHOSTY_CHAT_ALLOWLIST`, then turn `GHOSTY_ALLOW_UNLISTED=false`.

For a Tencent Lighthouse deployment, use:

```bash
sudo systemctl enable --now ghosty-runtime ghosty-feishu-bridge
sudo journalctl -u ghosty-feishu-bridge -f
```

## Commands

- `/status`
- `/threads`
- `/new`
- `/resume <thread_id>`
- `/model <name|default>`
- `/interrupt`
- `/compact`
- `/allow <approval_id> [remember]`
- `/deny <approval_id>`

Anything else is sent as a prompt. If group control is explicitly enabled,
messages should start with the Ghosty prefix `/cw`, for example:

```text
/cw check git status and tell me what is dirty
```
