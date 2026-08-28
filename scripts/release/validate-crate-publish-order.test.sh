#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "${tmp_dir}"' EXIT

metadata_file="${tmp_dir}/metadata.json"
cat >"${metadata_file}" <<'JSON'
{
  "workspace_members": ["build", "core", "tui", "app", "cli"],
  "packages": [
    {
      "id": "build",
      "name": "ghosty-build-support",
      "version": "0.9.5",
      "dependencies": []
    },
    {
      "id": "core",
      "name": "ghosty-core",
      "version": "0.9.5",
      "dependencies": [
        {"name": "ghosty-cli", "path": "/workspace/cli", "kind": "dev"}
      ]
    },
    {
      "id": "tui",
      "name": "ghosty-tui",
      "version": "0.9.5",
      "dependencies": [
        {"name": "ghosty-build-support", "path": "/workspace/build", "kind": "build"},
        {"name": "ghosty-core", "path": "/workspace/core", "kind": null}
      ]
    },
    {
      "id": "app",
      "name": "ghosty-app-server",
      "version": "0.9.5",
      "dependencies": [
        {"name": "ghosty-core", "path": "/workspace/core", "kind": null}
      ]
    },
    {
      "id": "cli",
      "name": "ghosty-cli",
      "version": "0.9.5",
      "dependencies": [
        {"name": "ghosty-tui", "path": "/workspace/tui", "kind": null},
        {"name": "ghosty-app-server", "path": "/workspace/app", "kind": null}
      ]
    }
  ]
}
JSON

expect_validation_failure() {
  local label="$1"
  local expected="$2"
  local fixture="$3"
  shift 3

  local output="${tmp_dir}/${label}.txt"
  if python3 "${script_dir}/validate-crate-publish-order.py" \
    --metadata-file "${fixture}" "$@" >"${output}" 2>&1; then
    echo "${label} unexpectedly passed" >&2
    exit 1
  fi
  grep -F "${expected}" "${output}" >/dev/null
  if grep -F "Traceback" "${output}" >/dev/null; then
    echo "${label} emitted an unhandled traceback" >&2
    exit 1
  fi
}

expect_validation_failure \
  duplicate-crate \
  "publish package list contains duplicates: ghosty-core" \
  "${metadata_file}" \
  ghosty-build-support \
  ghosty-core \
  ghosty-core \
  ghosty-tui \
  ghosty-app-server \
  ghosty-cli

expect_validation_failure \
  missing-crate \
  "publish package list is missing workspace crates: ghosty-app-server" \
  "${metadata_file}" \
  ghosty-build-support \
  ghosty-core \
  ghosty-tui \
  ghosty-cli

expect_validation_failure \
  extra-crate \
  "publish package list contains non-workspace crates: ghosty-extra" \
  "${metadata_file}" \
  ghosty-build-support \
  ghosty-core \
  ghosty-tui \
  ghosty-app-server \
  ghosty-cli \
  ghosty-extra

mixed_metadata_file="${tmp_dir}/mixed-metadata.json"
python3 - "${metadata_file}" "${mixed_metadata_file}" <<'PY'
import pathlib
import sys

source = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
mixed = source.replace('"version": "0.9.5"', '"version": "0.9.4"', 1)
if mixed == source:
    raise SystemExit("failed to create mixed-version Cargo metadata fixture")
pathlib.Path(sys.argv[2]).write_text(mixed, encoding="utf-8")
PY
expect_validation_failure \
  mixed-versions \
  "workspace packages have mixed versions: 0.9.4, 0.9.5" \
  "${mixed_metadata_file}" \
  ghosty-build-support \
  ghosty-core \
  ghosty-tui \
  ghosty-app-server \
  ghosty-cli

nonrelease_metadata_file="${tmp_dir}/nonrelease-metadata.json"
cat >"${nonrelease_metadata_file}" <<'JSON'
{
  "workspace_members": ["helper", "core"],
  "packages": [
    {
      "id": "helper",
      "name": "internal-helper",
      "version": "0.9.5",
      "dependencies": []
    },
    {
      "id": "core",
      "name": "ghosty-core",
      "version": "0.9.5",
      "dependencies": [
        {"name": "internal-helper", "path": "/workspace/helper", "kind": null}
      ]
    }
  ]
}
JSON
expect_validation_failure \
  nonrelease-workspace-dependency \
  "ghosty-core depends on workspace crate internal-helper [normal], which is not in the ghosty-* release inventory" \
  "${nonrelease_metadata_file}" \
  ghosty-core

bad_output="${tmp_dir}/bad-order.txt"
if python3 "${script_dir}/validate-crate-publish-order.py" \
  --metadata-file "${metadata_file}" \
  ghosty-build-support \
  ghosty-tui \
  ghosty-core \
  ghosty-app-server \
  ghosty-cli >"${bad_output}" 2>&1; then
  echo "v0.9.5 publication order unexpectedly passed" >&2
  exit 1
fi
grep -F \
  "ghosty-tui (position 2) depends on ghosty-core (position 3) [normal]" \
  "${bad_output}" >/dev/null

good_output="${tmp_dir}/good-order.txt"
python3 "${script_dir}/validate-crate-publish-order.py" \
  --metadata-file "${metadata_file}" \
  ghosty-build-support \
  ghosty-core \
  ghosty-tui \
  ghosty-app-server \
  ghosty-cli >"${good_output}"
grep -F $'version\t0.9.5\t' "${good_output}" >/dev/null
grep -F $'crate\tghosty-core\t1' "${good_output}" >/dev/null

# Keep the checked-in order synchronized with the live locked workspace graph.
# shellcheck source=scripts/release/crates.sh
source "${script_dir}/crates.sh"
python3 "${script_dir}/validate-crate-publish-order.py" \
  "${release_crates[@]}" >"${tmp_dir}/workspace-order.txt"

echo "crate publication order validation tests passed"
