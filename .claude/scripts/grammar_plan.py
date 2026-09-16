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
import re
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
    """The inputs a unit records. The clause TEXT is deliberately not among them.

    A unit is committed; the clause is verbatim OMG specification prose and this
    repository is MIT and public. What the unit keeps is the citation and, in
    `fingerprint.spec_clause`, a sha256 of the text — one way, so it is a drift
    detector and not a copy. `grammar_next.py` joins the text back from the local
    export when it emits a pack. Storing it here also duplicated each clause once per
    production in it: 1.2 MB for 170 KB of distinct text.
    """
    return {
        "spec_clause_ref": clause.get("ref", ""),
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


#: A terminal in the specification BNF is written in SCREAMING_SNAKE. Terminals come
#: from the lexer, not from derivation. Checked against the Xtext's own `kind` in
#: `_terminals`, which reports a divergence rather than absorbing it.
TERMINAL_NAME = re.compile(r"^[A-Z][A-Z0-9_]*$")


def _terminals(names: list[str]) -> set[str]:
    """The lexical productions among `names`, and a complaint if the two inventories disagree."""
    lexical = {n for n in names if TERMINAL_NAME.match(n)}
    xtext = load_json(GRAMMAR / "productions.json") or {"productions": []}
    marked = {p["name"] for p in xtext["productions"] if p.get("kind") == "terminal"}
    # A terminal the Xtext names that does not look like one is the convention breaking,
    # and the convention is what this function rests on.
    if unexpected := (marked & set(names)) - lexical:
        print(
            f"  warning: Xtext calls these terminal but they are not SCREAMING_SNAKE: {unexpected}"
        )
    return lexical


def _strip_clause_text(units: dict[str, Json]) -> int:
    """Drop clause text left in units by an earlier schema. Idempotent."""
    stripped = 0
    for unit in units.values():
        if unit.get("inputs", {}).pop("spec_clause_text", None) is not None:
            save_unit(unit)
            stripped += 1
    return stripped


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

    # The specification's inventory, not the pilot's. DERIVATION.md: the Xtext is a
    # second opinion and never the source, so planning from it omits every production
    # the pilot does not implement — 94 of them are recorded spec_only/follow_spec,
    # whose decision is to implement them — and plans 182 the pilot invented, all
    # recorded xtext_only/follow_spec, whose decision is not to. Each unit still
    # carries its Xtext rule as `xtext_rule_text`; that is where the second opinion
    # belongs.
    inventory = load_json(GRAMMAR / "bnf-productions.json")
    if not inventory:
        print("no inventory — run python3.11 scripts/extract_bnf.py")
        return 1
    src = _load_sources()
    units = load_units()

    stripped = _strip_clause_text(units)
    if stripped:
        print(f"      removed embedded clause text from {stripped} unit(s)")

    names: list[str] = inventory["productions"]
    derivable = [n for n in names if n not in _terminals(names)]

    outcomes = {"created": 0, "restale": 0, "carried": 0}
    for name in derivable:
        outcomes[_plan_one(name, units, src)] += 1

    retired = _retire_undeclared(set(derivable))
    print(
        f"plan: {outcomes['created']} new, {outcomes['restale']} stale (inputs moved), "
        f"{outcomes['carried']} carried forward, {retired} retired"
    )
    print(f"      units at {UNITS}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
