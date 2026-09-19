# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Build .claude/state/decisions.json from the frontmatter of docs/adr/*.md.

The ADRs stay markdown — they are arguments, and an argument does not become
more useful as JSON. This is an index of them, so an agent can see what is
decided and what is open without reading ten files.

    python3.12 .claude/scripts/index_decisions.py           write the index
    python3.12 .claude/scripts/index_decisions.py --check   fail if it is stale
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
ADR_DIR = Path("docs/adr")
# A markdown link to a sibling ADR file: `[ADR-0013](0013-....md)`.
ADR_LINK = re.compile(r"\[ADR-(\d{4})\]\((\d{4}-[^)]*\.md)\)")

# The status lifecycle. A decision is open only while it is still being made;
# superseded and rejected are closed, because both are answers.
OPEN_STATUSES = frozenset({"proposed"})
CLOSED_STATUSES = frozenset({"accepted", "superseded", "rejected"})
KNOWN_STATUSES = OPEN_STATUSES | CLOSED_STATUSES


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
    entry = {
        "id": adr.stem.split("-")[0],
        "title": fm.get("title", adr.stem),
        "status": fm.get("status", "unknown"),
        "date": fm.get("date", ""),
        "file": str(adr),
    }
    # Only when present, so the index does not grow a field for every ADR that has
    # never superseded anything.
    for key in ("supersedes", "superseded-by"):
        if fm.get(key):
            entry[key] = fm[key]
    return entry


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


def lifecycle_problems(records: list[dict[str, str]]) -> list[str]:
    """Every way the status fields across the ADRs fail to agree with each other.

    Supersession is a claim two records make about each other, and a claim only one of
    them makes is how ADR-0009 and ADR-0016 sat side by side deciding the same question
    with nothing saying which of them held. Checking it keeps that from recurring.
    """
    found: list[str] = []
    by_id = {r["id"]: r for r in records}
    for entry in records:
        adr_id, status = entry["id"], entry["status"]
        if status not in KNOWN_STATUSES:
            known = ", ".join(sorted(KNOWN_STATUSES))
            found.append(f"ADR-{adr_id}: status {status!r} is not one of: {known}")
        target_id = entry.get("superseded-by")
        if status == "superseded" and not target_id:
            found.append(f"ADR-{adr_id}: status is superseded but names no superseded-by")
        if target_id and status != "superseded":
            found.append(f"ADR-{adr_id}: names superseded-by but its status is {status!r}")
        if target_id:
            target = by_id.get(target_id)
            if target is None:
                found.append(f"ADR-{adr_id}: superseded-by {target_id}, which does not exist")
            elif target.get("supersedes") != adr_id:
                found.append(
                    f"ADR-{adr_id}: superseded-by {target_id}, "
                    f"which does not say it supersedes {adr_id}"
                )
    return found


def link_problems() -> list[str]:
    """Every `[ADR-NNNN](file.md)` link under docs/adr/ that points at the wrong place.

    Two ways to be wrong, and both had happened here: the file does not exist, or it
    exists and is a different decision than the link text names. The second is worse,
    because it reads correctly.
    """
    found: list[str] = []
    for adr in sorted(ADR_DIR.glob("*.md")):
        for match in ADR_LINK.finditer(adr.read_text()):
            named, target = match.group(1), match.group(2)
            if not (ADR_DIR / target).is_file():
                found.append(f"{adr}: links to {target}, which does not exist")
            elif not target.startswith(f"{named}-"):
                found.append(f"{adr}: link says ADR-{named} and points at {target}")
    return found


def build_index(records: list[dict[str, str]]) -> Json:
    """The decisions index document."""
    return {
        "_generated_by": ".claude/scripts/index_decisions.py",
        "_note": "Index only. docs/adr/ is the source of truth.",
        "open": [r["id"] for r in records if r["status"] in OPEN_STATUSES],
        "decisions": records,
    }


def main(argv: list[str] | None = None) -> int:
    """Rebuild decisions.json, or with --check fail if it is stale."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if the index is stale")
    args = parser.parse_args(argv)
    os.chdir(REPO_ROOT)

    records = all_records()
    # Refused in both modes, like a duplicate id: an index built over records that
    # disagree about which of them holds is worse than no index at all.
    problems = lifecycle_problems(records) + link_problems()
    if problems:
        for problem in problems:
            print(problem)
        return 1

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
            print("decisions.json stale — run python3.12 .claude/scripts/index_decisions.py")
            return 1
        print(f"decisions index current ({summary})")
        return 0
    INDEX.parent.mkdir(parents=True, exist_ok=True)
    INDEX.write_text(text)
    print(f"wrote {INDEX} ({summary})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
