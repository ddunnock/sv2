# sv2-ast Development Harness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Put the harness in place that makes `sv2-ast` get built accessor by accessor — a coverage ratchet against the node-kind inventory, two structural lints, a four-part test discipline, and the agent-facing rules and skill — then prove it by landing one accessor through it.

**Architecture:** The node kinds are already authored in `scripts/gen_syntax_kinds.py`'s `NODES`; that generator emits a derived JSON inventory, and `scripts/ast_coverage.py` counts `// node: Kind` markers against it with the parser's three states. `scripts/check_ast_leaks.py` enforces owns-no-data and no-scaffolding structurally. Everything that must *call* an accessor is a Rust integration test under `crates/sv2-ast/tests/`, picked up by the existing `cargo test` gate step. Each check lands with a negative control proving it fails when it should.

**Tech Stack:** Python 3.12 (ruff, mypy strict, pytest), Rust 1.98.1 (rowan 0.17 via `sv2-syntax`, proptest), `scripts/gate.sh`.

**Spec:** `docs/adr/0025-typed-accessors-are-ratcheted-and-proved-on-malformed-trees.md`

## Global Constraints

- **Python:** 3.12, invoked as `python3.12`. Line length 100. `from __future__ import annotations` at the top of every module. Return types on every function. No `assert` outside tests (`S101`). Exception messages assigned to a variable, never inlined (`EM101`/`EM102`). Private modules print nothing — they return values and raise; the entry point decides output (STD-001-PY §8.3).
- **Every script has `main(argv: list[str] | None = None) -> int`** and ends with `if __name__ == "__main__": raise SystemExit(main())`. Never calls `sys.exit` internally (STD-001-PY §2.2).
- **Every source file opens with the two-line header:** `# SPDX-License-Identifier: MIT` then `# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>`. Enforced by `scripts/check_headers.py`.
- **Banned names**, enforced by `scripts/check_rust_patterns.py:31` over every file under `crates/` including `tests/`: `utils`, `helpers`, `misc`, `common`, `manager`, `handler` — as a module name, a file stem, or the last word of a type name.
- **`scripts/` must not import from `.claude/scripts/`** (STD-001-PY §2 rule 2). Reading a derived JSON file from either tree is fine; importing a module across the boundary is not.
- **Test layout** (STD-002-RS §10): unit tests at the bottom of the file they test in `#[cfg(test)] mod tests`; integration tests in `tests/` using the public API only; golden/fixture inputs are data files, never values embedded in assertions.
- **Expectations come from the specification or the corpus, never from output** (`.claude/rules/tests.md`). Never weaken a failing test.
- **The marker is `// node: Kind`.** Three states: `implemented` (marker claims a kind in the inventory), `unimplemented` (a kind no marker claims — tracked, never a failure), `absent` (a marker claims a kind that does not exist — a defect).
- **464** authored node kinds; **311** corpus files under `vendor/corpus`; **299** rejection files under `tests/rejection`. `tests/corpus` does not exist yet — `corpus_roots()` returns only roots that exist.
- **The layer ordering may never gate.** It decides how the report prints and nothing else.
- **No network, ever.** `scripts/vendor_sync.py` is the only script that fetches.
- **Definition of done for every task:** `./scripts/gate.sh` is green.

---

### Task 1: The node-kind inventory

`scripts/gen_syntax_kinds.py` already owns the authored node list as `NODES`, a list of `(name, doc)` tuples. It emits a second derived artifact so nothing parses `kinds.rs` a second time.

**Files:**
- Modify: `scripts/gen_syntax_kinds.py` (add `NODE_KINDS` path, `node_inventory()`, wire into `main`)
- Modify: `scripts/tests/test_gen_syntax_kinds.py`
- Modify: `.claude/scripts/hook_protect_paths.py:79-81` (add the new path to the derived-state group)

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: `.claude/state/node-kinds.json`, shape `{"_generated_by": str, "_note": str, "counts": {"nodes": int}, "nodes": list[str]}` where `nodes` is in declaration order. Task 2 reads it.

- [ ] **Step 1: Write the failing test**

Append to `scripts/tests/test_gen_syntax_kinds.py`:

```python
def test_node_inventory_lists_every_authored_kind():
    import json

    from gen_syntax_kinds import NODES, node_inventory

    doc = json.loads(node_inventory())
    assert doc["counts"]["nodes"] == len(NODES)
    assert doc["nodes"] == [name for name, _ in NODES]


def test_node_inventory_keeps_declaration_order():
    import json

    from gen_syntax_kinds import node_inventory

    nodes = json.loads(node_inventory())["nodes"]
    assert nodes[0] == "RootNamespace"
    assert len(nodes) == len(set(nodes))
```

- [ ] **Step 2: Run it and watch it fail**

Run: `.venv/bin/python -m pytest scripts/tests/test_gen_syntax_kinds.py -q`
Expected: FAIL, `ImportError: cannot import name 'node_inventory'`

- [ ] **Step 3: Add the inventory to the generator**

In `scripts/gen_syntax_kinds.py`, beside `OUT` (line 33):

```python
NODE_KINDS = Path(".claude/state/node-kinds.json")
```

Add above `main`:

```python
def node_inventory() -> str:
    """The authored node kinds, as the inventory sv2-ast's coverage counts against.

    `kinds.rs` is generated from the same list. Emitting the inventory here rather than
    parsing the generated Rust is ADR-0025's rule: a second reader of generated code is
    how one derived fact becomes two that disagree.
    """
    doc = {
        "_generated_by": GENERATED_BY,
        "_note": (
            "The node kinds a tree can contain, authored in this generator's NODES. "
            "sv2-ast's coverage counts against it (ADR-0025). Declaration order is the "
            "kind's discriminant order, and is the order the worklist falls back to."
        ),
        "counts": {"nodes": len(NODES)},
        "nodes": [name for name, _ in NODES],
    }
    return json.dumps(doc, indent=2) + "\n"
```

- [ ] **Step 4: Wire it into `main`, both branches**

Replace the body of `main` after `text = build()` (around line 2500):

