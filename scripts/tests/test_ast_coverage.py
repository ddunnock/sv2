# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import json

import pytest

import ast_coverage
from ast_coverage import build_report, claims, is_stale, layer_of, worklist


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


def test_the_root_is_layer_zero():
    reference = {"grammar": {"RootNamespace": {"rule": {"k": "ref", "name": "Member"}}}}
    assert layer_of(reference)["RootNamespace"] == 0


def test_a_production_the_root_reaches_is_layer_one():
    reference = {
        "grammar": {
            "RootNamespace": {"rule": {"k": "ref", "name": "Member"}},
            "Member": {"rule": {"k": "kw", "text": "part"}},
        }
    }
    assert layer_of(reference)["Member"] == 1


def test_a_variant_unit_is_layered_under_its_base_name():
    # The grammar is keyed by unit, so RootNamespace may arrive as RootNamespace@kerml.
    reference = {
        "grammar": {
            "RootNamespace@kerml": {"rule": {"k": "ref", "name": "Member"}},
            "RootNamespace@sysml": {"rule": {"k": "ref", "name": "Other"}},
            "Member": {"rule": {"k": "kw", "text": "part"}},
            "Other": {"rule": {"k": "kw", "text": "item"}},
        }
    }
    assert layer_of(reference) == {"RootNamespace": 0, "Member": 1, "Other": 1}


def test_a_reference_nested_in_a_sequence_is_found():
    reference = {
        "grammar": {
            "RootNamespace": {"rule": {"k": "seq", "items": [{"k": "ref", "name": "Deep"}]}},
            "Deep": {"rule": {"k": "kw", "text": "x"}},
        }
    }
    assert layer_of(reference)["Deep"] == 1


def test_an_unreached_production_has_no_layer():
    reference = {
        "grammar": {
            "RootNamespace": {"rule": {"k": "kw", "text": "x"}},
            "Island": {"rule": {"k": "kw", "text": "y"}},
        }
    }
    assert "Island" not in layer_of(reference)


def test_a_cycle_terminates():
    reference = {
        "grammar": {
            "RootNamespace": {"rule": {"k": "ref", "name": "A"}},
            "A": {"rule": {"k": "ref", "name": "RootNamespace"}},
        }
    }
    assert layer_of(reference) == {"RootNamespace": 0, "A": 1}


def test_the_worklist_sorts_shallow_kinds_first():
    assert worklist(["Zebra", "Member", "RootNamespace"], {"RootNamespace": 0, "Member": 1}) == [
        "RootNamespace",
        "Member",
        "Zebra",
    ]


def test_a_kind_with_no_layer_keeps_declaration_order_among_its_peers():
    assert worklist(["Beta", "Alpha"], {}) == ["Beta", "Alpha"]
