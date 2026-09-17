# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""The planner refuses to run when it cannot fingerprint clauses.

Without the clause export every `spec_clause` fingerprint hashes empty text, and a
plan run that way resets every verified unit to pending. It happened once.
"""

import json

import pytest

import grammar_plan


@pytest.fixture
def no_writes(monkeypatch):
    def refuse(unit):
        msg = f"planner wrote {unit.get('production')} without a clause export"
        raise AssertionError(msg)

    monkeypatch.setattr(grammar_plan, "save_unit", refuse)


@pytest.mark.usefixtures("no_writes")
def test_missing_export_refuses_before_writing(monkeypatch, tmp_path, capsys):
    monkeypatch.setenv("SV2_WIKI_CLAUSES", str(tmp_path / "absent.json"))
    assert grammar_plan.main([]) == 1
    assert "refusing to plan" in capsys.readouterr().out


@pytest.mark.usefixtures("no_writes")
def test_empty_export_refuses_before_writing(monkeypatch, tmp_path):
    empty = tmp_path / "bnf-clauses.json"
    empty.write_text("{}")
    monkeypatch.setenv("SV2_WIKI_CLAUSES", str(empty))
    assert grammar_plan.main([]) == 1


def test_present_export_is_returned(monkeypatch, tmp_path):
    export = tmp_path / "bnf-clauses.json"
    clauses = {"Name": {"ref": "kerml clause 8.2.3.2", "text": "Name = ID"}}
    export.write_text(json.dumps(clauses))
    monkeypatch.setenv("SV2_WIKI_CLAUSES", str(export))
    assert grammar_plan._clause_export() == clauses
