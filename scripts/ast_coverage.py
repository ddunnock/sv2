# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Coverage of sv2-ast's typed accessors against the node-kind inventory (ADR-0025).

    implemented    a marker claims a kind                      -- counted
    unimplemented  a kind no marker claims                     -- legitimate
    absent         a marker claims a kind that does not exist  -- DEFECT
    unproved       an implemented kind with no malformed case  -- DEFECT

A claim is a ``// node: Kind`` marker in a file under ``crates/sv2-ast/``. The inventory
is ``.claude/state/node-kinds.json``, written by the generator that owns the list.

    python3.12 scripts/ast_coverage.py           write the report
    python3.12 scripts/ast_coverage.py --check   fail on absent, unproved, or staleness

`unimplemented` is the honest state of a crate being built and is never a failure. A new
crate reads 0 implemented and 463 unimplemented, which is the number, not a verdict.

`unproved` is what makes ADR-0025's central claim true rather than aspirational: "an
accessor lands with all of them or it does not land". Three of the four kinds of evidence
were already mechanical -- the sweep and the property tests run, the marker is counted.
The malformed case was prose only, so an accessor could land without one and the gate
stayed green. A kind is proved by at least one file named ``<kind-in-kebab>-*.sysml`` or
``.kerml`` under ``crates/sv2-ast/tests/malformed/``; ``RootNamespace`` is proved by
``root-namespace-unclosed-body.sysml``. The condition goes after the kind so one kind can
carry several cases, and so the filename says which accessor is on the hook.

This checks the case EXISTS, not that it asserts the right things. No gate can check that;
review does.

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
MALFORMED = Path("crates/sv2-ast/tests/malformed")
REFERENCE = Path(".claude/state/grammar/reference.json")
START = "RootNamespace"

#: A claim. The kind name is a Rust identifier, so the pattern admits nothing else.
MARKER = re.compile(r"//\s*node:\s*([A-Za-z_][A-Za-z0-9_]*)")

#: Where one word ends and the next begins in a node kind. No kind in the inventory
#: carries consecutive capitals, so a boundary is exactly a capital after another
#: character.
BOUNDARY = re.compile(r"(?<!^)(?=[A-Z])")

#: The suffixes a malformed case may carry — the two languages, nothing else.
CASE_SUFFIXES = ("sysml", "kerml")


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


def kebab(kind: str) -> str:
    """The filename stem a node kind's malformed cases carry.

    ``RootNamespace`` -> ``root-namespace``.
    """
    return BOUNDARY.sub("-", kind).lower()


def case_owner(filename: str, stems: dict[str, str]) -> str | None:
    """The one kind a malformed case file belongs to, or None if it names no kind.

    The LONGEST matching stem wins, and that is not a detail. 180 kinds in the inventory
    have a kebab form that prefixes another's -- `package` prefixes `package-body`,
    `type` prefixes `type-body`. Matching every prefix would let `package-body-unclosed`
    prove `Package` as well as `PackageBody`, so `Package` could land with no case of its
    own and the gate would stay green. That is the false green this check exists to stop,
    so the file belongs to `PackageBody` and to nothing else.
    """
    stem = filename.rsplit(".", 1)[0]
    best: str | None = None
    for kind, candidate in stems.items():
        if not stem.startswith(f"{candidate}-"):
            continue
        if best is None or len(candidate) > len(stems[best]):
            best = kind
    return best


def malformed_cases(kinds: list[str], root: Path = MALFORMED) -> dict[str, list[str]]:
    """Every kind with at least one malformed case, mapped to those files, name-sorted.

    A case is ``<kind-in-kebab>-<condition>.sysml`` or ``.kerml``. The condition goes
    last so one kind can carry several cases, and so the filename says which accessor is
    on the hook. A file naming no kind in the inventory is not an error here: the gate
    asks whether a kind has a case, never whether a file has a kind.
    """
    if not root.is_dir():
        return {}
    stems = {kind: kebab(kind) for kind in kinds}
    found: dict[str, list[str]] = {}
    for path in sorted(root.iterdir()):
        if not path.is_file() or path.suffix.lstrip(".") not in CASE_SUFFIXES:
            continue
        owner = case_owner(path.name, stems)
        if owner is not None:
            found.setdefault(owner, []).append(path.name)
    return found


