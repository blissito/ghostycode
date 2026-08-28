#!/usr/bin/env bash
# Verify a release commit is reachable from the canonical main branch.
#
# GitHub only processes "Closes #N" keywords when the closing commit lands on
# the repository default branch. Refuse branch-only release tags so release
# assets cannot ship from commits that main does not contain.
set -euo pipefail

usage() {
  cat >&2 <<'EOF'
usage: ensure-release-on-main.sh [--remote <name>] [--main <branch>] <commit-ish>

Defaults:
  --remote origin
  --main   main
EOF
}

remote="origin"
main_branch="main"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --remote)
      remote="${2:?missing value for --remote}"
      shift 2
      ;;
    --main)
      main_branch="${2:?missing value for --main}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    -*)
      echo "error: unknown option '$1'" >&2
      usage
      exit 2
      ;;
    *)
      break
      ;;
  esac
done

if [[ $# -ne 1 ]]; then
  usage
  exit 2
fi

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo}"

commit="$1"
sha="$(git rev-parse --verify "${commit}^{commit}" 2>/dev/null)" || {
  echo "error: '${commit}' is not a commit" >&2
  exit 2
}

main_ref="refs/remotes/${remote}/${main_branch}"
git fetch --no-tags "${remote}" "+refs/heads/${main_branch}:${main_ref}" >/dev/null

main_sha="$(git rev-parse --verify "${main_ref}^{commit}" 2>/dev/null)" || {
  echo "error: could not resolve ${main_ref}" >&2
  exit 2
}

if git merge-base --is-ancestor "${sha}" "${main_sha}"; then
  echo "Release source ${sha} is reachable from ${remote}/${main_branch} (${main_sha})."
  exit 0
fi

cat >&2 <<EOF
::error::Release source ${sha} is not reachable from ${remote}/${main_branch} (${main_sha}).
Merge the release PR into ${main_branch} before tagging so GitHub processes
closing keywords and the release PR shows as merged.
EOF
exit 1