```python
    text = build()
    inventory = node_inventory()
    if args.check:
        stale = [
            path
            for path, want in ((OUT, text), (NODE_KINDS, inventory))
            if not path.is_file() or path.read_text() != want
        ]
        if stale:
            for path in stale:
                print(f"{path} is stale — run python3.12 {GENERATED_BY}")
            return 1
        print(f"syntax kinds current ({OUT}, {NODE_KINDS})")
        return 0

    OUT.parent.mkdir(parents=True, exist_ok=True)
    write_derived(OUT, text)
    NODE_KINDS.parent.mkdir(parents=True, exist_ok=True)
    write_derived(NODE_KINDS, inventory)
    print(f"wrote {OUT} and {NODE_KINDS}")
    return 0
```

- [ ] **Step 5: Run the tests and the generator**

Run: `.venv/bin/python -m pytest scripts/tests/test_gen_syntax_kinds.py -q`
Expected: PASS

Run: `python3.12 scripts/gen_syntax_kinds.py`
Expected: `wrote crates/sv2-syntax/src/generated/kinds.rs and .claude/state/node-kinds.json`

Run: `python3.12 scripts/gen_syntax_kinds.py --check`
Expected: `syntax kinds current (...)`, exit 0

- [ ] **Step 6: Negative control — prove `--check` catches a stale inventory**

```bash
python3.12 - <<'PY'
import json, pathlib
p = pathlib.Path(".claude/state/node-kinds.json")
d = json.loads(p.read_text()); d["nodes"] = d["nodes"][:-1]
p.write_text(json.dumps(d, indent=2) + "\n")
PY
python3.12 scripts/gen_syntax_kinds.py --check; echo "exit=$?"
```

Expected: `.claude/state/node-kinds.json is stale`, `exit=1`

Restore: `python3.12 scripts/gen_syntax_kinds.py`

- [ ] **Step 7: Protect the derived file**

In `.claude/scripts/hook_protect_paths.py`, add `".claude/state/node-kinds.json"` to the tuple that already holds `".claude/state/coverage.json"` and `".claude/state/decisions.json"` (line 79-81).

- [ ] **Step 8: Verify the hook blocks a hand edit**

Attempt an `Edit` to `.claude/state/node-kinds.json`.
Expected: blocked, with the generator named in the message.

- [ ] **Step 9: Lint, gate, commit**

```bash
ruff check scripts .claude/scripts && ruff format --check scripts .claude/scripts
mypy scripts .claude/scripts
./scripts/gate.sh
git add scripts/gen_syntax_kinds.py scripts/tests/test_gen_syntax_kinds.py \
        .claude/scripts/hook_protect_paths.py .claude/state/node-kinds.json
git commit -m "Emit the node-kind inventory from the generator that owns the list"
```

---

### Task 2: `ast_coverage.py` — the three states

**Files:**
- Create: `scripts/ast_coverage.py`
- Create: `scripts/tests/test_ast_coverage.py`
- Modify: `scripts/gate.sh` (one `run_check` line after `bnf coverage`, line 83)
- Modify: `.claude/scripts/hook_protect_paths.py` (add `.claude/state/ast-coverage.json`)

**Interfaces:**
- Consumes: `.claude/state/node-kinds.json` from Task 1.
- Produces: `declared() -> list[str]`, `claims(root: Path) -> dict[str, list[str]]` mapping kind to the files claiming it, `build_report(declared, implemented, unimplemented, absent) -> dict[str, object]`, `is_stale(report) -> bool`, and `.claude/state/ast-coverage.json`. Task 3 re-orders the report; Task 7 moves it 0 → 1.

- [ ] **Step 1: Write the failing tests**

Create `scripts/tests/test_ast_coverage.py`:

```python
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import json

import pytest

from ast_coverage import build_report, claims, is_stale


def _crate(tmp_path, **files):
    src = tmp_path / "src"
    src.mkdir()
    for name, text in files.items():
        (src / name).write_text(text)
    return tmp_path


def test_a_marker_claims_its_kind(tmp_path):
    root = _crate(tmp_path, **{"ns.rs": "// node: RootNamespace\npub struct RootNamespace;"})
    assert claims(root) == {"RootNamespace": ["src/ns.rs"]}


def test_a_kind_claimed_twice_records_both_files(tmp_path):
    root = _crate(
        tmp_path,
        **{"a.rs": "// node: RootNamespace\n", "b.rs": "// node: RootNamespace\n"},
    )
    assert claims(root)["RootNamespace"] == ["src/a.rs", "src/b.rs"]


def test_spacing_in_the_marker_does_not_matter(tmp_path):
    root = _crate(tmp_path, **{"a.rs": "//node:RootNamespace\n"})
    assert "RootNamespace" in claims(root)


def test_a_kind_no_marker_claims_is_unimplemented():
    report = build_report({"A", "B"}, ["A"], ["B"], [])
    assert report["unimplemented"] == 1
    assert report["unimplemented_kinds"] == ["B"]
    assert report["absent"] == 0


def test_a_marker_naming_no_kind_is_absent():
    report = build_report({"A"}, [], ["A"], ["NotAKind"])
    assert report["absent"] == 1
    assert report["absent_markers"] == ["NotAKind"]


def test_an_empty_crate_reports_every_kind_unimplemented():
    report = build_report({"A", "B"}, [], ["A", "B"], [])
    assert (report["implemented"], report["unimplemented"], report["percent"]) == (0, 2, 0.0)


def test_a_report_matching_disk_is_not_stale(tmp_path, monkeypatch):
    import ast_coverage

    path = tmp_path / "ast-coverage.json"
    monkeypatch.setattr(ast_coverage, "REPORT", path)
    report = build_report({"A"}, [], ["A"], [])
    path.write_text(json.dumps(report, indent=2) + "\n")
    assert not is_stale(report)


@pytest.mark.parametrize("damage", ["missing", "truncated", "changed"])
def test_a_report_disagreeing_with_disk_is_stale(damage, tmp_path, monkeypatch):
    import ast_coverage

    path = tmp_path / "ast-coverage.json"
    monkeypatch.setattr(ast_coverage, "REPORT", path)
    report = build_report({"A"}, [], ["A"], [])
    if damage == "truncated":
        path.write_text('{"implemented":')
    elif damage == "changed":
        path.write_text(json.dumps(build_report({"A"}, ["A"], [], []), indent=2) + "\n")
    assert is_stale(report)
```

