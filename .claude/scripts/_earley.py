# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Independent validation oracle over the normalized derived grammar.

A lexer derived from the pinned keyword set, and an Earley recognizer.

The point is independence. This shares no code with the Rust parser, so when
both accept the same corpus you have two implementations agreeing rather than
one implementation checking itself. Earley because it handles ambiguity and
left recursion without the grammar needing to be massaged into LL form — the
derived grammar must describe the language, not a parsing strategy.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Callable, Iterable

# A token is ("kw", text) or ("tok", kind, text).
Token = tuple[str, ...]
# A symbol is a keyword or token class to scan, or a nonterminal ("nt") to predict.
Symbol = tuple[str, str]
Productions = dict[str, list[list[Symbol]]]
# An Earley item: production, alternative index, dot position, origin.
Item = tuple[str, int, int, int]

#: Trivia, per KerML 8.2.2.1-8.2.2.2: whitespace and the two note forms. A REGULAR_COMMENT
#: (`/* ... */`) is NOT trivia. It is the body of a Comment element, and a bare one is a
#: Comment in the model, so it is a token the grammar must see.
_SKIPPED = ("WS", "ML_NOTE", "SL_NOTE")
MAX_STATES = 400_000


def make_lexer(keywords: Iterable[str], operators: Iterable[str]) -> Callable[[str], list[Token]]:
    """A lexer that classifies names as keywords using the pinned keyword set."""
    # Longest-first so ':>>' wins over ':>' and ':'.
    ops = sorted(operators, key=len, reverse=True)
    kws = set(keywords)
    op_re = "|".join(re.escape(o) for o in ops) if ops else r"(?!)"
    # Token kinds carry the specification's terminal names (KerML 8.2.2), because a derived
    # rule scans {k: tok, name: DECIMAL_VALUE} and the recognizer compares names exactly.
    # Order is precedence among alternatives that can start at the same character:
    # ML_NOTE `//*` before SL_NOTE `//`; EXPONENTIAL_VALUE before DECIMAL_VALUE so `2e3` is
    # one token. A real's `.` is not part of any number token: RealValue is
    # DECIMAL_VALUE? '.' ( DECIMAL_VALUE | EXPONENTIAL_VALUE ), so `2.5` is three tokens
    # and `1..5` is DECIMAL_VALUE '..' DECIMAL_VALUE.
    master = re.compile(
        r"(?P<WS>\s+)"
        r"|(?P<ML_NOTE>//\*.*?\*/)"
        r"|(?P<SL_NOTE>//[^\n\r]*)"
        r"|(?P<REGULAR_COMMENT>/\*.*?\*/)"
        r"|(?P<STRING_VALUE>\"(?:[^\"\\]|\\.)*\")"
        r"|(?P<NAME>[A-Za-z_][A-Za-z0-9_]*|'(?:[^'\\]|\\.)*')"
        r"|(?P<EXPONENTIAL_VALUE>[0-9]+[eE][+-]?[0-9]+)"
        r"|(?P<DECIMAL_VALUE>[0-9]+)"
        r"|(?P<OP>" + op_re + r")",
        re.DOTALL,
    )

    def lex(text: str) -> list[Token]:
        tokens: list[Token] = []
        pos = 0
        while pos < len(text):
            m = master.match(text, pos)
            if not m:
                msg = f"lex error at offset {pos}: {text[pos : pos + 24]!r}"
                raise ValueError(msg)
            pos = m.end()
            kind = m.lastgroup or ""
            if kind in _SKIPPED:
                continue
            value = m.group()
            if kind == "OP" or (kind == "NAME" and value in kws):
                tokens.append(("kw", value))
            else:
                tokens.append(("tok", kind, value))
        return tokens

    return lex


def nullable_nonterminals(prods: Productions) -> frozenset[str]:
    """Every nonterminal that can derive the empty string, by fixed point."""
    nullable: set[str] = set()
    changed = True
    while changed:
        changed = False
        for name, alts in prods.items():
            if name in nullable:
                continue
            if any(all(k == "nt" and v in nullable for k, v in alt) for alt in alts):
                nullable.add(name)
                changed = True
    return frozenset(nullable)


@dataclass(slots=True)
class _Chart:
    """Earley state sets for one recognition; lives for one ``recognize`` call."""

    prods: Productions
    tokens: list[Token]
    sets: list[set[Item]]
    nullable: frozenset[str]

    def add(self, i: int, agenda: list[Item], item: Item) -> None:
        if item not in self.sets[i]:
            self.sets[i].add(item)
            agenda.append(item)

    def complete(self, i: int, agenda: list[Item], done: Item) -> None:
        name, _, _, origin = done
        for n2, a2, d2, o2 in list(self.sets[origin]):
            seq = self.prods[n2][a2]
            if d2 < len(seq) and seq[d2] == ("nt", name):
                self.add(i, agenda, (n2, a2, d2 + 1, o2))

    def step(self, i: int, agenda: list[Item], item: Item) -> str | None:
        """Apply complete, predict, or scan to one item. Returns an error message, if any."""
        name, alt_i, dot, origin = item
        alt = self.prods[name][alt_i]
        if dot == len(alt):
            self.complete(i, agenda, item)
            return None
        kind, value = alt[dot]
        if kind == "nt":
            if value not in self.prods:
                return f"undefined nonterminal {value!r}"
            for a2 in range(len(self.prods[value])):
                self.add(i, agenda, (value, a2, 0, i))
            # Aycock-Horspool: a nullable nonterminal is also skipped over directly.
            # Relying on completion alone misses it when the same nonterminal was
            # already predicted and completed at this position, so `A -> B B` with
            # nullable B never finished.
            if value in self.nullable:
                self.add(i, agenda, (name, alt_i, dot + 1, origin))
            return None
        if i < len(self.tokens) and self.tokens[i][0] == kind and self.tokens[i][1] == value:
            self.sets[i + 1].add((name, alt_i, dot + 1, origin))
        return None

    def verdict(self, start: str) -> tuple[bool, str]:
        n = len(self.tokens)
        for alt_i, alt in enumerate(self.prods[start]):
            if (start, alt_i, len(alt), 0) in self.sets[n]:
                return True, "ok"
        # Report the furthest position reached — far more useful than "failed".
        furthest = max((i for i, c in enumerate(self.sets) if c), default=0)
        if furthest < n:
            t = self.tokens[furthest]
            got = t[1] if t[0] == "kw" else f"{t[1]}({t[2]})"
            return False, f"no parse; stopped at token {furthest}/{n} near {got!r}"
        return False, f"no parse; consumed all {n} tokens but no complete {start}"


def recognize(
    prods: Productions, start: str, tokens: list[Token], max_states: int = MAX_STATES
) -> tuple[bool, str]:
    """Earley recognition of ``tokens`` from ``start``. Returns (accepted, reason)."""
    if start not in prods:
        return False, f"start symbol {start!r} is not defined"

    sets: list[set[Item]] = [set() for _ in range(len(tokens) + 1)]
    chart = _Chart(prods, tokens, sets, nullable_nonterminals(prods))
    for alt_i in range(len(prods[start])):
        chart.sets[0].add((start, alt_i, 0, 0))

    total = 0
    for i in range(len(tokens) + 1):
        agenda = list(chart.sets[i])
        while agenda:
            item = agenda.pop()
            total += 1
            if total > max_states:
                return False, "state explosion — grammar is likely pathologically ambiguous"
            error = chart.step(i, agenda, item)
            if error:
                return False, error
    return chart.verdict(start)
