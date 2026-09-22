# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Every departure from the specification is reported where the parser takes it.

ADR-0022: when text parses only because of a recorded deviation, the parser attaches a
`PARSE-DEVIATION` note naming the register entry, and `sv2 parse --strict` promotes the
notes to errors. That is only as good as its coverage, so this holds both directions:

  - every `note_deviation("NAME", ...)` in the parser carries a `// deviation: NAME`
    marker, and NAME is an entry in .claude/state/deviations.json whose decision departs
    from the specification (follow_xtext, follow_corpus, follow_spec_example);
  - every such entry is either sited in the parser or listed, with its reason, in
    .claude/state/deviation-sites-pending.txt -- and not both.

What this does NOT check: that a site fires on exactly the text its deviation admits.
That is what each site's tests are for.

    python3.12 scripts/check_deviation_sites.py
"""

from __future__ import annotations

import json
import os
import re
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
DEVIATIONS = Path(".claude/state/deviations.json")
PENDING = Path(".claude/state/deviation-sites-pending.txt")
PARSER = Path("crates/sv2-syntax/src/parser.rs")
DEPARTING = frozenset({"follow_xtext", "follow_corpus", "follow_spec_example"})
MARKER = re.compile(r"^\s*//\s*deviation:\s*([A-Za-z][\w-]*)\s*$", re.MULTILINE)
CALL = re.compile(r'note_deviation\(\s*"([^"]+)"')


# Any: deviations.json is a deserialized document (STD-001-PY §6).
def departing(document: dict[str, Any]) -> set[str]:
    """Register entries whose decision departs from the specification's BNF."""
    return {
        e["production"] for e in document.get("deviations", []) if e.get("decision") in DEPARTING
    }


def sites(source: str) -> tuple[list[str], list[str]]:
    """(names at `note_deviation` calls, names in `// deviation:` markers), in order."""
    return CALL.findall(source), MARKER.findall(source)


def pending(text: str) -> dict[str, str]:
    """NAME -> reason for each `NAME  # reason` line; blank and `#` lines are skipped."""
    out: dict[str, str] = {}
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        name, _, reason = line.partition("#")
        out[name.strip()] = reason.strip()
    return out


def problems(
    entries: set[str], calls: list[str], markers: list[str], listed: dict[str, str]
) -> list[str]:
    """Every way the sites, the markers, the pending list and the register disagree."""
    sited, marked, pend = set(calls), set(markers), set(listed)
    return [
        *(
            f"note_deviation({n!r}) has no `// deviation: {n}` marker"
            for n in sorted(sited - marked)
        ),
        *(f"`// deviation: {n}` marks no note_deviation call" for n in sorted(marked - sited)),
        *(
            f"{n}: sited, but not a departing entry in {DEVIATIONS}"
            for n in sorted(sited - entries)
        ),
        *(
            f"{n}: listed pending, but not a departing entry in {DEVIATIONS}"
            for n in sorted(pend - entries)
        ),
        *(f"{n}: sited AND listed pending; remove it from {PENDING}" for n in sorted(pend & sited)),
        *(f"{n}: listed pending with no reason" for n, why in sorted(listed.items()) if not why),
        *(
            f"{n}: departs from the specification but has no site and is not listed pending"
            for n in sorted(entries - sited - pend)
        ),
    ]


def main() -> int:
    """Check the sites against the register; print each disagreement."""
    os.chdir(ROOT)
    entries = departing(json.loads(DEVIATIONS.read_text(encoding="utf-8")))
    calls, markers = sites(PARSER.read_text(encoding="utf-8"))
    listed = pending(PENDING.read_text(encoding="utf-8")) if PENDING.is_file() else {}
    found = problems(entries, calls, markers, listed)
    for line in found:
        print(f"  {line}")
    if found:
        return 1
    sited = len(set(calls))
    print(f"deviation sites ok: {sited} sited, {len(listed)} pending, of {len(entries)} departures")
    return 0


if __name__ == "__main__":
    sys.exit(main())
