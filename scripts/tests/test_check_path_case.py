# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Two tracked paths that are one file on a case-insensitive disk must fail.

The first case is the pair that was actually tracked in this repository from
3eaa618 until the retired spelling was untracked.
"""

from check_path_case import collisions

UNITS = ".claude/state/grammar/units"


def test_the_pair_this_check_was_written_for_collides():
    paths = [
        f"{UNITS}/MetaClassificationTestOperator.json",
        f"{UNITS}/MetaclassificationTestOperator.json",
    ]
    assert collisions(paths) == [sorted(paths)]


def test_distinct_names_do_not_collide():
    # The negative case: similar names that are different files on any disk.
    paths = [f"{UNITS}/OwnedSubclassification.json", f"{UNITS}/OwnedSubclassifications.json"]
    assert collisions(paths) == []


def test_a_path_listed_once_does_not_collide_with_itself():
    paths = ["README.md", "README.md"]
    assert collisions(paths) == []


def test_directories_that_differ_only_by_case_collide():
    # No two FILES share a folded name here; the two directories do.
    assert collisions(["Docs/a.md", "docs/b.md"]) == [["Docs", "docs"]]


def test_unicode_normalization_forms_collide():
    composed = "café.md"  # é as one code point (NFC)
    decomposed = "café.md"  # e + combining acute (NFD)
    assert collisions([composed, decomposed]) == [sorted([composed, decomposed])]
