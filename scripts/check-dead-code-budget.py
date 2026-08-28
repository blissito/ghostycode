#!/usr/bin/env python3
"""Ratchet on `#[allow(dead_code)]` so the wall can shrink but never regrow (#4785).

Why this exists rather than a one-time sweep:

    issue filed   2026-07-??   464 attributes / 143 files
    audit         2026-07-26   426 / 111
    audit         2026-07-28   481 / 155

The sweep was working and the count still went *up*, because two large landings
added state whose accessors only their own tests read. A sweep is a snapshot; a
budget is a direction. This gate makes the number a one-way door.

It deliberately does NOT judge whether any individual attribute is justified —
plenty are. It only refuses to let the total rise, which is the property the
issue actually needs and the only one that can be checked mechanically.

Note the blind spot this compensates for: CI's clippy runs without
`--all-targets`, so it never lints `cfg(test)` or integration-test code. A prior
strip-and-check measured 197 attributes alive *only* because a test references
them — exactly the ones a test-blind lint can never adjudicate.

Usage:
    python3 scripts/check-dead-code-budget.py           # enforce
    python3 scripts/check-dead-code-budget.py --update  # rewrite the budget file
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
CRATES_DIR = REPO_ROOT / "crates"
BUDGET_PATH = REPO_ROOT / "scripts" / "dead-code-budget.json"

# Matches `#[allow(dead_code)]`, `#![allow(dead_code)]`, and combined forms like
# `#[allow(dead_code, clippy::large_enum_variant)]`.
PATTERN = re.compile(r"allow\(\s*dead_code\b")


def measure() -> tuple[int, dict[str, int]]:
    """Return (total occurrences, per-crate counts)."""
    per_crate: dict[str, int] = {}
    total = 0
    for path in sorted(CRATES_DIR.rglob("*.rs")):
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        hits = len(PATTERN.findall(text))
        if not hits:
            continue
        crate = path.relative_to(CRATES_DIR).parts[0]
        per_crate[crate] = per_crate.get(crate, 0) + hits
        total += hits
    return total, per_crate


def load_budget() -> dict:
    if not BUDGET_PATH.exists():
        sys.exit(f"missing budget file: {BUDGET_PATH.relative_to(REPO_ROOT)}")
    return json.loads(BUDGET_PATH.read_text(encoding="utf-8"))


def write_budget(total: int, per_crate: dict[str, int]) -> None:
    payload = {
        "_comment": (
            "Ceiling for `#[allow(dead_code)]` across crates/. This number may "
            "go down freely; raising it needs a reviewer to say why in the PR. "
            "Regenerate with: python3 scripts/check-dead-code-budget.py --update"
        ),
        "_issue": "https://github.com/blissito/ghostycode/issues/4785",
        "total": total,
        "per_crate": dict(sorted(per_crate.items())),
    }
    BUDGET_PATH.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--update",
        action="store_true",
        help="rewrite the budget file from the working tree",
    )
    args = parser.parse_args()

    total, per_crate = measure()

    if args.update:
        write_budget(total, per_crate)
        rel = BUDGET_PATH.relative_to(REPO_ROOT)
        print(f"[dead-code-budget] wrote {rel}: total={total}")
        return 0

    budget = load_budget()
    ceiling = int(budget["total"])

    if total > ceiling:
        print(
            f"[dead-code-budget] FAIL: {total} `#[allow(dead_code)]` attributes, "
            f"budget is {ceiling} (+{total - ceiling}).",
            file=sys.stderr,
        )
        print("", file=sys.stderr)
        print("Per crate now vs. budget:", file=sys.stderr)
        recorded = budget.get("per_crate", {})
        for crate in sorted(set(per_crate) | set(recorded)):
            now = per_crate.get(crate, 0)
            was = int(recorded.get(crate, 0))
            marker = "  <-- grew" if now > was else ""
            print(f"  {crate:<16} {now:>4}  (budget {was}){marker}", file=sys.stderr)
        print("", file=sys.stderr)
        print(
            "Either delete the dead item, or narrow the attribute to the one item\n"
            "that needs it instead of a whole module. If the growth is genuinely\n"
            "justified, run `python3 scripts/check-dead-code-budget.py --update`\n"
            "and say why in the PR description — the point of this gate is that\n"
            "raising the number is a visible decision, not an accident.",
            file=sys.stderr,
        )
        return 1

    if total < ceiling:
        print(
            f"[dead-code-budget] {total} attributes, budget {ceiling} "
            f"({ceiling - total} under). Lower the budget to lock in the win:\n"
            f"  python3 scripts/check-dead-code-budget.py --update"
        )
        return 0

    print(f"[dead-code-budget] PASS: {total} attributes, exactly at budget.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
