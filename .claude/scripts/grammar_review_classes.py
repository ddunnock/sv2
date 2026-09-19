# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Record the grammar differences whose evidence can be grounded mechanically.

Most of the Xtext-vs-specification differential is not a disagreement about the
language. Three groups are artefacts of one source's formalism, and for each the
supporting text can be located in the pinned inputs rather than judged:

    keyword factoring   Xtext `ActionDefKeyword: ActionKeyword 'def'` against a
                        specification that writes 'action' 'def' inline in
                        ActionDefinition
    operator literals   Xtext `AdditiveOperator: '+' | '-'` likewise
    lexical structure   specification ALL_CAPS productions (ALPHABETIC_CHARACTER)
                        that Xtext terminals absorb into a regular expression

Grounding is per production, never per class. An entry is written only when this
script can point at the pinned text that supports that specific production; what
it cannot ground it reports and leaves unreviewed, which is the honest state and
keeps the gate red until someone reads it. A class argument is not a licence to
record a member the argument does not actually cover.

Everything else in the differential — the expression-precedence chain above all,
where the Xtext encodes precedence as a rule chain because its parser generator
needs it to — is left alone. Those need a clause read, one at a time, through the
derive-grammar skill.

Entries written here are tagged MACHINE-GROUNDED in their evidence note, so a
re-run replaces them instead of layering a second opinion beside the first, and a
reviewer can tell them from the hand-written ones at a glance.

    python3.12 .claude/scripts/grammar_review_classes.py           write entries
    python3.12 .claude/scripts/grammar_review_classes.py --dry-run report only
"""

from __future__ import annotations

import argparse
import json
import os
import re
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
PILOT = Path("vendor/pilot")
DEVIATIONS = Path(".claude/state/deviations.json")
SPEC_INVENTORY = Path(".claude/state/grammar/bnf-productions.json")
DIFF = Path(".claude/state/grammar-diff.json")
TAG = "MACHINE-GROUNDED"

LITERAL = re.compile(r"'((?:[^'\\]|\\.)*)'")
REF = re.compile(r"\b([A-Za-z_][A-Za-z0-9_]*)\b")
HEAD = re.compile(
    r"^(?:fragment\s+|enum\s+|terminal\s+)?"
    r"(?P<name>[A-Za-z_][A-Za-z0-9_]*)"
    r"(?:\s+returns\s+[A-Za-z_][A-Za-z0-9_:]*)?"
    r"\s*:",
    re.M,
)

# Inventories, not syntactic uses. RESERVED_KEYWORD lists every keyword in the
# language and RESERVED_SYMBOL every symbol, so a literal search matches them for
# any input at all. Citing a word list as the place the grammar uses a keyword is
# a false citation, which is worse than a missing one: it looks checked.
INVENTORIES = frozenset({"RESERVED_KEYWORD", "RESERVED_SYMBOL"})

# KerML lexical clauses, with the page and receipt retrieved from the wiki.
# SysML 8.2.2.1.2 states its lexical structure "is identical to that of the KerML
# textual notation", which is why the KerML clause governs both languages.
LEXICAL_CLAUSES: dict[str, tuple[str, int, int, str]] = {
    "8.2.2.1": ("LineTerminatorsandWhiteSpace", 101, 75, "acb4b02de0361000"),
    "8.2.2.2": ("NotesandComments", 102, 76, "0ea08f080e64997f"),
    "8.2.2.3": ("Names", 102, 76, "4a2bb5aecf5e693e"),
    "8.2.2.4": ("NumericValues", 103, 77, "7082f1fcd5f42dc4"),
    "8.2.2.5": ("StringValue", 103, 77, "2d6256a1cfdb8254"),
    "8.2.2.6": ("ReservedWords", 103, 77, "f4ed93be1f57b626"),
    "8.2.2.7": ("Symbols", 104, 78, "c59561da3efdda5a"),
}
SYSML_DEFERS = "SysML 8.2.2.1.2 (PDF p.197, sha 4ceb9d82…)"


def strip_comments(text: str) -> str:
    """Xtext source with comments removed and quoted literals preserved intact."""
    out: list[str] = []
    i, n = 0, len(text)
    while i < n:
        if text.startswith("//", i):
            j = text.find("\n", i)
            i = n if j < 0 else j
        elif text.startswith("/*", i):
            j = text.find("*/", i + 2)
            i = n if j < 0 else j + 2
        elif text[i] == "'":
            j = i + 1
            while j < n and text[j] != "'":
                j += 2 if text[j] == "\\" else 1
            out.append(text[i : j + 1])
            i = j + 1
        else:
            out.append(text[i])
            i += 1
    return "".join(out)


def rule_bodies() -> dict[str, str]:
    """Every Xtext rule name mapped to its body, from the head colon to the semicolon."""
    found: dict[str, str] = {}
    for path in sorted(PILOT.glob("*.xtext")):
        text = strip_comments(path.read_text(encoding="utf-8"))
        marks = list(HEAD.finditer(text))
        for n, mark in enumerate(marks):
            end = marks[n + 1].start() if n + 1 < len(marks) else len(text)
            body = text[mark.end() : end]
            semi = body.rfind(";")
            found.setdefault(mark.group("name"), body[: semi if semi >= 0 else len(body)].strip())
    return found


def resolve_literals(name: str, bodies: dict[str, str], seen: set[str] | None = None) -> list[str]:
    """The literal sequence a rule accepts, following references to other literal rules."""
    seen = set() if seen is None else seen
    if name in seen:
        return []
    seen.add(name)
    body = bodies.get(name, "")
    out: list[str] = []
    position = 0
    for match in LITERAL.finditer(body):
        for ref in REF.findall(body[position : match.start()]):
            if ref in bodies and ref != name:
                out.extend(resolve_literals(ref, bodies, seen))
        out.append(match.group(1))
        position = match.end()
    for ref in REF.findall(body[position:]):
        if ref in bodies and ref != name:
            out.extend(resolve_literals(ref, bodies, seen))
    return out


# Any: bnf-productions.json is a deserialized document (STD-001-PY §6).
def spec_sites(literals: list[str], rules: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Specification rules whose body contains every one of these literals."""
    if not literals:
        return []
    wanted = [f"'{lit}'" for lit in literals]
    return [
        r for r in rules if r["name"] not in INVENTORIES and all(w in r["body"] for w in wanted)
    ]


