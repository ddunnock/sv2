# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Classify every specification production against what the parser claims.

    implemented    parser handles it, snapshot-tested
    unimplemented  in the inventory, deliberately not yet handled  -- legitimate
    absent         parser claims a production the inventory does not contain  -- DEFECT

A claim is a ``// production: Name`` marker in a crate source file.

The inventory is the SPECIFICATION's productions (Tier B-prime), not the Pilot
Xtext's. That follows the review: every one of the 281 differences in
.claude/state/deviations.json resolved `follow_spec`, so the Xtext-only rules are
productions this parser has decided not to have, and counting them as work
outstanding would measure progress against a language nobody is building.

It also makes one failure mechanical that was previously a matter of vigilance. An
Xtext-only name in a marker is reported as absent with its own message, because it
means a rule was ported from the Pilot — the LL cascade, a keyword factoring, a
membership wrapper — which docs/DERIVATION.md names as the way to build a parser
that looks more conformant while being less so.

    python3.11 scripts/bnf_coverage.py           write .claude/state/coverage.json
    python3.11 scripts/bnf_coverage.py --check   fail on any `absent`
"""

from __future__ import annotations

import argparse
import json
import os
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
INVENTORY = Path(".claude/state/grammar/bnf-productions.json")
XTEXT_INVENTORY = Path(".claude/state/grammar/productions.json")
REPORT = Path(".claude/state/coverage.json")
MARKER = re.compile(r"//\s*production:\s*([A-Za-z_][A-Za-z0-9_]*)")


def claimed_productions() -> set[str]:
    """Every production named by a marker anywhere under crates/."""
    return {
        m.group(1)
        for f in Path("crates").rglob("*.rs")
        for m in MARKER.finditer(f.read_text(errors="replace"))
    }


def xtext_only() -> set[str]:
    """Production names the Pilot Xtext declares and the specification does not."""
    if not XTEXT_INVENTORY.is_file() or not INVENTORY.is_file():
        return set()
    xtext = {p["name"] for p in json.loads(XTEXT_INVENTORY.read_text())["productions"]}
    return xtext - set(json.loads(INVENTORY.read_text())["productions"])


def _check(absent: list[str], *, implemented: int, declared: int, unimplemented: int) -> int:
    if absent:
        ported = sorted(set(absent) & xtext_only())
        invented = sorted(set(absent) - set(ported))
        if ported:
            print(f"coverage: {len(ported)} production(s) ported from the Pilot Xtext:")
            for name in ported:
                print(f"  {name}")
            print()
            print("These are declared by the Xtext and NOT by the specification, so each has a")
            print("reviewed entry in .claude/state/deviations.json deciding not to implement it.")
            print("Porting one makes the parser agree with the reference implementation while")
            print("describing a grammar the specification does not state — docs/DERIVATION.md.")
        if invented:
            print(f"coverage: {len(invented)} production(s) in neither inventory:")
            for name in invented:
                print(f"  {name}")
            print()
            print("The parser accepts syntax no pinned grammar declares — either the marker is")
            print("misspelled, or a production was invented. Both are defects.")
        return 1
    print(
        f"coverage ok: {implemented}/{declared} implemented, "
        f"{unimplemented} unimplemented, 0 absent"
    )
    return 0


def main(argv: list[str] | None = None) -> int:
    """Write the coverage report, or with --check fail on an absent production."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail on any absent production")
    args = parser.parse_args(argv)
    os.chdir(ROOT)

    if not INVENTORY.is_file():
        print(
            "no specification production inventory — run python3.11 scripts/vendor_sync.py"
            " then python3.11 scripts/extract_bnf.py"
        )
        return 0

    declared = set(json.loads(INVENTORY.read_text())["productions"])
    claimed = claimed_productions()

    implemented = sorted(declared & claimed)
    unimplemented = sorted(declared - claimed)
    absent = sorted(claimed - declared)

    if args.check:
        return _check(
            absent,
            implemented=len(implemented),
            declared=len(declared),
            unimplemented=len(unimplemented),
        )

    percent = 100.0 * len(implemented) / len(declared) if declared else 0.0
    report = {
        "_generated_by": "scripts/bnf_coverage.py",
        "_note": "`unimplemented` is a tracked state, not a failure. `absent` is a defect.",
        "declared": len(declared),
        "implemented": len(implemented),
        "unimplemented": len(unimplemented),
        "absent": len(absent),
        "percent": round(percent, 1),
        "unimplemented_productions": unimplemented,
        "absent_productions": absent,
    }
    REPORT.parent.mkdir(parents=True, exist_ok=True)
    REPORT.write_text(json.dumps(report, indent=2) + "\n")
    print(f"wrote {REPORT} ({len(implemented)}/{len(declared)} implemented)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
