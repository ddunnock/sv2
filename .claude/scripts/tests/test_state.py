# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import pytest

from _state import pin_value
from regen_state import is_stale

TARGET = """[corpus]
revision = "corpus-rev"   # an earlier table sharing the key name

[tier_a_omg]
namespace = "20250201"

[tier_b_pilot]
revision = "0123abc"
"""


@pytest.mark.parametrize(
    ("key", "expected"),
    [
        ("tier_b_pilot.revision", "0123abc"),
        ("corpus.revision", "corpus-rev"),
        ("tier_a_omg.namespace", "20250201"),
        ("tier_b_pilot.bundle_version", "unset"),
        ("missing_table.key", "unset"),
    ],
)
def test_pin_value_reads_from_the_named_table(tmp_path, key, expected):
    target = tmp_path / "target.toml"
    target.write_text(TARGET)
    assert pin_value(key, target) == expected


def test_pin_value_without_a_target_is_unset(tmp_path):
    assert pin_value("tier_a_omg.namespace", tmp_path / "absent.toml") == "unset"


BLOCK = {
    "coverage": {"declared": 558, "implemented": 5},
    "tests": {"unit": 55},
    "pins": {"vendor_pinned": 332},
    "commits": ["124b074 Initial commit"],
}


def test_a_new_commit_does_not_make_the_block_stale():
    # The reason COMPARED exists: the commit carrying state.json cannot record its
    # own hash, so a check that compared `commits` would be red after every commit.
    measured = BLOCK | {"commits": ["4499097 Record the initial commit", *BLOCK["commits"]]}
    assert not is_stale(BLOCK, measured)


@pytest.mark.parametrize("key", ["coverage", "tests", "pins"])
def test_a_measurement_that_moved_is_stale(key):
    assert is_stale(BLOCK, BLOCK | {key: {"moved": True}})


def test_a_measurement_that_went_missing_is_stale():
    assert is_stale({k: v for k, v in BLOCK.items() if k != "pins"}, BLOCK)
