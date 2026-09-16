# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
from validate_state import structural_errors

SCHEMA = {
    "required": ["name", "gates"],
    "properties": {
        "name": {"type": "string", "minLength": 3},
        "version": {"const": 1},
        "gates": {"type": "object", "additionalProperties": {"enum": ["pass", "fail"]}},
        "log": {
            "type": "array",
            "items": {"required": ["date"], "properties": {"gate": {"enum": ["green", "red"]}}},
        },
    },
}


def test_valid_document_has_no_errors():
    assert structural_errors("s.json", {"name": "abc", "gates": {"t": "pass"}}, SCHEMA) == []


def test_every_structural_rule_is_reported_with_its_location():
    doc = {"name": "x", "version": 2, "gates": {"t": "maybe"}, "log": [{"gate": "blue"}]}
    assert structural_errors("s.json", doc, SCHEMA) == [
        "  s.json: <root>/name: too short — this field needs a real answer, not a placeholder",
        "  s.json: <root>/version: expected 1",
        "  s.json: <root>/gates/t: 'maybe' not in ['pass', 'fail']",
        "  s.json: <root>/log[0]: missing required key 'date'",
        "  s.json: <root>/log[0]/gate: 'blue' not in ['green', 'red']",
    ]


def test_missing_required_key_is_reported():
    assert structural_errors("s.json", {"name": "abc"}, SCHEMA) == [
        "  s.json: <root>: missing required key 'gates'",
    ]
