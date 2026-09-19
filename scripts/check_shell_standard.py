# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Check shell scripts against the rules of STD-003-SH that ShellCheck and shfmt cannot express.

    python3.12 scripts/check_shell_standard.py              scripts/ and .claude/scripts/
    python3.12 scripts/check_shell_standard.py PATH...      specific files or directories

Each finding names the section of STD-003-SH it enforces (§12.3).
"""

from __future__ import annotations

import argparse
import os
import re
import stat
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_DIRS = ("scripts", ".claude/scripts")
SHEBANG = "#!/usr/bin/env bash"
STRICT = "set -euo pipefail"
MAX_LINES = 150

NAME = re.compile(r"_?[a-z0-9]+(?:-[a-z0-9]+)*\.sh")
SET_LINE = re.compile(r"^\s*set\s+[-+]")
EMBEDDED = re.compile(
    r"\b(?:python3?(?:\.\d+)?|perl|ruby|node)\s+(?:-(?:\s|$)|-[ce]\b)"
    r"|<<-?\s*['\"]?(?:PY|PYTHON|PERL|EOF_PY)\b"
)
AWK_OPEN = re.compile(r"\bawk\b[^']*'[^']*$")
EVAL = re.compile(r"(?:^|[;&|(\s])eval(?:\s|$)")
DIRECTIVE = re.compile(r"^\s*#\s*shellcheck\s+disable=")


@dataclass(frozen=True, slots=True)
class Finding:
    """One violation, located and attributed to its rule."""

    path: Path
    line: int
    message: str
    section: str

    def render(self) -> str:
        """The finding as one `path:line: message (section)` line."""
        return f"{self.path}:{self.line}: {self.message} (STD-003-SH {self.section})"


def _code_lines(lines: list[str]) -> list[tuple[int, str]]:
    """(1-based number, text) for lines that are not blank and not comments."""
    return [(i, s) for i, s in enumerate(lines, 1) if s.strip() and not s.lstrip().startswith("#")]


def _is_executable(path: Path) -> bool:
    return bool(path.stat().st_mode & (stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH))


def check_executable(path: Path, lines: list[str]) -> list[Finding]:
    """Shebang, executable bit, and strict mode before the first command."""
    found: list[Finding] = []
    if not lines or lines[0] != SHEBANG:
        found.append(Finding(path, 1, f"line 1 must be exactly `{SHEBANG}`", "§3.1"))
    if not _is_executable(path):
        found.append(Finding(path, 1, "executable script lacks the executable bit", "§3.1"))
    code = _code_lines(lines)
    if not code or code[0][1].strip() != STRICT:
        where = code[0][0] if code else 1
        found.append(Finding(path, where, f"`{STRICT}` must precede the first command", "§4.1"))
    return found


def check_library(path: Path, lines: list[str]) -> list[Finding]:
    """Sourced files: no shebang, no executable bit, no shell options."""
    found: list[Finding] = []
    if lines and lines[0].startswith("#!"):
        found.append(Finding(path, 1, "library must not have a shebang", "§3.1"))
    if _is_executable(path):
        found.append(Finding(path, 1, "library must not be executable", "§3.1"))
    found.extend(
        Finding(path, i, "library must not change shell options", "§3.1")
        for i, text in _code_lines(lines)
        if SET_LINE.match(text)
    )
    return found


def check_body(path: Path, lines: list[str]) -> list[Finding]:
    """Rules that apply to every shell file."""
    found: list[Finding] = []
    if not NAME.fullmatch(path.name):
        found.append(Finding(path, 1, "file name must be kebab-case `.sh`", "§3.1, §10"))
    if len(lines) > MAX_LINES:
        found.append(Finding(path, MAX_LINES + 1, f"longer than {MAX_LINES} lines", "§6.2"))
    code = _code_lines(lines)
    first_command = code[0][0] if code else len(lines) + 1
    found.extend(
        Finding(path, i, "file-wide shellcheck suppression", "§12.1")
        for i, text in enumerate(lines[: first_command - 1], 1)
        if DIRECTIVE.match(text)
    )
    for i, text in code:
        if EMBEDDED.search(text) or AWK_OPEN.search(text):
            found.append(
                Finding(path, i, "embedded interpreter program; write it in Python", "§2.1")
            )
        if EVAL.search(text):
            found.append(Finding(path, i, "`eval` is prohibited", "§5"))
    return found


def check_file(path: Path) -> list[Finding]:
    """Every finding for one shell file."""
    lines = path.read_text(errors="replace").splitlines()
    check_role = check_library if path.name.startswith("_") else check_executable
    return check_role(path, lines) + check_body(path, lines)


def shell_files(targets: list[str]) -> list[Path]:
    """The .sh files named by, or found under, each target."""
    files: set[Path] = set()
    for target in targets:
        p = Path(target)
        files.update(p.rglob("*.sh") if p.is_dir() else [p])
    return sorted(files)


def main(argv: list[str] | None = None) -> int:
    """Check every shell script; 1 if one breaks a rule ShellCheck cannot express."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="*", help="files or directories; default: script dirs")
    args = parser.parse_args(argv)
    if not args.paths:
        os.chdir(ROOT)

    files = shell_files(args.paths or list(DEFAULT_DIRS))
    findings = [f for path in files for f in check_file(path)]
    for finding in findings:
        print(finding.render())
    if findings:
        print(f"shell standard: {len(findings)} finding(s) in {len(files)} script(s)")
        return 1
    print(f"shell standard ok: {len(files)} script(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
