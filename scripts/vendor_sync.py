# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Fetch the pinned vendor set.

Requires network; this is a deliberate act, not something the gate or a build
ever does.

    python3.12 scripts/vendor_sync.py                fetch missing files, verify pinned ones
    python3.12 scripts/vendor_sync.py --accept-new   accept changed hashes (reviewed upgrade)
    python3.12 scripts/vendor_sync.py --dry-run      show what would be fetched
"""

from __future__ import annotations

import argparse
import hashlib
import os
import urllib.request
from dataclasses import dataclass, field
from pathlib import Path

from _lock import read_lock, read_target, write_hash

ROOT = Path(__file__).resolve().parents[1]
UNPINNED = "UNPINNED"


@dataclass(slots=True)
class Tally:
    """Per-run accumulator: what was fetched, verified, changed, or failed."""

    fetched: int = 0
    verified: int = 0
    changed: list[str] = field(default_factory=list)
    failed: list[tuple[str, str]] = field(default_factory=list)


def _resolve_url(entry: dict[str, str], pilot_sha: str) -> str | None:
    url = entry["url"]
    if "{PILOT_SHA}" not in url:
        return url
    if pilot_sha in ("UNSET", ""):
        print(f"  SKIP  {entry['path']}")
        print("        tier_b_pilot.revision is UNSET in docs/conformance-target.toml.")
        print("        Set it to a full commit sha (never a branch) before syncing Tier B.")
        return None
    return url.replace("{PILOT_SHA}", pilot_sha)


def _fetch(url: str) -> bytes:
    # S310: the URL comes from the reviewed lockfile; file:// serves offline fixtures.
    request = urllib.request.Request(url, headers={"User-Agent": "sv2-vendor-sync"})  # noqa: S310
    with urllib.request.urlopen(request, timeout=60) as response:  # noqa: S310
        data: bytes = response.read()
    return data


def _store(entry_path: str, data: bytes, digest: str) -> None:
    path = Path(entry_path)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)
    write_hash(entry_path, digest)


def _apply(entry: dict[str, str], data: bytes, *, accept_new: bool, tally: Tally) -> None:
    want = entry.get("sha256", UNPINNED)
    got = hashlib.sha256(data).hexdigest()
    if want == UNPINNED:
        _store(entry["path"], data, got)
        print(f"  pinned  {entry['path']}  {got[:16]}…  ({len(data)} bytes)")
        tally.fetched += 1
    elif got == want:
        tally.verified += 1
    elif accept_new:
        _store(entry["path"], data, got)
        print(f"  UPDATED {entry['path']}")
        print(f"          {want[:16]}… -> {got[:16]}…")
        tally.changed.append(entry["path"])
    else:
        print(f"  CHANGED UPSTREAM  {entry['path']}")
        print(f"          pinned {want}")
        print(f"          remote {got}")
        tally.changed.append(entry["path"])


def _sync_entry(
    entry: dict[str, str], pilot_sha: str, *, dry_run: bool, accept_new: bool, tally: Tally
) -> None:
    url = _resolve_url(entry, pilot_sha)
    if url is None:
        return
    path = Path(entry["path"])
    want = entry.get("sha256", UNPINNED)
    if path.exists() and want != UNPINNED and hashlib.sha256(path.read_bytes()).hexdigest() == want:
        tally.verified += 1
        return
    if dry_run:
        print(f"  would fetch  {entry['path']}")
        return
    try:
        data = _fetch(url)
    except (OSError, ValueError) as exc:
        # One unreachable file must not stop the rest; the summary reports it.
        tally.failed.append((entry["path"], str(exc)))
        print(f"  FAIL  {entry['path']}: {exc}")
        return
    _apply(entry, data, accept_new=accept_new, tally=tally)


def main(argv: list[str] | None = None) -> int:
    """Fetch the pinned vendor set and re-pin the lockfile. The only script that fetches."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--accept-new", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args(argv)
    os.chdir(ROOT)

    pilot_sha = read_target().get("tier_b_pilot.revision", "UNSET")
    tally = Tally()
    for entry in read_lock():
        _sync_entry(entry, pilot_sha, dry_run=args.dry_run, accept_new=args.accept_new, tally=tally)

    print()
    print(
        f"sync: {tally.fetched} newly pinned, {tally.verified} already current, "
        f"{len(tally.changed)} changed, {len(tally.failed)} failed"
    )

    if tally.changed and not args.accept_new:
        print()
        print("Upstream content changed at a pinned URL. Nothing was written.")
        print("Review the difference, then re-run with --accept-new and record the reason in")
        print(
            ".claude/state/deviations.json. A namespace-dated OMG URL changing content is"
            " unusual and"
        )
        print("worth understanding before accepting.")
        return 1
    return 1 if tally.failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
