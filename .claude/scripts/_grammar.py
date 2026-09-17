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

#: Fenced blocks inside a clause atom. Not every one holds grammar — a clause may
#: also carry an OCL constraint or an operator-to-library table.
_FENCE = re.compile(r"```[a-z]*\n(.*?)```", re.S)
#: A block is grammar if it defines at least one rule: `Name =` or `Name : Metaclass =`.
#: Without this, the operator table in 8.2.5.8.2 reads as an unclosed `[`, because
#: there the bracket IS the operator being described.
_DEFINES = re.compile(r"^\s*[A-Za-z_][A-Za-z0-9_]*\s*(?::\s*[A-Za-z_][A-Za-z0-9_:]*\s*)?=", re.M)
#: A single-quoted literal. Stripped before counting brackets, because `'('` and
#: `'['` are real keywords of the language.
_QUOTED = re.compile(r"'(?:[^'\n])*'")
_PAIRS = {"(": ")", "[": "]", "{": "}"}
_CLOSERS = {close: open_ for open_, close in _PAIRS.items()}


def _block_defects(block: str) -> list[str]:
    defects = []
    if block.count("'") % 2:
        defects.append("an odd number of ' — a literal is not terminated")
    stack: list[tuple[str, int]] = []
    for number, line in enumerate(_QUOTED.sub("", block).splitlines(), 1):
        for char in line:
            if char in _PAIRS:
                stack.append((char, number))
            elif char in _CLOSERS:
                if stack and stack[-1][0] == _CLOSERS[char]:
                    stack.pop()
                else:
                    defects.append(f"an unmatched {char!r} on line {number}")
                    return defects
    if stack:
        char, number = stack[0]
        defects.append(f"an unclosed {char!r} opened on line {number}")
    return defects


#: The head of a production inside a grammar block: a capitalised name followed by
#: `=` or by `:` (a metaclass). `::` and `:=` are not heads. The clauses write heads
#: several broken ways — `Name : Meta` with no `=`, `Name :` with no metaclass — so
#: the head is recognised by its name and its first separator, not by a well-formed `=`.
_HEAD = re.compile(r"^\s*([A-Z]\w*)\s*(?:=|:(?![:=]))")


def split_productions(block: str) -> list[tuple[str, str]]:
    """(name, text) for each production written in one grammar block, in order."""
    found: list[tuple[str, list[str]]] = []
    for line in block.splitlines():
        head = _HEAD.match(_QUOTED.sub("''", line))
        if head:
            found.append((head.group(1), [line]))
        elif found:
            found[-1][1].append(line)
    return [(name, "\n".join(lines)) for name, lines in found]


def defective_productions(text: str) -> dict[str, list[str]]:
    """Each production in a clause whose OWN text is malformed, with its defects.

    A clause atom states many productions, and one stray bracket belongs to one of
    them. Reported per clause, KerML 8.2.4.3.1's extra `)` flagged 24 productions
    when only Feature carries it; reported here, it flags Feature.
    """
    found: dict[str, list[str]] = {}
    for block in _FENCE.findall(text):
        if not _DEFINES.search(block) or not _block_defects(block):
            continue
        for name, body in split_productions(block):
            if defects := _block_defects(body):
                found.setdefault(name, []).extend(defects)
    return found


def clause_defects(text: str) -> list[str]:
    """Structural defects in a clause's grammar blocks, worst first, or empty.

    The clause text is the SOURCE a unit is derived from, so a malformed one is
    worth saying out loud rather than leaving to be noticed. Five of the 111
    pinned atoms carry one: two have a stray bracket and three an unterminated
    literal. Whether the defect is in the OMG document or in the wiki's
    extraction from the PDF is not determined here, and this says only that the
    text as exported does not parse as grammar.
    """
    return [
        d for block in _FENCE.findall(text) if _DEFINES.search(block) for d in _block_defects(block)
    ]


def hash_parts(*parts: str | None) -> str:
    """A NUL-separated sha256 over ``parts``; ``None`` hashes as empty."""
    digest = hashlib.sha256()
    for part in parts:
        digest.update((part or "").encode("utf-8"))
        digest.update(b"\x00")
    return digest.hexdigest()


# ---- two languages ----------------------------------------------------------
#
# KerML and SysML are sibling grammars that share a vocabulary, not one grammar:
# SysML.xtext extends KerMLExpressions, not KerML.xtext (ADR-0010), and the
# specification gives some production names a different body in each language —
# RootNamespace is NamespaceBodyElement* in KerML and PackageBodyElement* in SysML.
# A unit is therefore either SHARED, visible to both grammars, or a VARIANT scoped
# to one of them. ADR-0014.

#: A terminal in the specification BNF is written in SCREAMING_SNAKE. Terminals come
#: from the lexer, not from derivation, and a rule names one as {k: tok}, never as a ref.
TERMINAL_NAME = re.compile(r"^[A-Z][A-Z0-9_]*$")


def referable_productions(inventory: Json) -> list[str]:
    """The productions a rule may name as {k: ref}: the specification inventory's non-terminals.

    The specification's inventory, not the pilot's, for the reason grammar_plan.py plans
    from it: the Xtext omits productions the language has and adds ones it does not.
    """
    return sorted(n for n in inventory.get("productions", []) if not TERMINAL_NAME.match(n))


