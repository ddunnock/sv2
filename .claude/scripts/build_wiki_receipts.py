# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Pin the specification wikis by receipt rather than by copy.

Every clause citation in .claude/state/deviations.json rests on an atom of the
SysML v2 or KerML LLM wiki. Those wikis are built from the OMG specification PDFs
and their atoms carry ~2.3 MB of verbatim normative prose, so this repository does
not redistribute them: it records, for every atom, the identity and the two hashes
that make a citation checkable.

    region_sha256   the wiki's own receipt binding the atom to a verbatim span of
                    the source PDF, which is what a citation ultimately rests on
    file_sha256     this repository's receipt for the rendered atom file, which
                    catches an atom edited after it was cited

What this buys: any copy of a wiki can be checked against the manifest, and drift
is detected. What it does not buy: a fresh clone cannot build the wikis. That is
inherent — the source PDFs are not redistributable either — and is recorded in
deviations.json under source_selection.clause_retrieval rather than glossed.

Wikis are located by SV2_SYSML_WIKI / SV2_KERML_WIKI, else the default paths below.

    python3.12 .claude/scripts/build_wiki_receipts.py           write the manifest
    python3.12 .claude/scripts/build_wiki_receipts.py --check   fail if it is stale
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sqlite3
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = Path(".claude/state/wiki-receipts.json")
GENERATED_BY = ".claude/scripts/build_wiki_receipts.py"

WIKIS = {
    "sysml": ("SV2_SYSML_WIKI", "~/.sysml-v2-llm-wiki/wiki"),
    "kerml": ("SV2_KERML_WIKI", "~/.kerml-llm-wiki/wiki"),
}
# The validity condition the navigator skills state.
REQUIRED = ("llms.txt", "graph.sqlite", "index")
REGION = re.compile(r"region_sha256:\s*([0-9a-f]{64})")
PAGE = re.compile(r"pdf_page:\s*(\d+)")
PRINTED = re.compile(r"printed_page:\s*(\d+)")


def wiki_dir(key: str) -> Path | None:
    """The wiki directory for ``key``, or None when it is absent or incomplete."""
    env, default = WIKIS[key]
    path = Path(os.environ.get(env) or default).expanduser()
    if not path.is_dir() or not all((path / r).exists() for r in REQUIRED):
        return None
    return path


def receipts(key: str, path: Path) -> list[dict[str, Any]]:
    """One record per atom in one wiki, sorted by id."""
    con = sqlite3.connect(path / "graph.sqlite")
    rows = con.execute("select id, kind, clause, path from atoms").fetchall()
    out: list[dict[str, Any]] = []
    for atom_id, kind, clause, rel in rows:
        atom = path / rel
        if not atom.is_file():
            continue
        raw = atom.read_bytes()
        text = raw.decode("utf-8", errors="replace")
        region = REGION.search(text)
        page = PAGE.search(text)
        printed = PRINTED.search(text)
        out.append(
            {
                "wiki": key,
                "atom": atom_id,
                "kind": kind,
                "clause": str(clause) if clause is not None else "",
                "pdf_page": int(page.group(1)) if page else 0,
                "printed_page": int(printed.group(1)) if printed else 0,
                "region_sha256": region.group(1) if region else "",
                "file_sha256": hashlib.sha256(raw).hexdigest(),
                "path": rel,
            }
        )
    return sorted(out, key=lambda r: (r["wiki"], r["atom"]))


def build() -> tuple[dict[str, Any] | None, list[str]]:
    """(manifest, names of wikis that were not found)."""
    records: list[dict[str, Any]] = []
    missing: list[str] = []
    for key in WIKIS:
        path = wiki_dir(key)
        if path is None:
            missing.append(key)
            continue
        records.extend(receipts(key, path))
    if missing:
        return None, missing
    counts: dict[str, int] = {}
    for record in records:
        counts[record["wiki"]] = counts.get(record["wiki"], 0) + 1
    return {
        "_generated_by": GENERATED_BY,
        "_note": (
            "Receipts for the specification wikis that back every clause citation in "
            "deviations.json. Metadata only: no specification prose is stored here, "
            "because the wiki atoms carry verbatim normative text and this repository "
            "does not redistribute it. region_sha256 is the wiki's receipt to the source "
            "PDF span; file_sha256 is this repository's receipt to the rendered atom."
        ),
        "counts": {**counts, "total": len(records)},
        "receipts": records,
    }, []


def _render(data: dict[str, Any]) -> str:
    return json.dumps(data, indent=2) + "\n"


def main(argv: list[str] | None = None) -> int:
    """Write the wiki receipt manifest, or with --check fail if it is stale."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if the manifest is stale")
    args = parser.parse_args(argv)
    os.chdir(ROOT)

    manifest, missing = build()
    if manifest is None:
        # Inert without the wikis, like every other pinned-input check: this must
        # not fail on a machine that has not built them.
        print(f"wiki not found: {', '.join(missing)} — receipts not rebuilt")
        return 0

    if args.check:
        if not MANIFEST.exists() or MANIFEST.read_text() != _render(manifest):
            print("wiki receipts are stale")
            print(f"run python3.12 {GENERATED_BY}")
            return 1
        print(f"wiki receipts current ({manifest['counts']['total']} atoms)")
        return 0

    MANIFEST.parent.mkdir(parents=True, exist_ok=True)
    MANIFEST.write_text(_render(manifest))
    counts = manifest["counts"]
    print(
        f"wrote {MANIFEST} ({counts['total']} atoms: "
        + ", ".join(f"{k} {v}" for k, v in sorted(counts.items()) if k != "total")
        + ")"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
