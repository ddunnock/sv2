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

from _grammar import (
    SCOPES,
    grammar_view,
    kws_of,
    load_units,
    normalize,
    pinned_tokens,
    refs_of,
    render_ebnf,
    save_unit,
    unit_key,
)
from _state import REPO_ROOT, utc_now

if TYPE_CHECKING:
    from collections.abc import Callable

    from _state import Json

    #: The grammars a unit is checked against, by scope, each keyed by production.
    #: A variant belongs to one; a shared unit to both, and must hold in each.
    Views = dict[str, dict[str, Json]]


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


def _evidence_check(unit: Json, file_exists: Callable[[str], bool]) -> Check:
    """Corpus evidence must name a file that exists.

    Evidence that merely exists is not evidence. A path that resolves to nothing
    passed every other check and looked exactly as authoritative as a real one; ten
    refs missing their `corpus/` segment were verified that way before this existed.
    This proves only that the file is there, not that it holds the quoted instance.
    """
    missing = sorted(
        e["ref"]
        for e in unit.get("evidence") or []
        if e.get("kind") == "corpus" and not file_exists(str(e.get("ref", "")))
    )
    return Check(
        "corpus_evidence_resolves", not missing, f"corpus evidence names no file: {missing}"
    )


def _vocabulary_checks(rule: Json, allowed_kw: set[str], views: Views) -> list[Check]:
    """Every keyword is pinned; every ref names a live unit in EVERY grammar the unit is in."""
    bad_kw = sorted(k for k in kws_of(rule) if k not in allowed_kw)
    missing = {
        ref: sorted(scope for scope, view in views.items() if ref not in view)
        for ref in refs_of(rule)
    }
    bad_ref = sorted(ref for ref, scopes in missing.items() if scopes)
    where = "; ".join(f"{ref} (absent from {', '.join(missing[ref])})" for ref in bad_ref)
    return [
        Check(
            "keywords_pinned",
            not bad_kw,
            f"keywords not in the pinned token set: {bad_kw}",
        ),
        Check(
            "refs_declared",
            not bad_ref,
            f"references to undeclared productions: {where}. A lexical terminal "
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


def _normalization_check(unit: Json, views: Views, refs: set[str]) -> Check:
    """The AST, and those of its dependencies in every grammar it belongs to, normalize."""
    name = unit["production"]
    try:
        normalize({name: unit})
        for view in views.values():
            for dep in refs:
                dependency = view.get(dep)
                if dependency and dependency.get("rule"):
                    normalize({dep: dependency})
    except (ValueError, KeyError, TypeError, AttributeError) as exc:
        # A malformed AST is a failed acceptance check, reported on the unit.
        return Check("normalizes", passed=False, detail=f"AST will not normalize: {exc}")
    return Check("normalizes", passed=True)


def _rule_checks(unit: Json, views: Views, allowed_kw: set[str]) -> list[Check]:
    rule = unit["rule"]
    return [
        *_vocabulary_checks(rule, allowed_kw, views),
        _left_recursion_check(unit, rule),
        _normalization_check(unit, views, refs_of(rule)),
    ]


def _record(unit: Json, checks: list[Check]) -> bool:
    """Write acceptance, status and diagnostics onto the unit. Idempotent."""
    unit["acceptance"] = {c.name: "pass" if c.passed else "fail" for c in checks}
    ok = all(c.passed for c in checks)

    if unit.get("status") == "conflict":
        # These checks are about the shape of a rule, not about whether the
        # sources agree. Promoting here would retire an unresolved conflict by
        # writing it well, which is the one thing adjudication exists to prevent.
        unit["diagnostics"] = ["sources disagree; run the grammar-adjudicator, not this check"]
        return False

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


def views_for(unit: Json, units: dict[str, Json]) -> Views:
    """The grammars a unit belongs to: its own scope for a variant, both for a shared unit."""
    scopes = (unit["scope"],) if unit.get("scope") else SCOPES
    return {scope: grammar_view(units, scope) for scope in scopes}


def evaluate(
    unit: Json,
    views: Views,
    allowed_kw: set[str],
    file_exists: Callable[[str], bool] = os.path.isfile,
) -> bool:
    """Run every acceptance check and record the outcome on the unit.

    Writes nothing. The one read, whether a cited corpus file exists, goes through
    `file_exists` so a test can supply the filesystem.
    """
    checks = [*_presence_checks(unit), _evidence_check(unit, file_exists)]
    if unit.get("rule"):
        checks += _rule_checks(unit, views, allowed_kw)
    return _record(unit, checks)


def check_unit(unit: Json, units: dict[str, Json], allowed_kw: set[str]) -> bool:
    """Evaluate one unit against the grammars it belongs to, save it, and report."""
    ok = evaluate(unit, views_for(unit, units), allowed_kw)
    save_unit(unit)

    key = unit_key(unit)
    print(f"  {'verified' if ok else 'FAIL':9s} {key}")
    if unit.get("rule"):
        print(f"            {render_ebnf(key, unit['rule'])}")
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

    failed = False
    for key in targets:
        unit = units.get(key)
        if not unit:
            variants = sorted(k for k, u in units.items() if u["production"] == key)
            hint = f" — it is split per language: {', '.join(variants)}" if variants else ""
            print(f"  {key}: no such unit{hint}")
            failed = True
            continue
        failed = not check_unit(unit, units, allowed_kw) or failed
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
