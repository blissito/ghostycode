#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "${tmp_dir}"' EXIT

remote="${tmp_dir}/remote.git"
source_repo="${tmp_dir}/source"
git init --bare --quiet "${remote}"
git init --quiet "${source_repo}"
git -C "${source_repo}" config user.name "Release Test"
git -C "${source_repo}" config user.email "release-test@example.invalid"
git -C "${source_repo}" remote add origin "${remote}"

printf 'one\n' > "${source_repo}/payload"
git -C "${source_repo}" add payload
git -C "${source_repo}" commit --quiet -m one
first_sha="$(git -C "${source_repo}" rev-parse HEAD)"
git -C "${source_repo}" tag -a v1.2.3 -m v1.2.3
git -C "${source_repo}" push --quiet origin refs/tags/v1.2.3

"${repo_root}/scripts/release/verify-remote-tag.sh" \
  "${remote}" v1.2.3 "${first_sha}"

printf 'two\n' > "${source_repo}/payload"
git -C "${source_repo}" commit --quiet -am two
second_sha="$(git -C "${source_repo}" rev-parse HEAD)"
git -C "${source_repo}" tag -f -a v1.2.3 -m moved >/dev/null
git -C "${source_repo}" push --quiet --force origin refs/tags/v1.2.3

if "${repo_root}/scripts/release/verify-remote-tag.sh" \
  "${remote}" v1.2.3 "${first_sha}" >/dev/null 2>&1; then
  echo "moved tag unexpectedly passed the old-SHA check" >&2
  exit 1
fi
"${repo_root}/scripts/release/verify-remote-tag.sh" \
  "${remote}" v1.2.3 "${second_sha}"

echo "verify-remote-tag tests passed"
