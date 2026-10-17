# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Coverage of sv2-ast's typed accessors against the node-kind inventory (ADR-0025).

    implemented    a marker claims a kind                      -- counted
    unimplemented  a kind no marker claims                     -- legitimate
    absent         a marker claims a kind that does not exist  -- DEFECT
    unproved       an implemented kind with no malformed case  -- DEFECT
    unswept        an implemented kind the sweep never calls   -- DEFECT
    stray          a malformed case that names no kind         -- DEFECT

A claim is a ``// node: Kind`` marker in a file under ``crates/sv2-ast/src/``. Only
``src/``: a marker in a test file would otherwise count as implemented with no accessor
shipped, which inflates coverage, and ``src/`` is already the scope
``scripts/check_ast_leaks.py`` enforces its rules over. The two agreeing is the point --
one tool counting what the other does not inspect is how a number stops meaning anything.

The inventory is ``.claude/state/node-kinds.json``, written by the generator that owns the
list.

    python3.12 scripts/ast_coverage.py           write the report
    python3.12 scripts/ast_coverage.py --check   fail on any defect, or on staleness

`unimplemented` is the honest state of a crate being built and is never a failure. A new
crate reads 0 implemented and 463 unimplemented, which is the number, not a verdict.

`unproved` and `unswept` are what make ADR-0025's claim mechanical rather than
aspirational: "an accessor lands with all of them or it does not land". The marker was
already counted here; the malformed case and the sweep assertion were prose only, so an
accessor could land without either and the gate stayed green.

`unswept` closes the hole ADR-0025 named and left open: ``tests/sweep.rs`` calls each
assertion function by name, so an accessor could carry its marker and its malformed case
while never being exercised over a single tree -- "green because it has nothing to
check". A kind is swept when the sweep both DEFINES ``assert_<kind_in_snake>_accessor``
and CALLS it; defining one and forgetting the call is the whole failure, so requiring the
definition alone would re-open the hole one level down. See `swept`.

A kind is proved by at least one non-empty file under
``crates/sv2-ast/tests/malformed/<kind-in-kebab>/``; ``RootNamespace`` is proved by
``root-namespace/unclosed-body.sysml``. The kind is the directory, not a filename prefix,
because a filename cannot carry kind and condition unambiguously -- see
`malformed_cases`.

This checks the case EXISTS and is not empty, not that it asserts the right things. No
gate can check that; review does.

