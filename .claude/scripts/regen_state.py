# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Rewrite ONLY the "generated" object of .claude/state/state.json.

The "authored" object is never touched. ``--check`` compares everything except the
gates snapshot, which only a regeneration writes.

    python3.12 .claude/scripts/regen_state.py           rewrite
    python3.12 .claude/scripts/regen_state.py --check   fail if stale (ignoring the timestamp)
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path
from typing import TYPE_CHECKING

from _state import REPO_ROOT, STATE, load_json, pin_value, utc_now

if TYPE_CHECKING:
    from _state import Json

PY = sys.executable
GATES: dict[str, list[str]] = {
    "vendor": [PY, "scripts/vendor_verify.py"],
    "namespace": [PY, "scripts/check_namespace_link.py"],
    "derived": [PY, "scripts/extract_productions.py", "--check"],
    "metaclass_map": [PY, "scripts/check_metaclass_map.py"],
    "grammar_diff": [PY, "scripts/grammar_diff.py", "--check"],
    "coverage": [PY, "scripts/bnf_coverage.py", "--check"],
    "corpus": ["./scripts/corpus-sweep.sh"],
}
# The same commands scripts/gate.sh runs, so the two cannot report different results.
# `cargo-deny` rather than `cargo deny`: a missing binary then fails to start, which is
# reported as skipped rather than as a failing gate.
WORKSPACE = ["--workspace", "--all-features", "--locked"]
RUSTDOC = "RUSTDOCFLAGS=-D warnings"
CARGO_GATES: dict[str, list[str]] = {
    "tests": ["cargo", "test", *WORKSPACE],
    "clippy": ["cargo", "clippy", "--all-targets", *WORKSPACE, "--", "-D", "warnings"],
    "doc": ["env", RUSTDOC, "cargo", "doc", "--workspace", "--no-deps", "--locked"],
    "deny": ["cargo-deny", "--offline", "check", "bans", "licenses", "sources"],
}


def run_command(*cmd: str) -> subprocess.CompletedProcess[str] | None:
    """Run a command to completion, or None if it could not be started or timed out."""
    try:
        return subprocess.run(cmd, capture_output=True, text=True, timeout=900, check=False)
    except (OSError, subprocess.TimeoutExpired):
        return None


def gate_result(cmd: list[str]) -> str:
    """pass, fail, or skipped."""
    result = run_command(*cmd)
    if result is None:
        return "skipped"
    return "pass" if result.returncode == 0 else "fail"


def gates() -> dict[str, str]:
    """Every gate's current result."""
    results = {name: gate_result(cmd) for name, cmd in GATES.items()}
    have_cargo = shutil.which("cargo") is not None
    for name, cmd in CARGO_GATES.items():
        results[name] = gate_result(cmd) if have_cargo else "skipped"
    return results


def coverage() -> dict[str, Json]:
    """The coverage summary, from the report bnf_coverage.py writes."""
    cov = load_json(".claude/state/coverage.json", {})
    return {
        "declared": cov.get("declared", 0),
        "implemented": cov.get("implemented", 0),
        "unimplemented": cov.get("unimplemented", 0),
        "absent": cov.get("absent", 0),
        "percent": round(cov.get("percent", 0.0), 1),
    }


def _is_model(p: Path) -> bool:
    return p.suffix in (".sysml", ".kerml")


def count(pattern: str, root: str = ".") -> int:
    """Files matching ``pattern`` under ``root``, outside any target/ directory."""
    return sum(1 for p in Path(root).rglob(pattern) if "target" not in p.parts)


def count_models(root: str) -> int:
    """Model files under ``root``, outside any target/ directory."""
    return sum(1 for p in Path(root).rglob("*") if "target" not in p.parts and _is_model(p))


def tests() -> dict[str, int]:
    """Test and corpus counts."""
    unit = sum(
        len(re.findall(r"#\[test\]", f.read_text(errors="replace")))
        for f in Path("crates").rglob("*.rs")
    )
    # Every rejection case at any depth, except the recorded known-permissive ones,
    # which are counted on their own — the same split corpus-sweep.sh and
    # grammar_validate.py make.
    known_permissive = count_models("tests/rejection/known-permissive")
    negative = count_models("tests/rejection") - known_permissive
    return {
        "unit": unit,
        "snapshots": count("*.snap"),
        # Model files only. Counting README.md here would report documentation as coverage.
        "corpus_files": count_models("tests/corpus") + count_models("vendor/corpus"),
        "negative_cases": negative,
        "known_permissive": known_permissive,
    }


def pins() -> dict[str, Json]:
    """The conformance pins and how much of the vendor set is hashed."""
    lock = Path("vendor/sources.lock.toml")
    lock_text = lock.read_text() if lock.exists() else ""
    return {
        "omg_namespace": pin_value("tier_a_omg.namespace"),
        "pilot_revision": pin_value("tier_b_pilot.revision"),
        "pilot_bundle": pin_value("tier_b_pilot.bundle_version"),
        "vendor_pinned": len(re.findall(r'sha256\s*=\s*"[0-9a-f]{64}"', lock_text)),
        "vendor_total": len(re.findall(r"^\[\[file\]\]", lock_text, re.MULTILINE)),
    }


def commits() -> list[str]:
    """The last few commit subjects, or nothing outside a git repository."""
    result = run_command("git", "log", "--oneline", "-8")
    return result.stdout.strip().splitlines() if result and result.returncode == 0 else []


#: The measurements ``--check`` compares. ``commits`` is deliberately not one of
#: them: it is a rolling ``git log`` window rather than a property of the tree, and
#: the commit that carries state.json cannot record its own hash. Comparing it has
#: no fixed point — amending only moves the hash — so the gate would be red after
#: every commit, for ever. It is still measured and written, for whoever reads the
#: block at session start; it is simply not evidence of anything being stale.
COMPARED = ("coverage", "tests", "pins")


def is_stale(recorded: dict[str, Json], measured: dict[str, Json]) -> bool:
    """Whether the recorded generated block disagrees with what was just measured."""
    return {key: recorded.get(key) for key in COMPARED} != {
        key: measured.get(key) for key in COMPARED
    }


def main(argv: list[str] | None = None) -> int:
    """Rewrite the generated block, or with --check fail if it is stale."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if the generated block is stale")
    args = parser.parse_args(argv)
    os.chdir(REPO_ROOT)

    doc = json.loads(STATE.read_text())
    measured: dict[str, Json] = {
        "coverage": coverage(),
        "tests": tests(),
        "pins": pins(),
        "commits": commits(),
    }

    if args.check:
        # The gates block is a snapshot of the last regeneration, not a live claim:
        # the gate that runs this check has just measured every one of those results
        # itself. Re-running them here doubled every gate run's cargo and corpus work.
        if is_stale(doc["generated"], measured):
            print(
                "state.json generated block is stale. Run python3.12 .claude/scripts/regen_state.py"
            )
            return 1
        print("state.json current")
        return 0

    gate_results = gates()
    doc["generated"] = {"regenerated_utc": utc_now(), "gates": gate_results, **measured}
    STATE.write_text(json.dumps(doc, indent=2) + "\n")
    passing = sum(1 for v in gate_results.values() if v == "pass")
    print(f"state.json regenerated ({passing}/{len(gate_results)} gates passing)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
