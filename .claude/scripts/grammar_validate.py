# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Phase 5. Runs the derived grammar against the corpus with an INDEPENDENT Earley recognizer.

Positive corpus must parse; negative cases must not.

This validates the grammar without involving the Rust parser at all. When both
later agree on the same corpus, that is two implementations agreeing rather
than one checking itself.

Each file is recognized by its own language's grammar, chosen by suffix: a .kerml
file against the KerML grammar, a .sysml file against the SysML one (ADR-0014).
Checking both against one merged grammar would let a SysML construct in a KerML
file pass, and a positive-only corpus would never notice. For the same reason each
file is lexed with its own language's reserved words: KerML 8.2.2.6 and SysML
8.2.2.1.2 reserve different sets, and a SysML word reserved in a KerML file turns
a valid name into a keyword.

    python3.12 .claude/scripts/grammar_validate.py
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
from typing import TYPE_CHECKING

from _earley import make_lexer, recognize
from _grammar import (
    GRAMMAR,
    SCOPE_OF_SUFFIX,
    SCOPES,
    START,
    grammar_view,
    load_units,
    normalize,
    pinned_tokens,
    reserved_keywords,
)
from _state import REPO_ROOT

if TYPE_CHECKING:
    from collections.abc import Callable

    from _earley import Productions, Token
    from _state import Json

NOTE = (
    "Independent Earley oracle over the derived grammar. Positive-only acceptance is a weak "
    "claim; the negative set is what makes this mean anything."
)


def model_files(*roots: str) -> list[Path]:
    """Every .sysml and .kerml file under the roots that exist, sorted per root."""
    out: list[Path] = []
    for root in roots:
        p = Path(root)
        if p.exists():
            out += [f for f in sorted(p.rglob("*")) if f.suffix in (".sysml", ".kerml")]
    return out


def _accepts(prods: Productions, lex: Callable[[str], list[Token]], f: Path) -> tuple[bool, str]:
    try:
        return recognize(prods, START, lex(f.read_text(errors="replace")))
    except (ValueError, OSError) as exc:
        return False, f"lex error: {exc}"


def grammars_by_scope(units: dict[str, Json]) -> dict[str, Productions]:
    """Each language's normalized grammar, for the languages whose start symbol exists."""
    grammars: dict[str, Productions] = {}
    for scope in SCOPES:
        view = {n: u for n, u in grammar_view(units, scope).items() if u.get("rule")}
        if START in view:
            grammars[scope] = normalize(view)
    return grammars


def lexers_by_scope(scopes: list[str]) -> dict[str, Callable[[str], list[Token]]]:
    """A lexer per language, for each language whose reserved words are pinned."""
    _, operators = pinned_tokens()
    return {s: make_lexer(words, operators) for s in scopes if (words := reserved_keywords(s))}


def sweep(
    files: list[Path],
    grammars: dict[str, Productions],
    lexers: dict[str, Callable[[str], list[Token]]],
) -> tuple[list[str], list[dict[str, str]], int]:
    """(accepted, rejected with the reason, skipped) — each file by its own language."""
    accepted: list[str] = []
    rejected: list[dict[str, str]] = []
    skipped = 0
    for f in files:
        scope = SCOPE_OF_SUFFIX.get(f.suffix, "")
        prods, lex = grammars.get(scope), lexers.get(scope)
        if prods is None or lex is None:
            skipped += 1
            continue
        ok, why = _accepts(prods, lex, f)
        if ok:
            accepted.append(str(f))
        else:
            rejected.append({"file": str(f), "why": why})
    return accepted, rejected, skipped


def main(argv: list[str] | None = None) -> int:
    """Run the derived grammar over the corpus with the independent recognizer."""
    argparse.ArgumentParser(description=__doc__).parse_args(argv)
    os.chdir(REPO_ROOT)

    grammars = grammars_by_scope(load_units())
    if not grammars:
        print(f"start symbol {START} not derived in either language yet — validation inert")
        return 0
    if inert := [s for s in SCOPES if s not in grammars]:
        # One language checked is not a clean oracle: its files are simply not looked at.
        print(f"    note: {START} not derived for {', '.join(inert)} — those files are skipped")

    lexers = lexers_by_scope(sorted(grammars))
    if unlexed := [s for s in sorted(grammars) if s not in lexers]:
        # No fallback to the union of both languages' words: it is wrong for each.
        print(f"    note: no RESERVED_KEYWORD pinned for {', '.join(unlexed)} — files skipped")
    positive = model_files("vendor/corpus", "tests/corpus")
    negative = [f for f in model_files("tests/rejection") if "known-permissive" not in f.parts]
    accepted, missed, skipped_positive = sweep(positive, grammars, lexers)
    # For the negative set the directions invert: accepting a file is the failure.
    leaked, rejected, skipped_negative = sweep(negative, grammars, lexers)
    caught = [r["file"] for r in rejected]
    skipped = skipped_positive + skipped_negative

    report = {
        "_generated_by": ".claude/scripts/grammar_validate.py",
        "_note": NOTE,
        "start_symbol": START,
        "languages_checked": sorted(grammars),
        "skipped": skipped,
        "productions_normalized": {s: len(p) for s, p in sorted(grammars.items())},
        "accepted": len(accepted),
        "missed": len(missed),
        "caught": len(caught),
        "leaked": len(leaked),
        "missed_detail": missed[:40],
        "leaked_detail": leaked[:40],
    }
    (GRAMMAR / "validation.json").write_text(json.dumps(report, indent=2) + "\n")
    print(
        f"oracle: {len(accepted)} accepted / {len(missed)} missed | "
        f"{len(caught)} caught / {len(leaked)} leaked"
    )
    for miss in missed[:8]:
        print(f"    MISSED {miss['file']}: {miss['why']}")
    for leak in leaked[:8]:
        print(f"    LEAKED {leak}")
    if not negative:
        print("    no negative corpus — the acceptance claim is positive-only and therefore weak")
    if skipped:
        print(f"    {skipped} file(s) skipped: their language's grammar has no start symbol yet")
    return 0 if (not missed and not leaked and not skipped and negative) else 1


if __name__ == "__main__":
    raise SystemExit(main())
