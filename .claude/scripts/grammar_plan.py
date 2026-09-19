# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Phase 2. Deterministic. Builds the unit work list and computes input fingerprints.

Creates pending units; never modifies a derived rule.

    python3.12 .claude/scripts/grammar_plan.py

Environment: SV2_WIKI_CLAUSES overrides the specification clause export. Without an
export the plan refuses to run; see `_clause_export`.
"""

from __future__ import annotations

import argparse
import collections
import os
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING

from _grammar import (
    GRAMMAR,
    SCOPES,
    TERMINAL_NAME,
    UNITS,
    clause_for_scope,
    hash_parts,
    load_units,
    production_scopes,
    save_unit,
    unit_key,
    xtext_rule_text,
)
from _state import REPO_ROOT, load_json

if TYPE_CHECKING:
    from _state import Json

MODEL_SUFFIXES = (".sysml", ".kerml")
DEFAULT_CLAUSES = "vendor/wiki/bnf-clauses.json"
STALEABLE = ("derived", "verified")


@dataclass(frozen=True, slots=True)
class Sources:
    """Everything a unit's inputs are drawn from."""

    clauses: Json
    metaclass_of: dict[str, str]
    corpus: list[Path]
    corpus_fingerprint: str


def _clause_export() -> Json:
    """The clause export, or None when it is missing or empty.

    Every unit's `spec_clause` fingerprint hashes its clause text, so planning without
    the export hashes empty text for all of them: every derived and verified unit reads
    as having moved inputs and goes back to pending. That is not a plan, it is the
    whole derivation thrown away, so there is no partial mode to fall back to.
    """
    return load_json(os.environ.get("SV2_WIKI_CLAUSES", DEFAULT_CLAUSES), {}) or None


def _load_sources(clauses: Json) -> Sources:
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


def _clause_inputs(ref: str, xtext_file: str | None, xtext: str | None) -> dict[str, str]:
    """The inputs a unit records. The clause TEXT is deliberately not among them.

    A unit is committed; the clause is verbatim OMG specification prose and this
    repository is MIT and public. What the unit keeps is the citation and, in
    `fingerprint.spec_clause`, a sha256 of the text — one way, so it is a drift
    detector and not a copy. `grammar_next.py` joins the text back from the local
    export when it emits a pack. Storing it here also duplicated each clause once per
    production in it: 1.2 MB for 170 KB of distinct text.
    """
    return {
        "spec_clause_ref": ref,
        "xtext_file": xtext_file or "",
        "xtext_rule_text": xtext or "",
    }


def _language(xtext_file: str | None, scope: str | None) -> str:
    if scope:
        return scope
    if not xtext_file:
        return "shared"
    return "sysml" if "SysML" in xtext_file else "kerml"


def expected_keys(
    derivable: list[str], scopes: dict[str, tuple[str | None, ...]]
) -> list[tuple[str, str | None]]:
    """(production, scope) for every unit the inventory calls for, in a stable order.

    ``scopes`` is `production_scopes`: a production the two languages state differently
    gets one variant per language (ADR-0014); one only a single language's grammar
    reaches gets that language's variant alone (ADR-0015); every other production gets
    one shared unit.
    """
    return [(name, scope) for name in derivable for scope in scopes.get(name, (None,))]


def _rescope(shared: Json, fresh: Json, *, single: bool) -> str | None:
    """Carry a shared unit's work into the variant replacing it, where that is sound.

    ADR-0015. A shared unit gives way to variants in two cases. Either only one grammar
    reaches the production (``single``), or the SysML boundary splits it. Its rule,
    reasoning and evidence carry over when the variant's inputs hash the same as the
    shared unit's did: the same clause and the same Xtext, so the same derivation. The
    checks never carry, because they ran against both grammars, so a carried verified
    unit becomes derived and grammar_check_unit.py verifies it again against its own.

    Where the inputs differ, a single-language variant keeps the rule as the previous
    one and goes back to pending, as any unit whose inputs moved does. A split variant
    whose inputs differ is new work, because its language reads a different body, and
    nothing carries: returns None, and the caller creates it fresh.
    """
    same_inputs = shared["fingerprint"]["combined"] == fresh["fingerprint"]["combined"]
    if not same_inputs and not single:
        return None
    moved = {
        k: v
        for k, v in shared.items()
        if k not in ("scope", "fingerprint", "inputs", "acceptance", "diagnostics")
    }
    unit = {**fresh, **moved, "scope": fresh["scope"], "language": fresh["language"]}
    how = "alone" if single else "as one of its two variants"
    note = f"re-scoped from the shared unit to {fresh['scope']} {how} (ADR-0015)"
    if not same_inputs and unit["status"] in STALEABLE:
        unit["status"] = "pending"
        note += (
            "; its inputs as a variant differ, so the rule below is the previous one"
            " and must be re-derived"
        )
    elif unit["status"] == "verified":
        unit["status"] = "derived"
        note += "; awaiting grammar_check_unit.py against this language's grammar alone"
    unit["notes"] = (note + "\n" + shared.get("notes", "")).strip()
    save_unit(unit)
    return "rescoped"


