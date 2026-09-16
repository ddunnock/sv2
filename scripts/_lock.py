# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Readers for vendor/sources.lock.toml and docs/conformance-target.toml.

Both files are parsed with tomllib, so a key is always read from its own table:
a flat scan once took an earlier table's `revision` for the Pilot revision. Writing a hash
stays a line edit, which keeps the reviewed file's comments and layout intact.
"""

from __future__ import annotations

import re
import tomllib
from pathlib import Path

LOCKFILE = "vendor/sources.lock.toml"
TARGET = "docs/conformance-target.toml"


def _unquote(value: str) -> str:
    value = value.strip()
    if value.startswith('"') and value.endswith('"'):
        return value[1:-1]
    return value


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


def write_hash(target_path: str, digest: str, lockfile: str = LOCKFILE) -> None:
    """Replace the sha256 of the [[file]] block whose path matches target_path."""
    lock = Path(lockfile)
    lines = lock.read_text().splitlines()
    in_block = False
    for i, line in enumerate(lines):
        if line.strip() == "[[file]]":
            in_block = False
        if re.match(r"\s*path\s*=", line) and _unquote(line.split("=", 1)[1]) == target_path:
            in_block = True
            continue
        if in_block and re.match(r"\s*sha256\s*=", line):
            lines[i] = f'sha256  = "{digest}"'
            in_block = False
    lock.write_text("\n".join(lines) + "\n")
