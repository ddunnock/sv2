# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Classify every live grammar unit against what the parser claims.

    implemented    a marker claims the unit                                 -- counted
    unimplemented  a live unit no marker claims yet                         -- legitimate
    absent         a marker claims no unit, or cannot say which it means     -- DEFECT

A claim is a ``// production: Name`` marker in a crate source file, or
``// production: Name@kerml`` / ``Name@sysml`` for one language's variant.

THE DENOMINATOR IS THE LIVE GRAMMAR UNITS, not the inventory's names. ADR-0015 gives
every production the units the two grammars need: one SHARED unit when both languages
state it the same way or both reach it, one unit when only one language reaches it, and
TWO when the languages state it differently. ConnectorEnd is one unit; MultiplicityRange
is two, because KerML's is the `multiplicity` declaration and SysML's is `[1..*]`.
Counting names collapsed each split pair into one, so a marker on SysML's reading also
reported KerML's different production implemented; and it counted the lexical
terminals, which the lexer reads and no marker can claim. The units are the committed,
derived .claude/state/grammar/units/*.json; retired ones are history, not grammar.

A bare marker claims its name's one unit. On a split name a bare marker is a defect: it
must say which language's body it reads, and a function that reads both bodies carries
one marker for each. A scope on a shared unit is a defect too — it is one unit in both.

The Xtext check is kept. A marker naming an Xtext-only production means a rule was
ported from the Pilot — the LL cascade, a keyword factoring, a membership wrapper —
which docs/DERIVATION.md names as the way to build a parser that looks more conformant
while being less so; every one of the 281 differences in .claude/state/deviations.json
resolved `follow_spec`.

    python3.12 scripts/bnf_coverage.py           write .claude/state/coverage.json
    python3.12 scripts/bnf_coverage.py --check   fail on any `absent`, or on a stale report
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
UNITS = Path(".claude/state/grammar/units")
REPORT = Path(".claude/state/coverage.json")
MARKER = re.compile(r"//\s*production:\s*([A-Za-z_][A-Za-z0-9_]*(?:@(?:kerml|sysml))?)")

#: Each production's live unit scopes: ``None`` for its shared unit, else the languages.
Live = dict[str, set[str | None]]


def claimed_productions() -> set[str]:
    """Every claim a marker makes anywhere under crates/, scope suffix included."""
    return {
        m.group(1)
        for f in Path("crates").rglob("*.rs")
        for m in MARKER.finditer(f.read_text(errors="replace"))
    }


def live_units(directory: Path | None = None) -> Live:
    """Each production's live unit scopes, read from the unit files.

    A unit's identity is its ``production`` qualified by its ``scope`` when it has one,
    the rule `unit_key` in .claude/scripts/_grammar.py applies. Read here with the
    standard library rather than imported, so this script runs on its own.
    """
    live: Live = {}
    for path in sorted((UNITS if directory is None else directory).glob("*.json")):
        unit = json.loads(path.read_text())
        if unit.get("status") != "retired":
            live.setdefault(str(unit["production"]), set()).add(unit.get("scope"))
    return live


def unit_keys(live: Live) -> set[str]:
    """The denominator: every live unit, as `Name` or `Name@scope`."""
    return {
        name if scope is None else f"{name}@{scope}"
        for name, scopes in live.items()
        for scope in scopes
    }


def resolve(claim: str, live: Live) -> str | None:
    """The one unit a claim names, or None when it names none or cannot say which."""
    name, _, scope = claim.partition("@")
    scopes = live.get(name, set())
    if scope:
        return claim if scope in scopes else None
    if len(scopes) != 1:
        return None
    (only,) = scopes
    return name if only is None else f"{name}@{only}"


def xtext_only() -> set[str]:
    """Production names the Pilot Xtext declares and the specification does not."""
    if not XTEXT_INVENTORY.is_file() or not INVENTORY.is_file():
        return set()
    xtext = {p["name"] for p in json.loads(XTEXT_INVENTORY.read_text())["productions"]}
    return xtext - set(json.loads(INVENTORY.read_text())["productions"])


def build_report(
    declared: set[str], implemented: list[str], unimplemented: list[str], absent: list[str]
) -> dict[str, object]:
    """The report as it should be on disk for these units and these markers."""
    percent = 100.0 * len(implemented) / len(declared) if declared else 0.0
    return {
        "_generated_by": "scripts/bnf_coverage.py",
        "_note": (
            "Counted in live grammar units (ADR-0015), not names: a production the two "
            "languages state differently is two units, `Name@kerml` and `Name@sysml`. "
            "`unimplemented` is a tracked state, not a failure. `absent` is a defect."
        ),
        "declared": len(declared),
        "implemented": len(implemented),
        "unimplemented": len(unimplemented),
        "absent": len(absent),
        "percent": round(percent, 1),
        "unimplemented_productions": unimplemented,
        "absent_productions": absent,
    }


def is_stale(report: dict[str, object]) -> bool:
    """Whether the report on disk disagrees with the one just built.

    `--check` failing only on `absent` left the report free to drift: a marker added
    without regenerating left `coverage.json` reporting the previous count, and the gate
    stayed green while `state.json` published the stale number. A derived artifact that
    cannot be caught out of date is not derived, it is authored by accident.
    """
    if not REPORT.is_file():
        return True
    try:
        on_disk: object = json.loads(REPORT.read_text())
    except json.JSONDecodeError:
        # A half-written report is not a current one. Regenerating is always safe.
        return True
    return on_disk != report


def _explain(title: str, names: list[str], why: tuple[str, ...]) -> None:
    if not names:
        return
    print(f"coverage: {len(names)} {title}:")
    for name in names:
        print(f"  {name}")
    print()
    for line in why:
        print(line)


def _check(absent: list[str], report: dict[str, object], live: Live | None = None) -> int:
    if absent:
        live = live_units() if live is None else live
        named = {claim: claim.partition("@")[0] for claim in absent}
        ported = sorted(c for c, n in named.items() if n in xtext_only())
        unscoped = sorted(c for c, n in named.items() if "@" not in c and len(live.get(n, ())) > 1)
        misscoped = sorted(c for c, n in named.items() if "@" in c and n in live)
        invented = sorted(set(absent) - set(ported) - set(unscoped) - set(misscoped))
        _explain(
            "production(s) ported from the Pilot Xtext",
            ported,
            (
                "These are declared by the Xtext and NOT by the specification, so each has a",
                "reviewed entry in .claude/state/deviations.json deciding not to implement it.",
                "Porting one makes the parser agree with the reference implementation while",
                "describing a grammar the specification does not state — docs/DERIVATION.md.",
            ),
        )
        _explain(
            "marker(s) on a production the two languages state differently",
            unscoped,
            (
                "Each is two grammar units (ADR-0015), and a bare marker cannot say which",
                "body the function reads. Write `Name@kerml` or `Name@sysml` — both, as two",
                "markers, only if the function reads both languages' bodies.",
            ),
        )
        _explain(
            "marker(s) naming a scope the production has no live unit for",
            misscoped,
            (
                "A shared unit is one unit in both languages and takes no scope; a",
                "one-language unit takes only its own. Drop or correct the suffix.",
            ),
        )
        _explain(
            "production(s) with no live grammar unit",
            invented,
            (
                "The parser claims syntax no pinned grammar unit declares — the marker is",
                "misspelt, names a lexical terminal the lexer reads, or a production was",
                "invented. Each is a defect.",
            ),
        )
        return 1
    if is_stale(report):
        print(f"{REPORT} is stale. Run python3.12 scripts/bnf_coverage.py")
        return 1
    print(
        f"coverage ok: {report['implemented']}/{report['declared']} grammar units "
        f"implemented, {report['unimplemented']} unimplemented, 0 absent"
    )
    return 0


def main(argv: list[str] | None = None) -> int:
    """Write the coverage report, or with --check fail on an absent production."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check", action="store_true", help="fail on any absent production, or a stale report"
    )
    args = parser.parse_args(argv)
    os.chdir(ROOT)

    live = live_units()
    if not live:
        print(f"no live grammar units — run the derive-grammar pipeline, which writes {UNITS}")
        return 0

    declared = unit_keys(live)
    claims = claimed_productions()
    resolved = {claim: resolve(claim, live) for claim in claims}

    implemented = sorted({unit for unit in resolved.values() if unit is not None})
    unimplemented = sorted(declared - set(implemented))
    absent = sorted(claim for claim, unit in resolved.items() if unit is None)

    report = build_report(declared, implemented, unimplemented, absent)

    if args.check:
        return _check(absent, report, live)

    REPORT.parent.mkdir(parents=True, exist_ok=True)
    REPORT.write_text(json.dumps(report, indent=2) + "\n")
    print(f"wrote {REPORT} ({len(implemented)}/{len(declared)} grammar units implemented)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