SCOPES = ("kerml", "sysml")
#: Which grammar reads a model file.
SCOPE_OF_SUFFIX = {".kerml": "kerml", ".sysml": "sysml"}
#: Pilot grammar files each scope's Xtext rules come from. SysML extends only the
#: expression grammar, so KerML.xtext is not a second opinion for a SysML variant.
_XTEXT_FILES = {
    "kerml": ("KerML.xtext", "KerMLExpressions.xtext"),
    "sysml": ("SysML.xtext", "KerMLExpressions.xtext"),
}
#: An assignment prefix in a BNF body: `name =`, `name +=`, `name ?=`. Where the two
#: languages differ only in these they assign to different slots but accept the same text.
_ASSIGNMENT = re.compile(r"\w+\s*(?:\+=|\?=|=)\s*")


def unit_key(unit: Json) -> str:
    """The identity of a unit: its production, qualified by scope for a variant."""
    scope = unit.get("scope")
    return f"{unit['production']}@{scope}" if scope else str(unit["production"])


def load_units() -> dict[str, Json]:
    """Every unit file, keyed by unit_key, in key order.

    Refuses a file whose name disagrees with its content: the file name is how a
    unit is found and the content is what it claims to be, and a mismatch would let
    a variant shadow, or be shadowed by, the wrong unit without anything noticing.
    """
    if not UNITS.exists():
        return {}
    units: dict[str, Json] = {}
    for path in sorted(UNITS.glob("*.json")):
        unit = json.loads(path.read_text())
        if (key := unit_key(unit)) != path.stem:
            msg = f"{path} holds unit {key!r}; its file name must be {key}.json"
            raise ValueError(msg)
        units[key] = unit
    return units


def save_unit(unit: Json) -> None:
    """Write one unit to the file its own content names."""
    UNITS.mkdir(parents=True, exist_ok=True)
    (UNITS / f"{unit_key(unit)}.json").write_text(json.dumps(unit, indent=2) + "\n")


def grammar_view(units: dict[str, Json], scope: str) -> dict[str, Json]:
    """One language's grammar: live shared units, with that scope's variants in place.

    Keyed by production name, which is what rules reference. Retired units are not
    part of any grammar.
    """
    view: dict[str, Json] = {}
    for unit in units.values():
        if unit.get("status") == "retired" or unit.get("scope") not in (None, scope):
            continue
        name = unit["production"]
        if unit.get("scope") or name not in view:
            view[name] = unit
    return view


def shadowed(units: dict[str, Json]) -> list[str]:
    """Productions with a live shared unit AND a live variant — the plan retires one."""
    live = [u for u in units.values() if u.get("status") != "retired"]
    variants = {u["production"] for u in live if u.get("scope")}
    return sorted({u["production"] for u in live if not u.get("scope")} & variants)


def _body_shape(body: str) -> str:
    return re.sub(r"\s+", "", _ASSIGNMENT.sub("", body))


def divergent_productions(rules: list[Json]) -> set[str]:
    """Names the Tier B' inventory defines in BOTH languages with different bodies.

    Deterministic, so the plan decides which units split and no derivation does.
    Assignments and whitespace are ignored — they change which slot a value lands in,
    not what parses. A body that differs only in how it is factored still splits:
    judging two factorings equivalent is derivation work, and each variant is cheap.
    """
    shapes: dict[str, dict[str, set[str]]] = {}
    for rule in rules:
        scope = "kerml" if str(rule["file"]).startswith("KerML") else "sysml"
        shapes.setdefault(rule["name"], {}).setdefault(scope, set()).add(_body_shape(rule["body"]))
    return {
        name
        for name, bodies in shapes.items()
        if set(bodies) == set(SCOPES) and bodies["kerml"] != bodies["sysml"]
    }


def clause_for_scope(entry: Json, scope: str | None) -> tuple[str, str]:
    """(ref, text) of a clause export entry, narrowed to one language.

    The export carries both languages' clauses together when a production is stated in
    both. A shared unit sees both; a variant sees only its own, because the other
    language's body is by definition not the one it is deriving.
    """
    ref, text = str(entry.get("ref", "")), str(entry.get("text", ""))
    if scope is None:
        return ref, text
    refs = [r for r in ref.split(" | ") if r.startswith(f"{scope} clause")]
    sections = re.split(r"\n\n(?==== (?:kerml|sysml) clause )", text)
    mine = [s for s in sections if s.startswith(f"=== {scope} clause")]
    return " | ".join(refs), "\n\n".join(mine)


def pinned_tokens() -> tuple[list[str], list[str]]:
    """(keywords, operators) from the pinned token set; both empty when it is absent."""
    tokens = load_json(GRAMMAR / "keywords.json", {"keywords": [], "operators": []})
    return tokens.get("keywords", []), tokens.get("operators", [])


def xtext_rule_text(name: str, scope: str | None = None) -> tuple[str | None, str | None]:
    """(file name, raw text) of one Xtext rule, from the pinned grammars. Input, not truth.

    With a scope, only that language's grammar files are searched, so a SysML variant
    is never handed KerML.xtext's rule of the same name as its second opinion.
    """
    head = rf"^(?:fragment\s+|enum\s+|terminal\s+)?{re.escape(name)}\b"
    paths = sorted(PILOT.glob("*.xtext"))
    if scope is not None:
        paths = [PILOT / f for f in _XTEXT_FILES[scope] if (PILOT / f).exists()]
    for path in paths:
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
