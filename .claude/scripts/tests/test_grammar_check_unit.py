# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
from grammar_check_unit import evaluate

ALLOWED_KW = {"alias", "for", "filter", ";"}
DECLARED = {"Good", "MemberPrefix", "OwnedExpression", "SelfRef"}

GOOD_RULE = {
    "k": "seq",
    "items": [{"k": "ref", "name": "MemberPrefix"}, {"k": "kw", "text": "filter"}],
}


def unit(production="Good", **over):
    """A minimal unit that passes every check, overridable per test."""
    base = {
        "schema_version": 1,
        "production": production,
        "status": "derived",
        "rule": GOOD_RULE,
        "decision": "Follows the clause.",
        "evidence": [{"kind": "clause", "ref": "kerml clause 1.2.3"}],
    }
    base.update(over)
    return base


def grammar(*names):
    """A one-language view in which each named production is declared."""
    return {name: {"production": name} for name in names}


def check(u, units=None):
    view = {**grammar(*DECLARED), **(units or {})}
    return evaluate(u, {"kerml": view}, ALLOWED_KW)


def test_a_complete_unit_is_promoted_to_verified():
    u = unit()
    assert check(u) is True
    assert u["status"] == "verified"
    assert u["verified_utc"]
    assert set(u["acceptance"].values()) == {"pass"}


def test_diagnostics_never_touch_authored_notes():
    u = unit(rule={"k": "kw", "text": "nonpinned"}, notes="Author prose.")
    assert check(u) is False
    assert u["notes"] == "Author prose."
    assert any("pinned token set" in d for d in u["diagnostics"])


def test_a_lexical_terminal_written_as_a_ref_is_rejected_and_explained():
    u = unit(rule={"k": "ref", "name": "NAME"})
    assert check(u) is False
    assert u["acceptance"]["refs_declared"] == "fail"
    assert any("{k: tok}" in d for d in u["diagnostics"])


def test_diagnostics_are_replaced_not_accumulated():
    u = unit(rule={"k": "kw", "text": "nonpinned"})
    check(u)
    first = list(u["diagnostics"])
    check(u)
    assert u["diagnostics"] == first


def test_diagnostics_are_dropped_once_the_unit_is_clean():
    u = unit(rule={"k": "kw", "text": "nonpinned"})
    check(u)
    assert "diagnostics" in u
    u["rule"] = GOOD_RULE
    assert check(u) is True
    assert "diagnostics" not in u


def test_rechecking_a_verified_unit_changes_nothing():
    u = unit()
    check(u)
    stamped = u["verified_utc"]
    check(u)
    assert u["verified_utc"] == stamped


def test_a_verified_unit_that_stops_passing_is_demoted():
    u = unit()
    check(u)
    u["rule"] = {"k": "kw", "text": "nonpinned"}
    assert check(u) is False
    assert u["status"] == "derived"


def test_a_conflict_is_never_promoted_by_a_passing_check():
    # The checks are about the shape of a rule. A conflict is about the sources
    # disagreeing, and writing the rule well does not settle that.
    u = unit(status="conflict", notes="KerML says mandatory, the Xtext says optional.")
    assert check(u) is False
    assert u["status"] == "conflict"
    assert any("adjudicator" in d for d in u["diagnostics"])


def test_a_missing_rule_fails_without_running_the_rule_checks():
    u = unit(rule=None)
    assert check(u) is False
    assert u["acceptance"]["has_rule"] == "fail"
    assert "refs_declared" not in u["acceptance"]


def test_unmarked_left_recursion_fails():
    u = unit("SelfRef", rule={"k": "seq", "items": [{"k": "ref", "name": "SelfRef"}]})
    assert check(u) is False
    assert u["acceptance"]["no_unmarked_left_recursion"] == "fail"


def test_left_recursion_passes_once_declared_intentional():
    u = unit(
        "SelfRef",
        rule={"k": "seq", "items": [{"k": "ref", "name": "SelfRef"}]},
        left_recursion_intended=True,
    )
    assert check(u) is True


def test_prose_mentioning_left_recursion_does_not_satisfy_the_check():
    # Regression: the criterion used to look for this phrase in `notes`, and its
    # own failure message contained it, so a second run passed what the first
    # had failed. Only the explicit flag counts now.
    u = unit(
        "SelfRef",
        rule={"k": "seq", "items": [{"k": "ref", "name": "SelfRef"}]},
        notes="directly left-recursive; if that is intended, set it",
    )
    assert check(u) is False
    assert u["acceptance"]["no_unmarked_left_recursion"] == "fail"


def test_a_shared_unit_must_resolve_its_refs_in_both_languages():
    # ADR-0014: a shared unit is part of the KerML and the SysML grammar, so a ref
    # declared in only one of them is a hole in the other.
    u = unit(rule={"k": "ref", "name": "PackageBodyElement"})
    views = {
        "kerml": grammar("NamespaceBodyElement"),
        "sysml": grammar("PackageBodyElement"),
    }
    assert evaluate(u, views, ALLOWED_KW) is False
    assert u["acceptance"]["refs_declared"] == "fail"
    assert any("absent from kerml" in d for d in u["diagnostics"])


def test_a_variant_is_checked_only_against_its_own_language():
    u = unit(rule={"k": "ref", "name": "PackageBodyElement"}, scope="sysml")
    assert evaluate(u, {"sysml": grammar("PackageBodyElement")}, ALLOWED_KW) is True


def test_corpus_evidence_naming_no_file_fails():
    # Ten refs missing their `corpus/` path segment were once verified, because no
    # check looked. Evidence that resolves to nothing is not evidence.
    u = unit(evidence=[{"kind": "corpus", "ref": "vendor/omg/Missing.sysml"}])
    assert (
        evaluate(u, {"kerml": grammar(*DECLARED)}, ALLOWED_KW, file_exists=lambda _: False) is False
    )
    assert u["acceptance"]["corpus_evidence_resolves"] == "fail"
    assert any("vendor/omg/Missing.sysml" in d for d in u["diagnostics"])


def test_corpus_evidence_naming_a_real_file_passes():
    u = unit(evidence=[{"kind": "corpus", "ref": "vendor/corpus/omg/Real.sysml"}])
    assert (
        evaluate(u, {"kerml": grammar(*DECLARED)}, ALLOWED_KW, file_exists=lambda _: True) is True
    )


def test_clause_evidence_is_not_looked_up_as_a_file():
    u = unit(evidence=[{"kind": "clause", "ref": "kerml clause 1.2.3"}])
    assert (
        evaluate(u, {"kerml": grammar(*DECLARED)}, ALLOWED_KW, file_exists=lambda _: False) is True
    )
