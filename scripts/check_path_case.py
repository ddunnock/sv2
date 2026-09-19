# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""No two tracked paths may differ only by case or Unicode normalization.

macOS (APFS) and Windows file systems are case-insensitive by default, and APFS is
normalization-insensitive as well, so two such paths are one file on disk. Git
still tracks both: a checkout writes one over the other, the tree reports the
loser as modified, and a later checkout can silently swap which one is present.
That happened to a retired grammar unit and its live sibling
(MetaClassificationTestOperator / MetaclassificationTestOperator), and the only
symptom was a frozen-grammar hash nobody could reproduce.

Directories count too: `Docs/a` and `docs/b` collide as surely as two files do.

    python3.12 scripts/check_path_case.py

Reads `git ls-files`, so it checks what the index tracks, not what the disk holds.
Inert outside a git checkout.
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import unicodedata
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def fold(path: str) -> str:
    """The key under which a case- and normalization-insensitive disk stores a path."""
    return unicodedata.normalize("NFC", path).casefold()


def collisions(paths: list[str]) -> list[list[str]]:
    """Every group of distinct paths, files or directories, that fold to one key.

    Each tracked path contributes itself and every directory above it, so two
    directories that differ only by case are found even when no two files do.
    """
    spellings: defaultdict[str, set[str]] = defaultdict(set)
    for path in paths:
        parts = path.split("/")
        for depth in range(1, len(parts) + 1):
            prefix = "/".join(parts[:depth])
            spellings[fold(prefix)].add(prefix)
    return sorted(sorted(group) for group in spellings.values() if len(group) > 1)


def main(argv: list[str] | None = None) -> int:
    """List tracked paths that collide on a case-insensitive disk; 1 if any do."""
    argparse.ArgumentParser(description=__doc__).parse_args(argv)
    os.chdir(ROOT)

    git = shutil.which("git")
    if git is None or not Path(".git").exists():
        print("not a git checkout — check inert")
        return 0
    listed = subprocess.run(
        [git, "ls-files", "-z"], capture_output=True, text=True, check=True
    ).stdout
    found = collisions([p for p in listed.split("\0") if p])
    if found:
        print("tracked paths that are one file on a case-insensitive disk:")
        for group in found:
            print("  " + "  |  ".join(group))
        print()
        print("A checkout keeps only one of each group. Rename or untrack all but one,")
        print("and fix whatever generated the second spelling.")
        return 1
    print("no tracked paths collide by case or normalization")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
