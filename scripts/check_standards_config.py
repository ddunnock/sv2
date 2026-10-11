# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Assert that the repository configuration matches the standards that mandate it.

STD-001-PY §12 and STD-002-RS §13 make their enforcement configuration the
normative form of most of each document. That only holds while the configuration
in the standard and the configuration in the repository are the same text.
Nothing otherwise stops the two drifting, and a standard that describes a lint
setup nobody runs is worse than no standard, because it is trusted.

    python3.12 scripts/check_standards_config.py

A block is normative when its fence says which file it governs::

    ```toml title=clippy.toml

A plain ``toml`` fence is an illustration and is not read: that is what the member
manifest in STD-002-RS §13.1 is. The marker replaces two inferences that could not tell
configuration from prose — binding a block to the nearest ``###`` heading above it, and
skipping any block that failed to parse. Both failed silently, in the direction that makes
this check pass: a normative block with a TOML typo was read as an illustration, and a
block under a heading that did not name its file was dropped. A marked block must parse,
must name a governed file, and every governed file must have one.

Every key a normative block sets must have the same value in the governed file;
keys the repository adds are not compared.

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
from dataclasses import dataclass
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
STANDARDS = Path("docs/standards")
PYTHON_STANDARD = STANDARDS / "STD-001-PY-python-standards.md"
RUST_STANDARD = STANDARDS / "STD-002-RS-rust-standards.md"
PYPROJECT = Path("pyproject.toml")

#: Every file the standards govern. Each must be named by at least one normative block,
#: so dropping a marker is a finding rather than a silent return to green. pyproject.toml
#: is named by both standards: STD-002-RS §2.3 states the header table that lives there.
GOVERNED = (
    "pyproject.toml",
    "Cargo.toml",
    "clippy.toml",
    "rustfmt.toml",
    "rust-toolchain.toml",
    "deny.toml",
)

#: A fence opens and closes at the start of a line. Without the anchor, the indented
#: example of a marker in STD-001-PY §12 reads as a normative block itself, and takes
#: the rest of the document with it as its body.
TOML_FENCE = re.compile(r"^```toml(?P<info>[^\n]*)\n(?P<body>.*?)^```", re.DOTALL | re.MULTILINE)
#: What makes a fence normative. `title=` is the convention documentation renderers use
#: for a file name, so the marker reads as a caption rather than as machinery.
MARKER = re.compile(r"\btitle=(?P<target>\S+)")

# Any: TOML documents are deserialized and compared structurally (STD-001-PY §6).
Toml = dict[str, Any]


@dataclass(frozen=True)
class Block:
    """One normative configuration block, and where it was written."""

    standard: str
    line: int
    target: str
    body: str

    def where(self) -> str:
        """The block's position, for a message that someone has to go and act on."""
        return f"{self.standard}:{self.line}"


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


def normative_blocks(text: str, standard: str) -> list[Block]:
    """Every marked block in one standard, in document order. Unmarked fences are skipped."""
    blocks = []
    for fence in TOML_FENCE.finditer(text):
        marker = MARKER.search(fence.group("info"))
        if marker:
            line = text[: fence.start()].count("\n") + 1
            blocks.append(Block(standard, line, marker.group("target"), fence.group("body")))
    return blocks


def declared(standards: dict[str, str]) -> tuple[dict[str, Toml], list[str]]:
    """The configuration the standards declare per governed file, and any defects in it.

    A defect here is a defect in the standard, not drift: a marker naming a file nothing
    governs, a marked block that is not TOML, or a governed file no block declares.
    """
    per_file: dict[str, Toml] = {}
    problems: list[str] = []
    for standard, text in standards.items():
        for block in normative_blocks(text, standard):
            if block.target not in GOVERNED:
                problems.append(
                    f"{block.where()}: fence says title={block.target}, which is not one of "
                    f"the governed files ({', '.join(GOVERNED)})"
                )
                continue
            try:
                parsed = tomllib.loads(block.body)
            except tomllib.TOMLDecodeError as exc:
                problems.append(
                    f"{block.where()}: the block marked title={block.target} is normative "
                    f"configuration and is not valid TOML: {exc}"
                )
                continue
            deep_merge(per_file.setdefault(block.target, {}), parsed)
    problems += [
        f"no normative block declares {name}: a ```toml title={name} fence is how the "
        f"standard states its configuration, and without one nothing is compared"
        for name in GOVERNED
        if name not in per_file
    ]
    return per_file, problems


def findings(deviations: dict[str, str]) -> list[str]:
    """Every defect in the standards, every unrecorded drift, and every stale deviation."""
    expected_per_file, problems = declared(
        {
            str(PYTHON_STANDARD): PYTHON_STANDARD.read_text(),
            str(RUST_STANDARD): RUST_STANDARD.read_text(),
        }
    )
    governed: list[tuple[str, Toml, Toml]] = []
    for name, expected in expected_per_file.items():
        actual = tomllib.loads(Path(name).read_text()) if Path(name).is_file() else {}
        governed.append((name, expected, actual))

    drifted = {
        f"{name}:{key}": description
        for name, expected, actual in governed
        for key, description in drift(expected, actual, "", name)
    }
    issues = problems + [f"{key}: {text}" for key, text in drifted.items() if key not in deviations]
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
