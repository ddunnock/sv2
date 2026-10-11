# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Counting half-derived units: the check that holds a turn open on an unfinished rule."""

import json

from hook_stop_gate import _is_derived


def _unit(tmp_path, name, document):
    path = tmp_path / name
    path.write_text(document if isinstance(document, str) else json.dumps(document))
    return path


def test_a_unit_with_status_derived_counts(tmp_path):
    assert _is_derived(_unit(tmp_path, "a.json", {"production": "A", "status": "derived"}))


def test_a_unit_with_another_status_does_not_count(tmp_path):
    assert not _is_derived(_unit(tmp_path, "a.json", {"production": "A", "status": "accepted"}))


def test_spacing_does_not_decide_it(tmp_path):
    # The substring scan this replaced required exactly '"status": "derived"'. A writer
    # emitting compact JSON would have made every half-derived unit invisible.
    compact = '{"production":"A","status":"derived"}'
    assert _is_derived(_unit(tmp_path, "a.json", compact))


def test_the_literal_in_another_field_does_not_count(tmp_path):
    unit = {"production": "A", "status": "accepted", "notes": ['was "status": "derived" before']}
    assert not _is_derived(_unit(tmp_path, "a.json", unit))


def test_a_unit_that_is_not_json_does_not_count(tmp_path):
    assert not _is_derived(_unit(tmp_path, "a.json", "{not json"))


def test_a_missing_file_does_not_count(tmp_path):
    assert not _is_derived(tmp_path / "absent.json")
