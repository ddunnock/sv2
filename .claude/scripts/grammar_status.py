# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Summarise grammar derivation: unit statuses, the oracle result, and the frozen grammar.

python3.11 .claude/scripts/grammar_status.py
"""

from __future__ import annotations

import argparse
import os
from collections import Counter

from _grammar import GRAMMAR, load_units
from _state import REPO_ROOT, load_json

STATUSES = ("pending", "derived", "verified", "conflict", "retired")


def main(argv: list[str] | None = None) -> int:
    """Print the derivation status: unit counts, the oracle result, the frozen grammar."""
    argparse.ArgumentParser(description=__doc__).parse_args(argv)
    os.chdir(REPO_ROOT)

    units = load_units()
    if not units:
        print(
            "no units — run .claude/scripts/grammar-preflight.sh"
            " then python3.11 .claude/scripts/grammar_plan.py"
        )
        return 0

    counts = Counter(u["status"] for u in units.values())
    total, verified = len(units), counts["verified"]
    print(f"grammar derivation: {verified}/{total} verified ({100.0 * verified / total:.1f}%)")
    for status in STATUSES:
        if counts[status]:
            print(f"  {status:9s} {counts[status]}")

    oracle = load_json(GRAMMAR / "validation.json")
    if oracle:
        print(
            f"oracle: {oracle['accepted']} accepted / {oracle['missed']} missed | "
            f"{oracle['caught']} caught / {oracle['leaked']} leaked"
        )
    ref = load_json(GRAMMAR / "reference.json")
    if ref:
        print(
            f"frozen: {ref['productions']} productions @ {ref['grammar_sha256'][:16]} "
            f"({ref['frozen_utc']})"
        )
    conflicts = [n for n, u in units.items() if u["status"] == "conflict"]
    if conflicts:
        print("conflicts needing adjudication: " + ", ".join(sorted(conflicts)[:10]))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
