# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""PostToolUse logic: format and lint an edited Rust file.

Cannot undo the edit. Findings go back to Claude as JSON ``additionalContext``
on stdout with exit 0, which arrives as context rather than as a hook error.
Invoked through hook-post-edit.sh, whose failure policy is OPEN.
"""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys

CRATE = re.compile(r".*/crates/([^/]*)/.*")
FINDING = re.compile(r"^(error|warning)")


def format_file(path: str) -> None:
    """Run rustfmt on ``path``, or on the whole workspace if that fails."""
    if subprocess.run(["cargo", "fmt", "--", path], capture_output=True, check=False).returncode:
        subprocess.run(["cargo", "fmt", "--all"], capture_output=True, check=False)


def clippy_findings(path: str) -> tuple[str, list[str]]:
    """(scope, first 25 error or warning lines) from clippy for the crate owning ``path``."""
    m = CRATE.match(path)
    if m:
        scope, scope_args = m.group(1), ["-p", m.group(1)]
    else:
        scope, scope_args = "workspace", ["--workspace"]
    result = subprocess.run(
        ["cargo", "clippy", *scope_args, "--all-targets", "--message-format", "short"],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        check=False,
    )
    lines = [line for line in result.stdout.splitlines() if FINDING.match(line)]
    return scope, lines[:25]


def main() -> int:
    """Format the edited Rust file and hand any clippy findings back to the model."""
    try:
        payload = json.load(sys.stdin)
        path = payload["tool_input"]["file_path"]
    except (ValueError, KeyError, TypeError):
        return 0
    if not isinstance(path, str) or not path.endswith(".rs") or shutil.which("cargo") is None:
        return 0
    try:
        os.chdir(os.environ.get("CLAUDE_PROJECT_DIR") or ".")
    except OSError:
        return 0

    format_file(path)
    scope, findings = clippy_findings(path)
    if not findings:
        return 0
    context = (
        f"clippy findings in {scope}:\n" + "\n".join(findings) + "\n\nFix these before moving on."
    )
    output = {"hookSpecificOutput": {"hookEventName": "PostToolUse", "additionalContext": context}}
    print(json.dumps(output))
    return 0


def run() -> int:
    """``main`` behind the top-level handler that applies the OPEN failure policy."""
    try:
        return main()
    except Exception:  # noqa: BLE001  # top-level handler; policy OPEN, STD-003-SH §9.2
        return 0


if __name__ == "__main__":
    raise SystemExit(run())
