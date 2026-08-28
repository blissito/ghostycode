#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "${tmp_dir}"' EXIT

fixture="${tmp_dir}/repo"
src_dir="${fixture}/target/release"
dest_dir="${tmp_dir}/installed"
receipt_dir="${tmp_dir}/receipts"
fake_bin="${tmp_dir}/bin"
marker="${tmp_dir}/invocations.log"

mkdir -p "${fixture}/scripts/release" "${src_dir}" "${dest_dir}" "${fake_bin}"
cp "${repo_root}/scripts/release/install-dogfood.sh" \
  "${fixture}/scripts/release/install-dogfood.sh"
printf 'target/\n' >"${fixture}/.gitignore"

git -C "${fixture}" init --quiet
git -C "${fixture}" config user.name "Dogfood Test"
git -C "${fixture}" config user.email "dogfood-test@example.invalid"
git -C "${fixture}" add .gitignore scripts/release/install-dogfood.sh
git -C "${fixture}" -c commit.gpgsign=false commit --quiet -m "fixture"
source_sha="$(git -C "${fixture}" rev-parse HEAD)"

make_binary() {
  cat >"${src_dir}/ghosty" <<EOF
#!/usr/bin/env bash
set -euo pipefail
if [[ "\${1:-}" == "--version" ]]; then
  printf '%s\n' 'ghosty 0.9.1 (${source_sha})'
  printf '%s\n' "\${0##*/}" >>"\${DOGFOOD_TEST_MARKER}"
  exit 0
fi
exit 2
EOF
  chmod +x "${src_dir}/ghosty"
}

make_binary

# Reproduce the old dogfood state: ghosty-tui was a symlink to the dispatcher.
printf 'old dispatcher\n' >"${dest_dir}/ghosty"
ln -s "${dest_dir}/ghosty" "${dest_dir}/ghosty-tui"

cat >"${fake_bin}/zsh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ "${1:-}" == "-lc" ]]
case "${2:-}" in
  "command -v ghosty") printf '%s\n' "${DOGFOOD_TEST_DEST}/ghosty" ;;
  "command -v ghosty-tui") printf '%s\n' "${DOGFOOD_TEST_DEST}/ghosty-tui" ;;
  "ghosty --version") exec "${DOGFOOD_TEST_DEST}/ghosty" --version ;;
  "ghosty-tui --version") exec "${DOGFOOD_TEST_DEST}/ghosty-tui" --version ;;
  *) exit 2 ;;
esac
EOF
chmod +x "${fake_bin}/zsh"

HOME="${tmp_dir}/home" \
PATH="${fake_bin}:${PATH}" \
DOGFOOD_TEST_DEST="${dest_dir}" \
DOGFOOD_TEST_MARKER="${marker}" \
GHOSTY_INSTALL_DIRS="${dest_dir}" \
GHOSTY_DOGFOOD_RECEIPT_DIR="${receipt_dir}" \
  "${fixture}/scripts/release/install-dogfood.sh" "${src_dir}" >/dev/null

for name in ghosty ghosty-tui; do
  cmp -s "${src_dir}/ghosty" "${dest_dir}/${name}" || {
    echo "installed ${name} differs from the built fixture" >&2
    exit 1
  }
done

if [[ -L "${dest_dir}/ghosty-tui" ]]; then
  echo "dogfood install left ghosty-tui as a symlink" >&2
  exit 1
fi

[[ "$(grep -c '^ghosty-tui$' "${marker}")" -ge 1 ]] || {
  echo "the installed ghosty-tui command name was not exercised" >&2
  exit 1
}

receipt="$(find "${receipt_dir}" -type f -name '*.txt' -print -quit)"
[[ -n "${receipt}" ]]
grep -Fq "codew_sha256=" "${receipt}"
grep -Fq "fresh_shell_codew=${dest_dir}/ghosty-tui" "${receipt}"
grep -Fq "installed_path=${dest_dir}/ghosty-tui" "${receipt}"

echo "install-dogfood tests passed"
