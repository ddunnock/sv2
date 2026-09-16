# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Coverage is measured against the specification, and a ported Xtext rule is a defect.

The names below are real: PartDefinition is declared by both sources;
AdditiveExpression is one level of the Pilot's operator-precedence cascade, declared
by the Xtext only, and recorded in deviations.json as follow_spec — do not implement.
"""

import json

import bnf_coverage


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
        ["AdditiveExpression"], implemented=1, declared=1, unimplemented=0
    )
    out = capsys.readouterr().out
    assert code == 1
    assert "ported from the Pilot Xtext" in out
    assert "AdditiveExpression" in out


def test_an_invented_production_fails_with_the_other_message(tmp_path, monkeypatch, capsys):
    _write(tmp_path, ["PartDefinition"], ["PartDefinition"])
    monkeypatch.setattr(bnf_coverage, "INVENTORY", tmp_path / "bnf.json")
    monkeypatch.setattr(bnf_coverage, "XTEXT_INVENTORY", tmp_path / "xtext.json")
    code = bnf_coverage._check(["PartDefinitionn"], implemented=1, declared=1, unimplemented=0)
    out = capsys.readouterr().out
    assert code == 1
    assert "in neither inventory" in out
    assert "ported from the Pilot Xtext" not in out


def test_no_absent_productions_passes(capsys):
    assert bnf_coverage._check([], implemented=3, declared=558, unimplemented=555) == 0
    assert "3/558 implemented" in capsys.readouterr().out
