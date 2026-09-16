# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import pytest

from _state import pin_value

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
