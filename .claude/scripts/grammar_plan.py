# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Phase 2. Deterministic. Builds the unit work list and computes input fingerprints.

Creates pending units; never modifies a derived rule.

    python3.11 .claude/scripts/grammar_plan.py

Environment: SV2_WIKI_CLAUSES overrides the specification clause export.
"""

from __future__ import annotations

import argparse
import os
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING

from _grammar import GRAMMAR, UNITS, hash_parts, load_units, save_unit, xtext_rule_text
from _state import REPO_ROOT, load_json

if TYPE_CHECKING:
    from _state import Json

MODEL_SUFFIXES = (".sysml", ".kerml")
STALEABLE = ("derived", "verified")


@dataclass(frozen=True, slots=True)
class Sources:
    """Everything a unit's inputs are drawn from."""

    clauses: Json
    metaclass_of: dict[str, str]
    corpus: list[Path]
    corpus_fingerprint: str


def _load_sources() -> Sources:
    clauses = load_json(os.environ.get("SV2_WIKI_CLAUSES", "vendor/wiki/bnf-clauses.json"), {})
    mmap = load_json(GRAMMAR / "metaclass-map.json", {"map": {}})
    metaclass_of: dict[str, str] = {}
    for meta, rules in mmap.get("map", {}).items():
        for rule in rules:
            metaclass_of.setdefault(rule, meta)
    corpus = sorted(
        [p for p in Path("vendor/corpus").rglob("*") if p.suffix in MODEL_SUFFIXES]
        + [p for p in Path("tests/corpus").rglob("*") if p.suffix in MODEL_SUFFIXES]
    )
    fingerprint = hash_parts(*[p.name + str(p.stat().st_size) for p in corpus])
    return Sources(clauses, metaclass_of, corpus, fingerprint)


def _clause_inputs(clause: Json, xtext_file: str | None, xtext: str | None) -> dict[str, str]:
    return {
        "spec_clause_ref": clause.get("ref", ""),
        "spec_clause_text": clause.get("text", ""),
        "xtext_file": xtext_file or "",
        "xtext_rule_text": xtext or "",
    }


def _language(xtext_file: str | None) -> str:
    if not xtext_file:
        return "shared"
    return "sysml" if "SysML" in xtext_file else "kerml"


def _plan_one(name: str, units: dict[str, Json], src: Sources) -> str:
    """Create, re-stale, or carry forward one unit. Returns which of the three happened."""
    xtext_file, xtext = xtext_rule_text(name)
    clause = src.clauses.get(name, {})
    fp = {
        "spec_clause": hash_parts(clause.get("text", "")),
        "xtext_rule": hash_parts(xtext or ""),
        "corpus_instances": src.corpus_fingerprint,
    }
    # `combined` deliberately EXCLUDES the corpus. The corpus is evidence and a
    # validation oracle, not an input to the rule's shape — and it is global, so
    # including it would make adding one corpus file invalidate every unit in the
    # grammar and destroy the whole point of fingerprinting. Corpus changes are
    # caught by grammar_validate.py re-running, which is the correct mechanism.
    fp["combined"] = hash_parts(fp["spec_clause"], fp["xtext_rule"])

    unit = units.get(name)
    if unit is None:
        save_unit(
            {
                "schema_version": 1,
                "production": name,
                "language": _language(xtext_file),
                "status": "pending",
                "fingerprint": fp,
                "inputs": {
                    **_clause_inputs(clause, xtext_file, xtext),
                    "corpus_refs": [str(c) for c in src.corpus[:40]],
                    "metaclass": src.metaclass_of.get(name, ""),
                },
            }
        )
        return "created"
    if unit["fingerprint"]["combined"] == fp["combined"] or unit["status"] not in STALEABLE:
        return "carried"
    unit["status"] = "pending"  # inputs moved: the rule must be re-derived
    unit["fingerprint"] = fp
    unit["inputs"].update(_clause_inputs(clause, xtext_file, xtext))
    unit["notes"] = (
        "inputs changed since derivation; rule below is the previous one and "
        "must be re-derived\n" + unit.get("notes", "")
    ).strip()
    save_unit(unit)
    return "restale"


def _retire_undeclared(declared: set[str]) -> int:
    retired = 0
    for name, unit in load_units().items():
        if name not in declared and unit["status"] != "retired":
            unit["status"] = "retired"
            unit["notes"] = (
                "no longer declared in the pinned grammar\n" + unit.get("notes", "")
            ).strip()
            save_unit(unit)
            retired += 1
    return retired


def main(argv: list[str] | None = None) -> int:
    """Build the unit work list from the inventory; 1 if there is no inventory to plan from."""
    argparse.ArgumentParser(description=__doc__).parse_args(argv)
    os.chdir(REPO_ROOT)

    inventory = load_json(GRAMMAR / "productions.json")
    if not inventory:
        print("no inventory — run python3.11 scripts/extract_productions.py")
        return 1
    src = _load_sources()
    units = load_units()

    outcomes = {"created": 0, "restale": 0, "carried": 0}
    for production in inventory["productions"]:
        if production["kind"] == "terminal":
            continue  # terminals come from the lexer, not derivation
        outcomes[_plan_one(production["name"], units, src)] += 1

    retired = _retire_undeclared({p["name"] for p in inventory["productions"]})
    print(
        f"plan: {outcomes['created']} new, {outcomes['restale']} stale (inputs moved), "
        f"{outcomes['carried']} carried forward, {retired} retired"
    )
    print(f"      units at {UNITS}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
