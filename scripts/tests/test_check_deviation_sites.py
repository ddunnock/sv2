# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""A departure from the specification must be reported where the parser takes it."""

from pathlib import Path

from check_deviation_sites import PENDING, departing, parser_sources, pending, problems, sites

REGISTER = {
    "deviations": [
        {"production": "SendNode", "decision": "follow_xtext"},
        {"production": "Flow", "decision": "follow_xtext"},
        {"production": "PartUsage", "decision": "follow_spec"},
    ]
}


def test_only_departing_decisions_count():
    assert departing(REGISTER) == {"SendNode", "Flow"}


def test_a_sited_and_a_listed_departure_pass():
    calls, markers = sites('// deviation: SendNode\nself.note_deviation("SendNode", "x");')
    assert problems({"SendNode", "Flow"}, calls, markers, {"Flow": "KerML only"}) == []


def test_a_departure_with_no_site_and_no_listing_fails():
    assert problems({"Flow"}, [], [], {}) == [
        "Flow: departs from the specification but has no site and is not listed pending"
    ]


def test_a_site_naming_a_non_departure_fails():
    calls, markers = sites('// deviation: PartUsage\nself.note_deviation("PartUsage", "x");')
    assert any(
        "not a departing entry" in p
        for p in problems({"SendNode"}, calls, markers, {"SendNode": "r"})
    )


def test_a_call_without_its_marker_fails():
    calls, markers = sites('self.note_deviation("SendNode", "x");')
    assert problems({"SendNode"}, calls, markers, {}) == [
        "note_deviation('SendNode') has no `// deviation: SendNode` marker"
    ]


def test_sited_and_listed_is_stale():
    calls, markers = sites('// deviation: SendNode\nself.note_deviation("SendNode", "x");')
    found = problems({"SendNode"}, calls, markers, {"SendNode": "r"})
    assert found == [f"SendNode: sited AND listed pending; remove it from {PENDING}"]


def test_a_listing_needs_a_reason():
    assert pending("# header\n\nFlow  # KerML only\nStep\n") == {"Flow": "KerML only", "Step": ""}
    assert "Step: listed pending with no reason" in problems({"Step"}, [], [], {"Step": ""})


def test_sites_in_the_parser_child_modules_are_read(tmp_path: Path):
    parser = tmp_path / "parser.rs"
    parser.write_text("mod action_node;\n")
    (tmp_path / "parser").mkdir()
    child = tmp_path / "parser" / "action_node.rs"
    child.write_text('// deviation: SendNode\nself.note_deviation("SendNode", "x");\n')
    assert parser_sources(parser) == [parser, child]
    source = "\n".join(p.read_text() for p in parser_sources(parser))
    assert sites(source) == (["SendNode"], ["SendNode"])


def test_a_parser_without_child_modules_is_read_alone(tmp_path: Path):
    parser = tmp_path / "parser.rs"
    parser.write_text("")
    assert parser_sources(parser) == [parser]
