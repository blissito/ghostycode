#!/usr/bin/env bash

# Crates published for each ghosty release, in dependency order.
release_crates=(
  ghosty-build-support
  ghosty-mcp
  ghosty-paths
  ghosty-protocol
  ghosty-release
  ghosty-secrets
  ghosty-state
  ghosty-workflow
  ghosty-workflow-js
  ghosty-execpolicy
  ghosty-hooks
  ghosty-tools
  ghosty-config
  # Path+version dependency of cli/tui — must publish before those crates.
  ghosty-telemetry
  ghosty-lane
  ghosty-agent
  ghosty-core
  # Prototype command boundary depends on core; future TUI/commands adapters
  # consume it without changing current production dispatch in FEAT-014.
  ghosty-command-contract
  ghosty-tui
  ghosty-app-server
  ghosty-cli
)
