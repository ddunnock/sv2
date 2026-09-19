# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""The Pilot's Xtext must import the metamodel namespace pinned in Tier A.

The Xtext imports the metamodel by namespace URI:

    import "https://www.omg.org/spec/SysML/20250201" as SysML

That must match the namespace of the XMI pinned in Tier A. If the Pilot moves
ahead to a newer metamodel, this fires — which is the whole point of pinning
two sources instead of one.

    python3.12 scripts/check_namespace_link.py
"""

from __future__ import annotations

import argparse
import json
import os
import re
from pathlib import Path
from typing import Any

from _lock import read_target

ROOT = Path(__file__).resolve().parents[1]
PILOT = Path("vendor/pilot")
INVENTORY = Path(".claude/state/grammar/productions.json")
NAMESPACE_DATE = re.compile(r"/(\d{8})/?$")


# Any: productions.json is a deserialized document (STD-001-PY §6).
def mismatches(grammars: list[dict[str, Any]], namespace: str) -> list[tuple[str, str, str]]:
    """(file, uri, reason) for every OMG import whose namespace is not the pinned one."""
    bad: list[tuple[str, str, str]] = []
    for grammar in grammars:
        for imp in grammar["imports"]:
            uri = imp["uri"]
            if "omg.org/spec" not in uri:
                continue  # Ecore and friends
            date = NAMESPACE_DATE.search(uri)
            if not date:
                bad.append((grammar["file"], uri, "no namespace date in URI"))
            elif date.group(1) != namespace:
                bad.append((grammar["file"], uri, f"expected namespace {namespace}"))
    return bad


def main(argv: list[str] | None = None) -> int:
    """Compare the Xtext's metamodel namespace with the pinned one; 1 if they differ."""
    argparse.ArgumentParser(description=__doc__).parse_args(argv)
    os.chdir(ROOT)

    if not any(PILOT.glob("*.xtext")):
        print("no Xtext vendored — check inert")
        return 0
    if not INVENTORY.is_file():
        print("run python3.12 scripts/extract_productions.py first")
        return 0

    namespace = read_target().get("tier_a_omg.namespace", "")
    if not namespace:
        print("tier_a_omg.namespace unset — check inert")
        return 0

    bad = mismatches(json.loads(INVENTORY.read_text())["grammars"], namespace)
    if bad:
        print("metamodel namespace mismatch between Tier B grammar and Tier A metamodel:")
        for file_name, uri, why in bad:
            print(f"  {file_name}: {uri}  ({why})")
        print()
        print("The vendored Xtext was built against a different metamodel version than the")
        print("XMI you pinned. Either move Tier A to that namespace, or pin an older Pilot sha.")
        print("Do not proceed with them mismatched — the metaclass map will be wrong.")
        return 1
    print(f"namespace link ok: Xtext imports match Tier A namespace {namespace}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
