# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Coverage is measured against the specification, and a ported Xtext rule is a defect.

The names below are real: PartDefinition is declared by both sources;
AdditiveExpression is one level of the Pilot's operator-precedence cascade, declared
by the Xtext only, and recorded in deviations.json as follow_spec — do not implement.
"""

import json

import bnf_coverage


def _report(**kw):
    """A report dict of the shape build_report returns, with the counts a test needs."""
    base = {
        "_generated_by": "scripts/bnf_coverage.py",
        "_note": "`unimplemented` is a tracked state, not a failure. `absent` is a defect.",
        "declared": 1,
        "implemented": 1,
        "unimplemented": 0,
        "absent": 0,
        "percent": 100.0,
        "unimplemented_productions": [],
        "absent_productions": [],
    }
    return base | kw


def _write(tmp_path, spec, xtext):
    (tmp_path / "bnf.json").write_text(json.dumps({"productions": spec}))
    (tmp_path / "xtext.json").write_text(json.dumps({"productions": [{"name": n} for n in xtext]}))


def test_xtext_only_names_are_those_the_specification_does_not_declare(tmp_path, monkeypatch):
    _write(tmp_path, ["PartDefinition"], ["PartDefinition", "AdditiveExpression"])
    monkeypatch.setattr(bnf_coverage, "INVENTORY", tmp_path / "bnf.json")
    monkeypatch.setattr(bnf_coverage, "XTEXT_INVENTORY", tmp_path / "xtext.json")
    assert bnf_coverage.xtext_only() == {"AdditiveExpression"}


def test_a_shared_production_is_not_xtext_only(tmp_path, monkeypatch):
    _write(tmp_path, ["PartDefinition"], ["PartDefinition"])
    monkeypatch.setattr(bnf_coverage, "INVENTORY", tmp_path / "bnf.json")
    monkeypatch.setattr(bnf_coverage, "XTEXT_INVENTORY", tmp_path / "xtext.json")
    assert bnf_coverage.xtext_only() == set()


def test_missing_inventories_yield_no_xtext_only(tmp_path, monkeypatch):
    # Inert rather than wrong: with nothing pinned there is no basis to call a
    # marker ported, and guessing would fail the gate on a fresh clone.
    monkeypatch.setattr(bnf_coverage, "INVENTORY", tmp_path / "absent.json")
    monkeypatch.setattr(bnf_coverage, "XTEXT_INVENTORY", tmp_path / "absent.json")
    assert bnf_coverage.xtext_only() == set()


def test_a_ported_xtext_rule_fails_the_check(tmp_path, monkeypatch, capsys):
    _write(tmp_path, ["PartDefinition"], ["PartDefinition", "AdditiveExpression"])
    monkeypatch.setattr(bnf_coverage, "INVENTORY", tmp_path / "bnf.json")
    monkeypatch.setattr(bnf_coverage, "XTEXT_INVENTORY", tmp_path / "xtext.json")
    code = bnf_coverage._check(  # private seam, tested deliberately
        ["AdditiveExpression"], _report(absent=1, absent_productions=["AdditiveExpression"])
    )
    out = capsys.readouterr().out
    assert code == 1
    assert "ported from the Pilot Xtext" in out
    assert "AdditiveExpression" in out


def test_an_invented_production_fails_with_the_other_message(tmp_path, monkeypatch, capsys):
    _write(tmp_path, ["PartDefinition"], ["PartDefinition"])
    monkeypatch.setattr(bnf_coverage, "INVENTORY", tmp_path / "bnf.json")
    monkeypatch.setattr(bnf_coverage, "XTEXT_INVENTORY", tmp_path / "xtext.json")
    code = bnf_coverage._check(
        ["PartDefinitionn"], _report(absent=1, absent_productions=["PartDefinitionn"])
    )
    out = capsys.readouterr().out
    assert code == 1
    assert "in neither inventory" in out
    assert "ported from the Pilot Xtext" not in out


def test_no_absent_productions_and_a_current_report_passes(tmp_path, monkeypatch, capsys):
    report = _report(declared=558, implemented=3, unimplemented=555)
    path = tmp_path / "coverage.json"
    path.write_text(json.dumps(report))
    monkeypatch.setattr(bnf_coverage, "REPORT", path)
    assert bnf_coverage._check([], report) == 0
    assert "3/558 implemented" in capsys.readouterr().out


# -- staleness: a derived artifact that cannot be caught out of date is not derived --


def test_a_report_that_disagrees_with_the_markers_is_stale(tmp_path, monkeypatch):
    # The real drift: a `// production:` marker is added and coverage.json is not
    # regenerated, so it keeps publishing the previous count while the gate stays green.
    path = tmp_path / "coverage.json"
    path.write_text(json.dumps(_report(implemented=106)))
    monkeypatch.setattr(bnf_coverage, "REPORT", path)
    assert bnf_coverage.is_stale(_report(implemented=107))


def test_a_report_matching_the_markers_is_not_stale(tmp_path, monkeypatch):
    report = _report(implemented=107)
    path = tmp_path / "coverage.json"
    path.write_text(json.dumps(report))
    monkeypatch.setattr(bnf_coverage, "REPORT", path)
    assert not bnf_coverage.is_stale(report)


def test_an_absent_or_unreadable_report_is_stale(tmp_path, monkeypatch):
    monkeypatch.setattr(bnf_coverage, "REPORT", tmp_path / "gone.json")
    assert bnf_coverage.is_stale(_report())
    truncated = tmp_path / "half-written.json"
    truncated.write_text('{"declared": 55')
    monkeypatch.setattr(bnf_coverage, "REPORT", truncated)
    assert bnf_coverage.is_stale(_report())


def test_staleness_fails_the_check_even_with_nothing_absent(tmp_path, monkeypatch, capsys):
    path = tmp_path / "coverage.json"
    path.write_text(json.dumps(_report(implemented=106)))
    monkeypatch.setattr(bnf_coverage, "REPORT", path)
    assert bnf_coverage._check([], _report(implemented=107)) == 1
    assert "is stale" in capsys.readouterr().out


def test_build_report_counts_what_it_was_given():
    report = bnf_coverage.build_report({"A", "B", "C", "D"}, ["A"], ["B", "C"], ["D"])
    assert report["declared"] == 4
    assert report["implemented"] == 1
    assert report["percent"] == 25.0
    assert report["unimplemented_productions"] == ["B", "C"]
