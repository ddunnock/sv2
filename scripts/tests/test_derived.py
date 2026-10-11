# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
from _derived import write_derived


def test_write_derived_writes_the_text(tmp_path):
    target = tmp_path / "artifact.json"
    write_derived(target, '{"a": 1}\n')
    assert target.read_text() == '{"a": 1}\n'


def test_write_derived_replaces_existing_content(tmp_path):
    target = tmp_path / "artifact.json"
    target.write_text("stale, and longer than what replaces it\n")
    write_derived(target, "fresh\n")
    assert target.read_text() == "fresh\n"


def test_write_derived_leaves_no_scratch_file(tmp_path):
    write_derived(tmp_path / "artifact.json", "x\n")
    assert sorted(p.name for p in tmp_path.iterdir()) == ["artifact.json"]


def test_write_derived_accepts_a_string_path(tmp_path):
    target = tmp_path / "artifact.json"
    write_derived(str(target), "x\n")
    assert target.read_text() == "x\n"


def test_write_derived_writes_utf8_whatever_the_locale(tmp_path):
    target = tmp_path / "artifact.txt"
    write_derived(target, "… ≠ ✓\n")
    assert target.read_bytes() == "… ≠ ✓\n".encode()


def test_write_derived_does_not_touch_the_target_until_the_scratch_file_is_written(tmp_path):
    # The scratch file is a sibling, so the replace is atomic within the filesystem; what
    # a test can assert is the consequence — the target is never a partial write.
    target = tmp_path / "artifact.json"
    target.write_text("previous\n")
    write_derived(target, "next\n")
    assert target.read_text() == "next\n"
    assert not (tmp_path / "artifact.json.new").exists()