- [ ] **Step 2: Run them and watch them fail**

Run: `.venv/bin/python -m pytest scripts/tests/test_ast_coverage.py -q`
Expected: FAIL, `ModuleNotFoundError: No module named 'ast_coverage'`

- [ ] **Step 3: Write the script**

Create `scripts/ast_coverage.py`:

```python
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
```

- [ ] **Step 4: Run the tests**

Run: `.venv/bin/python -m pytest scripts/tests/test_ast_coverage.py -q`
Expected: PASS

- [ ] **Step 5: Generate the report and read it**

Run: `python3.12 scripts/ast_coverage.py`
Expected: `wrote .claude/state/ast-coverage.json (0/464 node kinds)`

- [ ] **Step 6: Negative control — a marker naming no kind is `absent`**

```bash
printf '// node: NotAKind\n' > crates/sv2-ast/src/control.rs
python3.12 scripts/ast_coverage.py --check; echo "exit=$?"
rm crates/sv2-ast/src/control.rs
```

Expected: `markers claim node kinds that do not exist:` then `NotAKind`, `exit=1`

- [ ] **Step 7: Negative control — a stale report fails**

```bash
python3.12 - <<'PY'
import json, pathlib
p = pathlib.Path(".claude/state/ast-coverage.json")
d = json.loads(p.read_text()); d["unimplemented"] = 0
p.write_text(json.dumps(d, indent=2) + "\n")
PY
python3.12 scripts/ast_coverage.py --check; echo "exit=$?"
python3.12 scripts/ast_coverage.py
```

Expected: `is stale`, `exit=1`, then regenerated

- [ ] **Step 8: Add the gate line and protect the report**

In `scripts/gate.sh`, immediately after the `bnf coverage` line (83):

```bash
  run_check "ast coverage" "${PY}" scripts/ast_coverage.py --check
```

In `.claude/scripts/hook_protect_paths.py`, add `".claude/state/ast-coverage.json"` to the same tuple Task 1 touched.

- [ ] **Step 9: Lint, gate, commit**

```bash
ruff check scripts && ruff format --check scripts && mypy scripts
./scripts/gate.sh
git add scripts/ast_coverage.py scripts/tests/test_ast_coverage.py scripts/gate.sh \
        .claude/scripts/hook_protect_paths.py .claude/state/ast-coverage.json
git commit -m "Count typed accessors against the node-kind inventory"
```

---

### Task 3: Root-downward worklist order

The report currently prints in declaration order. ADR-0025 asks for root-downward, derived from the frozen grammar, and says it may never gate.

**Files:**
- Modify: `scripts/ast_coverage.py` (add `layer_of()`, use it to sort)
- Modify: `scripts/tests/test_ast_coverage.py`

**Interfaces:**
- Consumes: `.claude/state/grammar/reference.json`. Its rules are under **`grammar`** — a dict of 554 entries keyed by *unit*, so a key is `Name` or `Name@kerml`/`Name@sysml` (ADR-0015) — each `{"rule": <AST>, "decision": …, "metaclass": …, "fingerprint": …}`. `productions` in that file is a **count**, not the rules. A reference inside a rule is `{"k": "ref", "name": "<Production>"}`, nested anywhere.
- Produces: `layer_of(reference: dict) -> dict[str, int]` mapping a **base** production name (scope stripped) to its breadth-first distance from `RootNamespace`, and `worklist(kinds, layers) -> list[str]`. Unreached names and kinds with no matching production are absent from the map and sort last.
- Verified against the real file while this plan was written: 526 base names, 525 reached from `RootNamespace`, max depth 16, layers 0–2 being `RootNamespace`, `NamespaceBodyElement`, `PackageBodyElement`, `NamespaceMember`, `PackageMember`, `Import`, `AliasMember`, `ElementFilterMember`.

- [ ] **Step 1: Write the failing tests**

Append to `scripts/tests/test_ast_coverage.py`:

```python
def test_the_root_is_layer_zero():
    from ast_coverage import layer_of

    reference = {"grammar": {"RootNamespace": {"rule": {"k": "ref", "name": "Member"}}}}
    assert layer_of(reference)["RootNamespace"] == 0


def test_a_production_the_root_reaches_is_layer_one():
    from ast_coverage import layer_of

    reference = {
        "grammar": {
            "RootNamespace": {"rule": {"k": "ref", "name": "Member"}},
            "Member": {"rule": {"k": "kw", "text": "part"}},
        }
    }
    assert layer_of(reference)["Member"] == 1


def test_a_variant_unit_is_layered_under_its_base_name():
    # The grammar is keyed by unit, so RootNamespace may arrive as RootNamespace@kerml.
    from ast_coverage import layer_of

    reference = {
        "grammar": {
            "RootNamespace@kerml": {"rule": {"k": "ref", "name": "Member"}},
            "RootNamespace@sysml": {"rule": {"k": "ref", "name": "Other"}},
            "Member": {"rule": {"k": "kw", "text": "part"}},
            "Other": {"rule": {"k": "kw", "text": "item"}},
        }
    }
    assert layer_of(reference) == {"RootNamespace": 0, "Member": 1, "Other": 1}


def test_a_reference_nested_in_a_sequence_is_found():
    from ast_coverage import layer_of

    reference = {
        "grammar": {
            "RootNamespace": {"rule": {"k": "seq", "items": [{"k": "ref", "name": "Deep"}]}},
            "Deep": {"rule": {"k": "kw", "text": "x"}},
        }
    }
    assert layer_of(reference)["Deep"] == 1


def test_an_unreached_production_has_no_layer():
    from ast_coverage import layer_of

    reference = {
        "grammar": {
            "RootNamespace": {"rule": {"k": "kw", "text": "x"}},
            "Island": {"rule": {"k": "kw", "text": "y"}},
        }
    }
    assert "Island" not in layer_of(reference)


def test_a_cycle_terminates():
    from ast_coverage import layer_of

    reference = {
        "grammar": {
            "RootNamespace": {"rule": {"k": "ref", "name": "A"}},
            "A": {"rule": {"k": "ref", "name": "RootNamespace"}},
        }
    }
    assert layer_of(reference) == {"RootNamespace": 0, "A": 1}


def test_the_worklist_sorts_shallow_kinds_first():
    from ast_coverage import worklist

    assert worklist(["Zebra", "Member", "RootNamespace"], {"RootNamespace": 0, "Member": 1}) == [
        "RootNamespace",
        "Member",
        "Zebra",
    ]


def test_a_kind_with_no_layer_keeps_declaration_order_among_its_peers():
    from ast_coverage import worklist

    assert worklist(["Beta", "Alpha"], {}) == ["Beta", "Alpha"]
```

