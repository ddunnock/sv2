# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Coverage of sv2-ast's typed accessors against the node-kind inventory (ADR-0025).

    implemented    a marker claims a kind                      -- counted
    unimplemented  a kind no marker claims                     -- legitimate
    absent         a marker claims a kind that does not exist  -- DEFECT

A claim is a ``// node: Kind`` marker in a file under ``crates/sv2-ast/``. The inventory
is ``.claude/state/node-kinds.json``, written by the generator that owns the list.

    python3.12 scripts/ast_coverage.py           write the report
    python3.12 scripts/ast_coverage.py --check   fail on any absent, or on a stale report

`unimplemented` is the honest state of a crate being built and is never a failure. A new
crate reads 0 implemented and 464 unimplemented, which is the number, not a verdict.

Network: none.
"""

from __future__ import annotations

import argparse
import json
import os
import re
from pathlib import Path

from _derived import write_derived

ROOT = Path(__file__).resolve().parents[1]
INVENTORY = Path(".claude/state/node-kinds.json")
REPORT = Path(".claude/state/ast-coverage.json")
SOURCE = Path("crates/sv2-ast")

#: A claim. The kind name is a Rust identifier, so the pattern admits nothing else.
MARKER = re.compile(r"//\s*node:\s*([A-Za-z_][A-Za-z0-9_]*)")


def declared(path: Path = INVENTORY) -> list[str]:
    """Every node kind the inventory lists, in declaration order."""
    if not path.is_file():
        return []
    nodes: list[str] = json.loads(path.read_text())["nodes"]
    return nodes


def claims(root: Path = SOURCE) -> dict[str, list[str]]:
    """Every kind claimed under `root`, mapped to the files claiming it, path-sorted."""
    found: dict[str, list[str]] = {}
    for file in sorted(root.rglob("*.rs")):
        relative = file.relative_to(root).as_posix()
        for kind in MARKER.findall(file.read_text(errors="replace")):
            found.setdefault(kind, []).append(relative)
    return found


def build_report(
    declared_kinds: set[str] | list[str],
    implemented: list[str],
    unimplemented: list[str],
    absent: list[str],
) -> dict[str, object]:
    """The report as it should be on disk for this inventory and these markers."""
    total = len(set(declared_kinds))
    percent = 100.0 * len(implemented) / total if total else 0.0
    return {
        "_generated_by": "scripts/ast_coverage.py",
        "_note": (
            "Typed accessors over CST node kinds (ADR-0025). `unimplemented` is a tracked "
            "state, not a failure. `absent` is a defect: a marker claiming a kind that "
            "does not exist. The order is the worklist order, never a verdict."
        ),
        "declared": total,
        "implemented": len(implemented),
        "unimplemented": len(unimplemented),
        "absent": len(absent),
        "percent": round(percent, 1),
        "implemented_kinds": implemented,
        "unimplemented_kinds": unimplemented,
        "absent_markers": absent,
    }


def is_stale(report: dict[str, object]) -> bool:
    """Whether the report on disk disagrees with the one just built.

    A marker added without regenerating would otherwise leave the report publishing the
    previous count while the gate stayed green — the hole bnf_coverage.py closed the
    same way.
    """
    if not REPORT.is_file():
        return True
    try:
        on_disk: object = json.loads(REPORT.read_text())
    except json.JSONDecodeError:
        return True
    return on_disk != report


def measure() -> dict[str, object]:
    """The report for the tree as it stands."""
    kinds = declared()
    claimed = claims()
    implemented = [k for k in kinds if k in claimed]
    unimplemented = [k for k in kinds if k not in claimed]
    absent = sorted(k for k in claimed if k not in set(kinds))
    return build_report(kinds, implemented, unimplemented, absent)


def main(argv: list[str] | None = None) -> int:
    """Write the coverage report, or with --check fail on absent or staleness."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail on absent or a stale report")
    args = parser.parse_args(argv)
    os.chdir(ROOT)

    report = measure()
    absent = report["absent_markers"]
    if args.check:
        if absent:
            print("markers claim node kinds that do not exist:")
            for kind in absent if isinstance(absent, list) else []:
                print(f"  {kind}")
            print("A kind is added to the parser's NODES, then regenerated. `absent` is a defect.")
            return 1
        if is_stale(report):
            print(f"{REPORT} is stale — run python3.12 scripts/ast_coverage.py")
            return 1
        print(
            f"ast coverage current ({report['implemented']}/{report['declared']} node kinds, "
            f"{report['unimplemented']} unimplemented)"
        )
        return 0

    REPORT.parent.mkdir(parents=True, exist_ok=True)
    write_derived(REPORT, json.dumps(report, indent=2) + "\n")
    print(f"wrote {REPORT} ({report['implemented']}/{report['declared']} node kinds)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
