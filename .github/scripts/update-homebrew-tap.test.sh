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
# There was a check here refusing any `ghosty-tui` mention in the formula. It
# dated from the 0.9.5 single-runtime bridge and was reversed by the 0.0.15
# rebase, which ships `ghosty-tui-*` assets again (see the release asset list).
# It contradicted the three assertions right above, which require exactly those
# strings, so this file could never pass. Removed rather than "fixed": the
# assertions above already pin what the formula must install.
if grep -Fq 'class DeepseekTui' "${formula}"; then
  echo "Primary Homebrew formula must be Ghosty, not DeepseekTui" >&2
  exit 1
fi

echo "update-homebrew-tap tests passed"
