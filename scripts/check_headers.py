# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Assert the licence header on every source file, where the repository requires one.

STD-001-PY §2.3 and STD-002-RS §2.3 make the header a per-repository setting. This
one is MIT, so the header is two lines — the SPDX identifier and the copyright —
and their whole job is to travel with a file that someone copies out of the
repository, which is the only thing the licence asks for.

    python3.11 scripts/check_headers.py

The required values come from ``[tool.sv2.headers]`` in pyproject.toml, and each
must appear in the file's opening comment block: ``#`` lines in Python and shell,
``//`` lines in Rust (never a ``//!`` or ``///`` doc comment, which would put the
licence text into rustdoc). In a shell script the block opens with the shebang and
the two required lines follow it (STD-003-SH §3.4). Checking the values rather than
the presence of a block is the point: a header naming the wrong licence passes a
presence check and fails a review.

Whether or not headers are required, a Python module must not start with a
shebang (STD-001-PY §2.3); the scripts here are run as ``python3.11 <path>``.
"""

from __future__ import annotations

import argparse
import os
import tomllib
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PYPROJECT = Path("pyproject.toml")
# How far into a file the header may begin. A header pushed below imports is not a header.
HEADER_WINDOW_LINES = 40
SCRIPT_SOURCES = ("scripts", ".claude/scripts")
# Every source file, which is what STD-002-RS §2.3 says. An integration test is a
# source file: it is compiled, it is committed, and it is exactly as likely to be
# copied out of the repository as anything under src/ — which is the whole job the
# two lines have. Excluding them left 40 files carrying the header by convention
# with nothing checking it, and a misspelled `SPDX-License-Indentifier` sat in one
# of them until this glob widened.
RUST_SOURCES = ("crates/*/src/**/*.rs", "crates/*/tests/**/*.rs")
# `#` in Python and shell, `//` in Rust. A shell script's shebang is part of its
# opening comment block, which costs nothing here: the required strings are on the
# two lines after it (STD-003-SH §3.4).
MARKERS = {".py": "#", ".sh": "#", ".rs": "//"}


@dataclass(frozen=True, slots=True)
class Settings:
    """The repository's header requirement, from [tool.sv2.headers]."""

    required: bool
    values: tuple[str, ...]


def settings(pyproject: str) -> Settings:
    """Read ``[tool.sv2.headers]``, defaulting to not required."""
    headers = tomllib.loads(pyproject).get("tool", {}).get("sv2", {}).get("headers", {})
    return Settings(
        required=bool(headers.get("required", False)),
        values=tuple(str(v) for v in headers.get("must_contain", []) if v),
    )


def leading_comment(text: str, marker: str) -> str:
    """The opening block of ``marker`` comment lines, before any code or docs."""
    lines: list[str] = []
    for line in text.splitlines()[:HEADER_WINDOW_LINES]:
        stripped = line.strip()
        if not stripped:
            if lines:
                break
            continue
        is_doc = marker == "//" and stripped.startswith(("//!", "///"))
        if not stripped.startswith(marker) or is_doc:
            break
        lines.append(stripped.removeprefix(marker).strip())
    return "\n".join(lines)


def sources() -> list[Path]:
    """Every file the standards govern: script and crate sources, tests included."""
    scripts = [
        p
        for root in SCRIPT_SOURCES
        for suffix in (".py", ".sh")
        for p in Path(root).rglob(f"*{suffix}")
        if "__pycache__" not in p.parts
    ]
    rust = [p for pattern in RUST_SOURCES for p in Path().glob(pattern)]
    return sorted([*scripts, *rust])


def check(path: Path, config: Settings) -> list[str]:
    """Why ``path`` fails the header rules, if it does."""
    text = path.read_text(encoding="utf-8")
    if path.suffix == ".py" and text.startswith("#!"):
        return [f"{path}: shebang present; STD-001-PY §2.3 forbids one"]
    if not config.required:
        return []
    header = leading_comment(text, MARKERS.get(path.suffix, "#"))
    if not header:
        return [f"{path}: no program header"]
    return [
        f"{path}: header does not carry {value!r}" for value in config.values if value not in header
    ]


def main(argv: list[str] | None = None) -> int:
    """Check every governed source file; 1 if a header is missing or carries the wrong values."""
    argparse.ArgumentParser(description=__doc__).parse_args(argv)
    os.chdir(ROOT)

    config = settings(PYPROJECT.read_text())
    files = sources()
    findings = [f for path in files for f in check(path, config)]
    for finding in findings:
        print(f"  {finding}")
    if findings:
        print(f"program headers: {len(findings)} finding(s) in {len(files)} file(s)")
        return 1
    if config.required:
        print(f"program headers present and correct on {len(files)} file(s)")
    else:
        print(
            "program headers not required ([tool.sv2.headers]); "
            f"{len(files)} file(s) checked for shebangs"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
