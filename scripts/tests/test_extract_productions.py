# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
from pathlib import Path

import pytest

from extract_productions import build_artifacts, extract, strip_comments

DATA = Path(__file__).parent / "data"


@pytest.mark.parametrize(
    ("text", "expected"),
    [
        ('import "http://www.eclipse.org/emf"', 'import "http://www.eclipse.org/emf"'),
        ("A : '/*' -> '*/' ;", "A : '/*' -> '*/' ;"),
        ("a /* one\ntwo */ b", "a \n b"),
        ("a // tail\nb", "a \nb"),
        ("a // no newline at end", "a "),
        (r"Q : '\'' ; // c", r"Q : '\'' ; "),
        ("'unterminated // still literal", "'unterminated // still literal"),
        ("x /* never closed", "x "),
    ],
)
def test_strip_comments_preserves_literals_and_line_numbers(text, expected):
    assert strip_comments(text) == expected


@pytest.fixture
def inventory():
    return extract([DATA / "sample.xtext"])


def test_extract_counts_every_rule_form(inventory):
    kinds = {p["name"]: p["kind"] for p in inventory.productions}
    assert kinds == {
        "RootNamespace": "rule",
        "PackageBody": "rule",
        "Identification": "fragment",
        "Name": "rule",
        "VisibilityKind": "enum",
        "Quote": "rule",
        "ML_NOTE": "terminal",
        "SL_NOTE": "terminal",
        "REGULAR_COMMENT": "terminal",
        "DECIMAL_VALUE": "terminal",
        "UNRESTRICTED_NAME": "terminal",
    }


def test_extract_keeps_qualified_return_types(inventory):
    assert inventory.metamap["KerML::Package"] == ["PackageBody"]


def test_extract_ignores_literals_inside_comments(inventory):
    assert "fake" not in inventory.keywords
    assert "alsofake" not in inventory.keywords


def test_extract_unescapes_quote_literal(inventory):
    assert "'" in inventory.keywords


def test_extract_reads_imports_through_comment_stripping(inventory):
    uris = [i["uri"] for i in inventory.grammars[0]["imports"]]
    assert uris == [
        "https://www.omg.org/spec/KerML/20250201",
        "http://www.eclipse.org/emf/2002/Ecore",
    ]


def test_artifacts_split_words_from_operators(inventory):
    keywords = build_artifacts([DATA / "sample.xtext"], inventory)["keywords.json"]
    assert "package" in keywords["keywords"]
    assert "{" in keywords["operators"]
    assert set(keywords["keywords"]).isdisjoint(keywords["operators"])


@pytest.mark.parametrize(
    "literal",
    [
        # Endpoints of the character ranges in DECIMAL_VALUE ('0'..'9') and of
        # ALPHABETIC_CHARACTER-style ranges. Not tokens of the language.
        "0",
        "9",
        # Letters naming escape sequences in UNRESTRICTED_NAME, not keywords: the
        # real grammar put A, E, Z, a, b, e, f, n, r, t and z in the keyword list
        # this way, and keywords.json drives the lexer.
        "b",
        "t",
        "n",
        "f",
        "r",
        # Comment delimiters belong to the lexical productions the specification
        # states at KerML 8.2.2.2, not to the operator set.
        "//*",
        "*/",
        "//",
        "/*",
    ],
)
def test_terminal_rule_literals_are_not_tokens(inventory, literal):
    assert literal not in inventory.keywords


def test_parser_rule_literals_are_still_collected(inventory):
    # The exclusion must be scoped to terminals. These come from PackageBody,
    # VisibilityKind and Quote, which are ordinary rules.
    for literal in ("package", "{", "}", ";", "public", "private", "'"):
        assert literal in inventory.keywords
