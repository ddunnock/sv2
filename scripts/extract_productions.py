# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Deterministic extraction from the pinned Xtext grammars.

Produces three artifacts, all regenerable byte-identically from pinned inputs:

    .claude/state/grammar/productions.json   -> drives the coverage gate
    .claude/state/grammar/keywords.json      -> drives the lexer
    .claude/state/grammar/metaclass-map.json -> drives phase-2 desugaring targets

What this deliberately does NOT extract: rule bodies. See docs/DERIVATION.md —
the Xtext encodes the pilot parser's LL limitations as if they were language
rules, and a hand-written recursive-descent parser must not inherit them.

    python3.12 scripts/extract_productions.py           write the artifacts
    python3.12 scripts/extract_productions.py --check   fail if any is stale
"""

from __future__ import annotations

import argparse
import json
import os
import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
PILOT = Path("vendor/pilot")
OUT_DIR = Path(".claude/state/grammar")
GENERATED_BY = "scripts/extract_productions.py"

# Xtext meta-syntax we need:
#   Name returns Meta::Class :  ...  ;
#   fragment Name returns Meta::Class : ... ;
#   enum Name returns Meta::Kind : ... ;
#   terminal NAME : ... ;
#   Name : ... ;                      (datatype rule, no returns)
# Two forms, because Xtext permits both and matching only the first silently
# under-counts the inventory — which would make the coverage gate report a
# smaller language than the grammar actually declares.
#   Header form:  Name returns Meta::Class :        (body on following lines)
#   Inline form:  Name : NAME ;                     (body on the same line)
_HEAD = (
    r"^(?P<kind>fragment\s+|enum\s+|terminal\s+)?"
    r"(?P<name>[A-Za-z_][A-Za-z0-9_]*)"
    r"(?:\s+returns\s+(?P<returns>[A-Za-z_][A-Za-z0-9_:]*))?"
    r"\s*:"
)
RULE = re.compile(_HEAD + r"\s*$")
RULE_INLINE = re.compile(_HEAD + r"\s*\S")
# The same head anchored to a line start, for splitting the file into rule blocks.
BLOCK_HEAD = re.compile(_HEAD, re.M)
GRAMMAR_DECL = re.compile(r"^\s*grammar\s+(?P<name>\S+)(?:\s+with\s+(?P<with>\S+))?")
IMPORT = re.compile(r'^\s*import\s+"(?P<uri>[^"]+)"(?:\s+as\s+(?P<alias>\S+))?')
LITERAL = re.compile(r"'((?:[^'\\]|\\.)*)'")
WORD = re.compile(r"[A-Za-z][A-Za-z0-9_]*")
KINDS = ("rule", "fragment", "enum", "terminal")


def _copy_quoted(text: str, start: int, out: list[str]) -> int:
    """Copy a quoted literal verbatim, honoring backslash escapes. Returns the next index."""
    quote, n = text[start], len(text)
    out.append(quote)
    i = start + 1
    while i < n:
        c = text[i]
        out.append(c)
        if c == "\\" and i + 1 < n:
            out.append(text[i + 1])
            i += 2
            continue
        i += 1
        if c == quote:
            break
    return i


def _skip_line_comment(text: str, start: int, out: list[str]) -> int:
    newline = text.find("\n", start + 2)
    if newline == -1:
        return len(text)
    out.append("\n")
    return newline + 1


def _skip_block_comment(text: str, start: int, out: list[str]) -> int:
    i, n = start + 2, len(text)
    while i < n:
        if text[i] == "*" and i + 1 < n and text[i + 1] == "/":
            return i + 2
        if text[i] == "\n":
            out.append("\n")  # keep line numbering stable
        i += 1
    return i


def strip_comments(text: str) -> str:
    """Remove Xtext comments without touching string literals.

    A naive regex is wrong here and fails on this exact grammar:
      - '//' appears inside every metamodel import URI (http://, https://)
      - '/*' and '*/' appear as quoted literals in the comment terminal rules
    Both would be swallowed, silently emptying the import list and the token
    set. Scan instead, tracking quote state.
    """
    out: list[str] = []
    i, n = 0, len(text)
    while i < n:
        c = text[i]
        nxt = text[i + 1] if i + 1 < n else ""
        if c in "'\"":
            i = _copy_quoted(text, i, out)
        elif c == "/" and nxt == "/":
            i = _skip_line_comment(text, i, out)
        elif c == "/" and nxt == "*":
            i = _skip_block_comment(text, i, out)
        else:
            out.append(c)
            i += 1
    return "".join(out)


@dataclass(slots=True)
class Inventory:
    """Everything extracted across the pinned grammar files."""

    productions: list[dict[str, str | None]] = field(default_factory=list)
    keywords: set[str] = field(default_factory=set)
    metamap: dict[str, list[str]] = field(default_factory=dict)
    grammars: list[dict[str, Any]] = field(default_factory=list)


def _grammar_header(file_name: str, body: str) -> dict[str, Any]:
    info: dict[str, Any] = {"file": file_name, "grammar": None, "extends": None, "imports": []}
    for line in body.splitlines():
        decl = GRAMMAR_DECL.match(line)
        if decl and info["grammar"] is None:
            info["grammar"], info["extends"] = decl.group("name"), decl.group("with")
        imp = IMPORT.match(line)
        if imp:
            info["imports"].append({"uri": imp.group("uri"), "alias": imp.group("alias")})
    return info


def _collect_rules(file_name: str, body: str, inv: Inventory) -> None:
    # Rule headers: Xtext puts the ':' at the end of the declaration line.
    for line in body.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith(("grammar", "import")):
            continue
        match = RULE.match(stripped) or RULE_INLINE.match(stripped)
        if not match:
            continue
        kind = (match.group("kind") or "rule").strip()
        name = match.group("name")
        returns = match.group("returns")
        inv.productions.append({"name": name, "kind": kind, "returns": returns, "file": file_name})
        if returns:
            inv.metamap.setdefault(returns, []).append(name)


def _rule_blocks(body: str) -> list[tuple[str, str]]:
    """(kind, block text) for each rule, a block running from its head to the next."""
    marks = list(BLOCK_HEAD.finditer(body))
    blocks: list[tuple[str, str]] = []
    for i, mark in enumerate(marks):
        end = marks[i + 1].start() if i + 1 < len(marks) else len(body)
        blocks.append(((mark.group("kind") or "rule").strip(), body[mark.end() : end]))
    return blocks


def _collect_literals(body: str, inv: Inventory) -> None:
    r"""Every quoted literal in a parser rule is a keyword or an operator.

    Terminal rules are excluded. A terminal defines lexical structure with character
    ranges and escapes — `'0'..'9'`, `'a'..'z'`, `'\\' ('b' | 't' | 'n' | 'f' | 'r')` —
    so scanning one yields the endpoints of ranges and the letters of escape
    sequences. Including them put A, E, Z, a, b, e, f, n, r, t and z in the keyword
    list and multi-line fragments of the grammar file in the operator list, none of
    which is a token of the language. keywords.json drives the lexer, so that is not
    cosmetic: it is a wrong token set.

    The lexical productions terminals implement are named by the specification
    (NAME, DECIMAL_VALUE, ESCAPE_SEQUENCE and the rest at KerML 8.2.2), and they are
    implemented from those clauses rather than from this inventory.
    """
    for kind, block in _rule_blocks(body):
        if kind == "terminal":
            continue
        for raw in LITERAL.findall(block):
            literal = raw.replace("\\'", "'").replace("\\\\", "\\")
            if literal:
                inv.keywords.add(literal)


def extract(files: list[Path]) -> Inventory:
    """Parse every grammar file into one inventory."""
    inv = Inventory()
    for f in files:
        body = strip_comments(f.read_text(encoding="utf-8", errors="replace"))
        inv.grammars.append(_grammar_header(f.name, body))
        _collect_rules(f.name, body, inv)
        _collect_literals(body, inv)
    inv.productions.sort(key=lambda p: (p["file"] or "", p["name"] or ""))
    return inv


def build_artifacts(files: list[Path], inv: Inventory) -> dict[str, dict[str, Any]]:
    """The three JSON documents, keyed by file name."""
    tokens = sorted(inv.keywords)
    words = [k for k in tokens if WORD.fullmatch(k)]
    symbols = [k for k in tokens if not WORD.fullmatch(k)]
    counts: dict[str, int] = {"total": len(inv.productions)}
    counts.update({k: sum(1 for p in inv.productions if p["kind"] == k) for k in KINDS})
    return {
        "productions.json": {
            "_generated_by": GENERATED_BY,
            "_source": [f.name for f in files],
            "grammars": inv.grammars,
            "counts": counts,
            "productions": inv.productions,
        },
        "keywords.json": {
            "_generated_by": GENERATED_BY,
            "counts": {"keywords": len(words), "operators": len(symbols)},
            "keywords": words,
            "operators": symbols,
        },
        "metaclass-map.json": {
            "_generated_by": GENERATED_BY,
            "_note": "Metaclass -> the productions that construct it. Phase-2 desugaring targets.",
            "counts": {"metaclasses": len(inv.metamap)},
            "map": {k: sorted(set(v)) for k, v in sorted(inv.metamap.items())},
        },
    }


def _render(data: dict[str, Any]) -> str:
    return json.dumps(data, indent=2) + "\n"


def _is_current(path: Path, data: dict[str, Any]) -> bool:
    return path.exists() and path.read_text() == _render(data)


def _check(artifacts: dict[str, dict[str, Any]], inv: Inventory) -> int:
    stale = [name for name, data in artifacts.items() if not _is_current(OUT_DIR / name, data)]
    if stale:
        print("derived artifacts are stale: " + ", ".join(stale))
        print("run python3.12 scripts/extract_productions.py")
        return 1
    print(
        f"derived artifacts current ({len(inv.productions)} productions, "
        f"{len(inv.keywords)} tokens)"
    )
    return 0


def _write(artifacts: dict[str, dict[str, Any]]) -> None:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    for name, data in artifacts.items():
        (OUT_DIR / name).write_text(_render(data))
    kinds = artifacts["productions.json"]["counts"]
    tokens = artifacts["keywords.json"]["counts"]
    metaclasses = artifacts["metaclass-map.json"]["counts"]["metaclasses"]
    print(
        f"wrote grammar/productions.json   {kinds['total']} productions "
        f"({kinds['rule']} rule, {kinds['fragment']} fragment, {kinds['enum']} enum, "
        f"{kinds['terminal']} terminal)"
    )
    print(
        f"wrote grammar/keywords.json      {tokens['keywords']} keywords, "
        f"{tokens['operators']} operators"
    )
    print(f"wrote grammar/metaclass-map.json {metaclasses} metaclasses")


def main(argv: list[str] | None = None) -> int:
    """Write the derived artifacts, or with --check fail if any is stale."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if any artifact is stale")
    args = parser.parse_args(argv)
    os.chdir(ROOT)

    files = sorted(PILOT.glob("*.xtext"))
    if not files:
        print("no Xtext grammars vendored — run python3.12 scripts/vendor_sync.py")
        return 0

    inv = extract(files)
    artifacts = build_artifacts(files, inv)
    if args.check:
        return _check(artifacts, inv)
    _write(artifacts)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
