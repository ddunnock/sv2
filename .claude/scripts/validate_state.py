# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Validate the authored JSON state files against their schemas.

Covers the two single documents — state.json and deviations.json — and every
derived grammar unit. The units are the bulk of the authored state by a wide
margin, and until they were validated here a schema could describe something no
unit had ever looked like without anything noticing.

Uses jsonschema when available; otherwise falls back to a structural check that
covers required keys, enums, and the minLength rules that stop "TBD" from being
accepted as a next step. The fallback never passes something the real validator
would reject on those grounds, but it is a subset: it does not understand $ref
or the conditional requirements in the unit schema, so a machine without
jsonschema checks the units less thoroughly than the gate does.

    python3.12 .claude/scripts/validate_state.py
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
from typing import TYPE_CHECKING

from _state import REPO_ROOT

if TYPE_CHECKING:
    from _state import Json

try:
    import jsonschema

    HAVE_JSONSCHEMA = True
except ImportError:  # optional dependency: the structural fallback below covers it
    HAVE_JSONSCHEMA = False

PAIRS = [
    (".claude/state/state.json", ".claude/state/schema/state.schema.json"),
    (".claude/state/deviations.json", ".claude/state/schema/deviations.schema.json"),
]

UNIT_SCHEMA = ".claude/state/schema/grammar-unit.schema.json"
UNIT_GLOB = ".claude/state/grammar/units/*.json"

# 708 units share one schema, so a single mistake in a generator prints 708
# times. Enough to diagnose it, then a count.
MAX_REPORTED = 20


def schema_errors(doc_path: str, doc: Json, schema: Json) -> list[str]:
    """Full JSON Schema validation messages."""
    return _validator_errors(doc_path, doc, jsonschema.Draft202012Validator(schema))


def _validator_errors(doc_path: str, doc: Json, validator: object) -> list[str]:
    errors = sorted(validator.iter_errors(doc), key=lambda e: e.path)  # type: ignore[attr-defined]
    return [
        f"  {doc_path}: {'/'.join(str(x) for x in e.path) or '<root>'}: {e.message}" for e in errors
    ]


def unit_paths() -> list[Path]:
    """Every derived grammar unit, resolved against the repository, not the CWD."""
    return sorted(REPO_ROOT.glob(UNIT_GLOB))


def unit_errors() -> list[str]:
    """Validate every derived grammar unit against the shared unit schema."""
    schema = json.loads((REPO_ROOT / UNIT_SCHEMA).read_text())
    validator = jsonschema.Draft202012Validator(schema) if HAVE_JSONSCHEMA else None

    messages: list[str] = []
    for path in unit_paths():
        where = str(path.relative_to(REPO_ROOT))
        doc = json.loads(path.read_text())
        messages += (
            _validator_errors(where, doc, validator)
            if validator is not None
            else structural_errors(where, doc, schema)
        )
    return messages


def _required(doc_path: str, obj: Json, schema: Json, where: str) -> list[str]:
    return [
        f"  {doc_path}: {where}: missing required key '{k}'"
        for k in schema.get("required", [])
        if k not in obj
    ]


def _additional(doc_path: str, obj: Json, schema: Json, where: str) -> list[str]:
    # additionalProperties carries the value schema for open maps such as
    # `gates`. Skipping it let any string through as a gate result.
    ap = schema.get("additionalProperties")
    if not isinstance(ap, dict) or "enum" not in ap:
        return []
    return [
        f"  {doc_path}: {where}/{k}: '{v}' not in {ap['enum']}"
        for k, v in obj.items()
        if k not in schema.get("properties", {}) and v not in ap["enum"]
    ]


def _property(doc_path: str, value: Json, sub: Json, where: str) -> list[str]:
    out = []
    if "enum" in sub and value not in sub["enum"]:
        out.append(f"  {doc_path}: {where}: '{value}' not in {sub['enum']}")
    if "const" in sub and value != sub["const"]:
        out.append(f"  {doc_path}: {where}: expected {sub['const']!r}")
    kind = sub.get("type")
    if kind == "string" and isinstance(value, str) and len(value) < sub.get("minLength", 0):
        out.append(
            f"  {doc_path}: {where}: too short — this field needs a real answer, not a placeholder"
        )
    if kind == "object" and isinstance(value, dict):
        out += structural_errors(doc_path, value, sub, where)
    if kind == "array" and isinstance(value, list) and "items" in sub:
        for i, item in enumerate(value):
            if isinstance(item, dict):
                out += structural_errors(doc_path, item, sub["items"], f"{where}[{i}]")
    return out


def structural_errors(doc_path: str, obj: Json, schema: Json, where: str = "<root>") -> list[str]:
    """The fallback check: required keys, enums, consts, and minimum lengths."""
    out = _required(doc_path, obj, schema, where) + _additional(doc_path, obj, schema, where)
    for k, sub in schema.get("properties", {}).items():
        if k in obj:
            out += _property(doc_path, obj[k], sub, f"{where}/{k}")
    return out


def main(argv: list[str] | None = None) -> int:
    """Validate the authored state files; 1 if any fails its schema."""
    argparse.ArgumentParser(description=__doc__).parse_args(argv)
    os.chdir(REPO_ROOT)

    check = schema_errors if HAVE_JSONSCHEMA else structural_errors
    messages: list[str] = []
    for doc_path, schema_path in PAIRS:
        doc = json.loads(Path(doc_path).read_text())
        schema = json.loads(Path(schema_path).read_text())
        messages += check(doc_path, doc, schema)
    messages += unit_errors()

    if messages:
        print("\n".join(messages[:MAX_REPORTED]))
        if len(messages) > MAX_REPORTED:
            print(f"  ... and {len(messages) - MAX_REPORTED} more")
        print()
        print("Schema validation failed. The state file is the handoff to the next session;")
        print("an invalid one is worse than none.")
        return 1

    unit_count = len(unit_paths())
    if HAVE_JSONSCHEMA:
        print(f"state schemas valid ({unit_count} units)")
    else:
        print(
            f"state schemas valid ({unit_count} units, structural fallback — "
            "pip install jsonschema for full checking)"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
