# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
from _lock import read_lock, read_target, write_hash

LOCK = """# header comment
[[file]]
path   = "vendor/a.xmi"   # trailing comment
url    = "https://example.invalid/a.xmi"
sha256 = "UNPINNED"

[[file]]
url = "https://example.invalid/no-path"

[[file]]
path   = "vendor/b.xmi"
sha256 = "old"
"""


def test_read_lock_skips_blocks_without_a_path(tmp_path):
    lock = tmp_path / "lock.toml"
    lock.write_text(LOCK)
    assert read_lock(str(lock)) == [
        {"path": "vendor/a.xmi", "url": "https://example.invalid/a.xmi", "sha256": "UNPINNED"},
        {"path": "vendor/b.xmi", "sha256": "old"},
    ]


def test_read_target_flattens_sections(tmp_path):
    target = tmp_path / "target.toml"
    target.write_text(
        '[tier_a_omg]\nnamespace = "20250201"  # dated\n[tier_b_pilot]\nrevision = "abc"\n'
    )
    assert read_target(str(target)) == {
        "tier_a_omg.namespace": "20250201",
        "tier_b_pilot.revision": "abc",
    }


def test_write_hash_replaces_only_the_matching_block(tmp_path):
    lock = tmp_path / "lock.toml"
    lock.write_text(LOCK)
    write_hash("vendor/b.xmi", "f" * 64, str(lock))
    entries = {e["path"]: e for e in read_lock(str(lock))}
    assert entries["vendor/b.xmi"]["sha256"] == "f" * 64
    assert entries["vendor/a.xmi"]["sha256"] == "UNPINNED"
