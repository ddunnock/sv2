# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Assert that the repository configuration matches the standards that mandate it.

STD-001-PY §12 and STD-002-RS §13 make their enforcement configuration the
normative form of most of each document. That only holds while the configuration
in the standard and the configuration in the repository are the same text.
Nothing otherwise stops the two drifting, and a standard that describes a lint
setup nobody runs is worse than no standard, because it is trusted.

    python3.11 scripts/check_standards_config.py

Every key a standard's TOML block sets must have the same value in the governed
file; keys the repository adds are not compared. A block is bound to its file by
the ``### 13.N`` heading above it (STD-002-RS) or is a workspace-root
``pyproject.toml`` table (STD-001-PY, top-level ``tool`` only). The member
manifest in STD-002-RS §13.1 is an example, not configuration, and is skipped.

A difference the repository makes on purpose is recorded in
``[tool.sv2.deviations]`` of pyproject.toml as ``"<file>:<dotted.key>" =
"<reason>"``. A recorded deviation that no longer differs is itself a finding,
so the register cannot go stale.
"""

from __future__ import annotations

import argparse
import os
import re
import tomllib
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
STANDARDS = Path("docs/standards")
PYTHON_STANDARD = STANDARDS / "STD-001-PY-python-standards.md"
RUST_STANDARD = STANDARDS / "STD-002-RS-rust-standards.md"
PYPROJECT = Path("pyproject.toml")
RUST_FILES = ("Cargo.toml", "clippy.toml", "rustfmt.toml", "rust-toolchain.toml", "deny.toml")
# STD-001-PY blocks also carry one distribution's [project] fragments; only tool
# tables are workspace configuration.
PYTHON_GOVERNED = ("tool",)

TOML_BLOCK = re.compile(r"```toml\n(.*?)```", re.DOTALL)
HEADING = re.compile(r"^#{2,4} .*$", re.MULTILINE)

# Any: TOML documents are deserialized and compared structurally (STD-001-PY §6).
Toml = dict[str, Any]


def deep_merge(into: Toml, other: Toml) -> None:
    """Merge ``other`` into ``into``, recursing through tables."""
    for key, value in other.items():
        if isinstance(value, dict) and isinstance(into.get(key), dict):
            deep_merge(into[key], value)
        else:
            into[key] = value


def drift(expected: Any, actual: Any, path: str, source: str) -> list[tuple[str, str]]:
    """(dotted key, description) for every key whose value differs from the standard."""
    if isinstance(expected, dict):
        if not isinstance(actual, dict):
            return [(path, f"the standard declares a table, {source} has {type(actual).__name__}")]
        found: list[tuple[str, str]] = []
        for key, value in expected.items():
            here = f"{path}.{key}" if path else key
            if key not in actual:
                found.append((here, f"absent from {source}"))
            else:
                found.extend(drift(value, actual[key], here, source))
        return found
    if expected != actual:
        return [(path, f"standard {expected!r}, repository {actual!r}")]
    return []


def python_config(text: str) -> Toml:
    """The workspace-root tool tables STD-001-PY declares."""
    merged: Toml = {}
    for block in TOML_BLOCK.findall(text):
        try:
            parsed = tomllib.loads(block)
        except tomllib.TOMLDecodeError:
            continue  # a deliberately partial fragment; examples are not config
        deep_merge(merged, {k: v for k, v in parsed.items() if k in PYTHON_GOVERNED})
    return merged


def rust_config(text: str) -> dict[str, Toml]:
    """Each STD-002-RS §13 block, merged per file named by the heading above it."""
    per_file: dict[str, Toml] = {}
    for match in TOML_BLOCK.finditer(text):
        headings = HEADING.findall(text, 0, match.start())
        heading = headings[-1] if headings else ""
        target = next((name for name in RUST_FILES if name in heading), None)
        parsed = tomllib.loads(match.group(1))
        if target is None or "package" in parsed:
            continue  # not a §13 file block, or the §13.1 member-manifest example
        deep_merge(per_file.setdefault(target, {}), parsed)
    return per_file


def findings(deviations: dict[str, str]) -> list[str]:
    """Every unrecorded drift, and every recorded deviation that no longer drifts."""
    pyproject = tomllib.loads(PYPROJECT.read_text())
    governed: list[tuple[str, Toml, Toml]] = [
        ("pyproject.toml", python_config(PYTHON_STANDARD.read_text()), pyproject)
    ]
    for name, expected in rust_config(RUST_STANDARD.read_text()).items():
        actual = tomllib.loads(Path(name).read_text()) if Path(name).is_file() else {}
        governed.append((name, expected, actual))

    drifted = {
        f"{name}:{key}": description
        for name, expected, actual in governed
        for key, description in drift(expected, actual, "", name)
    }
    issues = [f"{key}: {text}" for key, text in drifted.items() if key not in deviations]
    issues += [
        f"{key}: recorded as a deviation but matches the standard; remove the entry"
        for key in deviations
        if key not in drifted
    ]
    return issues


def main(argv: list[str] | None = None) -> int:
    """Compare the configuration with the standards; 1 on any unrecorded drift."""
    argparse.ArgumentParser(description=__doc__).parse_args(argv)
    os.chdir(ROOT)

    sv2 = tomllib.loads(PYPROJECT.read_text()).get("tool", {}).get("sv2", {})
    deviations = sv2.get("deviations", {})
    issues = findings(deviations)
    if issues:
        print("Configuration has drifted from the standards that mandate it:\n")
        for issue in issues:
            print(f"  {issue}")
        print(
            "\nCorrect the repository or the standard so they agree, or record a deliberate"
            "\ndifference in [tool.sv2.deviations] of pyproject.toml with its reason."
        )
        return 1
    print(f"Configuration matches the standards ({len(deviations)} recorded deviations).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