def build_report(
    declared_kinds: set[str] | list[str],
    implemented: list[str],
    unimplemented: list[str],
    absent: list[str],
    unproved: list[str],
) -> dict[str, object]:
    """The report as it should be on disk for this inventory and these markers."""
    total = len(set(declared_kinds))
    percent = 100.0 * len(implemented) / total if total else 0.0
    return {
        "_generated_by": "scripts/ast_coverage.py",
        "_note": (
            "Typed accessors over CST node kinds (ADR-0025). `unimplemented` is a tracked "
            "state, not a failure. `absent` is a defect: a marker claiming a kind that "
            "does not exist. `unproved` is a defect: an implemented kind with no malformed "
            "case under crates/sv2-ast/tests/malformed/. The order is the worklist order, "
            "never a verdict."
        ),
        "declared": total,
        "implemented": len(implemented),
        "unimplemented": len(unimplemented),
        "absent": len(absent),
        "unproved": len(unproved),
        "percent": round(percent, 1),
        "implemented_kinds": implemented,
        "unimplemented_kinds": unimplemented,
        "absent_markers": absent,
        "unproved_kinds": unproved,
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


def _references(rule: object) -> list[str]:
    """Every production a rule names, at any depth."""
    if isinstance(rule, dict):
        if rule.get("k") == "ref" and isinstance(rule.get("name"), str):
            return [rule["name"]]
        return [name for value in rule.values() for name in _references(value)]
    if isinstance(rule, list):
        return [name for item in rule for name in _references(item)]
    return []


def layer_of(reference: dict[str, object]) -> dict[str, int]:
    """Breadth-first distance from the start symbol, for every production it reaches.

    The frozen grammar is under `grammar` and keyed by UNIT, so one name can arrive twice
    as `Name@kerml` and `Name@sysml` (ADR-0015). The walk is over base names: a name's
    rules are every unit sharing it, and a variant does not get a layer of its own. A
    production the start symbol cannot reach has none either, and is absent from the map.
    Marking on enqueue means a cycle terminates.
    """
    grammar = reference.get("grammar")
    if not isinstance(grammar, dict):
        return {}
    rules: dict[str, list[object]] = {}
    for unit, body in grammar.items():
        if isinstance(body, dict):
            rules.setdefault(unit.split("@", 1)[0], []).append(body.get("rule"))
    if START not in rules:
        return {}
    depth = {START: 0}
    queue = [START]
    while queue:
        name = queue.pop(0)
        for rule in rules.get(name, []):
            for child in _references(rule):
                if child not in depth and child in rules:
                    depth[child] = depth[name] + 1
                    queue.append(child)
    return depth


def worklist(kinds: list[str], layers: dict[str, int]) -> list[str]:
    """`kinds` ordered root downward, ties and unreached kinds keeping declaration order.

    ADR-0025: this decides how the report prints and nothing else. The production-to-kind
    mapping is by name and is approximate, so it may not gate.
    """
    order = {kind: index for index, kind in enumerate(kinds)}
    far = max(layers.values(), default=0) + 1
    return sorted(kinds, key=lambda kind: (layers.get(kind, far), order[kind]))


def measure() -> dict[str, object]:
    """The report for the tree as it stands, worklist ordered root downward."""
    kinds = declared()
    layers = layer_of(json.loads(REFERENCE.read_text())) if REFERENCE.is_file() else {}
    ordered = worklist(kinds, layers)
    claimed = claims()
    implemented = [k for k in ordered if k in claimed]
    unimplemented = [k for k in ordered if k not in claimed]
    absent = sorted(k for k in claimed if k not in set(kinds))
    proved = malformed_cases(kinds)
    unproved = [k for k in implemented if k not in proved]
    return build_report(kinds, implemented, unimplemented, absent, unproved)


def main(argv: list[str] | None = None) -> int:
    """Write the coverage report, or with --check fail on absent or staleness."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check", action="store_true", help="fail on absent, unproved, or a stale report"
    )
    args = parser.parse_args(argv)
    os.chdir(ROOT)

    report = measure()
    absent = report["absent_markers"]
    unproved = report["unproved_kinds"]
    if args.check:
        if absent:
            print("markers claim node kinds that do not exist:")
            for kind in absent if isinstance(absent, list) else []:
                print(f"  {kind}")
            print("A kind is added to the parser's NODES, then regenerated. `absent` is a defect.")
            return 1
        if unproved:
            print("accessors with no malformed case (ADR-0025):")
            for kind in unproved if isinstance(unproved, list) else []:
                print(f"  {kind} — expected {MALFORMED}/{kebab(kind)}-<condition>.sysml")
            print(
                "An accessor lands with all four kinds of evidence or it does not land. "
                "Write the case; do not widen this check."
            )
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
