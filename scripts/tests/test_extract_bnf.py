# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Expectations read off the transcription in data/sample.kebnf, which is copied
verbatim from the pinned vendor/spec-bnf/SysML-textual-bnf.kebnf."""

from pathlib import Path

import pytest

from extract_bnf import HEAD, build_artifact, extract

DATA = Path(__file__).parent / "data"


@pytest.fixture
def rules():
    return {r["name"]: r for r in extract([DATA / "sample.kebnf"])}


def test_every_production_in_the_excerpt_is_found(rules):
    assert set(rules) == {
        "UsageElement",
        "ItemDefinition",
        "PartDefinition",
        "PartUsage",
        "PortDefinition",
        "ConjugatedPortDefinitionMember",
        "PortConjugation",
    }


@pytest.mark.parametrize(
    ("name", "metaclass"),
    [
        # "UsageElement : Usage =" declares the metaclass it constructs.
        ("UsageElement", "Usage"),
        ("ConjugatedPortDefinitionMember", "OwningMembership"),
        # "PartDefinition =" declares none, and an empty string is not "Usage".
        ("PartDefinition", ""),
        ("PortConjugation", ""),
    ],
)
def test_metaclass_is_read_only_when_the_rule_declares_one(rules, name, metaclass):
    assert rules[name]["metaclass"] == metaclass


@pytest.mark.parametrize(
    ("name", "clause"),
    [
        ("UsageElement", "8.2.2.5.2"),
        ("ItemDefinition", "8.2.2.10"),
        ("PartDefinition", "8.2.2.11"),
        # PartUsage carries no marker of its own; it inherits the one above it.
        ("PartUsage", "8.2.2.11"),
        ("PortDefinition", "8.2.2.12"),
        # "// (See Note 1)" sits between these two and is not a clause marker, so
        # the citation must still be 8.2.2.12 rather than empty.
        ("ConjugatedPortDefinitionMember", "8.2.2.12"),
    ],
)
def test_clause_is_the_nearest_marker_above_the_rule(rules, name, clause):
    assert rules[name]["clause"] == clause


def test_clause_title_is_kept(rules):
    assert rules["PartDefinition"]["clause_title"] == "Parts Textual Notation"


def test_body_keeps_the_abstract_syntax_action_block(rules):
    # The { ... } action is how the rule builds the metaclass; dropping it would
    # lose the only statement of what PortDefinition's conjugation does.
    assert rules["PortDefinition"]["body"] == (
        "    DefinitionPrefix 'port' 'def' Definition\n"
        "    ownedRelationship += ConjugatedPortDefinitionMember\n"
        "    { conjugatedPortDefinition.ownedPortConjugator.\n"
        "        originalPortDefinition = this }"
    )


def test_body_of_an_alternation_keeps_every_alternative(rules):
    assert rules["UsageElement"]["body"] == (
        "      NonOccurrenceUsageElement\n    | OccurrenceUsageElement"
    )


def test_body_excludes_the_comment_that_follows_the_rule(rules):
    # "// (See Note 1)" and the blank lines around it belong to the gap between
    # PortDefinition and the next head, not to PortDefinition's body.
    assert "See Note 1" not in rules["PortDefinition"]["body"]


def test_line_numbers_point_at_the_head(rules):
    lines = (DATA / "sample.kebnf").read_text().splitlines()
    for rule in rules.values():
        assert lines[rule["line"] - 1].startswith(rule["name"])


@pytest.mark.parametrize(
    "line",
    [
        "    | OccurrenceUsageElement",  # an indented alternative, not a head
        "// Clause 8.2.2.11 Parts Textual Notation",
        "      NonOccurrenceUsageElement",
        "",
        "    ownedRelationship += ConjugatedPortDefinitionMember",
    ],
)
def test_continuation_and_comment_lines_are_not_production_heads(line):
    assert not (line[:1].strip() and HEAD.match(line))


def test_productions_list_is_the_sorted_unique_names(rules):
    artifact = build_artifact([DATA / "sample.kebnf"], extract([DATA / "sample.kebnf"]))
    assert artifact["productions"] == sorted(rules)
    assert artifact["counts"]["with_metaclass"] == 2
    assert artifact["counts"]["with_clause"] == len(rules)
