# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Acceptance criteria for derived units. All deterministic — no judgment.

Promotes status derived -> verified only when every check passes.

    python3.11 .claude/scripts/grammar_check_unit.py              every derived unit
    python3.11 .claude/scripts/grammar_check_unit.py PartUsage    named units
"""

from __future__ import annotations

import argparse
import os
from typing import TYPE_CHECKING

from _grammar import kws_of, load_units, normalize, pinned_tokens, refs_of, render_ebnf, save_unit
from _state import REPO_ROOT, utc_now

if TYPE_CHECKING:
    from _state import Json


def _pass(passed: object) -> str:
    return "pass" if passed else "fail"


def _is_directly_left_recursive(name: str, rule: Json) -> bool:
    alts = rule.get("items", []) if rule.get("k") == "alt" else [rule]
    for alt in alts:
        if not isinstance(alt, dict):
            continue
        first = (alt.get("items") or [alt])[0]
        if isinstance(first, dict) and first.get("k") == "ref" and first.get("name") == name:
            return True
    return False


def _rule_checks(
    unit: Json, units: dict[str, Json], allowed_kw: set[str], declared: set[str]
) -> tuple[dict[str, str], list[str]]:
    name, rule = unit["production"], unit["rule"]
    acc: dict[str, str] = {}
    notes: list[str] = []
    refs, kws = refs_of(rule), kws_of(rule)
    bad_kw = sorted(k for k in kws if k not in allowed_kw)
    bad_ref = sorted(r for r in refs if r not in declared)
    acc["keywords_pinned"] = _pass(not bad_kw)
    acc["refs_declared"] = _pass(not bad_ref)
    if bad_kw:
        notes.append(f"keywords not in the pinned token set: {bad_kw}")
    if bad_ref:
        notes.append(f"references to undeclared productions: {bad_ref}")

    # Unintended direct left recursion. Earley tolerates it; a hand-written
    # recursive-descent parser does not, so flag it for a conscious decision.
    marked = not _is_directly_left_recursive(name, rule) or "left-recursive" in (
        unit.get("notes") or ""
    )
    acc["no_unmarked_left_recursion"] = _pass(marked)
    if not marked:
        notes.append("directly left-recursive; if intended, say so in notes")

    try:
        normalize({name: unit})
        for dep in refs:
            d = units.get(dep)
            if d and d.get("rule"):
                normalize({dep: d})
        acc["normalizes"] = "pass"
    except (ValueError, KeyError, TypeError, AttributeError) as exc:
        # A malformed AST is a failed acceptance check, reported on the unit.
        acc["normalizes"] = "fail"
        notes.append(f"AST will not normalize: {exc}")
    return acc, notes


def check_unit(
    unit: Json, units: dict[str, Json], allowed_kw: set[str], declared: set[str]
) -> bool:
    """Run every acceptance check, record the result on the unit, and save it."""
    acc = {
        "has_rule": _pass(bool(unit.get("rule"))),
        "has_evidence": _pass(bool(unit.get("evidence"))),
        "has_decision": _pass(bool(unit.get("decision"))),
    }
    notes: list[str] = []
    if unit.get("rule"):
        rule_acc, notes = _rule_checks(unit, units, allowed_kw, declared)
        acc.update(rule_acc)

    unit["acceptance"] = acc
    ok = all(v == "pass" for v in acc.values())
    if ok:
        unit["status"] = "verified"
        unit["verified_utc"] = utc_now()
    elif unit["status"] == "verified":
        unit["status"] = "derived"
    if notes:
        unit["notes"] = (unit.get("notes", "") + "\n" + "\n".join(notes)).strip()
    save_unit(unit)

    name = unit["production"]
    print(f"  {'verified' if ok else 'FAIL':9s} {name}")
    if unit.get("rule"):
        print(f"            {render_ebnf(name, unit['rule'])}")
    for note in notes:
        print(f"            {note}")
    return ok


def main(argv: list[str] | None = None) -> int:
    """Check the named units, or every derived one; 1 if any acceptance check fails."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("units", nargs="*", help="unit names; default is every derived unit")
    args = parser.parse_args(argv)
    os.chdir(REPO_ROOT)

    units = load_units()
    targets = args.units or [n for n, u in units.items() if u["status"] == "derived"]
    if not targets:
        print("no derived units to check")
        return 0

    keywords, operators = pinned_tokens()
    allowed_kw = set(keywords) | set(operators)
    declared = {u["production"] for u in units.values() if u["status"] != "retired"}

    failed = False
    for name in targets:
        unit = units.get(name)
        if not unit:
            print(f"  {name}: no such unit")
            failed = True
            continue
        failed = not check_unit(unit, units, allowed_kw, declared) or failed
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