- [ ] **Step 2: Run them and watch them fail**

Run: `.venv/bin/python -m pytest scripts/tests/test_ast_coverage.py -q`
Expected: FAIL, `ImportError: cannot import name 'layer_of'`

- [ ] **Step 3: Implement the walk**

Add to `scripts/ast_coverage.py`, above `measure`:

```python
REFERENCE = Path(".claude/state/grammar/reference.json")
START = "RootNamespace"


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
```

- [ ] **Step 4: Use it in `measure`**

Replace `measure`'s body:

```python
def measure() -> dict[str, object]:
    """The report for the tree as it stands, worklist ordered root downward."""
    kinds = declared()
    layers = layer_of(json.loads(REFERENCE.read_text())) if REFERENCE.is_file() else {}
    ordered = worklist(kinds, layers)
    claimed = claims()
    implemented = [k for k in ordered if k in claimed]
    unimplemented = [k for k in ordered if k not in claimed]
    absent = sorted(k for k in claimed if k not in set(kinds))
    return build_report(kinds, implemented, unimplemented, absent)
```

- [ ] **Step 5: Run the tests**

Run: `.venv/bin/python -m pytest scripts/tests/test_ast_coverage.py -q`
Expected: PASS

- [ ] **Step 6: Check the real worklist reads sensibly**

```bash
python3.12 scripts/ast_coverage.py
python3.12 -c "
import json
d = json.load(open('.claude/state/ast-coverage.json'))
print('first 12 of the worklist:', d['unimplemented_kinds'][:12])
"
```

Expected: `RootNamespace` first, then the namespace and membership kinds. Measured
while writing this plan: 525 of 526 base names are reachable, to a maximum depth of 16.

- [ ] **Step 7: Lint, gate, commit**

```bash
ruff check scripts && ruff format --check scripts && mypy scripts
./scripts/gate.sh
git add scripts/ast_coverage.py scripts/tests/test_ast_coverage.py .claude/state/ast-coverage.json
git commit -m "Order the accessor worklist root downward, advisory only"
```

---

### Task 4: `check_ast_leaks.py` — owns no data, no scaffolding

**Files:**
- Create: `scripts/check_ast_leaks.py`
- Create: `scripts/tests/test_check_ast_leaks.py`
- Modify: `scripts/gate.sh` (one `run_check` line after `rust patterns`, line 108)

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: `Finding` (a frozen dataclass with `path: str`, `line: int`, `message: str`), and `findings(text: str, path: str) -> list[Finding]`. Nothing later depends on it.

- [ ] **Step 1: Write the failing tests**

Create `scripts/tests/test_check_ast_leaks.py`:

```python
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import pytest

from check_ast_leaks import findings


def _messages(text):
    return [f.message for f in findings(text, "src/a.rs")]


def test_an_accessor_wrapping_one_syntax_node_is_clean():
    text = "// node: RootNamespace\npub struct RootNamespace {\n    node: SyntaxNode,\n}\n"
    assert _messages(text) == []


def test_a_tuple_struct_accessor_is_clean():
    # The idiomatic rowan shape. A checker that only understood braced structs would pass
    # every accessor in the crate without reading one of them.
    assert _messages("// node: RootNamespace\npub struct RootNamespace(SyntaxNode);\n") == []


def test_a_tuple_struct_owning_text_is_a_leak():
    text = "// node: Part\npub struct Part(SyntaxNode, String);\n"
    assert any("owns text" in m for m in _messages(text))


def test_a_tuple_struct_with_two_fields_is_a_leak():
    text = "// node: Part\npub struct Part(SyntaxNode, usize);\n"
    assert any("one SyntaxNode" in m for m in _messages(text))


def test_a_tuple_struct_without_its_marker_is_rejected():
    assert any("no `// node:` marker" in m for m in _messages("pub struct Part(SyntaxNode);\n"))


@pytest.mark.parametrize(
    "field",
    ["name: String", "names: Vec<String>", "text: Box<str>", "owned: std::string::String"],
)
def test_an_owned_text_field_is_a_leak(field):
    text = f"pub struct Part {{\n    node: SyntaxNode,\n    {field},\n}}\n"
    assert any("owns text" in m for m in _messages(text))


def test_a_second_field_is_a_leak_even_without_text():
    text = "pub struct Part {\n    node: SyntaxNode,\n    index: usize,\n}\n"
    assert any("one SyntaxNode" in m for m in _messages(text))


def test_a_public_fn_returning_string_is_a_leak():
    assert any("returns String" in m for m in _messages("pub fn name(&self) -> String {\n"))


def test_a_private_fn_returning_string_is_allowed():
    assert _messages("fn name(&self) -> String {\n") == []


@pytest.mark.parametrize("name", ["test_tree", "dump_node", "debug_print", "fixture_part"])
def test_scaffolding_in_the_public_api_is_rejected(name):
    assert any("scaffolding" in m for m in _messages(f"pub fn {name}() {{\n"))


def test_a_public_item_gated_on_cfg_test_is_rejected():
    text = "#[cfg(test)]\npub fn helper_for_tests() {}\n"
    assert any("cfg(test)" in m for m in _messages(text))


def test_a_private_item_gated_on_cfg_test_is_allowed():
    assert _messages("#[cfg(test)]\nmod tests {}\n") == []


