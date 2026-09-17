# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import pytest

from _grammar import kws_of, normalize, referable_productions, refs_of, render_ebnf, xtext_rule_text


def kw(t):
    return {"k": "kw", "text": t}


def ref(n):
    return {"k": "ref", "name": n}


RULE = {
    "k": "seq",
    "items": [
        kw("package"),
        ref("Name"),
        {
            "k": "alt",
            "items": [
                kw(";"),
                {"k": "seq", "items": [kw("{"), {"k": "star", "item": ref("Part")}, kw("}")]},
            ],
        },
    ],
}


def test_refs_and_keywords_are_collected_through_nesting():
    assert refs_of(RULE) == {"Name", "Part"}
    assert kws_of(RULE) == {"package", ";", "{", "}"}


@pytest.mark.parametrize(
    ("node", "expected"),
    [
        (RULE, "R ::= 'package' Name ( ';' | '{' Part* '}' ) ;"),
        ({"k": "alt", "items": [kw("a"), kw("b")]}, "R ::= 'a' | 'b' ;"),
        (
            {"k": "opt", "item": {"k": "seq", "items": [kw("<"), ref("N"), kw(">")]}},
            "R ::= ( '<' N '>' )? ;",
        ),
        ({"k": "plus", "item": {"k": "tok", "name": "NAME"}}, "R ::= NAME+ ;"),
        (kw("'"), "R ::= '\\'' ;"),
    ],
)
def test_render_ebnf(node, expected):
    assert render_ebnf("R", node) == expected


def test_normalize_names_generated_productions_parent_first():
    prods = normalize({"R": {"rule": RULE}})
    assert prods["R"] == [[("kw", "package"), ("nt", "Name"), ("nt", "__alt1")]]
    assert prods["__alt1"] == [[("kw", ";")], [("nt", "__seq2")]]
    assert prods["__star3"] == [[], [("nt", "Part"), ("nt", "__star3")]]


def test_normalize_desugars_plus_as_one_then_many():
    prods = normalize({"R": {"rule": {"k": "plus", "item": ref("X")}}})
    assert prods["__plus1"] == [[("nt", "X")], [("nt", "X"), ("nt", "__plus1")]]


def test_normalize_skips_units_without_rules():
    assert normalize({"R": {"status": "pending"}}) == {}


def test_normalize_rejects_unknown_node_kind():
    with pytest.raises(ValueError, match="unknown node kind 'bogus'"):
        normalize({"R": {"rule": {"k": "seq", "items": [{"k": "bogus"}]}}})


XTEXT = """grammar G

Header returns SysML::Package :
\t'package' Name
;

Inline : ID ;

After : 'x' ;
"""


@pytest.mark.parametrize(
    ("name", "expected"),
    [
        ("Header", "Header returns SysML::Package :\n\t'package' Name\n;"),
        ("Inline", "Inline : ID ;"),
        ("Missing", None),
    ],
)
def test_xtext_rule_text_handles_header_and_inline_rules(tmp_path, monkeypatch, name, expected):
    (tmp_path / "vendor/pilot").mkdir(parents=True)
    (tmp_path / "vendor/pilot/G.xtext").write_text(XTEXT)
    monkeypatch.chdir(tmp_path)
    assert xtext_rule_text(name)[1] == expected


def test_referable_productions_are_the_specification_non_terminals():
    # A pack built from the pilot's inventory forbade GeneralType, which only the
    # specification states, and offered FilterPackageMembershipImport, which only the
    # pilot does. Terminals are written {k: tok} and are never referable.
    inventory = {"productions": ["GeneralType", "NAME", "REGULAR_COMMENT", "Feature"]}
    assert referable_productions(inventory) == ["Feature", "GeneralType"]
    assert referable_productions({}) == []
