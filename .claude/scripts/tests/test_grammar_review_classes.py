# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Grounding rules for the machine-recorded deviation classes.

The bodies below are copied from the pinned inputs: `ActionKeyword: 'action';` and
`ActionDefKeyword: ActionKeyword 'def';` are in vendor/pilot/SysML.xtext, and
`OccurrenceDefinitionPrefix 'action' 'def' ...` is ActionDefinition in
vendor/spec-bnf/SysML-textual-bnf.kebnf at clause 8.2.2.17.1.
"""

import pytest

from grammar_review_classes import (
    INVENTORIES,
    _is_literal_rule,
    resolve_literals,
    spec_sites,
    strip_comments,
)

BODIES = {
    "ActionKeyword": "'action'",
    "ActionUsageKeyword": "ActionKeyword",
    "ActionDefKeyword": "ActionKeyword 'def'",
    "AdditiveOperator": "'+' | '-'",
    # Not a literal rule: it references a real production.
    "ActionDefinition": "OccurrenceDefinitionPrefix ActionDefKeyword Definition",
    "OccurrenceDefinitionPrefix": "'individual'?",
}

RULES = [
    {
        "name": "ActionDefinition",
        "clause": "8.2.2.17.1",
        "body": "OccurrenceDefinitionPrefix 'action' 'def'\n    DefinitionDeclaration ActionBody",
    },
    {
        "name": "RESERVED_KEYWORD",
        "clause": "8.2.2.1.2",
        # The real one lists every keyword in the language; two suffice here.
        "body": "'action' | 'def' | 'part'",
    },
]


def test_nested_keyword_references_resolve_to_their_literals():
    # ActionDefKeyword is ActionKeyword 'def', so it must reduce to both words in
    # order. Stopping at the direct literal would cite the wrong specification site.
    assert resolve_literals("ActionDefKeyword", BODIES) == ["action", "def"]


def test_a_plain_keyword_rule_resolves_to_one_literal():
    assert resolve_literals("ActionKeyword", BODIES) == ["action"]


def test_self_reference_terminates():
    assert resolve_literals("Loop", {"Loop": "Loop 'x'"}) == ["x"]


@pytest.mark.parametrize(
    ("name", "expected"),
    [
        ("ActionKeyword", True),
        ("ActionDefKeyword", True),
        ("AdditiveOperator", True),
        # A bare reference that resolves to 'action'. Same factoring device as
        # ActionKeyword, written without a literal of its own — it must qualify,
        # or ~20 *UsageKeyword rules go unreviewed over spelling.
        ("ActionUsageKeyword", True),
        # References a real production, so it is not a pure literal rule and must
        # be left for individual derivation rather than recorded by class.
        ("ActionDefinition", False),
    ],
)
def test_only_pure_literal_rules_qualify(name, expected):
    assert _is_literal_rule(name, BODIES[name], BODIES) is expected


def test_a_rule_reaching_no_literal_never_qualifies():
    # Nothing here bottoms out in a literal, so there is no keyword to ground a
    # citation with and the rule must be left alone.
    bodies = {"A": "B", "B": "A"}
    assert _is_literal_rule("A", "B", bodies) is False


def test_the_reserved_word_inventory_is_never_cited_as_a_site():
    # The defect this guards. RESERVED_KEYWORD contains every keyword, so a literal
    # search matches it for any input; citing a word list as the place the grammar
    # uses a keyword is a false citation, and a false one looks checked.
    sites = spec_sites(["action", "def"], RULES)
    assert [s["name"] for s in sites] == ["ActionDefinition"]
    assert "RESERVED_KEYWORD" in INVENTORIES


def test_literals_absent_from_every_body_ground_nothing():
    # WS and friends are character classes; they must return no site so the caller
    # leaves them unreviewed rather than inventing a citation.
    assert spec_sites([" ", "\\t"], RULES) == []


def test_empty_literals_ground_nothing():
    assert spec_sites([], RULES) == []


def test_strip_comments_keeps_literals_that_look_like_comments():
    assert strip_comments("A: '//' ; // tail") == "A: '//' ; "