def test_an_accessor_type_without_its_marker_is_rejected():
    text = "pub struct PartUsage {\n    node: SyntaxNode,\n}\n"
    assert any("no `// node:` marker" in m for m in _messages(text))


def test_a_finding_carries_its_line():
    text = "pub struct Part {\n    node: SyntaxNode,\n    name: String,\n}\n"
    assert [f.line for f in findings(text, "src/a.rs") if "owns text" in f.message] == [3]
```

- [ ] **Step 2: Run them and watch them fail**

Run: `.venv/bin/python -m pytest scripts/tests/test_check_ast_leaks.py -q`
Expected: FAIL, `ModuleNotFoundError: No module named 'check_ast_leaks'`

- [ ] **Step 3: Write the checker**

Create `scripts/check_ast_leaks.py`:

```python
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""sv2-ast owns no data, and ships no test scaffolding (ADR-0025).

    python3.12 scripts/check_ast_leaks.py           every crates/sv2-ast/src/**/*.rs

Three rules:

1. A public accessor type wraps exactly one `SyntaxNode` and nothing else. Copying text
   out of the tree means the tree has stopped being the single source of truth, which is
   half of what ADR-0004 bought with a lossless tree.
2. Nothing public is test scaffolding: no `test_`, `dump_`, `debug_` or `fixture_` name,
   and no public item gated on `#[cfg(test)]`.
3. An accessor type carries its `// node:` marker, so coverage cannot undercount.

