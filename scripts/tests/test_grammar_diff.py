# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Which Xtext rule kinds take part in the differential.

The names below are the ones the pinned grammars actually declare: STRING_VALUE and
UNRESTRICTED_NAME are `terminal` in vendor/pilot/KerMLExpressions.xtext and ordinary
productions in vendor/spec-bnf/KerML-textual-bnf.kebnf, and WS is a terminal the
specification BNF does not name at all.
"""

import pytest

from grammar_diff import XTEXT_KINDS, xtext_names

INVENTORY = [
    {"name": "PartDefinition", "kind": "rule"},
    {"name": "Identification", "kind": "fragment"},
    {"name": "PortionKind", "kind": "enum"},
    {"name": "STRING_VALUE", "kind": "terminal"},
    {"name": "WS", "kind": "terminal"},
]


def test_every_declared_kind_takes_part():
    assert xtext_names(INVENTORY) == {
        "PartDefinition",
        "Identification",
        "PortionKind",
        "STRING_VALUE",
        "WS",
    }


@pytest.mark.parametrize("kind", ["rule", "fragment", "enum", "terminal"])
def test_no_kind_is_silently_dropped(kind):
    assert xtext_names([{"name": "X", "kind": kind}]) == {"X"}


def test_terminals_are_compared_rather_than_dropped():
    # The defect this guards: with terminals excluded, STRING_VALUE was reported as
    # "specification only" although the Xtext declares it. A false difference is
    # worse than a real one — it sends a reviewer looking for a drift that is not
    # there, and it hides that the two sources actually agree.
    spec = {"PartDefinition", "STRING_VALUE"}
    assert spec - xtext_names(INVENTORY) == set()


def test_an_unknown_kind_is_excluded_rather_than_assumed():
    # A kind the extractor learns to emit later must not join the comparison just by
    # appearing; it joins when it is added to XTEXT_KINDS deliberately.
    assert xtext_names([{"name": "Later", "kind": "something-new"}]) == set()
    assert "something-new" not in XTEXT_KINDS
