# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Phase 5. Runs the derived grammar against the corpus with an INDEPENDENT Earley recognizer.

Positive corpus must parse; negative cases must not.

This validates the grammar without involving the Rust parser at all. When both
later agree on the same corpus, that is two implementations agreeing rather
than one checking itself.

    python3.11 .claude/scripts/grammar_validate.py
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
from typing import TYPE_CHECKING

from _earley import make_lexer, recognize
from _grammar import GRAMMAR, START, load_units, normalize, pinned_tokens
from _state import REPO_ROOT

if TYPE_CHECKING:
    from collections.abc import Callable

    from _earley import Productions, Token

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


def main(argv: list[str] | None = None) -> int:
    """Run the derived grammar over the corpus with the independent recognizer."""
    argparse.ArgumentParser(description=__doc__).parse_args(argv)
    os.chdir(REPO_ROOT)

    units = {n: u for n, u in load_units().items() if u.get("rule") and u["status"] != "retired"}
    if not units:
        print("no derived rules yet — validation inert")
        return 0
    if START not in units:
        print(f"start symbol {START} not derived yet — validation inert")
        return 0

    lex = make_lexer(*pinned_tokens())
    prods = normalize(units)

    positive = model_files("vendor/corpus", "tests/corpus")
    negative = [f for f in model_files("tests/rejection") if "known-permissive" not in f.parts]

    accepted: list[str] = []
    missed: list[dict[str, str]] = []
    for f in positive:
        ok, why = _accepts(prods, lex, f)
        if ok:
            accepted.append(str(f))
        else:
            missed.append({"file": str(f), "why": why})
    caught: list[str] = []
    leaked: list[str] = []
    for f in negative:
        leaks, _ = _accepts(prods, lex, f)
        if leaks:
            leaked.append(str(f))
        else:
            caught.append(str(f))

    report = {
        "_generated_by": ".claude/scripts/grammar_validate.py",
        "_note": NOTE,
        "start_symbol": START,
        "productions_normalized": len(prods),
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
    return 0 if (not missed and not leaked and negative) else 1


if __name__ == "__main__":
    raise SystemExit(main())
