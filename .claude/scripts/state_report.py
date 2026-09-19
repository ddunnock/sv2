# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Human-readable rendering of the JSON state. Prints to stdout; writes nothing.

Deliberately not a generated markdown file: a second stored rendering is a
second thing that goes stale. JSON is the only stored form.

    python3.12 .claude/scripts/state_report.py
"""

from __future__ import annotations

import argparse
import os
from typing import TYPE_CHECKING

from _state import REPO_ROOT, STATE, load_json

if TYPE_CHECKING:
    from _state import Json


def _print_generated(generated: Json) -> None:
    print(f"regenerated  {generated.get('regenerated_utc', 'never')}")
    gates = generated.get("gates", {})
    if gates:
        failing = [k for k, v in gates.items() if v == "fail"]
        passing = sum(1 for v in gates.values() if v == "pass")
        line = f"gates        {passing}/{len(gates)} pass"
        if failing:
            line += f"   FAILING: {', '.join(failing)}"
        print(line)
    c = generated.get("coverage", {})
    print(
        f"coverage     {c.get('implemented', 0)}/{c.get('declared', 0)} implemented "
        f"({c.get('percent', 0)}%), {c.get('unimplemented', 0)} unimplemented, "
        f"{c.get('absent', 0)} absent"
    )
    t = generated.get("tests", {})
    print(
        f"tests        {t.get('unit', 0)} unit, {t.get('snapshots', 0)} snapshots, "
        f"{t.get('corpus_files', 0)} corpus, {t.get('negative_cases', 0)} negative"
    )
    p = generated.get("pins", {})
    print(
        f"pins         OMG {p.get('omg_namespace', 'unset')} "
        f"| Pilot {str(p.get('pilot_revision', 'unset'))[:12]} "
        f"| vendor {p.get('vendor_pinned', 0)}/{p.get('vendor_total', 0)}"
    )


def _print_intent(authored: Json) -> None:
    print(f"\nOBJECTIVE\n  {authored.get('objective', '—')}")
    print(f"\nNEXT STEP\n  {authored.get('next_step', '—')}")
    pending = authored.get("pending_decisions", [])
    if pending:
        print("\nPENDING DECISIONS")
        for d in pending:
            print(f"  [{d['id']}] {d['question']}")
            if d.get("blocks"):
                print(f"        blocks: {', '.join(d['blocks'])}")


def _print_differences() -> None:
    diff = load_json(".claude/state/grammar-diff.json", {})
    if diff.get("unreviewed"):
        print(f"\nUNREVIEWED GRAMMAR DIFFERENCES  ({len(diff['unreviewed'])})")
        for name in diff["unreviewed"][:10]:
            print(f"  {name}")


def _print_log(authored: Json) -> None:
    log = authored.get("log", [])
    if log:
        print("\nRECENT SESSIONS")
        for entry in log[-5:]:
            print(f"  {entry['date']}  [{entry.get('gate', '?')}]  {entry['summary']}")


def main(argv: list[str] | None = None) -> int:
    """Print the state file as a report for a human. Writes nothing."""
    argparse.ArgumentParser(description=__doc__).parse_args(argv)
    os.chdir(REPO_ROOT)

    state = load_json(STATE, {})
    authored = state.get("authored", {})
    print("sv2 — state\n" + "=" * 60)
    _print_generated(state.get("generated", {}))
    _print_intent(authored)
    _print_differences()
    _print_log(authored)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
