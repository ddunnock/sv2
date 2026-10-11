# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import pytest

from _state import pin_value, write_derived
from regen_state import is_stale, pins

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


def test_write_derived_replaces_the_file(tmp_path):
    target = tmp_path / "state.json"
    target.write_text("stale, and longer than what replaces it\n")
    write_derived(target, "fresh\n")
    assert target.read_text() == "fresh\n"


def test_write_derived_leaves_no_scratch_file(tmp_path):
    write_derived(tmp_path / "state.json", "x\n")
    assert sorted(p.name for p in tmp_path.iterdir()) == ["state.json"]


def test_write_derived_writes_utf8_whatever_the_locale(tmp_path):
    target = tmp_path / "state.json"
    write_derived(target, "… ≠ ✓\n")
    assert target.read_bytes() == "… ≠ ✓\n".encode()


LOCK = """# a comment mentioning sha256 = "0000000000000000000000000000000000000000"
[[file]]
path = "vendor/a.xmi"
sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"

[[file]]
path = "vendor/b.xmi"
sha256 = 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb'

  [[file]]
  path = "vendor/c.xmi"
  sha256 = "UNPINNED"

[[file]]
path = "vendor/d.xmi"
sha256 = "CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC"
"""


@pytest.fixture
def pinned(tmp_path, monkeypatch):
    """A workspace with a lockfile whose shape defeats a regex scan."""
    (tmp_path / "vendor").mkdir()
    (tmp_path / "vendor" / "sources.lock.toml").write_text(LOCK)
    (tmp_path / "docs").mkdir()
    (tmp_path / "docs" / "conformance-target.toml").write_text(TARGET)
    monkeypatch.chdir(tmp_path)


@pytest.mark.usefixtures("pinned")
def test_pins_count_every_file_entry_however_it_is_written():
    # Four entries: one of them indented, which an ^-anchored scan of [[file]] missed.
    assert pins()["vendor_total"] == 4


@pytest.mark.usefixtures("pinned")
def test_pins_count_a_single_quoted_digest_as_pinned():
    # Two are real digests, one single-quoted; UNPINNED and the uppercase one are not.
    assert pins()["vendor_pinned"] == 2


@pytest.mark.usefixtures("pinned")
def test_pins_do_not_count_a_digest_inside_a_comment():
    assert pins()["vendor_pinned"] < pins()["vendor_total"]


@pytest.mark.usefixtures("pinned")
def test_pins_read_the_conformance_target_table_qualified():
    assert pins()["pilot_revision"] == "0123abc"
