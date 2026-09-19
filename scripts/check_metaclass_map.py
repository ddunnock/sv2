# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Every metaclass an Xtext rule takes from the OMG metamodel must exist in the pinned XMI.

A rule returning a metaclass the abstract syntax does not define means the
grammar and the metamodel have drifted apart — which would silently produce
wrong desugaring targets in phase 2.

Scoped to the OMG metamodel, because that is the only one pinned. The grammars
import two metamodels under separate aliases:

    import "http://www.eclipse.org/emf/2002/Ecore" as Ecore
    import "https://www.omg.org/spec/SysML/20250201" as SysML

``Ecore::EString`` is an EMF primitive, not a SysML metaclass, and asserting it
against SysML.xmi is a category error — it is absent by construction, not by
drift. The aliases are read from the grammars' own import lists rather than
hardcoded, so a grammar that imports a third metamodel does not silently fall
into whichever bucket was assumed. check_namespace_link.py draws the same
boundary for the same reason.

Out-of-scope metaclasses are reported, not hidden: an Ecore type appearing where
a SysML one belongs is a real problem, and it cannot be seen if the line is
silent.

    python3.12 scripts/check_metaclass_map.py
"""

from __future__ import annotations

import argparse
import json
import os
import re
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
METACLASS_MAP = Path(".claude/state/grammar/metaclass-map.json")
INVENTORY = Path(".claude/state/grammar/productions.json")
OMG = Path("vendor/omg")
OMG_NAMESPACE = "omg.org/spec"
# MOF XMI: owned types carry name="..." on packagedElement / ownedMember.
TYPE_NAME = re.compile(r'<(?:packagedElement|ownedMember|ownedType)[^>]*\bname="([^"]+)"')


def known_types() -> set[str]:
    """Every type name declared anywhere in the vendored XMI."""
    return {
        m.group(1)
        for xmi in OMG.rglob("*.xmi")
        for m in TYPE_NAME.finditer(xmi.read_text(encoding="utf-8", errors="replace"))
    }


# Any: productions.json is a deserialized document (STD-001-PY §6).
def omg_aliases(grammars: list[dict[str, Any]]) -> set[str]:
    """The import aliases that name an OMG metamodel, read from the grammars themselves."""
    return {
        imp["alias"]
        for grammar in grammars
        for imp in grammar["imports"]
        if OMG_NAMESPACE in imp["uri"] and imp.get("alias")
    }


def partition(
    metaclass_map: dict[str, list[str]], aliases: set[str]
) -> tuple[list[tuple[str, list[str]]], list[tuple[str, list[str]]]]:
    """(in scope, out of scope) entries, split on whether the alias is an OMG one.

    An unqualified name is in scope: it names no foreign metamodel, so the pinned
    XMI is the only thing it could come from, and defaulting it out of scope would
    let a genuinely missing metaclass through unchecked.
    """
    in_scope: list[tuple[str, list[str]]] = []
    out_of_scope: list[tuple[str, list[str]]] = []
    for qualified, rules in metaclass_map.items():
        alias = qualified.split("::")[0] if "::" in qualified else ""
        (out_of_scope if alias and alias not in aliases else in_scope).append((qualified, rules))
    return in_scope, out_of_scope


def main(argv: list[str] | None = None) -> int:
    """Check every mapped metaclass against the pinned XMI; 1 if one is absent."""
    argparse.ArgumentParser(description=__doc__).parse_args(argv)
    os.chdir(ROOT)

    if not METACLASS_MAP.is_file():
        print("run python3.12 scripts/extract_productions.py first")
        return 0
    if not any(OMG.glob("*/*.xmi")):
        print("no XMI vendored — check inert")
        return 0

    known = known_types()
    if not known:
        print("no type names found in the vendored XMI — extraction pattern may need widening")
        return 0

    data = json.loads(METACLASS_MAP.read_text())
    aliases = omg_aliases(json.loads(INVENTORY.read_text())["grammars"])
    in_scope, out_of_scope = partition(data["map"], aliases)
    missing = [(q, rules) for q, rules in in_scope if q.split("::")[-1] not in known]

    for qualified, rules in sorted(out_of_scope):
        print(f"  out of scope, not an OMG metaclass: {qualified}  ({len(rules)} rule(s))")

    if missing:
        print(
            f"{len(missing)} metaclass(es) named by grammar rules are absent from the pinned XMI:"
        )
        for qualified, rules in missing[:20]:
            print(f"  {qualified}   (rules: {', '.join(rules[:4])})")
        print()
        print("Either the Xtext is ahead of the metamodel, or the XMI type extraction missed a")
        print(
            "packaging form. Check python3.12 scripts/check_namespace_link.py first — a namespace"
        )
        print("mismatch explains this and is the more likely cause.")
        return 1

    print(
        f"metaclass map ok: {len(in_scope)} OMG metaclasses all present in the pinned XMI"
        f" ({len(out_of_scope)} from other metamodels, out of scope)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
