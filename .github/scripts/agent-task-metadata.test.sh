#!/usr/bin/env bash
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo}"

template=".github/ISSUE_TEMPLATE/agent-task.yml"
workflow=".github/workflows/agent-task-labels.yml"
retired_workflow=".github/workflows/v0868-milestone-sync.yml"

fail() {
  echo "agent-task metadata contract failed: $*" >&2
  exit 1
}

[[ -f "${template}" ]] || fail "missing ${template}"
[[ -f "${workflow}" ]] || fail "missing ${workflow}"
[[ ! -e "${retired_workflow}" ]] || fail "retired milestone sync still exists"

if grep -nE '^(title|labels):.*v[0-9]+\.[0-9]+\.[0-9]+' "${template}"; then
  fail "agent-task defaults must not pin a release version"
fi

grep -qF 'labels: ["agent-ready"]' "${template}" \
  || fail "agent-task template must opt into agent-ready without a version label"
grep -qF 'types: [opened]' "${workflow}" \
  || fail "agent-task labeling must run only when an issue is opened"
grep -qF "labels.has('agent-ready')" "${workflow}" \
  || fail "agent-task labeling must be gated by the explicit agent-ready label"

if grep -nE 'listMilestones|issues\.update|^[[:space:]]*milestone:' "${workflow}"; then
  fail "agent-task automation must never read or assign milestones"
fi

if grep -nE 'types:.*(labeled|milestoned)' "${workflow}"; then
  fail "agent-task automation must not react to later label or milestone edits"
fi

echo "Agent-task metadata is release-neutral and milestone-safe."
