# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""A citation naming a receipt that was never pinned is a citation to nothing.

The receipt below is the real one for the SysML Parts textual-notation atom, clause
8.2.2.11, as recorded in .claude/state/wiki-receipts.json.
"""

import pytest

from check_citations import bad_file_ids, cited_receipts, unresolved

PINNED = {"bb70446a3b699d718b785ea1da3a1610dc7692ee363ca7b8f36a355abc5491bd"}


def document(note: str, kind: str = "spec_clause", ref: str = "SysML 8.2.2.11") -> dict:
    return {
        "deviations": [
            {"production": "PartDefinition", "evidence": [{"kind": kind, "ref": ref, "note": note}]}
        ]
    }


def test_a_truncated_receipt_resolves_against_its_full_form():
    # Entries quote receipts truncated with an ellipsis; a prefix must still resolve,
    # or every hand-written citation would fail.
    cited = cited_receipts(document("region_sha256 bb70446a3b699d71… retrieved"))
    assert unresolved(cited, PINNED) == []


def test_a_full_receipt_resolves():
    cited = cited_receipts(document(f"region_sha256 {next(iter(PINNED))}"))
    assert unresolved(cited, PINNED) == []


def test_a_receipt_that_was_never_pinned_fails():
    # The defect this check exists for: a plausible-looking hash nothing ever recorded.
    cited = cited_receipts(document("region_sha256 0123456789abcdef… retrieved"))
    assert unresolved(cited, PINNED) == [("PartDefinition", "0123456789abcdef")]


def test_a_note_with_no_receipt_is_not_a_failure():
    # Plenty of evidence is a clause reference with no hash; absence is not drift.
    assert cited_receipts(document("Located by searching the pinned bodies.")) == []


def test_receipts_are_found_in_the_ref_as_well_as_the_note():
    doc = document("", ref="atom X, region_sha256 bb70446a3b699d71")
    assert unresolved(cited_receipts(doc), PINNED) == []


@pytest.mark.parametrize("ref", ["ptc/25-02-15", "ptc/25-04-04", "formal/24-01-02"])
def test_well_formed_omg_document_ids_pass(ref):
    assert bad_file_ids(document("", kind="file_id", ref=ref)) == []


@pytest.mark.parametrize("ref", ["SysML.xmi", "25-02-15", "ptc/2025-02-15", ""])
def test_malformed_omg_document_ids_fail(ref):
    # A file_id is what a conformance claim cites; a free-text one cannot be chased.
    assert bad_file_ids(document("", kind="file_id", ref=ref)) == [("PartDefinition", ref)]


def test_a_short_hex_run_is_not_mistaken_for_a_receipt():
    # "8.2.2.16" and similar must not be read as hashes; a receipt is >= 16 hex chars.
    assert cited_receipts(document("clause 8.2.2.16, page 210, see abc123")) == []
