#!/usr/bin/env bash
# Hermetic test for scripts/release/ensure-release-on-main.sh.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
guard="${script_dir}/ensure-release-on-main.sh"

work="$(mktemp -d)"
cleanup() { rm -rf "${work}"; }
trap cleanup EXIT

fail=0
check() {
  local desc="$1" needle="$2" hay
  hay="$(cat)"
  if grep -qF -- "${needle}" <<<"${hay}"; then
    echo "ok   - ${desc}"
  else
    echo "FAIL - ${desc}"
    echo "       expected to find: ${needle}"
    echo "------ output ------"
    echo "${hay}"
    echo "--------------------"
    fail=1
  fi
}

mkdir -p "${work}/repo/scripts/release"
cp "${guard}" "${work}/repo/scripts/release/ensure-release-on-main.sh"
guard="${work}/repo/scripts/release/ensure-release-on-main.sh"

cd "${work}/repo"
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null
git init -q -b main .
git config user.name "Release Test"
git config user.email "release-test@example.com"

commit() {
  echo "$2" >"$1"
  git add -A
  git commit -q -m "$3"
}

commit README.md "base" "base"
base_sha="$(git rev-parse HEAD)"

git init -q --bare "${work}/origin.git"
git remote add origin "${work}/origin.git"
git push -q -u origin main

# Prove the guard fetches origin/main when the remote-tracking ref is absent.
git update-ref -d refs/remotes/origin/main
base_out="$(bash "${guard}" "${base_sha}" 2>&1)"
check "main commit is accepted after fetching origin/main" \
  "is reachable from origin/main" <<<"${base_out}"

git switch -q -c release-only
commit release.txt "branch-only" "branch-only release work"
release_sha="$(git rev-parse HEAD)"

set +e
reject_out="$(bash "${guard}" "${release_sha}" 2>&1)"
reject_ec=$?
set -e
check "branch-only commit is rejected" \
  "is not reachable from origin/main" <<<"${reject_out}"
if [[ "${reject_ec}" -ne 1 ]]; then
  echo "FAIL - branch-only commit should exit 1, got ${reject_ec}"
  fail=1
else
  echo "ok   - branch-only commit exits non-zero"
fi

git switch -q main
git merge -q --ff-only release-only
git push -q origin main
git update-ref -d refs/remotes/origin/main
merged_out="$(bash "${guard}" "${release_sha}" 2>&1)"
check "commit is accepted after release branch lands on main" \
  "is reachable from origin/main" <<<"${merged_out}"

echo
if [[ "${fail}" -eq 0 ]]; then
  echo "ensure-release-on-main.test.sh: all checks passed"
else
  echo "ensure-release-on-main.test.sh: FAILURES above"
fi
exit "${fail}"
