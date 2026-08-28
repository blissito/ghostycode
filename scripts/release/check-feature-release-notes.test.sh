#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
checker="${repo_root}/scripts/release/check-feature-release-notes.sh"
fixture="$(mktemp -d)"
trap 'rm -rf "${fixture}"' EXIT

git -C "${fixture}" init -q
git -C "${fixture}" config user.name "Release Test"
git -C "${fixture}" config user.email "release-test@example.com"
mkdir -p "${fixture}/docs"
printf '# Changelog\n' >"${fixture}/CHANGELOG.md"
printf '# Archive\n' >"${fixture}/docs/CHANGELOG_ARCHIVE.md"
git -C "${fixture}" add CHANGELOG.md docs/CHANGELOG_ARCHIVE.md
git -C "${fixture}" commit -qm "chore: seed release notes"
git -C "${fixture}" tag v1.0.0

printf 'feature\n' >"${fixture}/feature.txt"
git -C "${fixture}" add feature.txt
git -C "${fixture}" commit -qm "feat(tui): add multiline composer (#123)"

if RELEASE_NOTE_REPO_ROOT="${fixture}" "${checker}" v1.0.0 HEAD >"${fixture}/missing.out" 2>&1; then
  echo "missing feature receipt unexpectedly passed" >&2
  exit 1
fi
grep -Fq 'references #123, but no release-note receipt exists' "${fixture}/missing.out"

printf '\n- Multiline composer mode (#123).\n' >>"${fixture}/CHANGELOG.md"
git -C "${fixture}" add CHANGELOG.md
git -C "${fixture}" commit -qm "docs: record multiline composer"
RELEASE_NOTE_REPO_ROOT="${fixture}" "${checker}" v1.0.0 HEAD >"${fixture}/present.out"
grep -Fq '1 linked issue reference(s) checked' "${fixture}/present.out"

printf 'color\n' >"${fixture}/color.txt"
git -C "${fixture}" add color.txt
git -C "${fixture}" commit -qm 'feat(tui): use brand cyan #48D7FF'
RELEASE_NOTE_REPO_ROOT="${fixture}" "${checker}" v1.0.0 HEAD >"${fixture}/color.out"
grep -Fq '1 linked issue reference(s) checked' "${fixture}/color.out"

printf 'reconciled\n' >"${fixture}/reconciled.txt"
git -C "${fixture}" add reconciled.txt
git -C "${fixture}" commit -qm 'feat(tui): reconcile earlier feature (#456)'
printf '\n- Earlier release already recorded this feature (#456).\n' >>"${fixture}/docs/CHANGELOG_ARCHIVE.md"
RELEASE_NOTE_REPO_ROOT="${fixture}" "${checker}" v1.0.0 HEAD >"${fixture}/archive.out"
grep -Fq '2 linked issue reference(s) checked' "${fixture}/archive.out"

echo "check-feature-release-notes tests passed"
