# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""SessionStart logic: inject the work state into the model's context.

On exit 0, stdout is injected into the model's context, so this emits the state
as JSON to be read as data, not prose. Invoked through hook-session-start.sh,
whose failure policy is OPEN.
"""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path

NO_STATE = '{"error":"no state file","fix":"run python3.12 .claude/scripts/regen_state.py"}'


def main() -> int:
    """Print the work state, so the session starts knowing where it is."""
    try:
        os.chdir(os.environ.get("CLAUDE_PROJECT_DIR") or ".")
    except OSError:
        return 0

    print("=== sv2 state (.claude/state/state.json) ===")
    state = Path(".claude/state/state.json")
    if state.exists():
        sys.stdout.write(state.read_text())
    else:
        print(NO_STATE)

    diff = Path(".claude/state/grammar-diff.json")
    if diff.exists():
        print("=== unreviewed grammar differences ===")
        try:
            unreviewed = json.loads(diff.read_text()).get("unreviewed", [])
        except (ValueError, OSError, AttributeError):
            return 0  # context injection is best-effort; a bad report must not block the session
        print(json.dumps({"unreviewed": unreviewed}))
    return 0


def run() -> int:
    """``main`` behind the top-level handler that applies the OPEN failure policy."""
    try:
        return main()
    except Exception:  # noqa: BLE001  # top-level handler; policy OPEN, STD-003-SH §9.2
        return 0


if __name__ == "__main__":
    raise SystemExit(run())
