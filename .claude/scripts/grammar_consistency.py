# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Phase 4. Structural checks ACROSS units — things no single unit can see.

Run once per language. KerML and SysML are two grammars that share most of their
units (ADR-0014), so a reference can be defined in one and dangle in the other, and
a production can be reachable from one start symbol and not the other.

python3.11 .claude/scripts/grammar_consistency.py
"""

from __future__ import annotations

import argparse
import os
from collections import Counter
from typing import TYPE_CHECKING

from _grammar import SCOPES, START, grammar_view, load_units, normalize, refs_of, shadowed
from _state import REPO_ROOT

if TYPE_CHECKING:
    from _state import Json


def undefined_references(have: dict[str, Json]) -> dict[str, list[str]]:
    """Units whose rules reference productions that have no rule yet."""
    undefined: dict[str, list[str]] = {}
    for name, unit in have.items():
        missing = sorted(r for r in refs_of(unit["rule"]) if r not in have)
        if missing:
            undefined[name] = missing
    return undefined


def unreachable(have: dict[str, Json], starts: list[str]) -> list[str]:
    """Derived productions not reachable from any start symbol."""
    reach, stack = set(starts), list(starts)
    while stack:
        for ref in refs_of(have[stack.pop()]["rule"]):
            if ref in have and ref not in reach:
                reach.add(ref)
                stack.append(ref)
    return sorted(set(have) - reach)


def cycle_hits(name: str, have: dict[str, Json]) -> int:
    """How many units reachable from ``name`` refer back to it."""
    hits = 0
    seen: set[str] = set()
    stack = [name]
    while stack:
        current = stack.pop()
        for ref in refs_of(have[current]["rule"]):
            if ref == name and current != name:
                hits += 1
                break
            if ref in have and ref not in seen:
                seen.add(ref)
                stack.append(ref)
    return hits


def _report_undefined(have: dict[str, Json]) -> bool:
    undefined = undefined_references(have)
    if not undefined:
        return False
    print(f"    {len(undefined)} unit(s) reference productions that are not yet derived:")
    for name, missing in list(undefined.items())[:12]:
        print(f"      {name} -> {', '.join(missing[:6])}")
    print("      (expected mid-derivation; must be empty before freeze)")
    return True


def _report_reachability(have: dict[str, Json]) -> None:
    if START not in have:
        print(f"    note: start symbol {START} not derived yet — reachability not checked")
        return
    starts = [START]
    orphans = unreachable(have, starts)
    if orphans:
        print(f"    {len(orphans)} derived production(s) unreachable from {starts}:")
        for orphan in orphans[:12]:
            print(f"      {orphan}")
        print("      Unreachable is not automatically wrong: a production can belong to the")
        print("      other language, or be reached only once more of this one is derived.")


def _report_normalization(have: dict[str, Json]) -> bool:
    try:
        prods = normalize(have)
    except (ValueError, KeyError, TypeError, AttributeError) as exc:
        print(f"    FAIL normalization: {exc}")
        return True
    print(f"    normalized: {len(have)} units -> {len(prods)} BNF productions")
    return False


def _check_scope(scope: str, units: dict[str, Json]) -> bool:
    """One language's grammar. True if it cannot yet assemble."""
    have = {n: u for n, u in grammar_view(units, scope).items() if u.get("rule")}
    print(f"  {scope}:")
    failed = _report_undefined(have)
    _report_reachability(have)
    failed = _report_normalization(have) or failed
    cycles = sum(cycle_hits(name, have) for name in have)
    if cycles:
        print(f"    note: {cycles} reference cycle(s) — expected in a nested language")
    return failed


def main(argv: list[str] | None = None) -> int:
    """Report cross-unit structure; 1 if the units cannot yet assemble into a grammar."""
    argparse.ArgumentParser(description=__doc__).parse_args(argv)
    os.chdir(REPO_ROOT)

    every = load_units()
    units = {k: u for k, u in every.items() if u["status"] != "retired"}

    failed = False
    if both := shadowed(every):
        # The plan retires the shared unit when it splits one; both live means the
        # languages would silently read different rules depending on load order.
        failed = True
        print(f"  FAIL {len(both)} production(s) have a live shared unit AND a variant:")
        print(f"    {', '.join(both[:12])} — re-run python3.11 .claude/scripts/grammar_plan.py")
    for scope in SCOPES:
        failed = _check_scope(scope, units) or failed

    counts = Counter(u["status"] for u in units.values())
    print("  status: " + ", ".join(f"{k}={v}" for k, v in sorted(counts.items())))
    if counts["conflict"]:
        failed = True
        print("  conflicts are unresolved — run the grammar-adjudicator agent on each")
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