Network: none.
"""

from __future__ import annotations

import argparse
import json
import os
import re
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Callable

from _derived import write_derived

ROOT = Path(__file__).resolve().parents[1]
INVENTORY = Path(".claude/state/node-kinds.json")
REPORT = Path(".claude/state/ast-coverage.json")
SOURCE = Path("crates/sv2-ast")
#: Markers count only here, and `check_ast_leaks.py` enforces only here. Relative to
#: SOURCE, so the report still prints the path a reader can open.
SRC = Path("src")
SWEEP = Path("crates/sv2-ast/tests/sweep.rs")
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
    """Every kind claimed under ``root/src``, mapped to the files claiming it, path-sorted.

    Only ``src/``. A marker anywhere under the crate used to count, so one in a test file
    read as implemented although no accessor shipped -- and `check_ast_leaks.py` would
    never have inspected it, because its rules run over ``src/`` alone. Paths stay
    relative to `root`, so the report prints ``src/ns.rs`` and not ``ns.rs``.
    """
    found: dict[str, list[str]] = {}
    for file in sorted((root / SRC).rglob("*.rs")):
        relative = file.relative_to(root).as_posix()
        for kind in MARKER.findall(file.read_text(errors="replace")):
            found.setdefault(kind, []).append(relative)
    return found


def kebab(kind: str) -> str:
    """The filename stem a node kind's malformed cases carry.

    ``RootNamespace`` -> ``root-namespace``.
    """
    return BOUNDARY.sub("-", kind).lower()


def snake(kind: str) -> str:
    """The Rust identifier stem a node kind's sweep assertion carries.

    ``RootNamespace`` -> ``root_namespace``, so the assertion is
    ``assert_root_namespace_accessor``.
    """
    return BOUNDARY.sub("_", kind).lower()


def _code(text: str) -> str:
    """``text`` with line comments stripped, so a prose mention is not a call site.

    A doc comment naming ``assert_x_accessor()`` would otherwise satisfy `swept` for a
    kind nothing calls -- the documentation standing in for the deed. This is a text
    scan, not a Rust parser: it would also cut a ``//`` inside a string literal, which
    `sweep.rs` does not contain, and whether an assertion asserts anything worth
    asserting remains a review question either way.
    """
    return "\n".join(line.split("//", 1)[0] for line in text.splitlines())


def swept(kinds: list[str], path: Path = SWEEP) -> set[str]:
    """Every kind whose accessor the sweep both defines an assertion for and calls.

    Two conditions, because either alone is satisfiable without exercising anything. A
    definition with no call site is the exact failure ADR-0025 named and left to
    discipline: the function compiles, the suite is green, and no tree ever reaches the
    accessor. A call with no definition does not compile, so it is only checked here for
    symmetry.
    """
    if not path.is_file():
        return set()
    code = _code(path.read_text(errors="replace"))
    found: set[str] = set()
    for kind in kinds:
        stem = snake(kind)
        defined = f"fn assert_{stem}_accessor" in code
        called = re.search(rf"(?<!fn )\bassert_{re.escape(stem)}_accessor\s*\(", code)
        if defined and called:
            found.add(kind)
    return found


def malformed_cases(kinds: list[str], root: Path = MALFORMED) -> dict[str, list[str]]:
    """Every kind with at least one malformed case, mapped to those files, name-sorted.

    A case is ``<kind-in-kebab>/<condition>.sysml`` or ``.kerml``: the kind names the
    DIRECTORY and the condition names the file. One directory can hold several
    conditions.

    The kind is a directory rather than a filename prefix because a filename cannot carry
    both unambiguously. `-` separates the words inside a kebab kind *and* would separate
    the kind from the condition, and 91 kinds have a kebab form that prefixes another's,
    so `package-body-missing` reads equally as `Package` + "body missing" and as
    `PackageBody` + "missing". No rule can tell those apart, because the ambiguity is in
    the encoding. A directory puts the two in separate namespaces, where they cannot
    collide at all.

    An empty file proves nothing, so it does not count. The check still cannot tell
    whether a case asserts the right things -- review does.
    """
    if not root.is_dir():
        return {}
    by_stem = {kebab(kind): kind for kind in kinds}
    found: dict[str, list[str]] = {}
    for directory in sorted(root.iterdir()):
        kind = by_stem.get(directory.name)
        if not directory.is_dir() or kind is None:
            continue
        cases = sorted(
            path.name
            for path in directory.iterdir()
            if path.is_file()
            and path.suffix.lstrip(".") in CASE_SUFFIXES
            and path.stat().st_size > 0
        )
        if cases:
            found[kind] = cases
    return found


def stray_cases(kinds: list[str], root: Path = MALFORMED) -> list[str]:
    """Anything under the malformed tree that names no kind, path-sorted.

    A case in the wrong place is worse than a missing one: the author believes the kind
    is proved. A misspelled directory would otherwise be ignored in silence, and a case
    left at the top level -- the layout before the kind became a directory -- would stop
    counting without saying so.
    """
    if not root.is_dir():
        return []
    stems = {kebab(kind) for kind in kinds}
    stray: list[str] = []
    for path in sorted(root.iterdir()):
        if path.is_dir():
            if path.name not in stems:
                stray.append(f"{path.name}/ names no node kind")
        elif path.suffix.lstrip(".") in CASE_SUFFIXES:
            stray.append(f"{path.name} is not inside a <kind-in-kebab>/ directory")
    return stray


@dataclass(frozen=True, slots=True)
class Tally:
    """Which state each node kind is in, and the cases that belong to no kind at all."""

    implemented: list[str]
    unimplemented: list[str]
    absent: list[str]
    unproved: list[str]
    unswept: list[str]
    stray: list[str]


def build_report(
    declared_kinds: set[str] | list[str],
    tally: Tally,
) -> dict[str, object]:
    """The report as it should be on disk for this inventory and these markers."""
    implemented = tally.implemented
    unimplemented, absent = tally.unimplemented, tally.absent
    unproved, unswept, stray = tally.unproved, tally.unswept, tally.stray
    total = len(set(declared_kinds))
    percent = 100.0 * len(implemented) / total if total else 0.0
    return {
        "_generated_by": "scripts/ast_coverage.py",
        "_note": (
            "Typed accessors over CST node kinds (ADR-0025). `unimplemented` is a tracked "
            "state, not a failure. `absent` is a defect: a marker claiming a kind that "
            "does not exist. `unproved` is a defect: an implemented kind with no malformed "
            "case under crates/sv2-ast/tests/malformed/<kind-in-kebab>/. `unswept` is a "
            "defect: an implemented kind whose assert_<kind>_accessor the sweep does not "
            "define and call, so no tree reaches it. `stray` is a defect: a case naming no "
            "kind. The order is the worklist order, never a verdict."
        ),
        "declared": total,
        "implemented": len(implemented),
        "unimplemented": len(unimplemented),
        "absent": len(absent),
        "unproved": len(unproved),
        "unswept": len(unswept),
        "stray": len(stray),
        "percent": round(percent, 1),
        "implemented_kinds": implemented,
        "unimplemented_kinds": unimplemented,
        "absent_markers": absent,
        "unproved_kinds": unproved,
        "unswept_kinds": unswept,
        "stray_cases": stray,
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
    exercised = swept(kinds)
    unswept = [k for k in implemented if k not in exercised]
    tally = Tally(implemented, unimplemented, absent, unproved, unswept, stray_cases(kinds))
    return build_report(kinds, tally)


@dataclass(frozen=True, slots=True)
class Defect:
    """One defect state: what to call it, what it listed, and what to do about it."""

    heading: str
    key: str
    item: Callable[[str], str]
    advice: str


#: The four defect states, in report order. A table rather than four near-identical
#: branches: the next state added is a row, and nothing about how it prints can drift from
#: how the others print.
DEFECTS = (
    Defect(
        heading="markers claim node kinds that do not exist:",
        key="absent_markers",
        item=lambda kind: f"  {kind}",
        advice="A kind is added to the parser's NODES, then regenerated. `absent` is a defect.",
    ),
    Defect(
        heading="malformed cases that name no node kind:",
        key="stray_cases",
        item=lambda case: f"  {case}",
        advice=(
            f"A case lives at {MALFORMED}/<kind-in-kebab>/<condition>.sysml. "
            "Misplaced, it proves nothing while looking as though it does."
        ),
    ),
    Defect(
        heading="accessors with no malformed case (ADR-0025):",
        key="unproved_kinds",
        item=lambda kind: f"  {kind} — expected {MALFORMED}/{kebab(kind)}/<condition>.sysml",
        advice=(
            "An accessor lands with all four kinds of evidence or it does not land. "
            "Write the case; do not widen this check."
        ),
    ),
    Defect(
        heading="accessors the sweep never exercises (ADR-0025):",
        key="unswept_kinds",
        item=lambda kind: (
            f"  {kind} — expected fn assert_{snake(kind)}_accessor in {SWEEP}, called"
        ),
        advice=(
            "An accessor nothing sweeps is green because it has nothing to check. "
            "Define the assertion AND call it from sweep(); do not widen this check."
        ),
    ),
)


def report_defects(report: dict[str, object]) -> bool:
    """Print the first defect state carrying anything. True when one did."""
    for defect in DEFECTS:
        found = report.get(defect.key)
        if not isinstance(found, list) or not found:
            continue
        print(defect.heading)
        for entry in found:
            print(defect.item(str(entry)))
        print(defect.advice)
        return True
    return False


def main(argv: list[str] | None = None) -> int:
    """Write the coverage report, or with --check fail on a defect state or staleness."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail on absent, unproved, unswept, stray, or a stale report",
    )
    args = parser.parse_args(argv)
    os.chdir(ROOT)

    report = measure()
    if args.check:
        if report_defects(report):
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
