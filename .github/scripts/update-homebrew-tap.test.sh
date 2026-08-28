#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "${tmp_dir}"' EXIT

manifest="${tmp_dir}/ghosty-artifacts-sha256.txt"
formula="${tmp_dir}/ghosty.rb"
legacy="${tmp_dir}/deepseek-tui.rb"

assets=(
  ghosty-macos-arm64
  ghosty-tui-macos-arm64
  ghosty-macos-x64
  ghosty-tui-macos-x64
  ghosty-linux-arm64
  ghosty-tui-linux-arm64
  ghosty-linux-x64
  ghosty-tui-linux-x64
)

for asset in "${assets[@]}"; do
  printf '%064d  %s\n' 0 "${asset}" >> "${manifest}"
done

TAG=v1.2.3 \
MANIFEST="${manifest}" \
TAP_REPO=blissito/homebrew-ghosty \
FORMULA_OUTPUT="${formula}" \
FORMULA_LEGACY_OUTPUT="${legacy}" \
  bash "${repo_root}/.github/scripts/update-homebrew-tap.sh"

ruby -c "${formula}" >/dev/null
ruby -c "${legacy}" >/dev/null
grep -Fq 'class Ghosty < Formula' "${formula}"
grep -Fq 'class DeepseekTui < Formula' "${legacy}"
grep -Fq 'deprecate! date: "2026-08-14", because: "renamed to ghosty"' "${legacy}"
grep -Fq 'desc "Agentic terminal for open-source and open-weight coding models"' "${formula}"
test "$(grep -Fc 'resource "ghosty-tui" do' "${formula}")" -eq 4
grep -Fq 'bin.install Dir["*"].first => "ghosty-tui"' "${formula}"
grep -Fq 'system "#{bin}/ghosty-tui", "--version"' "${formula}"
if grep -Fq 'ghosty-tui' "${formula}"; then
  echo "Homebrew formula must not install the legacy TUI compatibility asset" >&2
  exit 1
fi
if grep -Fq 'class DeepseekTui' "${formula}"; then
  echo "Primary Homebrew formula must be Ghosty, not DeepseekTui" >&2
  exit 1
fi

echo "update-homebrew-tap tests passed"
