# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Export the specification clause text the derivation reads, from the built wikis.

`grammar_next.py` puts `spec_clause_text` in every context pack, and phase 3 of the
`derive-grammar` skill treats it as **the source**. It therefore has to be the
specification clause, not the Tier B' transcription: DERIVATION.md says a
transcribed body is "a starting draft and a citation to check, not an answer", and
a unit is established only when its clause has been read and agrees. Feeding the
transcription in as the clause would have the derivation check Tier B' against
itself and silently turn every "proposed" body into an "established" one.

The clause text lives in the built LLM wikis, whose atoms are pinned by
`region_sha256` to spans of the specification PDFs and verified against
`.claude/state/wiki-receipts.json` by the gate.

**The output does not belong in the repository.** It carries verbatim OMG
specification prose, and this repository is MIT and public; that is the same
constraint that keeps the wikis themselves out (state.json, "wiki-reproducibility").
The default output path is outside the tree, and `--out` may not name one inside it.

    python3.12 .claude/scripts/export_wiki_clauses.py
    python3.12 .claude/scripts/export_wiki_clauses.py --out ~/somewhere/clauses.json

Then point the pipeline at it:

    export SV2_WIKI_CLAUSES=~/.sv2-derivation/bnf-clauses.json

Network: none. It reads two local directories and one derived index.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sqlite3
import sys
from pathlib import Path

from _grammar import defective_productions

ROOT = Path(__file__).resolve().parents[2]
INVENTORY = ROOT / ".claude/state/grammar/bnf-productions.json"
DEFAULT_OUT = Path.home() / ".sv2-derivation/bnf-clauses.json"
FRONTMATTER = re.compile(r"\A---\n(.*?)\n---\n", re.DOTALL)
RECEIPT = re.compile(r"^region_sha256:\s*([0-9a-f]{64})\s*$", re.MULTILINE)

#: Where each wiki is, in the order the navigator skills document.
WIKIS = {
    "sysml": (os.environ.get("SYSML_WIKI"), Path.home() / ".sysml-v2-llm-wiki/wiki"),
    "kerml": (os.environ.get("KERML_WIKI"), Path.home() / ".kerml-llm-wiki/wiki"),
}


def wiki_dir(tag: str) -> Path:
    """The directory for one wiki, or exit saying which one is missing."""
    override, fallback = WIKIS[tag]
    path = Path(override) if override else fallback
    if not (path / "graph.sqlite").is_file():
        sys.exit(f"{tag} wiki not found at {path} — set {tag.upper()}_WIKI")
    return path


def atoms_by_clause(tag: str) -> dict[tuple[str, str], Path]:
    """Every production atom in one wiki, keyed by (language, clause).

    Keyed by both, because the two specifications number their clauses
    independently: KerML 8.2.2.7 is Symbols and SysML 8.2.2.7 is not, so a map keyed
    by the number alone silently hands SysML productions KerML clause text wherever
    the numbers collide.
    """
    root = wiki_dir(tag)
    out: dict[tuple[str, str], Path] = {}
    with sqlite3.connect(root / "graph.sqlite") as db:
        rows = db.execute("select clause, path from atoms where kind = 'production'")
        for clause, path in rows:
            if clause:
                out.setdefault((tag, clause), root / path)
    return out


def clause_body(path: Path) -> tuple[str, str]:
    """The atom's text with its frontmatter removed, and its receipt."""
    raw = path.read_text()
    receipt = ""
    if m := RECEIPT.search(raw):
        receipt = m.group(1)
    return FRONTMATTER.sub("", raw).strip(), receipt


def language_of(rule: dict[str, str]) -> str:
    """Which specification a transcribed rule came from, by its Tier B' file."""
    return "sysml" if "SysML" in rule.get("file", "") else "kerml"


def build(
    inventory: list[dict[str, str]], atoms: dict[tuple[str, str], Path]
) -> tuple[dict[str, dict[str, str]], list[str]]:
    """The {production: {ref, text}} export, and the productions whose clause lacks them."""
    by_name: dict[str, list[dict[str, str]]] = {}
    for rule in inventory:
        by_name.setdefault(rule["name"], []).append(rule)

    export: dict[str, dict[str, str]] = {}
    absent: list[str] = []
    for name, rules in sorted(by_name.items()):
        refs: list[str] = []
        texts: list[str] = []
        for rule in rules:
            clause = rule["clause"]
            tag = language_of(rule)
            path = atoms.get((tag, clause))
            if not path:
                continue
            text, receipt = clause_body(path)
            refs.append(f"{tag} clause {clause} ({path.name}, region_sha256 {receipt[:16]})")
            texts.append(f"=== {tag} clause {clause} ===\n{text}")
        if not refs:
            absent.append(name)
            continue
        # Both readings travel together when a production is stated in both languages.
        # Choosing one here would be arbitrating, which is phase 3's job and not this
        # script's; the deriver needs to see that there are two.
        export[name] = {"ref": " | ".join(refs), "text": "\n\n".join(texts)}
    return export, absent


def unnamed(export: dict[str, dict[str, str]]) -> list[str]:
    """Productions whose own name does not appear in the clause text exported for them.

    Tier B' supplies the clause number printed beside each production. A wrong number
    points a production at a clause that does not define it, which is a transcription
    error rather than a language fact, and it would otherwise reach phase 3 disguised
    as a clause the deriver could not make sense of.
    """
    return sorted(name for name, entry in export.items() if name not in entry["text"])


def report_malformed(export: dict[str, dict[str, str]]) -> None:
    """Name each production whose own clause text does not parse as grammar.

    A clause atom states many productions and one stray bracket belongs to one of
    them, so the defect is reported against that production and not against every
    production its clause happens to define. This is a report, not a failure: the
    defects are upstream of this repository, and the derivation repairs them per
    unit, with the repair recorded there.
    """
    found: dict[str, tuple[str, list[str]]] = {}
    for entry in export.values():
        for name, defects in defective_productions(entry["text"]).items():
            found.setdefault(name, (entry["ref"].split(" | ")[0], defects))
    if not found:
        return
    print(f"  MALFORMED grammar in {len(found)} production(s):")
    for name, (ref, defects) in sorted(found.items()):
        print(f"    {name} — {'; '.join(defects)}")
        print(f"      {ref}")
    print("    Derive these against the Tier B' transcription and the Xtext, and record")
    print("    the repair on the unit. grammar_next.py repeats this for the production.")


def main(argv: list[str] | None = None) -> int:
    """Write the clause export and report what it could not cover."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT, help="where to write")
    args = parser.parse_args(argv)

    out = args.out.expanduser().resolve()
    if out.is_relative_to(ROOT):
        sys.exit(
            f"refusing to write inside the repository: {out}\n"
            "the export is verbatim OMG specification prose and this repository is "
            "MIT and public (state.json, wiki-reproducibility)"
        )

    inventory = json.loads(INVENTORY.read_text())["rules"]
    atoms = atoms_by_clause("sysml") | atoms_by_clause("kerml")  # disjoint: keyed by language
    export, absent = build(inventory, atoms)

    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(export, indent=2, sort_keys=True) + "\n")

    print(f"wrote {out} ({len(export)} productions from {len(atoms)} clause atoms)")
    if absent:
        print(f"  no clause atom for {len(absent)}: {', '.join(absent[:8])}")
    if mismatched := unnamed(export):
        print(f"  name absent from its own clause text for {len(mismatched)}:")
        for name in mismatched[:12]:
            print(f"    {name} — {export[name]['ref']}")
    report_malformed(export)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