Network: none.
"""

from __future__ import annotations

import argparse
import os
import re
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = Path("crates/sv2-ast/src")

STRUCT = re.compile(r"pub struct\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{(.*?)\n\}", re.DOTALL)
#: The idiomatic rowan shape, `pub struct Part(SyntaxNode);`. Without this the checker
#: would read no accessor in the crate and pass every one of them.
TUPLE_STRUCT = re.compile(r"pub struct\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)\s*;")
FIELD = re.compile(r"^\s*(?:pub\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*:\s*([^,\n]+)", re.MULTILINE)
PUB_FN_STRING = re.compile(r"pub fn\s+[A-Za-z_][A-Za-z0-9_]*\s*\([^)]*\)\s*->\s*String")
PUB_ITEM = re.compile(r"pub (?:fn|struct|enum|trait|const|static)\s+([A-Za-z_][A-Za-z0-9_]*)")
CFG_TEST_PUB = re.compile(r"#\[cfg\(test\)\]\s*\n\s*pub\s")
MARKER = re.compile(r"//\s*node:\s*([A-Za-z_][A-Za-z0-9_]*)")
SCAFFOLD = ("test_", "dump_", "debug_", "fixture_")
OWNED_TEXT = ("String", "Box<str>", "Vec<String>", "Cow<")


@dataclass(frozen=True, slots=True)
class Finding:
    """One rule broken, at one place."""

    path: str
    line: int
    message: str

    def render(self) -> str:
        """The finding as one `path:line: message` line."""
        return f"{self.path}:{self.line}: {self.message}"


def _line(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1


def _declared_types(text: str) -> list[tuple[str, str, list[tuple[str, str]], int]]:
    """(name, body, fields, offset) for every public struct, braced or tuple.

    A tuple struct's fields are positional, so they are named by index for the message.
    """
    found = []
    for match in STRUCT.finditer(text):
        found.append((match.group(1), match.group(2), FIELD.findall(match.group(2)), match.start()))
    for match in TUPLE_STRUCT.finditer(text):
        parts = [p.strip() for p in match.group(2).split(",") if p.strip()]
        fields = [(str(index), part) for index, part in enumerate(parts)]
        found.append((match.group(1), match.group(2), fields, match.start()))
    return found


def _struct_findings(text: str, path: str) -> list[Finding]:
    found: list[Finding] = []
    claimed = set(MARKER.findall(text))
    for name, body, fields, start in _declared_types(text):
        body_at = text.index(body, start) if body else start
        for field_name, field_type in fields:
            if any(owned in field_type for owned in OWNED_TEXT):
                at = body_at + (body.index(field_type) if field_type in body else 0)
                found.append(
                    Finding(
                        path,
                        _line(text, at),
                        f"`{name}.{field_name}` owns text; an accessor returns a node or a "
                        f"token and the caller asks it for text",
                    )
                )
        if len(fields) > 1:
            found.append(
                Finding(
                    path,
                    _line(text, start),
                    f"`{name}` holds {len(fields)} fields; an accessor type wraps one "
                    f"SyntaxNode and nothing else",
                )
            )
        if name not in claimed:
            found.append(
                Finding(
                    path,
                    _line(text, start),
                    f"`{name}` has no `// node:` marker, so coverage cannot count it",
                )
            )
    return found


def findings(text: str, path: str) -> list[Finding]:
    """Every rule this file breaks, in line order."""
    found = _struct_findings(text, path)
    found += [
        Finding(path, _line(text, m.start()), "a public fn returns String; return a token instead")
        for m in PUB_FN_STRING.finditer(text)
    ]
    found += [
        Finding(path, _line(text, m.start()), f"`{m.group(1)}` is test scaffolding in the public API")
        for m in PUB_ITEM.finditer(text)
        if m.group(1).startswith(SCAFFOLD)
    ]
    found += [
        Finding(path, _line(text, m.start()), "a public item is gated on cfg(test)")
        for m in CFG_TEST_PUB.finditer(text)
    ]
    return sorted(found, key=lambda f: (f.line, f.message))


def main(argv: list[str] | None = None) -> int:
    """Report every leak; 1 if there is one."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="*", help="files or directories; default: the crate's src")
    args = parser.parse_args(argv)
    os.chdir(ROOT)

    roots = [Path(p) for p in args.paths] or [SOURCE]
    files = [f for root in roots for f in ([root] if root.is_file() else sorted(root.rglob("*.rs")))]
    found = [
        finding
        for file in files
        for finding in findings(file.read_text(errors="replace"), file.as_posix())
    ]
    for finding in found:
        print(f"  {finding.render()}")
    if found:
        print(f"{len(found)} leak(s). sv2-ast owns no data and ships no scaffolding (ADR-0025).")
        return 1
    print(f"sv2-ast owns no data ({len(files)} file(s) checked)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
```

- [ ] **Step 4: Run the tests**

Run: `.venv/bin/python -m pytest scripts/tests/test_check_ast_leaks.py -q`
Expected: PASS

- [ ] **Step 5: Negative controls against the real crate**

```bash
cat > crates/sv2-ast/src/control.rs <<'RS'
pub struct Leaky {
    node: SyntaxNode,
    name: String,
}
pub fn test_only() {}
RS
python3.12 scripts/check_ast_leaks.py; echo "exit=$?"
rm crates/sv2-ast/src/control.rs
python3.12 scripts/check_ast_leaks.py; echo "exit=$?"
```

Expected: first run lists `owns text`, `holds 2 fields`, `no // node: marker`, `test scaffolding`, `exit=1`; second `exit=0`

- [ ] **Step 6: Add the gate line**

In `scripts/gate.sh`, immediately after the `rust patterns` line (108):

```bash
  run_check "ast leaks" "${PY}" scripts/check_ast_leaks.py
```

- [ ] **Step 7: Lint, gate, commit**

```bash
ruff check scripts && ruff format --check scripts && mypy scripts
./scripts/gate.sh
git add scripts/check_ast_leaks.py scripts/tests/test_check_ast_leaks.py scripts/gate.sh
git commit -m "Make owns-no-data and no-scaffolding gate failures"
```

---

### Task 5: The rules file and the skill

**Files:**
- Create: `.claude/rules/ast.md`
- Modify: `.claude/rules/syntax.md:1-5` (drop `crates/sv2-ast/**` from `paths`)
- Modify: `CLAUDE.md:47` (point sv2-ast at its own rules file)
- Create: `.claude/skills/add-accessor/SKILL.md`

**Interfaces:**
- Consumes: the marker format and three states from Task 2, the three lint rules from Task 4.
- Produces: the documented path Task 7 follows.

- [ ] **Step 1: Narrow the syntax rules to their own crate**

In `.claude/rules/syntax.md`, change the frontmatter from two globs to one:

```yaml
---
paths:
  - "crates/sv2-syntax/**"
---
```

- [ ] **Step 2: Write `.claude/rules/ast.md`**

```markdown
---
paths:
  - "crates/sv2-ast/**"
---

# sv2-ast — typed accessors over the CST

This crate is a view. It owns no data, and every rule below follows from that.

## Owns no data

An accessor type wraps exactly one `SyntaxNode` and nothing else. It returns nodes and
tokens; the caller asks those for text. No `String`, `Box<str>`, `Vec<String>` field, and
no public function returning `String`. `scripts/check_ast_leaks.py` fails on each.

ADR-0004 justifies the lossless tree partly because this crate makes it affordable.
Copying text out is how that stops being true.

## The marker, and the three states

Every accessor carries `// node: Kind`. `scripts/ast_coverage.py` counts markers against
`.claude/state/node-kinds.json`:

- **implemented** — a marker claims a kind. Counted.
- **unimplemented** — a kind no marker claims. Tracked, never a failure.
- **absent** — a marker claims a kind that does not exist. A defect: the kind is added to
  the parser's `NODES` and regenerated, or the marker is wrong.

A large unimplemented count is the honest state of this crate, not a problem to fix.

## A malformed case per accessor, in the same change

Three things, because absence is not the only way to be wrong:

1. the parse still yields a tree — it always does, and the accessor sees a partial one;
2. the accessor reports **absence** rather than guessing;
3. the neighbouring accessors still answer.

Where the grammar admits one child and the tree carries two, the case pins **which** is
chosen. `.claude/rules/tests.md` governs where expectations may come from; read it.

## What this crate does not do

It raises `ID-MALFORMED` and `ID-ORPHAN`, because attaching an `@id` note to its
declaration needs nothing beyond the current file. It does **not** allocate an identifier,
detect a duplicate, resolve a name, or construct a metaclass. Those need more than one
file, or need the metamodel, and writing one here is how the layering quietly dies.

## Test and development code

Unit tests at the bottom of the file they test, in `#[cfg(test)] mod tests` — the only
place a test reaches a private item, and compiled out of the shipped crate. Everything
else lives outside `src/`: shared support in `tests/trees/`, data in `tests/malformed/`,
tooling in `examples/`. `utils`, `helpers`, `common`, `misc`, `manager` and `handler` are
banned names anywhere under `crates/`, test files included.
```

- [ ] **Step 3: Correct the CLAUDE.md layout table**

`CLAUDE.md:47`, change the rules file for `crates/sv2-ast`:

```
crates/sv2-ast        typed accessors over CST nodes           .claude/rules/ast.md
```

- [ ] **Step 4: Write the skill**

Create `.claude/skills/add-accessor/SKILL.md`:

```markdown
---
name: add-accessor
description: Implement one typed accessor in sv2-ast end to end — accessor, `// node:` marker, a positive case from the corpus, a named malformed case, the differential assertion, coverage and the gate. Use when adding an accessor to sv2-ast, when the user says "add an accessor" or names a node kind, or when working the ast-coverage worklist.
---

# Add an accessor

One accessor per invocation. If the request names several, do the first and say what
remains: batching is how half-built accessors get marked complete.

## 1. Take the next kind off the worklist

```bash
python3.12 -c "
import json
d = json.load(open('.claude/state/ast-coverage.json'))
print('\n'.join(d['unimplemented_kinds'][:10]))
"
```

The order is root downward and **advisory** (ADR-0025). Taking a kind out of order is
fine; say why.

## 2. Find a real example before writing code

The kind must occur in a real file. Expectations come from the specification or the
corpus, never from output (`.claude/rules/tests.md`).

```bash
grep -rln "<a keyword the kind starts with>" vendor/corpus | head -3
```

If no corpus file contains it, say so — that is a signal about whether the accessor is
worth writing yet, not a reason to invent an example.

## 3. Write the failing tests first

Both of them, before the accessor exists:

- the positive case, reading the real file found in step 2;
- the malformed case in `tests/malformed/<condition>.sysml`, named for the condition,
  with a header comment saying what it violates — and asserting all three parts: the
  parse yields a tree, the accessor reports absence rather than guessing, the neighbours
  still answer.

Run them. They must fail for the right reason.

## 4. Implement

The accessor wraps one `SyntaxNode`, carries `// node: Kind`, and returns nodes and
tokens — never a `String`. The differential assertion re-derives the same fact by walking
children directly.

## 5. Verify, in this order

```bash
python3.12 scripts/ast_coverage.py          # the kind moves to implemented
python3.12 scripts/check_ast_leaks.py       # owns no data, marker present
cargo test -p sv2-ast                       # positive, malformed, sweep, properties
./scripts/gate.sh                           # the definition of done
```

A sweep failure on a tree you did not write is the point of the sweep. Read the tree
before changing the accessor.

## 6. Commit

The accessor, both tests, the malformed data file and the regenerated report, in one
commit. An accessor without its malformed case is not finished.
```

- [ ] **Step 5: Verify the rules file loads for the right paths**

Confirm `.claude/rules/ast.md` frontmatter lists only `crates/sv2-ast/**`, and `syntax.md` no longer claims it:

```bash
head -5 .claude/rules/ast.md .claude/rules/syntax.md
grep -n "sv2-ast" CLAUDE.md
```

Expected: no file claims `crates/sv2-ast/**` twice; CLAUDE.md points at `ast.md`.

- [ ] **Step 6: Gate and commit**

```bash
./scripts/gate.sh
git add .claude/rules/ast.md .claude/rules/syntax.md CLAUDE.md .claude/skills/add-accessor/SKILL.md
git commit -m "Give sv2-ast its own rules and an add-accessor skill"
```

---

### Task 6: The crate and its test suite

Lands the four test disciplines with nothing yet to test, so Task 7 can prove they bite.

**Files:**
- Modify: `crates/sv2-ast/Cargo.toml` (dependencies, dev-dependencies)
- Modify: `crates/sv2-ast/src/lib.rs` (the `AstNode` trait, no accessors yet)
- Create: `crates/sv2-ast/tests/trees/mod.rs`
- Create: `crates/sv2-ast/tests/sweep.rs`
- Create: `crates/sv2-ast/tests/properties.rs`

**Interfaces:**
- Consumes: `sv2_syntax::{Language, SyntaxKind, SyntaxNode, parse}`.
- Produces: `sv2_ast::AstNode` with `fn cast(node: SyntaxNode) -> Option<Self>` and `fn syntax(&self) -> &SyntaxNode`; and in `tests/trees/mod.rs`, `fn corpus_files() -> Vec<PathBuf>`, `fn rejection_files() -> Vec<PathBuf>`, `fn tree(source: &str, language: Language) -> SyntaxNode`. Task 7's tests use all four.

- [ ] **Step 1: Declare the dependencies**

`crates/sv2-ast/Cargo.toml`:

```toml
[dependencies]
sv2-syntax = { workspace = true }

[target.'cfg(not(target_os = "wasi"))'.dev-dependencies]
proptest = { workspace = true }
```

- [ ] **Step 2: Write the trait**

Replace `crates/sv2-ast/src/lib.rs`'s stub paragraph with the trait, keeping the two header lines and the module docs:

```rust
/// A typed view of one CST node.
///
/// Implementors own no data: the type wraps one [`SyntaxNode`] and nothing else, so the
/// tree stays the single source of truth and the text stays authoritative (ADR-0001).
pub trait AstNode: Sized {
    /// The kind this view is a view of.
    const KIND: SyntaxKind;

    /// A view of `node`, or `None` when `node` is some other kind.
    fn cast(node: SyntaxNode) -> Option<Self>;

    /// The node this view borrows.
    fn syntax(&self) -> &SyntaxNode;
}
```

with `use sv2_syntax::{SyntaxKind, SyntaxNode};` above it.

- [ ] **Step 3: Write the shared test support**

Create `crates/sv2-ast/tests/trees/mod.rs`:

```rust
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Shared support for sv2-ast's integration tests: where the trees come from.
//!
//! Not shipped: this lives under `tests/`, so it is compiled only for them.

use std::path::{Path, PathBuf};

use sv2_syntax::{Language, SyntaxNode, parse};

/// The workspace root, from this crate's manifest directory.
pub fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root is two levels above the crate manifest")
}

fn model_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("sysml" | "kerml")
            ) {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// Every positive-corpus file. These parse; the accessors see well-formed trees.
pub fn corpus_files() -> Vec<PathBuf> {
    model_files(&workspace().join("vendor/corpus"))
}

/// Every rejection case. These do NOT parse cleanly, which is the point: the trees are
/// partial, and a partial tree is the normal case for an editor (ADR-0002).
pub fn rejection_files() -> Vec<PathBuf> {
    model_files(&workspace().join("tests/rejection"))
}

/// The language a file's suffix names.
pub fn language_of(path: &Path) -> Language {
    match path.extension().and_then(|e| e.to_str()) {
        Some("kerml") => Language::KerMl,
        _ => Language::SysMl,
    }
}

/// The root node of `source`.
pub fn tree(source: &str, language: Language) -> SyntaxNode {
    parse(source, language).syntax()
}
```

- [ ] **Step 4: Write the sweep**

Create `crates/sv2-ast/tests/sweep.rs`:

```rust
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Every accessor, over every tree this repository holds (ADR-0025).
//!
//! The rejection corpus is malformed by construction, so the hardest inputs for this
//! layer arrive free and grow as the corpora grow.

mod trees;

use sv2_syntax::SyntaxNode;

/// What every accessor must satisfy at every node, whatever the tree.
fn assert_node_invariants(node: &SyntaxNode) {
    for child in node.children() {
        assert!(
            node.text_range().contains_range(child.text_range()),
            "child {:?} at {:?} is outside its parent {:?} at {:?}",
            child.kind(),
            child.text_range(),
            node.kind(),
            node.text_range()
        );
    }
}

fn sweep(paths: Vec<std::path::PathBuf>) -> usize {
    let mut visited = 0;
    for path in paths {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let root = trees::tree(&source, trees::language_of(&path));
        for node in root.descendants() {
            assert_node_invariants(&node);
            visited += 1;
        }
    }
    visited
}

#[test]
fn every_corpus_tree_holds_the_node_invariants() {
    let visited = sweep(trees::corpus_files());
    assert!(visited > 0, "the corpus produced no nodes; is vendor/corpus populated?");
}

#[test]
fn every_rejection_tree_holds_the_node_invariants() {
    let visited = sweep(trees::rejection_files());
    assert!(visited > 0, "the rejection set produced no nodes");
}
```

- [ ] **Step 5: Write the property test**

Create `crates/sv2-ast/tests/properties.rs`:

```rust
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Accessor invariants over generated input: the cases nobody thought to write.

#![cfg(not(target_os = "wasi"))]

mod trees;

use proptest::prelude::*;
use sv2_syntax::Language;

/// A fragment alphabet that reaches real productions without being a grammar.
fn fragment() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        prop_oneof![
            Just("package ".to_owned()),
            Just("part ".to_owned()),
            Just("def ".to_owned()),
            Just("x".to_owned()),
            Just("{".to_owned()),
            Just("}".to_owned()),
            Just(";".to_owned()),
            Just("// note\n".to_owned()),
            Just("é".to_owned()),
        ],
        0..40,
    )
    .prop_map(|parts| parts.concat())
}

proptest! {
    #[test]
    fn a_tree_from_any_fragment_holds_its_ranges(source in fragment()) {
        let root = trees::tree(&source, Language::SysMl);
        for node in root.descendants() {
            for child in node.children() {
                prop_assert!(node.text_range().contains_range(child.text_range()));
            }
        }
    }

    #[test]
    fn truncating_at_any_byte_never_panics(source in fragment(), cut in 0usize..200) {
        let at = source.char_indices().map(|(i, _)| i).nth(cut).unwrap_or(source.len());
        let root = trees::tree(&source[..at], Language::SysMl);
        prop_assert!(root.text_range().len() <= (source.len() as u32).into());
    }
}
```

- [ ] **Step 6: Run the suite**

Run: `cargo test -p sv2-ast`
Expected: PASS — `every_corpus_tree_holds_the_node_invariants`, `every_rejection_tree_holds_the_node_invariants`, and both properties.

- [ ] **Step 7: Negative control — prove the sweep can fail**

Temporarily invert the sweep's assertion to `!node.text_range().contains_range(...)`, run `cargo test -p sv2-ast`, confirm it FAILS and names a node kind and range, then restore it.

- [ ] **Step 8: Gate and commit**

```bash
cargo fmt --all && cargo clippy -p sv2-ast --all-targets -- -D warnings
./scripts/gate.sh
git add crates/sv2-ast
git commit -m "Wire sv2-ast to the tree, and land the sweep before the accessors"
```

---

### Task 7: The first accessor, through the skill

The acceptance test for the whole harness. Built by invoking the `add-accessor` skill, not by hand.

**Files:**
- Modify: `crates/sv2-ast/src/lib.rs` (or a new `src/namespace.rs` it declares)
- Create: `crates/sv2-ast/tests/malformed.rs`
- Create: `crates/sv2-ast/tests/malformed/root-namespace-unclosed-body.sysml`
- Modify: `.claude/state/ast-coverage.json` (regenerated)

**Interfaces:**
- Consumes: `AstNode` and `tests/trees/mod.rs` from Task 6.
- Produces: `sv2_ast::RootNamespace`, implementing `AstNode` with `KIND = SyntaxKind::RootNamespace`, plus `fn members(&self) -> impl Iterator<Item = SyntaxNode>`.

- [ ] **Step 1: Invoke the skill**

Invoke `add-accessor` for `RootNamespace` — it is first on the worklist, and every tree has exactly one.

- [ ] **Step 2: Confirm the report moves**

Run: `python3.12 scripts/ast_coverage.py`
Expected: `wrote .claude/state/ast-coverage.json (1/464 node kinds)`

- [ ] **Step 3: Confirm the leak lint still passes on real code**

Run: `python3.12 scripts/check_ast_leaks.py`
Expected: `sv2-ast owns no data (N file(s) checked)`, exit 0

- [ ] **Step 4: Confirm the malformed case proves all three parts**

The case file asserts the parse yields a tree, the accessor reports absence rather than
guessing, and the neighbours still answer. Run `cargo test -p sv2-ast` and read the test
body to confirm all three assertions are present, not just the first.

- [ ] **Step 5: Gate and commit**

```bash
./scripts/gate.sh
git add crates/sv2-ast .claude/state/ast-coverage.json
git commit -m "Add the RootNamespace accessor, and prove the harness bites"
```

- [ ] **Step 6: Record what the harness cost**

Write one line in the commit body or the session log: how long the sweep takes
(`cargo test -p sv2-ast -- --nocapture` reports it), so the cost is tracked from the
first accessor rather than noticed at the fiftieth.

---

### Task 8: Hand the session over

**Files:**
- Modify: `.claude/state/state.json` (authored block only)

- [ ] **Step 1: Run the close-session skill**

Invoke `close-session`. It runs the gates, regenerates the derived state, writes the
authored block and appends the log entry.

- [ ] **Step 2: Confirm the objective reads correctly**

The new objective names ADR-0025, the harness, and the first accessor; `next_step` names
the second kind on the worklist.

---

## Self-review

**Spec coverage.** Inventory → Task 1. Three states and the report → Task 2. Root-downward order, advisory → Task 3. Owns-no-data and no-scaffolding → Task 4. Rules file, skill, CLAUDE.md → Task 5. Four evidence kinds: sweep and properties → Task 6, malformed case and differential → Task 7 via the skill. Negative controls → Tasks 1, 2, 4, 6. Hook protection → Tasks 1 and 2. Test/dev separation → Task 6's layout.

**Known gap, deliberate.** `tests/malformed.rs` and the first data file land in Task 7 rather than Task 6, because a malformed case needs an accessor to be malformed *for*. Task 6's suite is the two disciplines that can exist without one.

**Deferred, per the ADR.** Recording which kinds the sweep exercises, so green can tell an untested accessor from a passing one. It says nothing until there are accessors.