def _plan_one(
    name: str, scope: str | None, units: dict[str, Json], src: Sources, *, single: bool = False
) -> str:
    """Create, re-stale, revive or carry forward one unit. Returns which happened."""
    xtext_file, xtext = xtext_rule_text(name, scope)
    ref, text = clause_for_scope(src.clauses.get(name, {}), scope)
    fp = {
        "spec_clause": hash_parts(text),
        "xtext_rule": hash_parts(xtext or ""),
        "corpus_instances": src.corpus_fingerprint,
    }
    # `combined` deliberately EXCLUDES the corpus. The corpus is evidence and a
    # validation oracle, not an input to the rule's shape — and it is global, so
    # including it would make adding one corpus file invalidate every unit in the
    # grammar and destroy the whole point of fingerprinting. Corpus changes are
    # caught by grammar_validate.py re-running, which is the correct mechanism.
    fp["combined"] = hash_parts(fp["spec_clause"], fp["xtext_rule"])

    fresh: Json = {
        "schema_version": 1,
        "production": name,
        **({"scope": scope} if scope else {}),
        "language": _language(xtext_file, scope),
        "status": "pending",
        "fingerprint": fp,
        "inputs": {
            **_clause_inputs(ref, xtext_file, xtext),
            "metaclass": src.metaclass_of.get(name, ""),
        },
    }
    unit = units.get(unit_key(fresh))
    shared = units.get(name)
    if unit is None and scope and shared and shared["status"] != "retired":
        carried = _rescope(shared, fresh, single=single)
        if carried:
            return carried
    if unit is None:
        save_unit(fresh)
        return "created"
    if unit["status"] == "retired":
        # Declared again — typically a production whose two languages stopped
        # diverging, or started. Its old rule was for inputs that no longer apply.
        fresh["notes"] = ("declared again after retirement\n" + unit.get("notes", "")).strip()
        save_unit(fresh)
        return "revived"
    if unit["fingerprint"]["combined"] == fp["combined"] or unit["status"] not in STALEABLE:
        return "carried"
    unit["status"] = "pending"  # inputs moved: the rule must be re-derived
    unit["fingerprint"] = fp
    unit["inputs"].update(_clause_inputs(ref, xtext_file, xtext))
    unit["notes"] = (
        "inputs changed since derivation; rule below is the previous one and "
        "must be re-derived\n" + unit.get("notes", "")
    ).strip()
    save_unit(unit)
    return "restale"


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


#: Input fields an earlier schema wrote and nothing reads. `spec_clause_text` was
#: verbatim OMG prose (see `_clause_inputs`). `corpus_refs` was the first 40 corpus
#: paths in the unit's language — the same list for every unit of that language, about
#: 2 MB across the grammar, and no rule's evidence: `grammar_next.py` finds a unit's
#: real instances itself when it emits the pack.
RETIRED_INPUTS = ("spec_clause_text", "corpus_refs")


def _strip_retired_inputs(units: dict[str, Json]) -> int:
    """Drop input fields left in units by an earlier schema. Idempotent."""
    stripped = 0
    for unit in units.values():
        inputs = unit.get("inputs", {})
        if [inputs.pop(field) for field in RETIRED_INPUTS if field in inputs]:
            save_unit(unit)
            stripped += 1
    return stripped


def _retire_undeclared(expected: set[str], scopes: dict[str, tuple[str | None, ...]]) -> int:
    retired = 0
    for key, unit in load_units().items():
        if key in expected or unit["status"] == "retired":
            continue
        planned = scopes.get(unit["production"], (None,))
        if not unit.get("scope") and planned == SCOPES:
            reason = (
                "split per language (ADR-0014): KerML and SysML state this production "
                "differently, so it is derived as one variant per language instead"
            )
        elif not unit.get("scope") and len(planned) == 1 and planned[0]:
            reason = (
                f"re-scoped (ADR-0015): only the {planned[0]} grammar reaches this "
                f"production, so its work now lives in {unit['production']}@{planned[0]}"
            )
        else:
            reason = "no longer declared in the pinned grammar"
        unit["status"] = "retired"
        unit["notes"] = (reason + "\n" + unit.get("notes", "")).strip()
        save_unit(unit)
        retired += 1
    return retired


def main(argv: list[str] | None = None) -> int:
    """Build the unit work list from the inventory; 1 if there is no inventory or clause export."""
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
        print("no inventory — run python3.12 scripts/extract_bnf.py")
        return 1
    clauses = _clause_export()
    if clauses is None:
        path = os.environ.get("SV2_WIKI_CLAUSES", DEFAULT_CLAUSES)
        print(f"no clause export at {path} — refusing to plan, because every unit would go stale")
        print("  run python3.12 .claude/scripts/export_wiki_clauses.py, then")
        print("  export SV2_WIKI_CLAUSES=~/.sv2-derivation/bnf-clauses.json")
        return 1
    src = _load_sources(clauses)
    units = load_units()

    stripped = _strip_retired_inputs(units)
    if stripped:
        print(f"      removed retired input fields from {stripped} unit(s)")

    names: list[str] = inventory["productions"]
    derivable = [n for n in names if n not in _terminals(names)]
    scopes = production_scopes(inventory["rules"])
    keys = expected_keys(derivable, scopes)

    outcomes = {"created": 0, "restale": 0, "revived": 0, "carried": 0, "rescoped": 0}
    for name, scope in keys:
        single = len(scopes.get(name, (None,))) == 1
        outcomes[_plan_one(name, scope, units, src, single=single)] += 1

    expected = {f"{name}@{scope}" if scope else name for name, scope in keys}
    retired = _retire_undeclared(expected, scopes)
    print(
        f"plan: {outcomes['created']} new, {outcomes['restale']} stale (inputs moved), "
        f"{outcomes['revived']} revived, {outcomes['carried']} carried forward, "
        f"{outcomes['rescoped']} re-scoped to one language, {retired} retired"
    )
    split = sum(1 for n in derivable if scopes.get(n) == SCOPES)
    alone = collections.Counter(
        s[0] for n in derivable if len(s := scopes.get(n, (None,))) == 1 and s[0]
    )
    print(f"      {split} production(s) split into a KerML and a SysML variant")
    print(f"      {alone['kerml']} KerML-only and {alone['sysml']} SysML-only production(s)")
    print(f"      units at {UNITS}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
