# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Build .claude/state/decisions.json from the frontmatter of docs/adr/*.md.

The ADRs stay markdown — they are arguments, and an argument does not become
more useful as JSON. This is an index of them, so an agent can see what is
decided and what is open without reading ten files.

    python3.11 .claude/scripts/index_decisions.py           write the index
    python3.11 .claude/scripts/index_decisions.py --check   fail if it is stale
"""

from __future__ import annotations

import argparse
import json
import os
import re
from pathlib import Path
from typing import TYPE_CHECKING

from _state import REPO_ROOT

if TYPE_CHECKING:
    from _state import Json

INDEX = Path(".claude/state/decisions.json")
FRONTMATTER = re.compile(r"---\n(.*?)\n---", re.DOTALL)


def frontmatter(text: str) -> dict[str, str]:
    """Flat ``key: value`` pairs from a leading frontmatter block."""
    m = FRONTMATTER.match(text)
    if not m:
        return {}
    fields: dict[str, str] = {}
    for line in m.group(1).splitlines():
        if ":" in line:
            key, value = line.split(":", 1)
            fields[key.strip()] = value.strip().strip('"')
    return fields


def record(adr: Path) -> dict[str, str]:
    """The index entry for one ADR file."""
    fm = frontmatter(adr.read_text())
    return {
        "id": adr.stem.split("-")[0],
        "title": fm.get("title", adr.stem),
        "status": fm.get("status", "unknown"),
        "date": fm.get("date", ""),
        "file": str(adr),
    }


def all_records() -> list[dict[str, str]]:
    """Every ADR under docs/adr/, in filename order."""
    return [record(adr) for adr in sorted(Path("docs/adr").glob("[0-9]*.md"))]


def duplicate_ids(records: list[dict[str, str]]) -> dict[str, list[str]]:
    """Ids claimed by more than one file, each mapped to the files claiming it.

    An id is derived from the leading number of a filename, so two files numbered
    0011 both answer to ADR-0011. Nothing downstream can tell them apart: the index
    gets two entries under one id, and every citation of that id becomes ambiguous.
    Silence here is what let a duplicate reach the index and only surface later as a
    stale-index failure, which names neither file.
    """
    by_id: dict[str, list[str]] = {}
    for entry in records:
        by_id.setdefault(entry["id"], []).append(entry["file"])
    return {adr_id: files for adr_id, files in by_id.items() if len(files) > 1}


def build_index(records: list[dict[str, str]]) -> Json:
    """The decisions index document."""
    return {
        "_generated_by": ".claude/scripts/index_decisions.py",
        "_note": "Index only. docs/adr/ is the source of truth.",
        "open": [r["id"] for r in records if r["status"] != "accepted"],
        "decisions": records,
    }


def main(argv: list[str] | None = None) -> int:
    """Rebuild decisions.json, or with --check fail if it is stale."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if the index is stale")
    args = parser.parse_args(argv)
    os.chdir(REPO_ROOT)

    records = all_records()
    clashes = duplicate_ids(records)
    if clashes:
        # Refused in both modes. Writing an ambiguous index is worse than not
        # writing one: the gate would go green over two decisions sharing a name.
        for adr_id, files in sorted(clashes.items()):
            print(f"ADR id {adr_id} is claimed by {len(files)} files: {', '.join(files)}")
        print("an id names one decision — renumber all but one, and its internal references")
        return 1

    index = build_index(records)
    text = json.dumps(index, indent=2) + "\n"
    summary = f"{len(index['decisions'])} ADRs, {len(index['open'])} open"
    if args.check:
        if not INDEX.exists() or INDEX.read_text() != text:
            print("decisions.json stale — run python3.11 .claude/scripts/index_decisions.py")
            return 1
        print(f"decisions index current ({summary})")
        return 0
    INDEX.parent.mkdir(parents=True, exist_ok=True)
    INDEX.write_text(text)
    print(f"wrote {INDEX} ({summary})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
