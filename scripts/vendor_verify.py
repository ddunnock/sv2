# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Offline verification of the vendored files against the lockfile.

This is the gate's check, so it must never touch the network — the tool has to
build and test inside an air-gapped enclave. Fetching is a separate, deliberate
act (vendor_sync.py).

    python3.12 scripts/vendor_verify.py
"""

from __future__ import annotations

import argparse
import hashlib
import os
from dataclasses import dataclass, field
from pathlib import Path

from _lock import LOCKFILE, read_lock

ROOT = Path(__file__).resolve().parents[1]
UNPINNED = "UNPINNED"


@dataclass(slots=True)
class Verification:
    """Each lockfile entry sorted by how its file on disk compares to the pin."""

    ok: int = 0
    missing: list[str] = field(default_factory=list)
    unpinned: list[str] = field(default_factory=list)
    mismatch: list[tuple[str, str, str]] = field(default_factory=list)


def _verify(entries: list[dict[str, str]]) -> Verification:
    result = Verification()
    for entry in entries:
        path = Path(entry["path"])
        want = entry.get("sha256", UNPINNED)
        if not path.exists():
            (result.unpinned if want == UNPINNED else result.missing).append(entry["path"])
            continue
        got = hashlib.sha256(path.read_bytes()).hexdigest()
        if want == UNPINNED:
            result.unpinned.append(entry["path"] + "  (present on disk but not pinned)")
        elif got != want:
            result.mismatch.append((entry["path"], want, got))
        else:
            result.ok += 1
    return result


def _report_drift(result: Verification) -> None:
    for path_text, want, got in result.mismatch:
        print(f"  HASH MISMATCH  {path_text}")
        print(f"      expected {want}")
        print(f"      actual   {got}")
    for path_text in result.missing:
        print(f"  MISSING (pinned but not on disk)  {path_text}")
    print()
    print("A vendored file no longer matches the lockfile. Either restore it, or — if the")
    print(
        "change is intentional — re-fetch with python3.12 scripts/vendor_sync.py --accept-new and"
    )
    print("record why in .claude/state/deviations.json.")


def main(argv: list[str] | None = None) -> int:
    """Verify every vendored file against the lockfile; 1 on any mismatch."""
    argparse.ArgumentParser(description=__doc__).parse_args(argv)
    os.chdir(ROOT)

    # Inert before vendoring starts, like every other gate check: a fresh clone
    # has no lockfile yet and must not fail its own gate.
    if not Path(LOCKFILE).is_file():
        print(f"vendor: no lockfile at {LOCKFILE} — check inert")
        return 0

    entries = read_lock()
    result = _verify(entries)

    if not any(Path(e["path"]).exists() for e in entries):
        print(
            f"vendor: nothing fetched yet ({len(entries)} files in lockfile)"
            " — run python3.12 scripts/vendor_sync.py"
        )
        return 0

    if result.mismatch or result.missing:
        _report_drift(result)
        return 1

    if result.unpinned:
        print(f"vendor: {result.ok} verified, {len(result.unpinned)} not yet pinned:")
        for item in result.unpinned[:10]:
            print(f"    {item}")
        return 0

    print(f"vendor: {result.ok}/{len(entries)} files verified against the lockfile")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
