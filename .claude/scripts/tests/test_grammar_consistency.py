# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
from grammar_consistency import cycle_hits, undefined_references, unreachable


def unit(*refs):
    return {"rule": {"k": "seq", "items": [{"k": "ref", "name": r} for r in refs]}}


HAVE = {"Root": unit("A"), "A": unit("B"), "B": unit("A", "Missing"), "Orphan": unit()}


def test_undefined_references_lists_missing_targets():
    assert undefined_references(HAVE) == {"B": ["Missing"]}


def test_unreachable_excludes_everything_reachable_from_start():
    assert unreachable(HAVE, ["Root"]) == ["Orphan"]


def test_cycle_hits_counts_back_references():
    assert cycle_hits("A", HAVE) == 1
    assert cycle_hits("Root", HAVE) == 0
