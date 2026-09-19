# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Every clause citation must resolve to a pinned receipt.

Invariant 4 says every claim about the language traces to a source. Until now that
was a convention: a deviation entry could cite a `region_sha256` that belonged to no
atom this repository has ever seen, and nothing would notice. This closes it.

For each evidence note in .claude/state/deviations.json, any hexadecimal run that
looks like a region receipt must be a prefix of a receipt in
.claude/state/wiki-receipts.json. A citation naming a receipt that is not pinned is
a citation to nothing, and it fails.

What this does NOT check: that the cited clause actually says what the entry claims.
No script can check that. It checks that the thing cited exists and is pinned, which
is the part a script can own.

    python3.12 scripts/check_citations.py
"""

from __future__ import annotations

import argparse
import json
import os
import re
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
DEVIATIONS = Path(".claude/state/deviations.json")
RECEIPTS = Path(".claude/state/wiki-receipts.json")
# A receipt is quoted as at least 16 hex characters, often truncated with an ellipsis.
HEX_RUN = re.compile(r"\b([0-9a-f]{16,64})\b")
# OMG document identifiers, which are checked for shape rather than resolved.
FILE_ID = re.compile(r"^(ptc|formal|dtc|omg)/\d{2}-\d{2}-\d{2}$")


# Any: deviations.json is a deserialized document (STD-001-PY §6).
def cited_receipts(document: dict[str, Any]) -> list[tuple[str, str]]:
    """(production, hex run) for every receipt-looking string in an evidence note."""
    out: list[tuple[str, str]] = []
    for entry in document.get("deviations", []):
        for item in entry.get("evidence", []):
            for text in (item.get("note", ""), item.get("ref", "")):
                out.extend((entry["production"], m.group(1)) for m in HEX_RUN.finditer(text))
    return out


def bad_file_ids(document: dict[str, Any]) -> list[tuple[str, str]]:
    """(production, ref) for every file_id evidence whose ref is not an OMG document id."""
    return [
        (entry["production"], item.get("ref", ""))
        for entry in document.get("deviations", [])
        for item in entry.get("evidence", [])
        if item.get("kind") == "file_id" and not FILE_ID.match(item.get("ref", ""))
    ]


def unresolved(cited: list[tuple[str, str]], pinned: set[str]) -> list[tuple[str, str]]:
    """Citations whose receipt is not a prefix of any pinned receipt."""
    return [(p, h) for p, h in cited if not any(r.startswith(h) for r in pinned)]


def main(argv: list[str] | None = None) -> int:
    """Check that every cited receipt is pinned; 1 if any is not."""
    argparse.ArgumentParser(description=__doc__).parse_args(argv)
    os.chdir(ROOT)

    if not DEVIATIONS.is_file():
        print("no deviation register — check inert")
        return 0
    document = json.loads(DEVIATIONS.read_text())
    cited = cited_receipts(document)
    malformed = bad_file_ids(document)

    if not RECEIPTS.is_file():
        # Inert before the wikis are pinned, like every other pinned-input check.
        print(f"no wiki receipts at {RECEIPTS} — {len(cited)} citation(s) unverifiable")
        print("run python3.12 .claude/scripts/build_wiki_receipts.py on a machine with the wikis")
        return 0

    pinned = {r["region_sha256"] for r in json.loads(RECEIPTS.read_text())["receipts"]}
    pinned.discard("")
    missing = unresolved(cited, pinned)

    if missing or malformed:
        for production, digest in missing:
            print(f"  {production}: cites receipt {digest[:16]}… which is not pinned")
        for production, ref in malformed:
            print(f"  {production}: file_id evidence {ref!r} is not an OMG document identifier")
        print()
        print("A citation naming a receipt this repository has never pinned is a citation to")
        print("nothing. Either the receipt is wrong, or the wiki it came from was never")
        print("recorded — rebuild with .claude/scripts/build_wiki_receipts.py.")
        return 1

    print(
        f"citations ok: {len(cited)} receipt(s) across "
        f"{len(document['deviations'])} deviations, all pinned"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