def _literal_rule_entry(
    name: str, body: str, literals: list[str], sites: list[dict[str, Any]]
) -> dict[str, Any]:
    clauses = sorted({s["clause"] for s in sites if s["clause"]})
    used_by = sorted({s["name"] for s in sites})[:6]
    kind = "keyword factoring" if name.endswith("Keyword") else "operator/kind literal set"
    return {
        "production": name,
        "bucket": "xtext_only",
        "upstream": f"Xtext defines `{name}: {body};`.",
        "problem": (
            f"A {kind} rule with no counterpart in the specification. Its body reduces to the "
            f"literal sequence {literals}, which the specification does not name as a "
            f"production — it writes those literals inline in {', '.join(used_by)}"
            + (" and others" if len(sites) > len(used_by) else "")
            + ". The rule is an Xtext factoring convenience, not a language construct."
        ),
        "decision": "follow_spec",
        "rationale": (
            "Do not create a production for this. The parser matches the literals directly "
            "where the specification writes them, so there is no rule here to mark for "
            "coverage and nothing is lost: the accepted input is identical either way. "
            "Porting the factoring would add a production the language does not have and "
            "would measure coverage against an inventory the specification does not recognise."
        ),
        "evidence": [
            {
                "kind": "spec_clause",
                "ref": (
                    f"clause {', '.join(clauses) if clauses else 'n/a'} — vendor/spec-bnf, "
                    f"production(s) {', '.join(used_by)}"
                ),
                "note": (
                    f"{TAG}. Each of these specification productions contains the literal "
                    f"sequence {literals} written inline. Located by searching the pinned "
                    "Tier B-prime rule bodies for the literals, not by name similarity; the "
                    "reserved-word and reserved-symbol inventories are excluded, since they "
                    "list every keyword and would otherwise match everything."
                ),
            },
            {
                "kind": "file_id",
                "ref": "ptc/25-02-15",
                "note": (
                    f"Xtext side read from the pinned grammar: body `{body}`, which "
                    "references no non-literal production."
                ),
            },
        ],
    }


