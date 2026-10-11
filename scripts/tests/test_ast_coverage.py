# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import json

import pytest

import ast_coverage
from ast_coverage import build_report, claims, is_stale


def _crate(tmp_path, **files):
    src = tmp_path / "src"
    src.mkdir()
    for name, text in files.items():
        (src / name).write_text(text)
    return tmp_path


def test_a_marker_claims_its_kind(tmp_path):
    root = _crate(tmp_path, **{"ns.rs": "// node: RootNamespace\npub struct RootNamespace;"})
    assert claims(root) == {"RootNamespace": ["src/ns.rs"]}


def test_a_kind_claimed_twice_records_both_files(tmp_path):
    root = _crate(
        tmp_path,
        **{"a.rs": "// node: RootNamespace\n", "b.rs": "// node: RootNamespace\n"},
    )
    assert claims(root)["RootNamespace"] == ["src/a.rs", "src/b.rs"]


def test_spacing_in_the_marker_does_not_matter(tmp_path):
    root = _crate(tmp_path, **{"a.rs": "//node:RootNamespace\n"})
    assert "RootNamespace" in claims(root)


def test_a_kind_no_marker_claims_is_unimplemented():
    report = build_report({"A", "B"}, ["A"], ["B"], [])
    assert report["unimplemented"] == 1
    assert report["unimplemented_kinds"] == ["B"]
    assert report["absent"] == 0


def test_a_marker_naming_no_kind_is_absent():
    report = build_report({"A"}, [], ["A"], ["NotAKind"])
    assert report["absent"] == 1
    assert report["absent_markers"] == ["NotAKind"]


def test_an_empty_crate_reports_every_kind_unimplemented():
    report = build_report({"A", "B"}, [], ["A", "B"], [])
    assert (report["implemented"], report["unimplemented"], report["percent"]) == (0, 2, 0.0)


def test_a_report_matching_disk_is_not_stale(tmp_path, monkeypatch):
    path = tmp_path / "ast-coverage.json"
    monkeypatch.setattr(ast_coverage, "REPORT", path)
    report = build_report({"A"}, [], ["A"], [])
    path.write_text(json.dumps(report, indent=2) + "\n")
    assert not is_stale(report)


@pytest.mark.parametrize("damage", ["missing", "truncated", "changed"])
def test_a_report_disagreeing_with_disk_is_stale(damage, tmp_path, monkeypatch):
    path = tmp_path / "ast-coverage.json"
    monkeypatch.setattr(ast_coverage, "REPORT", path)
    report = build_report({"A"}, [], ["A"], [])
    if damage == "truncated":
        path.write_text('{"implemented":')
    elif damage == "changed":
        path.write_text(json.dumps(build_report({"A"}, ["A"], [], []), indent=2) + "\n")
    assert is_stale(report)
