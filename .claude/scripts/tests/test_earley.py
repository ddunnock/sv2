# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import pytest

from _earley import STATES_PER_TOKEN, make_lexer, nullable_nonterminals, recognize


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


def test_lexer_skips_notes_but_keeps_regular_comments(lex):
    # KerML 8.2.2.2: `//* ... */` and `// ...` are notes, which are trivia. A `/* ... */`
    # is a REGULAR_COMMENT, the body of a Comment element, and must reach the grammar.
    assert lex("/* c */ //* note */ // line\npart") == [
        ("tok", "REGULAR_COMMENT", "/* c */"),
        ("kw", "part"),
    ]


def test_lexer_multiline_regular_comment_is_one_token(lex):
    assert lex("/*\n * a\n */part") == [
        ("tok", "REGULAR_COMMENT", "/*\n * a\n */"),
        ("kw", "part"),
    ]


@pytest.fixture
def numlex():
    return make_lexer([], [".", "..", "*", "[", "]"])


@pytest.mark.parametrize(
    ("text", "expected"),
    [
        ("42", [("tok", "DECIMAL_VALUE", "42")]),
        ("2e3", [("tok", "EXPONENTIAL_VALUE", "2e3")]),
        ("7E+10", [("tok", "EXPONENTIAL_VALUE", "7E+10")]),
        # A real's `.` is its own token (KerML 8.2.2.4, RealValue).
        ("2.5", [("tok", "DECIMAL_VALUE", "2"), ("kw", "."), ("tok", "DECIMAL_VALUE", "5")]),
        (".5", [("kw", "."), ("tok", "DECIMAL_VALUE", "5")]),
        (
            "2.5e-3",
            [("tok", "DECIMAL_VALUE", "2"), ("kw", "."), ("tok", "EXPONENTIAL_VALUE", "5e-3")],
        ),
        # A range is two decimals around `..`, not a real.
        (
            "[1..5]",
            [
                ("kw", "["),
                ("tok", "DECIMAL_VALUE", "1"),
                ("kw", ".."),
                ("tok", "DECIMAL_VALUE", "5"),
                ("kw", "]"),
            ],
        ),
        # An exponent needs digits; `3e` is a decimal and a name.
        ("3e", [("tok", "DECIMAL_VALUE", "3"), ("tok", "NAME", "e")]),
    ],
)
def test_lexer_numbers_use_the_specification_terminals(numlex, text, expected):
    assert numlex(text) == expected


REAL = {
    # RealValue and LiteralInteger as verified: KerML 8.2.5.8.4.
    "RealValue": [
        [("tok", "DECIMAL_VALUE"), kw("."), ("tok", "DECIMAL_VALUE")],
        [("tok", "DECIMAL_VALUE"), kw("."), ("tok", "EXPONENTIAL_VALUE")],
        [kw("."), ("tok", "DECIMAL_VALUE")],
        [kw("."), ("tok", "EXPONENTIAL_VALUE")],
        [("tok", "EXPONENTIAL_VALUE")],
    ],
    "LiteralInteger": [[("tok", "DECIMAL_VALUE")]],
}


@pytest.mark.parametrize("text", ["2.5", ".5", "2.5e-3", "6e23"])
def test_numbers_reach_real_value(numlex, text):
    assert recognize(REAL, "RealValue", numlex(text)) == (True, "ok")


@pytest.mark.parametrize("text", ["5.", "5", "1..5"])
def test_numbers_that_are_not_reals_are_rejected(numlex, text):
    assert recognize(REAL, "RealValue", numlex(text))[0] is False


def test_integer_reaches_literal_integer(numlex):
    assert recognize(REAL, "LiteralInteger", numlex("42")) == (True, "ok")


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


def test_keyword_symbol_accepts_a_name_the_language_does_not_reserve():
    # KerML's constructor expression writes 'new', and KerML 8.2.2.6 does not reserve
    # it: the word lexes as a NAME and still satisfies the literal, as a contextual keyword.
    lex = make_lexer(["expr"], ["(", ")"])
    prods = {"S": [[kw("new"), ("tok", "NAME"), kw("("), kw(")")]]}
    assert recognize(prods, "S", lex("new T()")) == (True, "ok")


def test_unreserved_word_stays_usable_as_a_name():
    # `expr at` is KerML: `at` is reserved in SysML only.
    lex = make_lexer(["expr"], ["{", "}"])
    prods = {"S": [[kw("expr"), ("tok", "NAME"), kw("{"), kw("}")]]}
    assert recognize(prods, "S", lex("expr at { }")) == (True, "ok")


def test_reserved_word_is_not_a_name():
    lex = make_lexer(["expr", "at"], ["{", "}"])
    prods = {"S": [[kw("expr"), ("tok", "NAME"), kw("{"), kw("}")]]}
    assert recognize(prods, "S", lex("expr at { }"))[0] is False


def test_keyword_symbol_does_not_accept_a_different_name():
    lex = make_lexer([], [])
    assert recognize({"S": [[kw("new")]]}, "S", lex("old"))[0] is False


def test_input_length_alone_does_not_trip_the_state_guard():
    # The budget is per token, so a long input under a linear grammar is accepted
    # however long it gets. Under the absolute cap this guard used to carry, length
    # alone was enough to fail a file: the two copies of SimpleVehicleModel need
    # ~996k items at ~143 per token, and the cap was 400_000.
    lex = make_lexer(["part"], [";"])
    prods = {"S": [[nt("S"), nt("Item")], []], "Item": [[kw("part"), ("tok", "NAME"), kw(";")]]}
    assert recognize(prods, "S", lex("part a ; " * 2000)) == (True, "ok")


def test_the_default_budget_is_the_per_token_one():
    # Pins the default to STATES_PER_TOKEN per token rather than to a constant: an
    # input given exactly that budget behaves the same as one given the default,
    # and one token's worth less is not enough for this grammar.
    lex = make_lexer(["part"], [";"])
    prods = {"S": [[nt("S"), nt("Item")], []], "Item": [[kw("part"), ("tok", "NAME"), kw(";")]]}
    tokens = lex("part a ; " * 200)
    assert recognize(prods, "S", tokens) == (True, "ok")
    assert recognize(prods, "S", tokens, max_states=STATES_PER_TOKEN * (len(tokens) + 1)) == (
        True,
        "ok",
    )


def test_exhausting_the_budget_reports_explosion_rather_than_no_parse():
    # The guard's own message, distinct from an ordinary rejection. A caller that
    # cannot tell the two apart would read a resource limit as a grammar defect.
    lex = make_lexer(["part"], [";"])
    prods = {"S": [[kw("part"), ("tok", "NAME"), kw(";")]]}
    ok, why = recognize(prods, "S", lex("part a ;"), max_states=1)
    assert ok is False
    assert "state explosion" in why


def test_an_explicit_max_states_still_overrides_the_per_token_budget():
    # grammar_validate.py relies on the default, but the diagnostic that calibrated
    # STATES_PER_TOKEN raises it deliberately. The override must keep working.
    lex = make_lexer(["part"], [";"])
    prods = {"S": [[kw("part"), ("tok", "NAME"), kw(";")]]}
    assert recognize(prods, "S", lex("part a ;"), max_states=10_000) == (True, "ok")
