# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""The release-upgrade path, and the reason the whole workflow is fingerprinted.

After re-pinning to a new OMG release and re-running extract_productions.py,
this reports exactly which units must be re-derived. Only the delta goes near
an AI; unchanged units carry forward untouched with their evidence intact.

Run AFTER grammar_plan.py, which does the reclassification.

    python3.12 .claude/scripts/grammar_rebase.py
"""

from __future__ import annotations

import argparse
import json
import os
from typing import TYPE_CHECKING

from _grammar import GRAMMAR, load_units
from _state import REPO_ROOT, load_json

if TYPE_CHECKING:
    from _state import Json


def classify(units: dict[str, Json], previous: dict[str, Json]) -> dict[str, list[str]]:
    """Sort units into unchanged, changed, new, and removed against the frozen grammar."""
    groups: dict[str, list[str]] = {"unchanged": [], "changed": [], "new": [], "removed": []}
    for name, unit in units.items():
        if unit["status"] == "retired":
            group = "removed"
        elif name not in previous:
            group = "new"
        elif previous[name].get("fingerprint") != unit["fingerprint"]["combined"]:
            group = "changed"
        else:
            group = "unchanged"
        groups[group].append(name)
    return groups


def main(argv: list[str] | None = None) -> int:
    """Report which units a moved pin invalidates, and which carry forward."""
    argparse.ArgumentParser(description=__doc__).parse_args(argv)
    os.chdir(REPO_ROOT)

    units = load_units()
    ref = load_json(GRAMMAR / "reference.json")
    if not ref:
        print("no previously frozen grammar — nothing to rebase from")
        return 0

    groups = classify(units, ref.get("grammar", {}))
    report = {
        "_generated_by": ".claude/scripts/grammar_rebase.py",
        "from_namespace": ref.get("omg_namespace"),
        "from_pilot": ref.get("pilot_revision"),
        "from_sha256": ref.get("grammar_sha256"),
        **{k: sorted(v) for k, v in groups.items()},
        "work": sorted(groups["changed"] + groups["new"]),
    }
    (GRAMMAR / "rebase.json").write_text(json.dumps(report, indent=2) + "\n")

    total = len(groups["unchanged"]) + len(groups["changed"]) + len(groups["new"])
    work = len(report["work"])
    print(f"rebase from {ref.get('omg_namespace')} / {str(ref.get('pilot_revision'))[:12]}")
    print(f"  unchanged {len(groups['unchanged']):4d}   carry forward untouched")
    print(f"  changed   {len(groups['changed']):4d}   inputs moved — re-derive")
    print(f"  new       {len(groups['new']):4d}   derive from scratch")
    print(f"  removed   {len(groups['removed']):4d}   retired")
    percent = 100.0 * work / total if total else 0
    print(f"  -> {work}/{total} units need derivation ({percent:.1f}% of the grammar)")
    for name in sorted(groups["changed"])[:15]:
        print(f"       changed: {name}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
