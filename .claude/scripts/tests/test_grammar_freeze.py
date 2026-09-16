# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""The freeze check tolerates a derivation in progress and nothing else."""

from __future__ import annotations

import pytest

import grammar_freeze


def unit(name: str, *, status: str, rule: bool) -> dict:
    """One unit as the freeze check reads it."""
    out: dict = {"production": name, "status": status, "fingerprint": {"combined": name}}
    if rule:
        out["rule"] = {"k": "ref", "name": "Other"}
        out["inputs"] = {"metaclass": ""}
    return out


@pytest.fixture
def unfrozen(monkeypatch, tmp_path):
    """No reference grammar on disk, which is the state before a first freeze."""
    monkeypatch.setattr(grammar_freeze, "REFERENCE", tmp_path / "absent.json")


def check(units: dict[str, dict]) -> int:
    """Run the check the way main does, with a body and digest over the same units."""
    return grammar_freeze.check(units, grammar_freeze.grammar_body(units), "d" * 64)


@pytest.mark.usefixtures("unfrozen")
def test_nothing_derived_is_inert(capsys):
    # A fresh clone must not fail its own gate.
    units = {"A": unit("A", status="pending", rule=False)}
    assert check(units) == 0
    assert "inert" in capsys.readouterr().out


@pytest.mark.usefixtures("unfrozen")
def test_a_derivation_in_progress_is_inert(capsys):
    # The derive-grammar skill puts the checkpoint on the unit files so a session can
    # derive a batch and stop. A check that went red on the first rule would mean no
    # session that reached phase 3 could end green.
    units = {
        "A": unit("A", status="verified", rule=True),
        "B": unit("B", status="pending", rule=False),
    }
    assert check(units) == 0
    out = capsys.readouterr().out
    assert "in progress" in out
    assert "1/2 verified" in out


@pytest.mark.usefixtures("unfrozen")
def test_a_finished_but_unfrozen_derivation_fails(capsys):
    # The state the check exists to catch: nothing left to derive and no frozen
    # reference. This is the boundary — one unverified unit above, none here.
    units = {
        "A": unit("A", status="verified", rule=True),
        "B": unit("B", status="verified", rule=True),
    }
    assert check(units) == 1
    assert "never been frozen" in capsys.readouterr().out


def test_a_frozen_grammar_that_no_longer_matches_fails(monkeypatch, tmp_path, capsys):
    reference = tmp_path / "reference.json"
    reference.write_text('{"grammar_sha256": "' + "a" * 64 + '"}')
    monkeypatch.setattr(grammar_freeze, "REFERENCE", reference)
    units = {"A": unit("A", status="verified", rule=True)}
    assert check(units) == 1
    assert "no longer matches" in capsys.readouterr().out
