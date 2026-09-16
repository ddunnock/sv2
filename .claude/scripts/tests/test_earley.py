# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import pytest

from _earley import make_lexer, nullable_nonterminals, recognize


def kw(text):
    return ("kw", text)


def nt(name):
    return ("nt", name)


@pytest.fixture
def lex():
    return make_lexer(["package", "part"], [":", ":>", ":>>", ";", "{", "}"])


def test_lexer_classifies_keywords_names_and_operators(lex):
    assert lex("package P { part a :>> b ; }") == [
        ("kw", "package"),
        ("tok", "NAME", "P"),
        ("kw", "{"),
        ("kw", "part"),
        ("tok", "NAME", "a"),
        ("kw", ":>>"),
        ("tok", "NAME", "b"),
        ("kw", ";"),
        ("kw", "}"),
    ]


def test_lexer_skips_comments_and_notes(lex):
    assert lex("/* c */ //* note */ // line\npart") == [("kw", "part")]


def test_lexer_reports_offset_of_unlexable_input(lex):
    with pytest.raises(ValueError, match="lex error at offset 5"):
        lex("part @oops")


PRODS = {
    "S": [[kw("package"), ("tok", "NAME"), nt("Body")]],
    "Body": [[kw(";")], [kw("{"), nt("Parts"), kw("}")]],
    "Parts": [[], [kw("part"), ("tok", "NAME"), kw(";"), nt("Parts")]],
}


@pytest.mark.parametrize(
    "tokens",
    [
        [kw("package"), ("tok", "NAME", "P"), kw(";")],
        [kw("package"), ("tok", "NAME", "P"), kw("{"), kw("}")],
        [
            kw("package"),
            ("tok", "NAME", "P"),
            kw("{"),
            kw("part"),
            ("tok", "NAME", "a"),
            kw(";"),
            kw("}"),
        ],
    ],
)
def test_recognize_accepts_sentences(tokens):
    assert recognize(PRODS, "S", tokens) == (True, "ok")


def test_recognize_reports_furthest_token_on_rejection():
    tokens = [kw("package"), ("tok", "NAME", "P"), kw("{"), kw(";")]
    assert recognize(PRODS, "S", tokens) == (False, "no parse; stopped at token 3/4 near ';'")


def test_recognize_reports_incomplete_input():
    tokens = [kw("package"), ("tok", "NAME", "P")]
    assert recognize(PRODS, "S", tokens) == (
        False,
        "no parse; consumed all 2 tokens but no complete S",
    )


def test_recognize_rejects_undefined_nonterminal():
    assert recognize({"S": [[nt("Missing")]]}, "S", []) == (
        False,
        "undefined nonterminal 'Missing'",
    )


def test_recognize_rejects_undefined_start():
    assert recognize({}, "S", []) == (False, "start symbol 'S' is not defined")


def test_recognize_accepts_left_recursion():
    prods = {"L": [[nt("L"), kw("x")], [kw("x")]]}
    assert recognize(prods, "L", [kw("x")] * 4) == (True, "ok")


def test_recognize_completes_repeated_nullable_nonterminal():
    prods = {"S": [[nt("A"), kw("x")]], "A": [[nt("B"), nt("B")]], "B": [[]]}
    assert recognize(prods, "S", [kw("x")]) == (True, "ok")


@pytest.mark.parametrize(
    "prods",
    [
        {"S": [[nt("A"), kw("x")]], "A": [[nt("B"), nt("B")]], "B": [[]]},
        {"S": [[nt("B"), nt("B"), nt("B"), kw("x")]], "B": [[], [nt("C")]], "C": [[]]},
    ],
)
def test_recognize_skips_repeated_nullable_nonterminals(prods):
    assert recognize(prods, "S", [kw("x")]) == (True, "ok")


def test_nullable_nonterminals_reaches_fixed_point():
    prods = {
        "A": [[nt("B"), nt("C")]],
        "B": [[]],
        "C": [[nt("B")], [kw("x")]],
        "D": [[nt("Undefined")]],
    }
    assert nullable_nonterminals(prods) == {"A", "B", "C"}
