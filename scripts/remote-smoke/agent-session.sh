#!/usr/bin/env bash
# Source into an interactive agent shell (tmux, ssh) to export the provider
# key and set defaults that systemd normally handles via EnvironmentFile=.
#
# Usage (as the ghosty user):
#   . /opt/whalebro/ghosty/scripts/remote-smoke/agent-session.sh
#   ghosty models           # should list deepseek-v4-pro
#   gh auth status             # should show the fine-grained PAT
#
# The runtime.env file is 0640 root:ghosty, readable by the ghosty user.
set -a
# shellcheck disable=SC1091
. /etc/ghosty/runtime.env
set +a
export GHOSTY_MODEL="${GHOSTY_MODEL:-deepseek-v4-pro}"
