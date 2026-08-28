#!/usr/bin/env python3
"""Measure canonical production-built tool catalogs by visible mode.

This delegates to an ignored Rust test that runs the production registry,
catalog, and active-request planner with inert wiring. Token counts are
deterministic estimates using ceil(serialized_bytes/4).
"""

from __future__ import annotations

import json
import subprocess
import sys


MARKER = "TOOL_CATALOG_METRICS "


def main() -> int:
    cmd = [
        "cargo",
        "test",
        "--locked",
        "-p",
        "ghosty-tui",
        "--lib",
        "core::engine::tests::print_mode_tool_catalog_metrics",
        "--",
        "--ignored",
        "--exact",
        "--nocapture",
        "--test-threads=1",
    ]
    proc = subprocess.run(cmd, text=True, capture_output=True, check=False)
    sys.stderr.write(proc.stderr)

    combined = proc.stdout.splitlines() + proc.stderr.splitlines()
    for line in combined:
        if MARKER in line:
            metrics = json.loads(line.split(MARKER, 1)[1])
            print(json.dumps(metrics, indent=2, sort_keys=True))
            return proc.returncode

    sys.stdout.write(proc.stdout)
    sys.stderr.write("missing TOOL_CATALOG_METRICS marker\n")
    return proc.returncode or 1


if __name__ == "__main__":
    raise SystemExit(main())
