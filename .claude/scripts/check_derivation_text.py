# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""No committed derivation unit may quote its clause verbatim.

The unit files are committed; the specification clause is not, because it is OMG
document prose and this repository is MIT and public. `grammar_plan.py` keeps the
clause out of the unit, but nothing stopped a `decision` or a `notes` field from
pasting the clause's own sentences back in — and that is exactly what happened
while deriving KerML 8.2.5.8.1, where a note reproduced the clause's note 2 and its
precedence table in full.

Discipline did not hold, so this is a check. It flags any run of `WINDOW`
consecutive words shared between a unit's authored prose and the clause text it was
derived from. Short quotations survive; a pasted paragraph or table does not.

    python3.11 .claude/scripts/check_derivation_text.py

The clause text is local and outside the repository, so this is inert without it —
a fresh clone cannot run it, and is told so rather than passing quietly.

Network: none.
"""

from __future__ import annotations

import argparse
import os
import re
from pathlib import Path

from _grammar import load_units
from _state import REPO_ROOT, load_json

#: Consecutive shared words that count as reproduction rather than quotation. Ten is
#: long enough that citing a production or naming a rule is safe, and short enough
#: that a pasted sentence is caught.
WINDOW = 10

#: The fields a unit carries that are authored prose rather than derived structure.
PROSE_FIELDS = ("decision", "notes")

CLAUSES = os.environ.get("SV2_WIKI_CLAUSES", str(Path.home() / ".sv2-derivation/bnf-clauses.json"))
WORD = re.compile(r"[A-Za-z0-9_']+")
#: Fenced blocks in a clause atom hold the BNF itself. That content is separately
#: published as vendor/spec-bnf/*.kebnf, pinned under EPL-2.0 with the licence
#: text vendored beside it, so restating a production is not reproducing the OMG
#: document. The prose around the fence — descriptions, notes, tables — is.
FENCE = re.compile(r"```.*?```", re.DOTALL)


def shingles(text: str, window: int = WINDOW) -> set[tuple[str, ...]]:
    """Every run of `window` consecutive lowercase words in `text`."""
    words = [w.lower() for w in WORD.findall(text)]
    return {tuple(words[i : i + window]) for i in range(len(words) - window + 1)}


def prose_of(unit: dict[str, object]) -> str:
    """Everything a human wrote on a unit, joined."""
    parts = [str(unit.get(f, "")) for f in PROSE_FIELDS]
    evidence = unit.get("evidence") or []
    if isinstance(evidence, list):
        parts += [str(e.get("note", "")) for e in evidence if isinstance(e, dict)]
    return "\n".join(parts)


def overlaps(units: dict[str, object], clauses: dict[str, object]) -> list[tuple[str, str]]:
    """(production, the shared run) for every unit whose prose reproduces its clause."""
    found: list[tuple[str, str]] = []
    for name, unit in sorted(units.items()):
        entry = clauses.get(name)
        if not isinstance(entry, dict) or not isinstance(unit, dict):
            continue
        prose = prose_of(unit)
        if not prose.strip():
            continue
        clause_prose = FENCE.sub(" ", str(entry.get("text", "")))
        shared = shingles(prose) & shingles(clause_prose)
        if shared:
            found.append((name, " ".join(sorted(shared)[0])))
    return found


def main(argv: list[str] | None = None) -> int:
    """Fail if any unit's authored prose reproduces its clause."""
    argparse.ArgumentParser(description=__doc__).parse_args(argv)
    os.chdir(REPO_ROOT)

    clauses = load_json(CLAUSES, {})
    if not clauses:
        print(f"no clause export at {CLAUSES} — cannot check; build it with")
        print("  python3.11 .claude/scripts/export_wiki_clauses.py")
        return 0

    units = {n: u for n, u in load_units().items() if u.get("status") != "retired"}
    found = overlaps(units, clauses)
    if not found:
        print(f"derivation prose clean ({len(units)} units, {WINDOW}-word window)")
        return 0

    print(f"{len(found)} unit(s) reproduce their clause verbatim:")
    for name, run in found:
        print(f"  {name}: ...{run}...")
    print("Cite the clause; do not paste it. The units are committed and the clause is not.")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
