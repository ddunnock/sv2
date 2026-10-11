# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import pytest

from _lock import LockError, read_lock, read_target, write_hash

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

PIN = "f" * 64

# A digest carrying a quote and a newline closes the sha256 string and opens keys of its
# own. The payload below leaves the file valid TOML, so only a check on the value, or on
# the parsed result, catches it: before this was validated it repointed `url` at another
# host, and vendor_sync.py fetches from `url`.
INJECTION = f'{"a" * 64}"\nurl = "https://attacker.invalid/payload.xmi"\nunused = "'


def _lockfile(tmp_path, text=LOCK):
    lock = tmp_path / "lock.toml"
    lock.write_text(text)
    return lock


def test_read_lock_skips_blocks_without_a_path(tmp_path):
    lock = _lockfile(tmp_path)
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
    lock = _lockfile(tmp_path)
    write_hash("vendor/b.xmi", digest=PIN, lockfile=str(lock))
    entries = {e["path"]: e for e in read_lock(str(lock))}
    assert entries["vendor/b.xmi"]["sha256"] == PIN
    assert entries["vendor/a.xmi"]["sha256"] == "UNPINNED"


def test_write_hash_pins_a_block_whose_path_line_carries_a_comment(tmp_path):
    lock = _lockfile(tmp_path)
    write_hash("vendor/a.xmi", digest=PIN, lockfile=str(lock))
    entries = {e["path"]: e for e in read_lock(str(lock))}
    assert entries["vendor/a.xmi"]["sha256"] == PIN


def test_write_hash_keeps_every_other_line_byte_for_byte(tmp_path):
    lock = _lockfile(tmp_path)
    write_hash("vendor/b.xmi", digest=PIN, lockfile=str(lock))
    was = LOCK.split("\n")
    now = lock.read_text().split("\n")
    assert len(was) == len(now)
    assert [i for i, (a, b) in enumerate(zip(was, now, strict=True)) if a != b] == [11]


def test_write_hash_keeps_the_sha256_key_alignment(tmp_path):
    lock = _lockfile(tmp_path, '[[file]]\npath   = "vendor/a.xmi"\nsha256 = "old"  # pinned\n')
    write_hash("vendor/a.xmi", digest=PIN, lockfile=str(lock))
    assert lock.read_text() == f'[[file]]\npath   = "vendor/a.xmi"\nsha256 = "{PIN}"  # pinned\n'


@pytest.mark.parametrize(
    "digest",
    [
        INJECTION,
        "old",
        "",
        "F" * 64,
        "a" * 63,
        "a" * 65,
        "a" * 64 + " ",
        "../../etc/passwd",
    ],
)
def test_write_hash_rejects_a_digest_that_is_not_a_sha256(tmp_path, digest):
    lock = _lockfile(tmp_path)
    with pytest.raises(LockError):
        write_hash("vendor/b.xmi", digest=digest, lockfile=str(lock))
    assert lock.read_text() == LOCK


def test_write_hash_message_does_not_echo_the_rejected_value(tmp_path):
    lock = _lockfile(tmp_path)
    with pytest.raises(LockError) as caught:
        write_hash("vendor/b.xmi", digest=INJECTION, lockfile=str(lock))
    assert "attacker.invalid" not in str(caught.value)


def test_write_hash_rejects_a_path_that_is_absent(tmp_path):
    lock = _lockfile(tmp_path)
    with pytest.raises(LockError):
        write_hash("vendor/absent.xmi", digest=PIN, lockfile=str(lock))
    assert lock.read_text() == LOCK


def test_write_hash_rejects_a_duplicated_path(tmp_path):
    text = (
        '[[file]]\npath = "vendor/a.xmi"\nsha256 = "one"\n\n'
        '[[file]]\npath = "vendor/a.xmi"\nsha256 = "two"\n'
    )
    lock = _lockfile(tmp_path, text)
    with pytest.raises(LockError):
        write_hash("vendor/a.xmi", digest=PIN, lockfile=str(lock))
    assert lock.read_text() == text


def test_write_hash_rejects_a_block_with_no_sha256_key(tmp_path):
    text = '[[file]]\npath = "vendor/a.xmi"\nurl = "https://example.invalid/a.xmi"\n'
    lock = _lockfile(tmp_path, text)
    with pytest.raises(LockError):
        write_hash("vendor/a.xmi", digest=PIN, lockfile=str(lock))
    assert lock.read_text() == text


def test_write_hash_ignores_a_path_key_outside_a_file_block(tmp_path):
    text = (
        '[tool]\npath = "vendor/a.xmi"\nsha256 = "untouched"\n\n'
        '[[file]]\npath = "vendor/a.xmi"\nsha256 = "old"\n'
    )
    lock = _lockfile(tmp_path, text)
    write_hash("vendor/a.xmi", digest=PIN, lockfile=str(lock))
    assert lock.read_text() == text.replace('"old"', f'"{PIN}"')


def test_write_hash_leaves_no_scratch_file_behind(tmp_path):
    lock = _lockfile(tmp_path)
    write_hash("vendor/b.xmi", digest=PIN, lockfile=str(lock))
    assert [p.name for p in tmp_path.iterdir()] == ["lock.toml"]
