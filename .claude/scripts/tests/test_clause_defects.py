# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""The clause text is the source a unit is derived from.

Five of the pinned atoms are malformed — a stray bracket in KerML 8.2.4.3.1 and
8.2.5.9.2, an unterminated literal in three more — and that was found by accident,
part way through deriving from one of them. These tests cover the detector that
now says so up front, and in particular the two ways it could be useless: missing
a real defect, or crying wolf on a clause that is fine.
"""

from _grammar import clause_defects, defective_productions, split_productions


def ebnf(body):
    return f"# Something\n\n```ebnf\n{body}\n```\n"


def test_a_well_formed_clause_has_no_defects():
    assert clause_defects(ebnf("Feature =\n    ( A | B )\n    C?\n")) == []


def test_an_unmatched_closing_bracket_is_reported_with_its_line():
    # KerML 8.2.4.3.1 as pinned: one ')' more than it opens.
    defects = clause_defects(ebnf("Feature =\n    ( A\n      B\n      )\n    | C\n    )\n    D"))
    assert defects == ["an unmatched ')' on line 6"]


def test_an_unclosed_bracket_is_reported_with_the_line_it_opened_on():
    # KerML 8.2.5.9.2 as pinned: PayloadFeature opens a group and never closes it.
    defects = clause_defects(ebnf("PayloadFeature =\n      A\n    | ( B\n      C\n"))
    assert defects == ["an unclosed '(' opened on line 3"]


def test_an_unterminated_literal_is_reported():
    # SysML 8.2.2.17.4 as pinned: `kind = ( 'at | 'after' )`.
    assert clause_defects(ebnf("X =\n    kind = ( 'at | 'after' )")) == [
        "an odd number of ' — a literal is not terminated"
    ]


def test_bracket_literals_are_not_counted_as_brackets():
    # '(' '[' and '{' are real keywords of this language. Counting them would
    # report every production that mentions one.
    assert clause_defects(ebnf("ArgumentList =\n    '(' A? ')'")) == []
    assert clause_defects(ebnf("Sequence =\n    '[' A ']'")) == []


def test_a_block_that_defines_no_production_is_not_grammar_and_is_skipped():
    # KerML 8.2.5.8.2 carries an operator-to-library table where the bracket IS
    # the operator. Reading it as grammar reported an unclosed '[' on 27 units.
    table = "# Operators\n\n```\n[\n     BaseFunctions::'['\n```\n"
    assert clause_defects(table) == []


def test_an_ocl_constraint_alongside_the_grammar_is_skipped():
    text = ebnf("Flow =\n    A B") + "\n```\nowningType.directionOf(x->at(1))\n```\n"
    assert clause_defects(text) == []


def test_every_block_in_a_clause_is_checked_not_only_the_first():
    text = ebnf("Good =\n    A") + ebnf("Bad =\n    ( A")
    assert clause_defects(text) == ["an unclosed '(' opened on line 2"]


def test_nesting_is_tracked_rather_than_counted():
    # A naive depth counter calls this balanced; the brackets interleave.
    assert clause_defects(ebnf("X =\n    ( A ]")) == ["an unmatched ']' on line 2"]


# ---- locating the defect -------------------------------------------------------


def test_a_defect_is_charged_to_the_production_that_contains_it():
    # KerML 8.2.4.3.1 as pinned: the stray ')' is Feature's, and its sibling in the
    # same clause is well-formed. Charged per clause, it flagged 24 productions.
    text = ebnf(
        "Feature =\n    ( A\n      )\n    | B\n    )\n\n"
        "EndFeaturePrefix : Feature =\n    'const'? 'end'"
    )
    assert defective_productions(text) == {"Feature": ["an unmatched ')' on line 5"]}


def test_production_heads_are_found_even_when_written_without_an_equals_sign():
    # KERML11-108 and SYSML21-335 report heads written `Name :` or `Name : Meta`
    # with no `=`. A splitter that required `=` would merge them into their neighbour.
    block = (
        "BasicFeaturePrefix : Feature :\n    'derived'?\n"
        "FeaturePrefix :\n    A\n"
        "PackageMember : OwningMembership\nMemberPrefix\n( A | B )"
    )
    assert [n for n, _ in split_productions(block)] == [
        "BasicFeaturePrefix",
        "FeaturePrefix",
        "PackageMember",
    ]


def test_a_body_line_that_mentions_a_path_or_assignment_is_not_a_head():
    block = "MembershipImport =\n    Name ( '::' '**' )?\n    Other := x"
    assert [n for n, _ in split_productions(block)] == ["MembershipImport"]
