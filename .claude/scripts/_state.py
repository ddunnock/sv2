# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Shared access to the repository state the Claude Code tooling reads and writes."""

from __future__ import annotations

import json
import tomllib
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
CONFORMANCE_TARGET = Path("docs/conformance-target.toml")
STATE = Path(".claude/state/state.json")

# Any: JSON documents are deserialized from disk and narrowed by their callers
# (STD-001-PY §6, deserialization boundary).
Json = Any


def load_json(path: str | Path, default: Json = None) -> Json:
    """The parsed document at ``path``, or ``default`` when the file does not exist."""
    p = Path(path)
    return json.loads(p.read_text()) if p.exists() else default


def write_derived(path: str | Path, text: str) -> None:
    """Replace ``path`` with ``text`` in one step, through a scratch file beside it.

    A plain write truncates first, so an interrupted one leaves a derived file empty or
    half written and the next reader cannot tell that from a real result. It matters most
    where the file cannot be regenerated: ``state.json`` carries a hand-written block, and
    the frozen-grammar ledger is append-only. ``Path.replace`` is atomic within a
    filesystem, which is why the scratch file is a sibling rather than a file in /tmp.

    The sibling is also why this exists twice, once here and once as
    ``scripts/_derived.py``: STD-001-PY §2 keeps the two script trees from sharing code.
    The encoding is named rather than left to the locale, because the gate compares these
    files byte for byte, on two machines.
    """
    target = Path(path)
    scratch = target.with_name(f"{target.name}.new")
    scratch.write_text(text, encoding="utf-8")
    scratch.replace(target)


def utc_now() -> str:
    """The current time as the ISO-8601 UTC stamp every state file uses."""
    return datetime.now(UTC).strftime("%Y-%m-%dT%H:%M:%SZ")


def pin_value(table_key: str, path: Path = CONFORMANCE_TARGET) -> str:
    """The value of ``table.key`` in the conformance target, or "unset".

    Always table-qualified: several tables share key names such as ``revision``.
    """
    if not path.exists():
        return "unset"
    table, _, key = table_key.partition(".")
    values = tomllib.loads(path.read_text()).get(table, {})
    value = values.get(key) if isinstance(values, dict) else None
    return "unset" if value is None else str(value)