def _lexical_entry(name: str, clause: str) -> dict[str, Any]:
    atom, pdf, printed, sha = LEXICAL_CLAUSES[clause]
    return {
        "production": name,
        "bucket": "spec_only",
        "upstream": (
            "The Xtext declares no rule of this name. KerMLExpressions.xtext implements the "
            "lexical level with a small set of terminals (ID, STRING_VALUE, DECIMAL_VALUE, "
            "EXP_VALUE, ML_NOTE, SL_NOTE, REGULAR_COMMENT, WS, UNRESTRICTED_NAME) whose "
            "regular expressions absorb this production rather than naming it."
        ),
        "problem": (
            f"The specification states the lexical structure as named BNF productions, and "
            f"`{name}` is one of them, defined at KerML {clause}. An Xtext terminal is a "
            "regular expression, so its sub-productions disappear into the regex and cannot "
            "appear in the Xtext inventory. The difference is one of formalism, not of what "
            "text is accepted."
        ),
        "decision": "follow_spec",
        "rationale": (
            "Implement the lexical structure as the specification states it. A hand-written "
            "lexer is not bound by Xtext's terminal formalism, and naming these productions "
            "is what lets a rejection test cite the exact lexical rule an input violates, "
            "which one opaque terminal regex cannot do. Expect no separate parser function "
            "per production: the coverage marker belongs on the lexer routine implementing "
            "the clause."
        ),
        "evidence": [
            {
                "kind": "spec_clause",
                "ref": f"KerML {clause} ({atom}), PDF p.{pdf}, printed p.{printed}",
                "note": (
                    f"{TAG}. Retrieved as wiki production atom {atom}, region_sha256 {sha}… "
                    f"{SYSML_DEFERS} states that the SysML lexical structure 'is identical to "
                    "that of the KerML textual notation', which is why the KerML clause "
                    "governs both languages."
                ),
            },
            {
                "kind": "file_id",
                "ref": "ptc/25-04-04",
                "note": "The KerML Tier A pin the governing clause belongs to.",
            },
        ],
    }


def _is_literal_rule(name: str, body: str, bodies: dict[str, str]) -> bool:
    """Whether a rule reduces to keyword literals and nothing else.

    The literal need not appear in the body itself. `ActionUsageKeyword: ActionKeyword`
    is a bare reference that resolves to 'action', and it is the same factoring device
    as `ActionKeyword: 'action'` — requiring a literal in the body would leave roughly
    twenty of these unreviewed for a reason that is about spelling, not substance.
    What matters is that nothing outside the keyword/operator rules is reachable and
    that some literal is reached in the end.
    """
    if not body:
        return False
    refs = {r for r in REF.findall(LITERAL.sub(" ", body)) if r in bodies}
    if {r for r in refs if not r.endswith(("Keyword", "Operator"))}:
        return False
    return bool(resolve_literals(name, bodies))


def build(
    bodies: dict[str, str], rules: list[dict[str, Any]], diff: dict[str, Any], have: set[str]
) -> tuple[list[dict[str, Any]], list[tuple[str, str]]]:
    """(entries to record, productions deliberately left unreviewed with the reason)."""
    entries: list[dict[str, Any]] = []
    skipped: list[tuple[str, str]] = []
    by_name: dict[str, list[dict[str, Any]]] = {}
    for rule in rules:
        by_name.setdefault(rule["name"], []).append(rule)

    for name in diff["xtext_only"]:
        if name in have:
            continue
        body = bodies.get(name, "")
        if not _is_literal_rule(name, body, bodies):
            continue  # not one of these classes; leave it for individual derivation
        literals = resolve_literals(name, bodies)
        sites = spec_sites(literals, rules)
        if not sites:
            skipped.append((name, f"literals {literals} not located in any specification body"))
            continue
        entries.append(_literal_rule_entry(name, body, literals, sites))

    for name in diff["spec_only"]:
        if name in have or not name.isupper():
            continue
        records = by_name.get(name, [])
        preferred = [r for r in records if r["file"].startswith("KerML")] or records
        if not preferred:
            skipped.append((name, "no specification record"))
            continue
        clause = preferred[0]["clause"]
        if clause not in LEXICAL_CLAUSES:
            skipped.append((name, f"clause {clause} has no retrieved lexical atom"))
            continue
        entries.append(_lexical_entry(name, clause))

    return entries, skipped


def main(argv: list[str] | None = None) -> int:
    """Write the groundable entries into the deviation register; report what it could not."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dry-run", action="store_true", help="report without writing")
    args = parser.parse_args(argv)
    os.chdir(ROOT)

    if not SPEC_INVENTORY.is_file() or not DIFF.is_file():
        print("run scripts/extract_bnf.py and scripts/grammar_diff.py first")
        return 0

    document = json.loads(DEVIATIONS.read_text())
    kept = [
        d
        for d in document["deviations"]
        if not any(TAG in e.get("note", "") for e in d["evidence"])
    ]
    rules = json.loads(SPEC_INVENTORY.read_text())["rules"]
    diff = json.loads(DIFF.read_text())
    entries, skipped = build(rule_bodies(), rules, diff, {d["production"] for d in kept})

    print(f"{len(entries)} groundable, {len(skipped)} left unreviewed:")
    for name, why in skipped:
        print(f"    {name}: {why}")
    if args.dry_run:
        return 0

    document["deviations"] = sorted(kept + entries, key=lambda d: d["production"])
    DEVIATIONS.write_text(json.dumps(document, indent=2) + "\n")
    print(f"wrote {DEVIATIONS} ({len(document['deviations'])} deviations)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
