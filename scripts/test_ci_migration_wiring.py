#!/usr/bin/env python3
"""Hermetic tests for the FEAT-015 migration-gate CI wiring.

Verifies `.github/workflows/ci.yml` keeps both migration checker commands
(self-tests then live scan) under the same `heavy` condition as the existing
command-contract boundary step, and that the boundary step itself remains
intact.
"""

from __future__ import annotations

import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CI_PATH = ROOT / ".github" / "workflows" / "ci.yml"


def load_ci() -> str:
    return CI_PATH.read_text(encoding="utf-8")


def boundary_step_block(ci: str) -> str:
    """Extract the 'Check command-contract prototype boundary' step block."""
    marker = "Check command-contract prototype boundary"
    start = ci.index(marker)
    step_start = ci.rindex("- name:", 0, start)
    # The step ends at the next "- name:" after the marker.
    next_step = ci.index("- name:", start + len(marker))
    return ci[step_start:next_step]


def migration_step_block(ci: str) -> str:
    marker = "Check command migration manifest"
    start = ci.index(marker)
    step_start = ci.rindex("- name:", 0, start)
    next_step = ci.index("- name:", start + len(marker))
    return ci[step_start:next_step]


class CiWiringTests(unittest.TestCase):
    def test_boundary_step_still_present(self) -> None:
        ci = load_ci()
        self.assertIn("Check command-contract prototype boundary", ci)
        block = boundary_step_block(ci)
        self.assertIn("test_check_command_crate_boundaries.py", block)
        self.assertIn("check-command-crate-boundaries.py", block)

    def test_migration_self_test_present(self) -> None:
        ci = load_ci()
        block = migration_step_block(ci)
        self.assertIn("test_check_command_migration_manifest.py", block)

    def test_migration_live_scan_present(self) -> None:
        ci = load_ci()
        block = migration_step_block(ci)
        self.assertIn("check-command-migration-manifest.py", block)

    def test_migration_live_scan_receives_fetched_baseline(self) -> None:
        block = migration_step_block(load_ci())
        self.assertIn("PR_BASE_SHA", block)
        self.assertIn("PUSH_BEFORE_SHA", block)
        self.assertIn("git fetch --no-tags origin", block)
        self.assertNotIn(
            "--depth",
            block,
            "baseline fetch must preserve the full ancestry used by later CI range checks",
        )
        self.assertIn('--baseline-ref "${baseline}"', block)

    def test_migration_commands_are_ordered_self_test_first(self) -> None:
        ci = load_ci()
        block = migration_step_block(ci)
        self.assertLess(
            block.index("test_check_command_migration_manifest.py"),
            block.index("check-command-migration-manifest.py"),
            "checker self-tests must run before the live migration scan",
        )

    def test_migration_step_uses_heavy_condition(self) -> None:
        ci = load_ci()
        block = migration_step_block(ci)
        self.assertIn("needs.changes.outputs.heavy == 'true'", block)

    def test_boundary_step_condition_matches_migration_step(self) -> None:
        ci = load_ci()
        boundary = boundary_step_block(ci)
        migration = migration_step_block(ci)
        self.assertIn("needs.changes.outputs.heavy == 'true'", boundary)
        self.assertEqual(
            "needs.changes.outputs.heavy == 'true'" in boundary,
            "needs.changes.outputs.heavy == 'true'" in migration,
        )

    def test_migration_step_does_not_remove_boundary_step(self) -> None:
        ci = load_ci()
        # Both steps must coexist (the migration step is added beside, never
        # replacing, the boundary step).
        self.assertLess(
            ci.index("Check command-contract prototype boundary"),
            ci.index("Check command migration manifest"),
        )

    def test_main_ci_concurrency_is_keyed_by_sha(self) -> None:
        ci = load_ci()
        self.assertIn("github.workflow, github.sha", ci)
        self.assertIn("ci-pr-{0}", ci)
        self.assertNotIn(
            "github.event.pull_request.number || github.ref",
            ci,
            "main must not share one concurrency group per ref — pending runs get cancelled",
        )
        self.assertIn("name: Safety gate", ci)
        self.assertIn("Hermetic safety and authorization tests", ci)

    def test_safety_gate_is_hermetic_for_config_home(self) -> None:
        ci = load_ci()
        start = ci.index("Hermetic safety and authorization tests")
        next_step = ci.index("- name:", start + 1)
        block = ci[start:next_step]
        self.assertIn("GHOSTY_HOME:", block)
        self.assertIn("cw-hermetic-home", block)
        self.assertIn("unset GHOSTY_CONFIG_PATH DEEPSEEK_CONFIG_PATH DEEPSEEK_HOME", block)
        self.assertIn("command_safety auto_review authority sandbox", block)

    def test_valid_wiring_passes_all_assertions(self) -> None:
        # The live workflow must satisfy every structural invariant above.
        ci = load_ci()
        self.assertIn("test_check_command_migration_manifest.py", ci)
        self.assertIn("check-command-migration-manifest.py", ci)
        self.assertIn("needs.changes.outputs.heavy == 'true'", ci)


if __name__ == "__main__":
    unittest.main()
