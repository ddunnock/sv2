# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""KerML and SysML as two grammars sharing a vocabulary (ADR-0014).

The failure this machinery exists to prevent is silent: one language's rule quietly
standing in for the other's. These tests are about the ways that could still happen.
"""

import json

import pytest

import _grammar
from _grammar import (
    body_references,
    clause_for_scope,
    divergent_productions,
    grammar_view,
    production_scopes,
    reachable,
    shadowed,
    unit_key,
)
from grammar_freeze import blockers
from grammar_plan import _rescope, expected_keys


def u(production, scope=None, status="verified", **extra):
    return {
        "production": production,
        "status": status,
        **({"scope": scope} if scope else {}),
        **extra,
    }


def rule(name, file, body):
    return {"name": name, "file": file, "body": body}


# ---- identity -----------------------------------------------------------------


def test_a_variant_key_names_its_language_and_a_shared_key_does_not():
    assert unit_key(u("RootNamespace", "sysml")) == "RootNamespace@sysml"
    assert unit_key(u("Import")) == "Import"


def test_a_unit_file_whose_name_disagrees_with_its_content_is_refused(tmp_path, monkeypatch):
    # Otherwise a variant saved under the shared name would silently replace it.
    monkeypatch.setattr(_grammar, "UNITS", tmp_path)
    (tmp_path / "RootNamespace.json").write_text(json.dumps(u("RootNamespace", "kerml")))
    with pytest.raises(ValueError, match=r"RootNamespace@kerml\.json"):
        _grammar.load_units()


def test_save_unit_writes_to_the_file_its_content_names(tmp_path, monkeypatch):
    monkeypatch.setattr(_grammar, "UNITS", tmp_path)
    _grammar.save_unit(u("RootNamespace", "kerml"))
    _grammar.save_unit(u("Import"))
    assert sorted(p.name for p in tmp_path.iterdir()) == ["Import.json", "RootNamespace@kerml.json"]
    assert set(_grammar.load_units()) == {"Import", "RootNamespace@kerml"}


# ---- views ----------------------------------------------------------------------


UNITS = {
    "Import": u("Import"),
    "RootNamespace@kerml": u(
        "RootNamespace", "kerml", rule={"k": "ref", "name": "NamespaceBodyElement"}
    ),
    "RootNamespace@sysml": u(
        "RootNamespace", "sysml", rule={"k": "ref", "name": "PackageBodyElement"}
    ),
    "RootNamespace": u("RootNamespace", status="retired"),
    "Gone": u("Gone", status="retired"),
}


def test_each_language_sees_its_own_variant_and_every_shared_unit():
    kerml, sysml = grammar_view(UNITS, "kerml"), grammar_view(UNITS, "sysml")
    assert kerml["RootNamespace"]["rule"]["name"] == "NamespaceBodyElement"
    assert sysml["RootNamespace"]["rule"]["name"] == "PackageBodyElement"
    assert kerml["Import"] is sysml["Import"] is UNITS["Import"]


def test_retired_units_belong_to_no_grammar():
    assert "Gone" not in grammar_view(UNITS, "kerml")


def test_a_variant_wins_over_a_live_shared_unit_regardless_of_order():
    # Dict order is file order; a shared unit sorting after its variant must not win.
    both = {"X@kerml": u("X", "kerml", tag="variant"), "X": u("X", tag="shared")}
    assert grammar_view(both, "kerml")["X"]["tag"] == "variant"
    assert grammar_view(both, "sysml")["X"]["tag"] == "shared"


def test_a_live_shared_unit_beside_a_variant_is_reported():
    assert shadowed({"X@kerml": u("X", "kerml"), "X": u("X")}) == ["X"]
    assert shadowed(UNITS) == []  # the shared RootNamespace is retired


# ---- what splits ----------------------------------------------------------------


def test_a_production_stated_differently_in_each_language_splits():
    rules = [
        rule("RootNamespace", "KerML-textual-bnf.kebnf", "NamespaceBodyElement*"),
        rule("RootNamespace", "SysML-textual-bnf.kebnf", "PackageBodyElement*"),
    ]
    assert divergent_productions(rules) == {"RootNamespace"}


def test_whitespace_and_assignments_do_not_split_a_production():
    # EmptyFeature is `{ }` in one file and `{}` in the other, and the two languages
    # often assign to different slots while accepting the same text.
    rules = [
        rule("EmptyFeature", "KerML-textual-bnf.kebnf", "{ }"),
        rule("EmptyFeature", "SysML-textual-bnf.kebnf", "{}"),
        rule("Arg", "KerML-textual-bnf.kebnf", "value = OwnedExpression"),
        rule("Arg", "SysML-textual-bnf.kebnf", "ownedRelatedElement += OwnedExpression"),
    ]
    assert divergent_productions(rules) == set()


def test_a_keyword_literal_containing_an_equals_sign_is_not_mistaken_for_an_assignment():
    rules = [
        rule("Binding", "KerML-textual-bnf.kebnf", "A '=' B"),
        rule("Binding", "SysML-textual-bnf.kebnf", "A '==' B"),
    ]
    assert divergent_productions(rules) == {"Binding"}


def test_a_production_stated_in_one_language_only_does_not_split():
    assert (
        divergent_productions([rule("PackageBodyElement", "SysML-textual-bnf.kebnf", "A")]) == set()
    )


def test_the_plan_gives_each_production_the_units_its_scopes_name():
    scopes = {"RootNamespace": ("kerml", "sysml"), "Feature": ("kerml",)}
    assert expected_keys(["Import", "RootNamespace", "Feature"], scopes) == [
        ("Import", None),
        ("RootNamespace", "kerml"),
        ("RootNamespace", "sysml"),
        ("Feature", "kerml"),
    ]


# ---- which grammar reaches a production (ADR-0015) -------------------------------

K, S = "KerML-textual-bnf.kebnf", "SysML-textual-bnf.kebnf"

#: A miniature of the real shape: KerML's kernel, an expression layer both use, a
#: SysML-only usage layer, and one reference from the expression layer back into
#: KerML's kernel.
MINI = [
    rule("RootNamespace", K, "Element*"),
    rule("Element", K, "Class | Expr"),
    rule("Class", K, "'class' NAME Body"),
    rule("Body", K, "';' | '{' Element* '}'"),
    rule("Expr", K, "'(' Expr ')' | ExprBody | NAME"),
    rule("ExprBody", K, "'{' Element* '}'"),
    rule("RootNamespace", S, "Usage*"),
    rule("Usage", S, "'part' NAME ( '=' Expr )? ';'"),
    rule("Calc", S, "'{' Usage* '}'"),
]


def test_body_references_ignore_literals_properties_and_terminals():
    known = {"Expr", "Usage", "QualifiedName", "NAME"}
    body = "'Expr' value = Expr [QualifiedName] { isUsage = true } NAME // Usage"
    assert body_references(body, known) == {"Expr", "QualifiedName"}


def test_sysml_falls_back_to_kerml_only_for_what_it_does_not_state():
    kerml, sysml = reachable(MINI, "kerml", {}), reachable(MINI, "sysml", {})
    assert "Usage" not in kerml  # KerML never borrows from SysML
    assert {"Usage", "Expr", "ExprBody"} <= sysml
    # ExprBody's `Element*` carries KerML's kernel into SysML: the leak the boundary cuts.
    assert "Class" in sysml


def test_a_boundary_entry_cuts_the_fallback_and_splits_the_production():
    boundary = {"ExprBody": "Calc"}
    assert "Class" not in reachable(MINI, "sysml", boundary)
    scopes = production_scopes(MINI, boundary)
    assert scopes["ExprBody"] == ("kerml", "sysml")
    assert scopes["Class"] == ("kerml",)
    assert scopes["Body"] == ("kerml",)
    assert scopes["Expr"] == (None,)  # stated once, reached by both: shared
    assert scopes["Usage"] == ("sysml",)
    assert scopes["RootNamespace"] == ("kerml", "sysml")


def test_without_the_boundary_the_leaked_kernel_stays_shared():
    scopes = production_scopes(MINI, {})
    assert scopes["Class"] == (None,)


def test_a_production_no_grammar_reaches_keeps_the_language_that_states_it():
    scopes = production_scopes([*MINI, rule("Orphan", S, "'orphan'")], {})
    assert scopes["Orphan"] == ("sysml",)


def test_rescoping_carries_the_work_and_re_checks_it(tmp_path, monkeypatch):
    monkeypatch.setattr(_grammar, "UNITS", tmp_path)
    fp = {"combined": "same", "spec_clause": "a", "xtext_rule": "b"}
    shared = u(
        "Class",
        rule={"k": "kw", "text": "class"},
        decision="as the clause states it",
        evidence=[{"kind": "clause", "ref": "kerml clause 8.2.4.2"}],
        acceptance={"has_rule": "pass"},
        fingerprint=fp,
        inputs={"spec_clause_ref": "x"},
    )
    fresh = u("Class", "kerml", status="pending", language="kerml", fingerprint=fp, inputs={})
    assert _rescope(shared, fresh, single=True) == "rescoped"
    moved = _grammar.load_units()["Class@kerml"]
    assert moved["rule"] == shared["rule"]
    assert moved["decision"] == shared["decision"]
    assert moved["status"] == "derived"  # verified again only against its own grammar
    assert "acceptance" not in moved
    assert moved["notes"].startswith("re-scoped from the shared unit to kerml alone")


def test_rescoping_a_unit_whose_inputs_moved_sends_it_back_to_pending(tmp_path, monkeypatch):
    monkeypatch.setattr(_grammar, "UNITS", tmp_path)
    shared = u(
        "Class", rule={"k": "kw", "text": "class"}, fingerprint={"combined": "old"}, inputs={}
    )
    fresh = u(
        "Class",
        "kerml",
        status="pending",
        language="kerml",
        fingerprint={"combined": "new"},
        inputs={},
    )
    _rescope(shared, fresh, single=True)
    moved = _grammar.load_units()["Class@kerml"]
    assert moved["status"] == "pending"
    assert moved["rule"] == shared["rule"]
    assert "must be re-derived" in moved["notes"]


# ---- clauses --------------------------------------------------------------------


ENTRY = {
    "ref": "kerml clause 8.2.3.4.1 (Namespaces.md) | sysml clause 8.2.2.5.1 (Packages.md)",
    "text": (
        "=== kerml clause 8.2.3.4.1 ===\n# Namespaces\n\nbody k\n\nmore k"
        "\n\n=== sysml clause 8.2.2.5.1 ===\n# Packages\n\nbody s"
    ),
}


def test_a_variant_is_given_only_its_own_languages_clause():
    ref, text = clause_for_scope(ENTRY, "sysml")
    assert ref == "sysml clause 8.2.2.5.1 (Packages.md)"
    assert text == "=== sysml clause 8.2.2.5.1 ===\n# Packages\n\nbody s"
    ref, text = clause_for_scope(ENTRY, "kerml")
    assert "sysml" not in ref
    assert "sysml" not in text
    assert text.endswith("more k")  # blank lines inside a clause do not cut it short


def test_a_shared_unit_is_given_both_clauses():
    assert clause_for_scope(ENTRY, None) == (ENTRY["ref"], ENTRY["text"])


# ---- the oracle -----------------------------------------------------------------


def test_an_oracle_that_skipped_a_language_does_not_permit_a_freeze():
    clean_but_partial = {"accepted": 10, "missed": 0, "caught": 3, "leaked": 0, "skipped": 4}
    assert any("skipped" in b for b in blockers({"A": u("A")}, clean_but_partial))


def test_a_split_variant_with_the_shared_units_inputs_carries_its_work(tmp_path, monkeypatch):
    # The boundary splits ExpressionBody: KerML's variant has exactly the shared unit's
    # inputs, so its derivation stands; SysML's reads a different body and starts fresh.
    monkeypatch.setattr(_grammar, "UNITS", tmp_path)
    shared = u(
        "ExpressionBody",
        rule={"k": "ref", "name": "FunctionBodyPart"},
        fingerprint={"combined": "k"},
        inputs={},
    )
    kerml = u(
        "ExpressionBody",
        "kerml",
        status="pending",
        language="kerml",
        fingerprint={"combined": "k"},
        inputs={},
    )
    sysml = u(
        "ExpressionBody",
        "sysml",
        status="pending",
        language="sysml",
        fingerprint={"combined": "s"},
        inputs={},
    )
    assert _rescope(shared, kerml, single=False) == "rescoped"
    assert _rescope(shared, sysml, single=False) is None
    assert set(_grammar.load_units()) == {"ExpressionBody@kerml"}
