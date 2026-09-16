# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Deterministic extraction from the pinned specification BNF (Tier B-prime).

``vendor/spec-bnf/*.kebnf`` is the specification's textual BNF in machine-readable
form, carrying the spec's own clause numbers as comments. Produces one artifact:

    .claude/state/grammar/bnf-productions.json

    "productions"  the sorted name list scripts/grammar_diff.py differences against
    "rules"        one record per production: metaclass, clause, file, line, body

Unlike the Xtext (Tier B), the rule *bodies* here are in scope: the transcription
follows the specification rather than a parser generator's limitations. It is still
not normative — its own header reads "Manual corrections by HP de Koning" — so the
clause recorded beside each rule is the citation, and the PDF clause arbitrates
wherever the two disagree. See .claude/state/deviations.json under source_selection.

    python3.11 scripts/extract_bnf.py           write the artifact
    python3.11 scripts/extract_bnf.py --check   fail if it is stale
"""

from __future__ import annotations

import argparse
import json
import os
import re
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
SPEC_BNF = Path("vendor/spec-bnf")
OUT_DIR = Path(".claude/state/grammar")
ARTIFACT = "bnf-productions.json"
GENERATED_BY = "scripts/extract_bnf.py"

# A production head is the only thing that starts in column 0 that is not a comment
# or a blank line — verified across both pinned files, which is what makes the split
# unambiguous rather than heuristic. Two forms:
#
#   Name =                     no metaclass declared (lexical rules, alternations)
#   Name : Metaclass =         constructs Metaclass, and the body carries the
#                              abstract-syntax assignments that build it
HEAD = re.compile(
    r"^(?P<name>[A-Za-z_][A-Za-z0-9_]*)"
    r"(?:\s*:\s*(?P<metaclass>[A-Za-z_][A-Za-z0-9_]*))?"
    r"\s*=(?P<rest>.*)$"
)
# "// Clause 8.2.2.11 Parts Textual Notation". The number is the citation; the title
# is kept because a reviewer reads it and a bare number tells them nothing.
CLAUSE = re.compile(r"^//\s*Clause\s+(?P<number>[0-9]+(?:\.[0-9]+)*)\s*(?P<title>.*?)\s*$")


def _blocks(lines: list[str]) -> list[tuple[int, str, list[str]]]:
    """(line number, head line, body lines) for each production in one file.

    A block runs from its head to the line before the next head, so the body is
    whatever the transcription put there — including the ``{ ... }`` abstract-syntax
    actions, which are part of the rule and must not be dropped.
    """
    starts = [i for i, line in enumerate(lines) if line[:1].strip() and HEAD.match(line)]
    blocks = []
    for n, start in enumerate(starts):
        end = starts[n + 1] if n + 1 < len(starts) else len(lines)
        blocks.append((start + 1, lines[start], lines[start + 1 : end]))
    return blocks


def _clause_at(lines: list[str], index: int) -> tuple[str, str]:
    """The nearest ``// Clause`` marker at or above ``index``, as (number, title)."""
    for line in reversed(lines[:index]):
        match = CLAUSE.match(line.strip())
        if match:
            return match.group("number"), match.group("title")
    return "", ""


def _body(head_rest: str, body_lines: list[str]) -> str:
    """The rule body, with the transcription's own line breaks and indentation kept.

    A block runs to the next head, so its tail picks up the gap before that head:
    blank lines, the next section's ``// Clause`` marker, and standalone notes such
    as ``// (See Note 1)``. Those are dropped — attributing the next clause's marker
    to this rule would put a wrong citation on it, which is worse than none.
    Interior blank lines are kept: a multi-part body uses them, and losing them
    changes what the rule looks like to the reviewer who has to check it.
    """
    parts = [head_rest.rstrip(), *body_lines]
    while parts and (not parts[-1].strip() or parts[-1].lstrip().startswith("//")):
        parts.pop()
    while parts and not parts[0].strip():
        parts.pop(0)
    return "\n".join(parts)


def extract(files: list[Path]) -> list[dict[str, Any]]:
    """One record per production across every pinned BNF file, sorted by name."""
    rules: list[dict[str, Any]] = []
    for path in files:
        lines = path.read_text(encoding="utf-8").splitlines()
        for line_number, head, body_lines in _blocks(lines):
            match = HEAD.match(head)
            if match is None:  # pragma: no cover - _blocks only yields matching heads
                continue
            number, title = _clause_at(lines, line_number - 1)
            rules.append(
                {
                    "name": match.group("name"),
                    "metaclass": match.group("metaclass") or "",
                    "clause": number,
                    "clause_title": title,
                    "file": path.name,
                    "line": line_number,
                    "body": _body(match.group("rest"), body_lines),
                }
            )
    return sorted(rules, key=lambda r: (r["name"], r["file"]))


def build_artifact(files: list[Path], rules: list[dict[str, Any]]) -> dict[str, Any]:
    """The JSON document written to .claude/state/grammar/bnf-productions.json."""
    names = sorted({r["name"] for r in rules})
    return {
        "_generated_by": GENERATED_BY,
        "_note": (
            "Tier B-prime: the specification textual BNF as transcribed in "
            "SysML-v2-Release/bnf. Not normative — the PDF clause named in each rule's "
            "'clause' field arbitrates. 'productions' is the name list grammar_diff.py "
            "differences against the Xtext inventory."
        ),
        "_source": [f.name for f in files],
        "counts": {
            "total": len(rules),
            "unique_names": len(names),
            "with_metaclass": sum(1 for r in rules if r["metaclass"]),
            "with_clause": sum(1 for r in rules if r["clause"]),
        },
        "productions": names,
        "rules": rules,
    }


def _render(data: dict[str, Any]) -> str:
    return json.dumps(data, indent=2) + "\n"


def _check(artifact: dict[str, Any]) -> int:
    path = OUT_DIR / ARTIFACT
    if not path.exists() or path.read_text() != _render(artifact):
        print(f"derived BNF inventory is stale: {ARTIFACT}")
        print(f"run python3.11 {GENERATED_BY}")
        return 1
    counts = artifact["counts"]
    print(f"BNF inventory current ({counts['total']} productions, {counts['with_clause']} cited)")
    return 0


def _write(artifact: dict[str, Any]) -> None:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    (OUT_DIR / ARTIFACT).write_text(_render(artifact))
    counts = artifact["counts"]
    print(
        f"wrote grammar/{ARTIFACT}  {counts['total']} productions "
        f"({counts['with_metaclass']} with a metaclass, {counts['with_clause']} with a clause)"
    )


def main(argv: list[str] | None = None) -> int:
    """Write the derived BNF inventory, or with --check fail if it is stale."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if the artifact is stale")
    args = parser.parse_args(argv)
    os.chdir(ROOT)

    files = sorted(SPEC_BNF.glob("*.kebnf"))
    if not files:
        # Inert before vendoring, like every other check: a fresh clone has nothing
        # pinned yet and must not fail its own gate.
        print("no specification BNF vendored — run python3.11 scripts/vendor_sync.py")
        return 0

    artifact = build_artifact(files, extract(files))
    if args.check:
        return _check(artifact)
    _write(artifact)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
