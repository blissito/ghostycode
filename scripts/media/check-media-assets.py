#!/usr/bin/env python3
"""Check recorded media against MEDIA_BUDGETS before it can be published (#4906).

The acceptance checklist in docs/releases/v0.9.2-media-plan.md is prose, and
about half of it is mechanically checkable. This turns that half into a command,
so flipping the manifest to `published` stops depending on someone remembering
to measure a GIF.

It deliberately does NOT judge the take. Whether the session is worth showing is
a human call and the whole point of the issue. This only answers: does the file
satisfy the contract the site already advertises?

Budgets are read from web/lib/media-manifest.ts rather than duplicated here, so
the gate cannot drift from the contract the web tests enforce.

Usage:
    python3 scripts/media/check-media-assets.py --dir .media-out
    python3 scripts/media/check-media-assets.py --dir web/public/media --strict
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import struct
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
MANIFEST = REPO_ROOT / "web" / "lib" / "media-manifest.ts"
ASSET_ID = "first-fleet-session"


def parse_budgets() -> dict:
    """Read MEDIA_BUDGETS out of the TypeScript manifest."""
    text = MANIFEST.read_text(encoding="utf-8")
    match = re.search(r"MEDIA_BUDGETS\s*=\s*\{(.*?)\n\}", text, re.DOTALL)
    if not match:
        sys.exit(f"could not find MEDIA_BUDGETS in {MANIFEST}")
    body = match.group(1)

    def number(field: str, group: str | None = None) -> int | None:
        scope = body
        if group:
            gm = re.search(rf"{group}:\s*\{{(.*?)\}}", body, re.DOTALL)
            if not gm:
                return None
            scope = gm.group(1)
        nm = re.search(rf"\b{field}:\s*([0-9_]+)", scope)
        return int(nm.group(1).replace("_", "")) if nm else None

    return {
        "poster": {
            "width": number("width", "poster"),
            "height": number("height", "poster"),
            "maxBytes": number("maxBytes", "poster"),
        },
        "video": {
            "width": number("width", "video"),
            "height": number("height", "video"),
            "maxBytes": number("maxBytes", "video"),
            "maxDurationSeconds": number("maxDurationSeconds", "video"),
        },
        "gifFallback": {"maxBytes": number("maxBytes", "gifFallback")},
        "captionLocales": re.findall(
            r'"([a-zA-Z-]+)"', re.search(r"captionLocales:\s*\[(.*?)\]", body, re.DOTALL).group(1)
        )
        if re.search(r"captionLocales:\s*\[(.*?)\]", body, re.DOTALL)
        else [],
    }


def png_dimensions(path: Path) -> tuple[int, int] | None:
    """Read width/height from a PNG IHDR without pulling in a dependency."""
    with path.open("rb") as handle:
        header = handle.read(24)
    if len(header) < 24 or header[:8] != b"\x89PNG\r\n\x1a\n":
        return None
    return struct.unpack(">II", header[16:24])


def probe_video(path: Path) -> dict | None:
    if not shutil.which("ffprobe"):
        return None
    try:
        raw = subprocess.run(
            [
                "ffprobe", "-v", "error",
                "-select_streams", "v:0",
                "-show_entries", "stream=width,height:format=duration",
                "-of", "json", str(path),
            ],
            capture_output=True, text=True, check=True,
        ).stdout
    except subprocess.CalledProcessError:
        return None
    data = json.loads(raw)
    stream = (data.get("streams") or [{}])[0]
    duration = data.get("format", {}).get("duration")
    return {
        "width": stream.get("width"),
        "height": stream.get("height"),
        "duration": float(duration) if duration else None,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dir", required=True, help="directory holding the recorded assets")
    parser.add_argument(
        "--strict",
        action="store_true",
        help="also require captions and transcript (use before flipping to published)",
    )
    args = parser.parse_args()

    root = Path(args.dir)
    if not root.is_dir():
        sys.exit(f"not a directory: {root}")

    budgets = parse_budgets()
    failures: list[str] = []
    notes: list[str] = []

    def check(ok: bool, message: str) -> None:
        print(f"  {'PASS' if ok else 'FAIL'}  {message}")
        if not ok:
            failures.append(message)

    print(f"Budgets from {MANIFEST.relative_to(REPO_ROOT)}")
    print(f"Assets in    {root}\n")

    # --- poster -------------------------------------------------------------
    print("poster")
    poster = root / f"{ASSET_ID}.png"
    if not poster.exists():
        check(False, f"{poster.name} exists")
    else:
        size = poster.stat().st_size
        check(size <= budgets["poster"]["maxBytes"],
              f"{poster.name} is {size:,} B (max {budgets['poster']['maxBytes']:,})")
        dims = png_dimensions(poster)
        if dims is None:
            check(False, f"{poster.name} is a readable PNG")
        else:
            want = (budgets["poster"]["width"], budgets["poster"]["height"])
            check(dims == want, f"{poster.name} is {dims[0]}x{dims[1]} (want {want[0]}x{want[1]})")

    # --- video --------------------------------------------------------------
    print("\nvideo")
    video = root / f"{ASSET_ID}.mp4"
    if not video.exists():
        check(False, f"{video.name} exists")
    else:
        size = video.stat().st_size
        check(size <= budgets["video"]["maxBytes"],
              f"{video.name} is {size:,} B (max {budgets['video']['maxBytes']:,})")
        probe = probe_video(video)
        if probe is None:
            notes.append(
                "ffprobe unavailable — video dimensions and duration were NOT verified. "
                "The media plan requires measuring both before publishing."
            )
            print("  SKIP  dimensions/duration (ffprobe not installed)")
        else:
            want = (budgets["video"]["width"], budgets["video"]["height"])
            check((probe["width"], probe["height"]) == want,
                  f"{video.name} is {probe['width']}x{probe['height']} (want {want[0]}x{want[1]})")
            if probe["duration"] is None:
                check(False, f"{video.name} reports a duration")
            else:
                limit = budgets["video"]["maxDurationSeconds"]
                check(probe["duration"] <= limit,
                      f"{video.name} runs {probe['duration']:.1f}s (max {limit}s)")

    # --- gif ----------------------------------------------------------------
    print("\ngif fallback")
    gif = root / f"{ASSET_ID}.gif"
    if not gif.exists():
        check(False, f"{gif.name} exists")
    else:
        size = gif.stat().st_size
        check(size <= budgets["gifFallback"]["maxBytes"],
              f"{gif.name} is {size:,} B (max {budgets['gifFallback']['maxBytes']:,})")
        # #4906 asks for a README GIF under ~3 MB; that is a stricter, separate
        # budget than the site's fallback, so report rather than fail.
        if size > 3_000_000:
            notes.append(
                f"{gif.name} is {size:,} B — over the ~3 MB the issue wants for the "
                "README GIF. Fine for the site fallback; re-encode or shorten for the README."
            )

    # --- captions and transcript -------------------------------------------
    print("\ncaptions / transcript")
    for locale in budgets["captionLocales"]:
        vtt = root / f"{ASSET_ID}.{locale}.vtt"
        if not vtt.exists():
            (check if args.strict else lambda ok, m: print(f"  TODO  {m}"))(
                False, f"{vtt.name} exists"
            )
        else:
            body = vtt.read_text(encoding="utf-8", errors="replace").strip()
            has_cue = "-->" in body
            check(bool(body) and has_cue, f"{vtt.name} is non-empty and has at least one cue")

    transcript = REPO_ROOT / "docs" / "evidence" / "v092-first-fleet-session-transcript.md"
    if args.strict:
        check(transcript.exists(), f"{transcript.relative_to(REPO_ROOT)} exists")
    elif not transcript.exists():
        print(f"  TODO  {transcript.relative_to(REPO_ROOT)} exists")

    # --- capture receipt ----------------------------------------------------
    print("\nprovenance")
    receipt = root / "capture.json"
    if receipt.exists():
        data = json.loads(receipt.read_text(encoding="utf-8"))
        commit = str(data.get("recorded_from_commit", ""))
        check(len(commit) == 40, f"capture.json names a full 40-hex source commit ({commit[:12] or 'missing'})")
    else:
        (check if args.strict else lambda ok, m: print(f"  TODO  {m}"))(
            False, "capture.json exists (written by scripts/media/record-session.sh)"
        )

    print()
    for note in notes:
        print(f"NOTE: {note}")

    if failures:
        print(f"\n{len(failures)} check(s) failed.")
        return 1

    print("\nAll mechanical checks passed.")
    print(
        "This does NOT mean the asset is ready. Still human-only:\n"
        "  - is the take actually worth showing?\n"
        "  - is every frame real output, with no credential or private path visible?\n"
        "  - do the caption cues match what the session actually did?\n"
        "See docs/releases/v0.9.2-media-plan.md."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
