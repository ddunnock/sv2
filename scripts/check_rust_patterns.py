# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Reject the Rust patterns STD-002-RS forbids and neither rustc nor Clippy can catch.

    python3.12 scripts/check_rust_patterns.py              every crates/**/*.rs
    python3.12 scripts/check_rust_patterns.py PATH...      specific files or directories

Rules (STD-002-RS §13.6 table):
  - §8.4   every ``#[instrument]`` uses ``skip_all``: by default it records every
           argument with ``Debug``, which leaks inputs, manifests, and credentials
  - §8.1   a tracing event's message is a constant literal: no ``format!`` and no
           ``{...}`` interpolation, so the fields carry the values and the message
           groups as one event
  - §10.6  no bare ``#[ignore]``; it must say why: ``#[ignore = "..."]``
  - §9     no module or type named, or ending in, utils, helpers, misc, common,
           manager, or handler (the suffix rule of STD-001-PY §9, which §9 cites)

Lexical, not a parser: comments are removed first, and string literals are
blanked except where a rule needs to read one.
"""

from __future__ import annotations

import argparse
import os
import re
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BANNED = ("utils", "helpers", "misc", "common", "manager", "handler")

LINE_COMMENT = re.compile(r"//[^\n]*")
BLOCK_COMMENT = re.compile(r"/\*.*?\*/", re.DOTALL)
STRING = re.compile(r'r(#*)"(?:.|\n)*?"\1|b?"(?:[^"\\]|\\.)*"')
INSTRUMENT = re.compile(r"#\[\s*(?:tracing::)?instrument\b\s*(\((?:[^\[\]]|\[[^\]]*\])*\))?\s*\]")
BARE_IGNORE = re.compile(r"#\[\s*ignore\s*\]")
EVENT = re.compile(r"\b(?:tracing::)?(trace|debug|info|warn|error|event)!\s*\(")
MODULE_DECL = re.compile(r"\bmod\s+([a-z_][a-z0-9_]*)\s*[;{]")
TYPE_DECL = re.compile(r"\b(?:struct|enum|trait|union|type)\s+([A-Z][A-Za-z0-9]*)")
INTERPOLATION = re.compile(r"(?<!\{)\{(?!\{)[^}]*\}")


@dataclass(frozen=True, slots=True)
class Finding:
    """One forbidden pattern at one line."""

    path: Path
    line: int
    message: str
    section: str

    def render(self) -> str:
        """The finding as one `path:line: message (section)` line."""
        return f"{self.path}:{self.line}: {self.message} (STD-002-RS {self.section})"


def strip_comments(source: str) -> str:
    """Remove comments, keeping newlines so line numbers survive, and leaving strings intact."""
    out: list[str] = []
    pos = 0
    pattern = re.compile(
        f"{STRING.pattern}|{BLOCK_COMMENT.pattern}|{LINE_COMMENT.pattern}", re.DOTALL
    )
    for match in pattern.finditer(source):
        out.append(source[pos : match.start()])
        text = match.group()
        out.append(text if text.startswith(("r", "b", '"')) else "\n" * text.count("\n"))
        pos = match.end()
    out.append(source[pos:])
    return "".join(out)


def blank_strings(code: str) -> str:
    """Replace every string literal's contents with spaces, keeping newlines."""
    return STRING.sub(lambda m: re.sub(r"[^\n]", " ", m.group()), code)


def _line(code: str, offset: int) -> int:
    return code.count("\n", 0, offset) + 1


def _is_banned(name: str) -> bool:
    lowered = name.lower()
    return any(lowered == b or lowered.endswith(f"_{b}") for b in BANNED)


def _banned_type(name: str) -> bool:
    words = re.findall(r"[A-Z][a-z0-9]*", name)
    return bool(words) and words[-1].lower() in BANNED


def _call_arguments(code: str, open_paren: int) -> tuple[list[str], int]:
    """Top-level comma-separated arguments of the call whose ``(`` is at ``open_paren``."""
    depth, start, args = 0, open_paren + 1, []
    i = open_paren
    while i < len(code):
        match = STRING.match(code, i)
        if match:
            i = match.end()
            continue
        char = code[i]
        if char in "([{":
            depth += 1
        elif char in ")]}":
            depth -= 1
            if depth == 0:
                args.append(code[start:i])
                return [a.strip() for a in args if a.strip()], i
        elif char == "," and depth == 1:
            args.append(code[start:i])
            start = i + 1
        i += 1
    return [a.strip() for a in args if a.strip()], i


def event_findings(path: Path, code: str) -> list[Finding]:
    """Tracing events whose message is formatted rather than constant."""
    found = []
    for match in EVENT.finditer(code):
        args, _ = _call_arguments(code, match.end() - 1)
        line = _line(code, match.start())
        if any(re.match(r"(?:std::)?format!\s*\(", a) for a in args):
            found.append(
                Finding(path, line, f"{match.group(1)}! message built with format!", "§8.1")
            )
            continue
        literal = next((a for a in args if STRING.fullmatch(a)), None)
        if literal and INTERPOLATION.search(literal):
            found.append(
                Finding(path, line, f"{match.group(1)}! message interpolates values", "§8.1")
            )
    return found


def file_findings(path: Path, source: str) -> list[Finding]:
    """Every rule, for one Rust source file."""
    code = strip_comments(source)
    bare = blank_strings(code)
    found = [
        Finding(path, _line(bare, m.start()), "#[instrument] without skip_all", "§8.4")
        for m in INSTRUMENT.finditer(bare)
        if "skip_all" not in (m.group(1) or "")
    ]
    found += [
        Finding(path, _line(bare, m.start()), 'bare #[ignore]; use #[ignore = "why"]', "§10 rule 6")
        for m in BARE_IGNORE.finditer(bare)
    ]
    found += [
        Finding(path, _line(bare, m.start()), f"module `{m.group(1)}` has a banned name", "§9")
        for m in MODULE_DECL.finditer(bare)
        if _is_banned(m.group(1))
    ]
    found += [
        Finding(path, _line(bare, m.start()), f"type `{m.group(1)}` has a banned name", "§9")
        for m in TYPE_DECL.finditer(bare)
        if _banned_type(m.group(1))
    ]
    if _is_banned(path.stem) and path.stem not in ("mod", "lib", "main"):
        found.append(Finding(path, 1, f"module file `{path.name}` has a banned name", "§9"))
    return found + event_findings(path, code)


def rust_files(targets: list[str]) -> list[Path]:
    """The .rs files named by, or found under, each target, outside target/ directories."""
    files: set[Path] = set()
    for target in targets:
        p = Path(target)
        found = p.rglob("*.rs") if p.is_dir() else [p]
        files.update(f for f in found if "target" not in f.parts)
    return sorted(files)


def main(argv: list[str] | None = None) -> int:
    """Scan the crate sources; 1 if any forbidden pattern is present."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="*", help="files or directories; default: crates/")
    args = parser.parse_args(argv)
    if not args.paths:
        os.chdir(ROOT)

    files = rust_files(args.paths or ["crates"])
    findings = [f for path in files for f in file_findings(path, path.read_text(errors="replace"))]
    for finding in findings:
        print(finding.render())
    if findings:
        print(f"rust patterns: {len(findings)} finding(s) in {len(files)} file(s)")
        return 1
    print(f"rust patterns ok: {len(files)} file(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
