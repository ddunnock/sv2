# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Stop logic: keep the turn open while the gate is red.

A JSON ``decision: "block"`` on stdout (exit 0) forces Claude to keep working
instead of ending the turn, with the gate output as the reason. Invoked through
hook-stop-gate.sh, whose failure policy is OPEN: blocking a turn because this
hook itself is broken would trap the session rather than protect anything.

Loop safety: Claude Code sets stop_hook_active once this hook has already
blocked. Honoring it is mandatory — without it a persistently red gate loops
forever instead of stopping.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

RED_GATE = """The gate is red. Do not end the turn here.

Fix the failures above, or — if a failure is correct and expected — record it
explicitly: mark the production `unimplemented` in the coverage report, or add
the case to tests/rejection/known-permissive/ with a clause citation.

Do not weaken a test, do not narrow an assertion, and do not add #[ignore] to
make this pass. If you cannot resolve it, say so and stop; do not work around it.
"""


def half_derived_units() -> int:
    """Units whose rule is written but has not passed its acceptance checks."""
    return sum(
        1
        for p in Path(".claude/state/grammar/units").glob("*.json")
        if '"status": "derived"' in p.read_text(errors="replace")
    )


def block(reason: str) -> int:
    """Keep the turn open, delivering ``reason`` to Claude."""
    print(json.dumps({"decision": "block", "reason": reason}))
    return 0


def main() -> int:
    """Hold the turn open while the gate is red, once per turn."""
    try:
        payload = json.load(sys.stdin)
    except ValueError:
        return 0
    active = payload.get("stop_hook_active") if isinstance(payload, dict) else None
    if active is True or active in ("true", "True"):
        sys.stderr.write("stop-gate: already blocked once this turn; releasing to avoid a loop.\n")
        return 0

    os.chdir(os.environ.get("CLAUDE_PROJECT_DIR") or ".")
    if Path(".claude/gate-off").exists():
        return 0

    # A unit left in "derived" is an unfinished derivation: the rule is written but
    # has not passed its acceptance checks. Ending the turn there leaves the next
    # session unable to tell a finished unit from an abandoned one.
    half = half_derived_units()
    if half:
        return block(
            f"{half} grammar unit(s) are derived but not verified.\n"
            "Run python3.11 .claude/scripts/grammar_check_unit.py and fix what fails"
            " before ending the turn."
        )

    gate = subprocess.run(
        ["./scripts/gate.sh"],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        check=False,
    )
    if gate.returncode == 0:
        return 0
    return block(gate.stdout.rstrip("\n") + "\n\n" + RED_GATE.rstrip("\n"))


def run() -> int:
    """``main`` behind the top-level handler that applies the OPEN failure policy."""
    try:
        return main()
    except Exception:  # noqa: BLE001  # top-level handler; policy OPEN, STD-003-SH §9.2
        return 0


if __name__ == "__main__":
    raise SystemExit(run())
