#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/release/crates.sh
source "${script_dir}/crates.sh"

mode="${1:-dry-run}"
case "${mode}" in
  dry-run|publish) ;;
  *)
    echo "usage: $0 [dry-run|publish]" >&2
    exit 1
    ;;
esac

packages=("${release_crates[@]}")
crates_user_agent="GhostyCode release publish check (https://github.com/blissito/ghostycode)"

workspace_version=""
workspace_ghosty_packages=()
workspace_package_dep_flags=()

metadata_inventory="$(
  python3 "${script_dir}/validate-crate-publish-order.py" "${packages[@]}"
)"
while IFS=$'\t' read -r kind name value; do
  case "${kind}" in
    version)
      workspace_version="${name}"
      ;;
    crate)
      workspace_ghosty_packages+=("${name}")
      workspace_package_dep_flags+=("${value}")
      ;;
  esac
done <<<"${metadata_inventory}"

if [[ -z "${workspace_version}" ]]; then
  echo "Could not determine workspace version." >&2
  exit 1
fi

echo "Crate publication order OK: ${#packages[@]} workspace crates."

if [[ "${mode}" == "publish" ]]; then
  "${script_dir}/require-release-tag-checkout.sh"
  "${script_dir}/verify-release-assets.sh"
fi

package_has_workspace_deps() {
  local package_name="$1"
  local index
  for ((index = 0; index < ${#workspace_ghosty_packages[@]}; index += 1)); do
    if [[ "${workspace_ghosty_packages[$index]}" == "${package_name}" ]]; then
      [[ "${workspace_package_dep_flags[$index]}" == "1" ]]
      return
    fi
  done

  echo "Unknown workspace crate: ${package_name}" >&2
  return 1
}

crate_version_exists() {
  local crate_name="$1"
  local crate_version="$2"
  curl -fsSL -A "${crates_user_agent}" "https://crates.io/api/v1/crates/${crate_name}/${crate_version}" >/dev/null 2>&1
}

wait_for_crate_version() {
  local crate_name="$1"
  local crate_version="$2"
  local attempts=30

  for ((attempt = 1; attempt <= attempts; attempt += 1)); do
    if crate_version_exists "${crate_name}" "${crate_version}"; then
      return 0
    fi
    echo "Waiting for ${crate_name} ${crate_version} to appear on crates.io (${attempt}/${attempts})..."
    sleep 10
  done

  echo "Timed out waiting for ${crate_name} ${crate_version} to appear on crates.io" >&2
  return 1
}

for package in "${packages[@]}"; do
  echo "::group::${mode} ${package}"
  if [[ "${mode}" == "dry-run" ]]; then
    if package_has_workspace_deps "${package}"; then
      cargo package --allow-dirty --locked --list -p "${package}" >/dev/null
      echo "Verified package contents for ${package}; full crates.io dry-run requires workspace dependencies at ${workspace_version} to be published first."
    else
      cargo publish --dry-run --locked --allow-dirty -p "${package}"
    fi
  else
    if crate_version_exists "${package}" "${workspace_version}"; then
      echo "Skipping ${package} ${workspace_version}; already published."
    else
      cargo publish --locked -p "${package}"
      wait_for_crate_version "${package}" "${workspace_version}"
    fi
  fi
  echo "::endgroup::"
done
