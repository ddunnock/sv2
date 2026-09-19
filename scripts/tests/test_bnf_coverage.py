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
        ["AdditiveExpression"],
        _report(absent=1, absent_productions=["AdditiveExpression"]),
        LIVE,
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
        ["PartDefinitionn"], _report(absent=1, absent_productions=["PartDefinitionn"]), LIVE
    )
    out = capsys.readouterr().out
    assert code == 1
    assert "no live grammar unit" in out
    assert "ported from the Pilot Xtext" not in out


def test_no_absent_productions_and_a_current_report_passes(tmp_path, monkeypatch, capsys):
    report = _report(declared=554, implemented=3, unimplemented=551)
    path = tmp_path / "coverage.json"
    path.write_text(json.dumps(report))
    monkeypatch.setattr(bnf_coverage, "REPORT", path)
    assert bnf_coverage._check([], report) == 0
    assert "3/554 grammar units implemented" in capsys.readouterr().out


# -- the denominator is live grammar units, not names (ADR-0015) --
#
# Real shapes: ConnectorEnd is stated alike in both languages, so it is one shared unit;
# InitialNodeMember is SysML's alone; MultiplicityRange is stated differently in the two
# (KerML's `multiplicity` declaration, SysML's `[1..*]`), so it is two units.

LIVE = {
    "ConnectorEnd": {None},
    "InitialNodeMember": {"sysml"},
    "MultiplicityRange": {"kerml", "sysml"},
}


def test_the_denominator_counts_a_split_production_twice():
    assert bnf_coverage.unit_keys(LIVE) == {
        "ConnectorEnd",
        "InitialNodeMember@sysml",
        "MultiplicityRange@kerml",
        "MultiplicityRange@sysml",
    }


def test_a_bare_marker_claims_its_names_one_unit():
    assert bnf_coverage.resolve("ConnectorEnd", LIVE) == "ConnectorEnd"
    assert bnf_coverage.resolve("InitialNodeMember", LIVE) == "InitialNodeMember@sysml"


def test_a_bare_marker_on_a_split_production_claims_nothing():
    # The defect this accounting exists to catch: SysML's `[1..*]` reading must not
    # report KerML's `multiplicity` declaration implemented along with it.
    assert bnf_coverage.resolve("MultiplicityRange", LIVE) is None
    assert bnf_coverage.resolve("MultiplicityRange@sysml", LIVE) == "MultiplicityRange@sysml"


def test_a_scope_the_production_has_no_unit_for_claims_nothing():
    assert bnf_coverage.resolve("ConnectorEnd@sysml", LIVE) is None  # shared: one unit
    assert bnf_coverage.resolve("InitialNodeMember@kerml", LIVE) is None
    assert bnf_coverage.resolve("Nonexistent", LIVE) is None


def test_retired_units_are_not_grammar(tmp_path):
    def unit(production, status, scope=None):
        name = production if scope is None else f"{production}@{scope}"
        body = {"production": production, "status": status}
        if scope is not None:
            body["scope"] = scope
        (tmp_path / f"{name}.json").write_text(json.dumps(body))

    unit("SourceEnd", "retired")
    unit("SourceEnd", "verified", "sysml")
    unit("ConnectorEnd", "verified")
    assert bnf_coverage.live_units(tmp_path) == {
        "SourceEnd": {"sysml"},
        "ConnectorEnd": {None},
    }


def test_an_unscoped_split_marker_fails_with_its_own_message(tmp_path, monkeypatch, capsys):
    monkeypatch.setattr(bnf_coverage, "INVENTORY", tmp_path / "absent.json")
    code = bnf_coverage._check(["MultiplicityRange"], _report(absent=1), LIVE)
    out = capsys.readouterr().out
    assert code == 1
    assert "state differently" in out
    assert "no live grammar unit" not in out


def test_a_misscoped_marker_fails_with_its_own_message(tmp_path, monkeypatch, capsys):
    monkeypatch.setattr(bnf_coverage, "INVENTORY", tmp_path / "absent.json")
    code = bnf_coverage._check(["ConnectorEnd@sysml"], _report(absent=1), LIVE)
    out = capsys.readouterr().out
    assert code == 1
    assert "no live unit for" in out


def test_the_marker_pattern_reads_a_scope_suffix():
    text = "// production: A\n// production: B@kerml\n// production: C@sysml\n"
    assert [m.group(1) for m in bnf_coverage.MARKER.finditer(text)] == [
        "A",
        "B@kerml",
        "C@sysml",
    ]


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
