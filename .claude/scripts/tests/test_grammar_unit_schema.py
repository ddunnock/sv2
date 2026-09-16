# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""The unit schema has to describe what units actually are.

It once did not: `decision` was a four-value enum while every unit on disk held
prose, and nothing noticed because nothing validated the units. These tests are
the thing that would have noticed.
"""

import json
from pathlib import Path

import pytest

from validate_state import UNIT_SCHEMA, unit_errors, unit_paths

jsonschema = pytest.importorskip("jsonschema")


def _verified():
    """A unit that has been through the checker and passed."""
    return {
        "schema_version": 1,
        "production": "Thing",
        "status": "verified",
        "fingerprint": {"combined": "abc"},
        "inputs": {},
        "rule": {"k": "kw", "text": "thing"},
        "decision": "Follows the clause.",
        "evidence": [{"kind": "clause", "ref": "kerml clause 1.2.3"}],
        "acceptance": {"has_rule": "pass"},
    }


@pytest.fixture(scope="module")
def validator():
    schema = json.loads((Path(__file__).parents[3] / UNIT_SCHEMA).read_text())
    return jsonschema.Draft202012Validator(schema)


def errors(validator, **over):
    unit = _verified()
    unit.update(over)
    return [e.message for e in validator.iter_errors(unit)]


def test_every_unit_on_disk_validates():
    assert unit_errors() == []


def test_the_repository_actually_has_units_to_validate():
    # Guards the test above from passing because the glob matched nothing.
    assert len(unit_paths()) > 100


def test_a_representative_verified_unit_validates(validator):
    assert errors(validator) == []


def test_decision_is_prose_not_an_enum(validator):
    assert errors(validator, decision="The clause and the Xtext agree exactly.") == []


def test_decision_rejects_a_placeholder(validator):
    assert errors(validator, decision="TBD")


def test_verified_requires_the_evidence_of_having_been_checked(validator):
    for missing in ("rule", "decision", "evidence", "acceptance"):
        unit = {k: v for k, v in _verified().items() if k != missing}
        assert [e.message for e in validator.iter_errors(unit)], f"{missing} went unnoticed"


def test_a_conflict_must_record_both_readings(validator):
    # `required` alone is satisfied by an empty string, which records nothing.
    assert errors(validator, status="conflict", notes="")
    assert errors(validator, status="conflict", notes="KerML 8.2 says X; SysML 8.3 says Y.") == []


def test_verified_cannot_claim_an_empty_acceptance_record(validator):
    assert errors(validator, acceptance={})


def test_pending_units_carry_no_rule_and_that_is_fine(validator):
    unit = {
        "schema_version": 1,
        "production": "Thing",
        "status": "pending",
        "fingerprint": {"combined": "abc"},
        "inputs": {},
    }
    assert [e.message for e in validator.iter_errors(unit)] == []


def test_evidence_kinds_are_the_ones_units_use(validator):
    for kind in ("clause", "corpus", "xtext", "deviation", "omg_issue", "file_id"):
        assert errors(validator, evidence=[{"kind": kind, "ref": "x"}]) == [], kind
    assert errors(validator, evidence=[{"kind": "spec_clause", "ref": "x"}])


def test_diagnostics_are_separate_from_notes(validator):
    assert errors(validator, notes="prose", diagnostics=["a check failed"]) == []


def test_an_unknown_top_level_key_is_rejected(validator):
    assert errors(validator, freeform="anything")


def test_a_lexical_terminal_is_a_tok_node(validator):
    assert errors(validator, rule={"k": "tok", "name": "NAME"}) == []
    assert errors(validator, rule={"k": "terminal", "name": "NAME"})
