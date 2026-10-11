# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Readers for vendor/sources.lock.toml and docs/conformance-target.toml.

Both files are parsed with tomllib, so a key is always read from its own table:
a flat scan once took an earlier table's `revision` for the Pilot revision. Writing a hash
stays a line edit, which keeps the reviewed file's comments and layout intact — but the
block it edits is chosen from the parsed document, and the rewritten text is parsed again
and compared with the original before it replaces the file. A line edit that builds TOML
from a string can invent structure; one that has to survive that comparison cannot (§4.4).
"""

from __future__ import annotations

import re
import tomllib
from pathlib import Path
from typing import Any

LOCKFILE = "vendor/sources.lock.toml"
TARGET = "docs/conformance-target.toml"

# A pin is a sha256 and nothing else: 64 lowercase hex digits. A value reaching the writer
# that is not one is a bug upstream, or an attempt to inject TOML into the file that pins
# every vendored input.
DIGEST = re.compile(r"[0-9a-f]{64}")

# The other pinned value, and the other half of the same rule: tier_b_pilot.revision is a
# full commit sha. A branch name resolves and fetches, and then pins whatever that branch
# pointed at today as though it had been chosen — which is the failure pinning exists to
# prevent. Anything carrying `?`, `#`, `@` or a path segment reshapes the fetch URL it is
# substituted into (vendor_sync.py).
REVISION = re.compile(r"[0-9a-f]{40}")

_FILE_HEADER = re.compile(r"\s*\[\[file\]\]\s*(#.*)?")
_SHA256_LINE = re.compile(r'(?P<head>\s*sha256\s*=\s*)(?P<value>"[^"]*"|\S+)(?P<rest>\s*(#.*)?)')

# A [[file]] entry as tomllib deserializes it: a boundary shape, narrowed by its readers (§6).
Entry = dict[str, Any]


class LockError(Exception):
    """A lockfile could not be re-pinned. The entry point decides what the user sees."""


def read_lock(path: str = LOCKFILE) -> list[dict[str, str]]:
    """Every [[file]] entry that names a path, with its values as strings."""
    document = tomllib.loads(Path(path).read_text())
    entries = [{k: str(v) for k, v in entry.items()} for entry in document.get("file", [])]
    return [e for e in entries if e.get("path")]


def read_target(path: str = TARGET) -> dict[str, str]:
    """Every key in the conformance target's tables, flattened to ``table.key``."""
    document = tomllib.loads(Path(path).read_text())
    return {
        f"{table}.{key}": str(value)
        for table, values in document.items()
        if isinstance(values, dict)
        for key, value in values.items()
    }


def _block_index(entries: list[Entry], target_path: str, *, lockfile: str) -> int:
    """The index of the one [[file]] entry whose path is `target_path`."""
    matches = [i for i, entry in enumerate(entries) if entry.get("path") == target_path]
    if len(matches) != 1:
        message = f"{lockfile}: {len(matches)} [[file]] blocks have path {target_path}, wanted 1"
        raise LockError(message)
    return matches[0]


def _repinned(text: str, index: int, digest: str) -> str | None:
    """`text` with [[file]] block `index`'s sha256 replaced, or None if it has no such key.

    Splitting on a newline rather than with splitlines() round-trips the file exactly, so
    a line ending or a missing final newline is preserved rather than normalized.
    """
    lines = text.split("\n")
    block = -1
    for i, line in enumerate(lines):
        if _FILE_HEADER.fullmatch(line):
            block += 1
        elif block == index and (key := _SHA256_LINE.fullmatch(line)):
            lines[i] = f'{key["head"]}"{digest}"{key["rest"]}'
            return "\n".join(lines)
    return None


def _is_only_repin(before: list[Entry], after: list[Entry], index: int, digest: str) -> bool:
    """Whether `after` differs from `before` in nothing but entry `index`'s sha256."""
    expected = [dict(entry) for entry in before]
    expected[index] = {**before[index], "sha256": digest}
    return after == expected


def write_hash(target_path: str, *, digest: str, lockfile: str = LOCKFILE) -> None:
    """Replace the sha256 of the [[file]] block whose path is `target_path`.

    The block is chosen from the parsed document, so a `path` key outside a [[file]] table
    cannot match it and a path that is absent, duplicated, or followed by a comment is an
    error rather than a silent no-op. The edit stays a line edit, so comments and alignment
    survive; the rewritten text is then parsed and compared with the original, and the only
    admissible difference is this one entry's sha256. The file is replaced atomically, so a
    failure part way through cannot leave the repository's pins truncated.

    Args:
        target_path: the `path` of the [[file]] block to re-pin.
        digest: the sha256 to write, as 64 lowercase hex digits.
        lockfile: the lockfile to edit.

    Raises:
        LockError: the digest is not a sha256; the block is absent, duplicated, or carries
            no sha256 key; or the rewritten file would differ by more than that one pin.
    """
    if not DIGEST.fullmatch(digest):
        message = f"{lockfile}: refusing to write a value that is not a sha256 digest"
        raise LockError(message)

    lock = Path(lockfile)
    original = lock.read_text()
    before: list[Entry] = tomllib.loads(original).get("file", [])
    index = _block_index(before, target_path, lockfile=lockfile)

    candidate = _repinned(original, index, digest)
    if candidate is None:
        message = f"{lockfile}: the [[file]] block for {target_path} has no sha256 key"
        raise LockError(message)
    try:
        after: list[Entry] = tomllib.loads(candidate).get("file", [])
    except tomllib.TOMLDecodeError as exc:
        message = f"{lockfile}: re-pinning {target_path} would not leave it parseable"
        raise LockError(message) from exc
    if not _is_only_repin(before, after, index, digest):
        message = f"{lockfile}: re-pinning {target_path} would change more than its sha256"
        raise LockError(message)

    scratch = lock.with_name(f"{lock.name}.new")
    scratch.write_text(candidate)
    scratch.replace(lock)
