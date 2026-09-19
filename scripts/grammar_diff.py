# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Differential between the two independent sources of concrete syntax.

    Tier B        the Pilot's Xtext  (what the reference implementation accepts)
    Tier B-prime  the specification textual BNF (what the language normatively is)

Differences are EXPECTED and permanent — the Xtext carries LL workarounds the
spec does not. The gate fails on an *unreviewed* difference, never on a
difference. Reviewing one means adding it to .claude/state/deviations.json.

    python3.12 scripts/grammar_diff.py           write .claude/state/grammar-diff.json
    python3.12 scripts/grammar_diff.py --check   fail on any unreviewed difference

Environment: SV2_SPEC_BNF overrides the specification inventory path.
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
INVENTORY = Path(".claude/state/grammar/productions.json")
SPEC_INVENTORY = Path(".claude/state/grammar/bnf-productions.json")
DEVIATIONS = Path(".claude/state/deviations.json")
REPORT = Path(".claude/state/grammar-diff.json")
NOTE = (
    "Xtext (what the reference implementation accepts) vs the specification BNF (what the "
    "language normatively is). Differences are expected and permanent; see docs/DERIVATION.md. "
    "The gate fails on unreviewed differences, never on differences."
)


def reviewed_productions() -> set[str]:
    """Productions with an entry in the structured deviation register.

    Reviewed differences come from the structured register, not from grepping prose.
    Grepping markdown for backticked names matched incidental code spans, so a
    difference could be marked reviewed because someone quoted its name in passing.
    """
    if not DEVIATIONS.exists():
        return set()
    register = json.loads(DEVIATIONS.read_text())
    return {
        entry["production"]
        for section in ("deviations", "unpatched")
        for entry in register.get(section, [])
    }


# Every kind the Xtext declares. `terminal` belongs here even though it is lexical:
# the specification BNF states its lexical structure as ordinary productions, and
# DECIMAL_VALUE, REGULAR_COMMENT, STRING_VALUE and UNRESTRICTED_NAME are declared by
# both sources. Excluding terminals reported those four as "specification only",
# which is a false difference — the Xtext has them, as terminals.
#
# Including them is not a way to make the differential smaller. It moves those four
# into `shared` and moves five Xtext lexical terminals (EXP_VALUE, ID, ML_NOTE,
# SL_NOTE, WS) into `xtext_only`, so the unreviewed count goes up, not down. The
# point is that each side's lexical productions are now compared rather than one
# side's being silently dropped.
XTEXT_KINDS = ("rule", "fragment", "enum", "terminal")


# Any: productions.json is a deserialized document (STD-001-PY §6).
def xtext_names(productions: list[dict[str, Any]]) -> set[str]:
    """Every production name the Xtext declares, across all four rule kinds."""
    return {p["name"] for p in productions if p["kind"] in XTEXT_KINDS}


def _check(
    unreviewed: list[str], xtext: set[str], *, shared: int, xtext_only: int, spec_only: int
) -> int:
    if unreviewed:
        print(f"{len(unreviewed)} grammar difference(s) not yet reviewed:")
        for name in unreviewed[:25]:
            where = "Xtext only" if name in xtext else "spec only"
            print(f"  {name}  ({where})")
        print()
        print("Each needs an entry in .claude/state/deviations.json with at least one piece of")
        print("evidence: a retrieved spec clause, a corpus file, or an OMG issue.")
        print("Xtext-only usually means a pilot parser workaround; spec-only usually means")
        print("the pilot has not implemented it. Both are legitimate; neither is automatic.")
        return 1
    print(f"grammar diff reviewed: {shared} shared, {xtext_only} Xtext-only, {spec_only} spec-only")
    return 0


def main(argv: list[str] | None = None) -> int:
    """Write the differential, or with --check fail on an unreviewed difference."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail on unreviewed differences")
    args = parser.parse_args(argv)
    os.chdir(ROOT)

    spec_path = Path(os.environ.get("SV2_SPEC_BNF", str(SPEC_INVENTORY)))
    if not INVENTORY.is_file():
        print("run python3.12 scripts/extract_productions.py first")
        return 0
    if not spec_path.is_file():
        print(f"no specification BNF inventory at {spec_path}")
        print("  Derive it from the pinned Tier B-prime transcription with")
        print("  python3.12 scripts/extract_bnf.py, or set SV2_SPEC_BNF. Until then the")
        print("  differential is inert and the Xtext is the only source — which")
        print("  docs/DERIVATION.md explains is not sufficient on its own.")
        return 0

    inventory = json.loads(INVENTORY.read_text())
    document = json.loads(spec_path.read_text())
    spec = set(document if isinstance(document, list) else document.get("productions", []))
    xtext = xtext_names(inventory["productions"])

    both = sorted(xtext & spec)
    xt_only = sorted(xtext - spec)
    spec_only = sorted(spec - xtext)
    reviewed = reviewed_productions()
    unreviewed = [n for n in xt_only + spec_only if n not in reviewed]

    if args.check:
        return _check(
            unreviewed,
            xtext,
            shared=len(both),
            xtext_only=len(xt_only),
            spec_only=len(spec_only),
        )

    report = {
        "_generated_by": "scripts/grammar_diff.py",
        "_note": NOTE,
        "shared": len(both),
        "xtext_only": xt_only,
        "spec_only": spec_only,
        "reviewed": sorted(reviewed),
        "unreviewed": unreviewed,
    }
    REPORT.parent.mkdir(parents=True, exist_ok=True)
    REPORT.write_text(json.dumps(report, indent=2) + "\n")
    print(f"wrote {REPORT} ({len(unreviewed)} unreviewed)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
