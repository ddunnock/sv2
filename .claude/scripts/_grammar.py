# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Shared grammar-workflow helpers: fingerprints, the unit store, AST walking, EBNF rendering."""

from __future__ import annotations

import hashlib
import json
import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import TYPE_CHECKING

from _state import load_json

if TYPE_CHECKING:
    from _earley import Productions, Symbol
    from _state import Json

GRAMMAR = Path(".claude/state/grammar")
UNITS = GRAMMAR / "units"
PILOT = Path("vendor/pilot")
START = "RootNamespace"

_LEAF = {"kw": ("kw", "text"), "tok": ("tok", "name"), "ref": ("nt", "name")}
_REPEAT = {"opt": "?", "star": "*", "plus": "+"}


def hash_parts(*parts: str | None) -> str:
    """A NUL-separated sha256 over ``parts``; ``None`` hashes as empty."""
    digest = hashlib.sha256()
    for part in parts:
        digest.update((part or "").encode("utf-8"))
        digest.update(b"\x00")
    return digest.hexdigest()


def load_units() -> dict[str, Json]:
    """Every unit file, keyed by production name, in name order."""
    if not UNITS.exists():
        return {}
    return {p.stem: json.loads(p.read_text()) for p in sorted(UNITS.glob("*.json"))}


def save_unit(unit: Json) -> None:
    """Write one unit back to its file."""
    UNITS.mkdir(parents=True, exist_ok=True)
    (UNITS / f"{unit['production']}.json").write_text(json.dumps(unit, indent=2) + "\n")


def pinned_tokens() -> tuple[list[str], list[str]]:
    """(keywords, operators) from the pinned token set; both empty when it is absent."""
    tokens = load_json(GRAMMAR / "keywords.json", {"keywords": [], "operators": []})
    return tokens.get("keywords", []), tokens.get("operators", [])


def xtext_rule_text(name: str) -> tuple[str | None, str | None]:
    """(file name, raw text) of one Xtext rule, from the pinned grammars. Input, not truth."""
    head = rf"^(?:fragment\s+|enum\s+|terminal\s+)?{re.escape(name)}\b"
    for path in sorted(PILOT.glob("*.xtext")):
        text = path.read_text(errors="replace")
        # [^:\n]* was wrong: a qualified return type (SysML::Package) contains
        # colons, so every rule with one silently produced no text. Match to the
        # trailing colon at end of line instead, or an inline one-line rule.
        m = re.search(head + r"[^\n]*:\s*$", text, re.MULTILINE) or re.search(
            head + r"[^\n]*?:\s*\S[^\n]*$", text, re.MULTILINE
        )
        if not m:
            continue
        rest = text[m.start() :]
        # An inline rule (`Name : NAME ;`) is one line. Scanning to the next
        # "\n;" would swallow every following rule up to the next terminator,
        # so its captured text changed whenever an unrelated rule was appended —
        # producing a false "inputs moved" on rebase and needless re-derivation.
        if not m.group(0).rstrip().endswith(":"):
            return path.name, rest.split("\n", 1)[0].rstrip()
        end = rest.find("\n;")
        return path.name, (rest[: end + 2] if end != -1 else rest[:4000])
    return None, None


# ---- rule AST ----------------------------------------------------------------


def _collect(node: Json, kind: str, attr: str) -> set[str]:
    out: set[str] = set()

    def walk(n: Json) -> None:
        if not isinstance(n, dict):
            return
        if n.get("k") == kind and n.get(attr):
            out.add(n[attr])
        for child in n.get("items") or []:
            walk(child)
        for key in ("item", "sep"):
            if n.get(key):
                walk(n[key])

    walk(node)
    return out


def refs_of(node: Json) -> set[str]:
    """Every production this rule references."""
    return _collect(node, "ref", "name")


def kws_of(node: Json) -> set[str]:
    """Every keyword or operator literal this rule uses."""
    return _collect(node, "kw", "text")


def _render(n: Json, *, top: bool = False) -> str:
    k = n.get("k")
    if k == "kw":
        return "'" + str(n["text"]).replace("'", "\\'") + "'"
    if k in ("tok", "ref"):
        return str(n["name"])
    if k == "seq":
        return " ".join(_render(x) for x in n.get("items", []))
    if k == "alt":
        body = " | ".join(_render(x) for x in n.get("items", []))
        return body if top else "( " + body + " )"
    inner = _render(n["item"])
    if n["item"].get("k") in ("seq", "alt") and not inner.startswith("("):
        inner = "( " + inner + " )"
    suffix = _REPEAT.get(k or "")
    if suffix is None:
        return "?"
    return inner + suffix


def render_ebnf(name: str, node: Json) -> str:
    """Deterministic EBNF text rendered from the AST.

    Never stored — a second stored form is a second thing that drifts.
    """
    return f"{name} ::= {_render(node, top=True)} ;"


@dataclass(slots=True)
class _Desugarer:
    """Builds plain BNF productions; lives for one ``normalize`` call."""

    prods: Productions = field(default_factory=dict)
    counter: int = 0

    def symbol(self, n: Json) -> Symbol:
        k = n.get("k")
        leaf = _LEAF.get(k)
        if leaf is not None:
            return (leaf[0], n[leaf[1]])
        if k not in ("seq", "alt", *_REPEAT):
            msg = f"unknown node kind {k!r}"
            raise ValueError(msg)
        self.counter += 1
        generated = f"__{k}{self.counter}"
        self.prods[generated] = self._alternatives(k, n, generated)
        return ("nt", generated)

    def _alternatives(self, k: str, n: Json, generated: str) -> list[list[Symbol]]:
        if k == "seq":
            return [[self.symbol(x) for x in n.get("items", [])]]
        if k == "alt":
            return [[self.symbol(x)] for x in n.get("items", [])]
        if k == "opt":
            return [[], [self.symbol(n["item"])]]
        if k == "star":
            return [[], [self.symbol(n["item"]), ("nt", generated)]]
        # plus desugars its item once per occurrence, so a compound item yields two
        # generated productions. Hoisting the call would renumber every later name.
        return [[self.symbol(n["item"])], [self.symbol(n["item"]), ("nt", generated)]]

    def define(self, name: str, rule: Json) -> None:
        """Add a named top-level production; a top-level seq or alt is not wrapped."""
        k = rule.get("k")
        if k in ("seq", "alt"):
            self.prods[name] = self._alternatives(k, rule, name)
        else:
            self.prods[name] = [[self.symbol(rule)]]


def normalize(units: dict[str, Json]) -> Productions:
    """Desugar the AST into plain BNF productions for the Earley oracle.

    opt/star/plus/alt become generated nonterminals; the oracle then only ever
    sees sequences of symbols.
    """
    desugarer = _Desugarer()
    for name, unit in units.items():
        rule = unit.get("rule")
        if rule:
            desugarer.define(name, rule)
    return desugarer.prods
