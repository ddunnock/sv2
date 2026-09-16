# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Acceptance criteria for derived units. All deterministic — no judgment.

Promotes status derived -> verified when every check passes, and demotes a
verified unit that has stopped passing.

Two properties this file is written to hold:

Diagnostics never touch `notes`. Why a check failed goes in `diagnostics`, a
separate machine-owned field. `notes` is the author's prose. A checker that
appends to `notes` can satisfy its own next run — the left-recursion criterion
used to look for the phrase "left-recursive" in `notes`, and its own failure
message contained that phrase, so a second run passed a unit the first had
failed. The acknowledgement is now an explicit `left_recursion_intended` flag.

Re-running changes nothing. A unit that passes and passed before is written back
byte-identical: `verified_utc` records when a unit reached verified, not when it
was last looked at, so re-checking a clean tree produces no diff.

    python3.11 .claude/scripts/grammar_check_unit.py              every derived unit
    python3.11 .claude/scripts/grammar_check_unit.py PartUsage    named units
"""

from __future__ import annotations

import argparse
import os
from dataclasses import dataclass
from typing import TYPE_CHECKING

from _grammar import kws_of, load_units, normalize, pinned_tokens, refs_of, render_ebnf, save_unit
from _state import REPO_ROOT, utc_now

if TYPE_CHECKING:
    from _state import Json


@dataclass(frozen=True)
class Check:
    """One acceptance criterion: its name, whether it held, and if not, why not."""

    name: str
    passed: bool
    detail: str = ""


def _is_directly_left_recursive(name: str, rule: Json) -> bool:
    alts = rule.get("items", []) if rule.get("k") == "alt" else [rule]
    for alt in alts:
        if not isinstance(alt, dict):
            continue
        first = (alt.get("items") or [alt])[0]
        if isinstance(first, dict) and first.get("k") == "ref" and first.get("name") == name:
            return True
    return False


def _presence_checks(unit: Json) -> list[Check]:
    """A unit must carry a rule, its reasoning, and evidence for it."""
    return [
        Check("has_rule", bool(unit.get("rule")), "no rule"),
        Check("has_evidence", bool(unit.get("evidence")), "no evidence"),
        Check("has_decision", bool(unit.get("decision")), "no decision"),
    ]


def _vocabulary_checks(rule: Json, allowed_kw: set[str], declared: set[str]) -> list[Check]:
    """Every keyword is in the pinned token set; every ref names a live unit."""
    bad_kw = sorted(k for k in kws_of(rule) if k not in allowed_kw)
    bad_ref = sorted(r for r in refs_of(rule) if r not in declared)
    return [
        Check(
            "keywords_pinned",
            not bad_kw,
            f"keywords not in the pinned token set: {bad_kw}",
        ),
        Check(
            "refs_declared",
            not bad_ref,
            f"references to undeclared productions: {bad_ref}. A lexical terminal "
            f"such as NAME is written {{k: tok}}, not {{k: ref}}",
        ),
    ]


def _left_recursion_check(unit: Json, rule: Json) -> Check:
    """Earley tolerates left recursion; recursive descent does not. Make it deliberate."""
    recursive = _is_directly_left_recursive(unit["production"], rule)
    acknowledged = bool(unit.get("left_recursion_intended"))
    return Check(
        "no_unmarked_left_recursion",
        not recursive or acknowledged,
        'directly left-recursive; if that is intended, set "left_recursion_intended": true',
    )


def _normalization_check(unit: Json, units: dict[str, Json], refs: set[str]) -> Check:
    """The AST, and those of its dependencies, must survive normalization."""
    name = unit["production"]
    try:
        normalize({name: unit})
        for dep in refs:
            dependency = units.get(dep)
            if dependency and dependency.get("rule"):
                normalize({dep: dependency})
    except (ValueError, KeyError, TypeError, AttributeError) as exc:
        # A malformed AST is a failed acceptance check, reported on the unit.
        return Check("normalizes", passed=False, detail=f"AST will not normalize: {exc}")
    return Check("normalizes", passed=True)


def _rule_checks(
    unit: Json, units: dict[str, Json], allowed_kw: set[str], declared: set[str]
) -> list[Check]:
    rule = unit["rule"]
    return [
        *_vocabulary_checks(rule, allowed_kw, declared),
        _left_recursion_check(unit, rule),
        _normalization_check(unit, units, refs_of(rule)),
    ]


def _record(unit: Json, checks: list[Check]) -> bool:
    """Write acceptance, status and diagnostics onto the unit. Idempotent."""
    unit["acceptance"] = {c.name: "pass" if c.passed else "fail" for c in checks}
    ok = all(c.passed for c in checks)

    if ok:
        # Only stamp the transition, so re-checking a verified unit is a no-op.
        if unit.get("status") != "verified" or not unit.get("verified_utc"):
            unit["verified_utc"] = utc_now()
        unit["status"] = "verified"
    elif unit["status"] == "verified":
        unit["status"] = "derived"

    # Replaced in full, never merged, and dropped once the unit is clean.
    failures = [c.detail for c in checks if not c.passed and c.detail]
    if failures:
        unit["diagnostics"] = failures
    else:
        unit.pop("diagnostics", None)
    return ok


def evaluate(unit: Json, units: dict[str, Json], allowed_kw: set[str], declared: set[str]) -> bool:
    """Run every acceptance check and record the outcome on the unit. No I/O."""
    checks = _presence_checks(unit)
    if unit.get("rule"):
        checks += _rule_checks(unit, units, allowed_kw, declared)
    return _record(unit, checks)


def check_unit(
    unit: Json, units: dict[str, Json], allowed_kw: set[str], declared: set[str]
) -> bool:
    """Evaluate one unit, save it, and report the outcome."""
    ok = evaluate(unit, units, allowed_kw, declared)
    save_unit(unit)

    name = unit["production"]
    print(f"  {'verified' if ok else 'FAIL':9s} {name}")
    if unit.get("rule"):
        print(f"            {render_ebnf(name, unit['rule'])}")
    for detail in unit.get("diagnostics", []):
        print(f"            {detail}")
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
