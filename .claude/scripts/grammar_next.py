# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Phase 3 helper. Emits the context pack for ONE pending unit and nothing else.

Isolation is the reproducibility mechanism. If a unit is derived with only its
own clause, its own Xtext rule, and its own corpus instances in context, the
output does not depend on what was derived before it — same inputs, same
result, regardless of session boundaries or order. Any workflow that shows the
model the whole grammar at once forfeits that.

    python3.11 .claude/scripts/grammar_next.py                      next pending unit
    python3.11 .claude/scripts/grammar_next.py PartUsage            a specific unit
    python3.11 .claude/scripts/grammar_next.py RootNamespace@sysml  one language's variant

A production KerML and SysML state differently is two units, one per language
(ADR-0014). Each variant's pack carries only its own language's clause, Xtext and
corpus, because the other language's body is not the one it is deriving.
"""

from __future__ import annotations

import argparse
import json
import os
import re
from itertools import islice
from pathlib import Path
from typing import TYPE_CHECKING

from _grammar import (
    GRAMMAR,
    SCOPE_OF_SUFFIX,
    clause_defects,
    clause_for_scope,
    load_units,
    pinned_tokens,
    unit_key,
)
from _state import REPO_ROOT, load_json

if TYPE_CHECKING:
    from _state import Json

MAX_SNIPPETS = 8
#: Where the clause text lives. Not in the units and not in the repository: it is
#: verbatim OMG specification prose, and this repository is MIT and public. Build it
#: with .claude/scripts/export_wiki_clauses.py.
CLAUSES = os.environ.get("SV2_WIKI_CLAUSES", str(Path.home() / ".sv2-derivation/bnf-clauses.json"))
HARD_RULES = [
    "Derive from spec_clause_text. The Xtext is a second opinion, never the source.",
    "Do NOT port Xtext LL workarounds: inlined bodies, narrowed ranges, -> and => predicates.",
    "Every kw text must be in allowed_keywords. Every ref name must be in allowed_refs.",
    "At least one evidence entry. If sources disagree, set status 'conflict' and stop.",
    "Touch no other unit file. Read no other unit file.",
]


def _select(units: dict[str, Json], wanted: str | None) -> tuple[Json | None, Json | None]:
    """(unit, message): exactly one of the two is set."""
    if wanted:
        unit = units.get(wanted)
        if unit:
            return unit, None
        variants = sorted(
            k for k, u in units.items() if u["production"] == wanted and u.get("scope")
        )
        if variants:
            return None, {
                "error": f"{wanted!r} is stated differently in each language; name a variant",
                "variants": variants,
            }
        return None, {"error": f"no unit {wanted!r}"}
    pending = [u for u in units.values() if u["status"] == "pending"]
    if not pending:
        return None, {
            "done": True,
            "message": "no pending units",
            "next": "python3.11 .claude/scripts/grammar_consistency.py",
        }
    # Deterministic order: fewest dependencies first, then alphabetical, so the
    # same repository always yields the same sequence.
    return min(
        pending, key=lambda u: (len(u["inputs"].get("xtext_rule_text", "")), unit_key(u))
    ), None


def corpus_instances(xtext_rule_text: str, scope: str | None = None) -> list[dict[str, str]]:
    """Corpus lines mentioning this construct's keywords — evidence, not truth.

    A variant reads only its own language's files; a shared unit reads both.
    """
    literals = re.findall(r"'((?:[^'\\]|\\.)*)'", xtext_rule_text)
    words = [k for k in literals if re.fullmatch(r"[a-z][a-z0-9_]*", k)][:3]
    if not words:
        return []
    pattern = re.compile(
        r"^.*\b(" + "|".join(re.escape(w) for w in words) + r")\b.*$", re.MULTILINE
    )
    snippets: list[dict[str, str]] = []
    suffixes = [s for s, lang in SCOPE_OF_SUFFIX.items() if scope in (None, lang)]
    files = [
        f
        for root in ("vendor/corpus", "tests/corpus")
        for suffix in suffixes
        for f in sorted(Path(root).rglob(f"*{suffix}"))
    ]
    for f in files:
        snippets.extend(
            {"file": str(f), "line": m.group().strip()[:160]}
            for m in islice(pattern.finditer(f.read_text(errors="replace")), 2)
        )
        if len(snippets) >= MAX_SNIPPETS:
            break
    return snippets


def clause_text(name: str, scope: str | None = None) -> tuple[str, str]:
    """The clause text for one unit, and a complaint if it is absent or malformed."""
    clauses = load_json(CLAUSES, {})
    if not clauses:
        return "", (
            f"no clause export at {CLAUSES} — run "
            "python3.11 .claude/scripts/export_wiki_clauses.py, then set SV2_WIKI_CLAUSES"
        )
    entry = clauses.get(name)
    if not entry:
        return "", f"{name} has no clause in {CLAUSES}"
    _, text = clause_for_scope(entry, scope)
    if not text:
        return "", f"{name} has no {scope} clause in {CLAUSES}"
    defects = clause_defects(text)
    if defects:
        # The clause is the source. Saying this up front is the difference between
        # repairing it deliberately and not noticing it needed repair.
        return text, (
            f"THE CLAUSE TEXT IS MALFORMED: it has {', and '.join(defects)}. "
            "Do not derive from it as written. Work out the repair, check it against "
            "the Tier B' transcription in vendor/spec-bnf and against the Xtext, and "
            "if exactly one repair is coherent, derive from that and record on the "
            "unit what was wrong and why the repair is the only reading. If more than "
            "one repair is coherent, the unit is a conflict. Do not assert whether the "
            "defect is in the OMG document or in the wiki's extraction of it unless "
            "you have checked the PDF."
        )
    return text, ""


def context_pack(unit: Json) -> Json:
    """The complete, isolated input for deriving one unit."""
    name, scope, key = unit["production"], unit.get("scope"), unit_key(unit)
    inputs = unit["inputs"]
    text, complaint = clause_text(name, scope)
    keywords, operators = pinned_tokens()
    inventory = load_json(GRAMMAR / "productions.json", {"productions": []})
    return {
        "unit": key,
        "production": name,
        "scope": scope or "shared",
        "status": unit["status"],
        "fingerprint": unit["fingerprint"]["combined"][:16],
        "metaclass": inputs.get("metaclass", ""),
        "inputs": {
            "spec_clause_ref": inputs.get("spec_clause_ref", ""),
            "spec_clause_text": text,
            "spec_clause_problem": complaint,
            "xtext_file": inputs.get("xtext_file", ""),
            "xtext_rule_text": inputs.get("xtext_rule_text", ""),
            "corpus_instances": corpus_instances(inputs.get("xtext_rule_text", ""), scope),
        },
        "allowed_keywords": keywords + operators,
        "allowed_refs": sorted({p["name"] for p in inventory["productions"]}),
        "contract": {
            "write_to": f".claude/state/grammar/units/{key}.json",
            "set_fields": ["rule", "decision", "evidence", "status", "derived_utc", "notes"],
            "status_must_be": "derived",
            "rule_is": "a JSON AST per .claude/state/schema/grammar-unit.schema.json",
            "hard_rules": HARD_RULES,
        },
        "then_run": f"python3.11 .claude/scripts/grammar_check_unit.py {key}",
    }


def main(argv: list[str] | None = None) -> int:
    """Emit the context pack for one pending unit, as JSON on stdout."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("unit", nargs="?", help="a specific unit instead of the next pending one")
    args = parser.parse_args(argv)
    os.chdir(REPO_ROOT)

    units = load_units()
    if not units:
        print(
            json.dumps(
                {"error": "no units", "fix": "run python3.11 .claude/scripts/grammar_plan.py"}
            )
        )
        return 1
    unit, message = _select(units, args.unit)
    if unit is None:
        print(json.dumps(message))
        return 0 if message and message.get("done") else 1
    print(json.dumps(context_pack(unit), indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
