# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Acceptance sweep over the pinned corpus and the rejection set.

    python3.11 scripts/corpus_sweep.py             check the sweep against the ledger
    python3.11 scripts/corpus_sweep.py --record    rewrite the ledger from this sweep

Two halves, and each is worthless without the other: a parser that accepts
everything passes a positive-only sweep, and one that rejects everything passes a
negative-only sweep.

**The ledger.** `tests/corpus-accepted.txt` names every positive-corpus file the
parser is recorded as accepting. Demanding that all of them parse would keep this
red for years — the parser implements a fraction of the grammar, and a file parses
only when every production it uses does. What is checkable today is that the set
never shrinks. A file that parsed yesterday and does not today is a regression. A
file that newly parses is progress, and it fails the gate until it is recorded,
for the same reason a snapshot does: a ledger that silently absorbs new acceptances
cannot tell "the parser improved" from "the ledger was rewritten over a regression".

Network: none.
"""

from __future__ import annotations

import argparse
import os
import subprocess
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LEDGER = ROOT / "tests" / "corpus-accepted.txt"
NEGATIVE = ROOT / "tests" / "rejection"

# Both are swept. tests/corpus is authored and vendor/corpus is pinned upstream;
# regen_state.py counts the two together, and so does this.
CORPUS_ROOTS = ("tests/corpus", "vendor/corpus")
SUFFIXES = (".sysml", ".kerml")

# Cases the parser is recorded as accepting on purpose, so they are neither caught
# nor leaked. grammar_validate.py excludes the same tree.
PRUNED = "known-permissive"

LEDGER_HEADER = """\
# Positive-corpus files the parser accepts, one repository-relative path per line.
# Written by `python3.11 scripts/corpus_sweep.py --record`; do not hand-edit.
#
# This set may grow and must never shrink. See the module docstring of
# scripts/corpus_sweep.py for why both directions fail the gate.
"""


@dataclass(frozen=True, slots=True)
class Tally:
    """How many files of one language the parser accepts, out of how many there are."""

    accepted: int
    total: int

    def render(self, suffix: str) -> str:
        """The tally as one `.suffix: n/m accepted (p%)` line."""
        percent = 100.0 * self.accepted / self.total if self.total else 0.0
        return f"{suffix}: {self.accepted}/{self.total} accepted ({percent:.1f}%)"


def corpus_roots() -> list[Path]:
    """The positive corpus roots: `SV2_CORPUS_DIR` when set, else those that exist."""
    override = os.environ.get("SV2_CORPUS_DIR")
    if override:
        return [Path(override)]
    return [ROOT / name for name in CORPUS_ROOTS if (ROOT / name).is_dir()]


def model_files(root: Path) -> list[Path]:
    """Every model file under `root`, with the known-permissive tree pruned."""
    found = [
        path
        for path in root.rglob("*")
        if path.suffix in SUFFIXES and PRUNED not in path.relative_to(root).parts
    ]
    return sorted(found)


def rel(path: Path) -> str:
    """`path` as the repository-relative POSIX string the ledger stores."""
    return path.resolve().relative_to(ROOT).as_posix()


def accepts(binary: Path, path: Path) -> bool:
    """Whether `sv2 parse` reports nothing against `path`.

    Acceptance is "the parser reported nothing", never "a tree came back": `parse`
    always returns a tree, because the tree keeps every byte whether or not the text
    is well formed (ADR-0004).
    """
    done = subprocess.run(
        [str(binary), "parse", "--quiet", str(path)],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    return done.returncode == 0


def read_ledger() -> set[str]:
    """The recorded acceptance set, or an empty set when the ledger does not exist."""
    if not LEDGER.is_file():
        return set()
    lines = LEDGER.read_text(encoding="utf-8").splitlines()
    return {line.strip() for line in lines if line.strip() and not line.startswith("#")}


def write_ledger(accepted: set[str]) -> None:
    """Rewrite the ledger with `accepted`, sorted, one path per line."""
    body = "".join(f"{path}\n" for path in sorted(accepted))
    LEDGER.parent.mkdir(parents=True, exist_ok=True)
    LEDGER.write_text(LEDGER_HEADER + body, encoding="utf-8")


def tally(files: list[Path], accepted: set[str], suffix: str) -> Tally:
    """The accepted-of-total count for the files of one language."""
    subset = [path for path in files if path.suffix == suffix]
    return Tally(sum(1 for path in subset if rel(path) in accepted), len(subset))


def check_positive(files: list[Path], accepted: set[str]) -> int:
    """Report every difference between the sweep and the ledger; the count of them."""
    recorded = read_ledger()
    regressed = sorted(recorded - accepted)
    unrecorded = sorted(accepted - recorded)
    for path in regressed:
        marker = "" if (ROOT / path).is_file() else "  (and the file is gone)"
        print(f"  regression, was accepted and is not: {path}{marker}")
    for path in unrecorded:
        print(f"  newly accepted, not in the ledger: {path}")
    if unrecorded and not regressed:
        print("  the parser improved; record it with `scripts/corpus_sweep.py --record`")
    for suffix in SUFFIXES:
        print(f"  {tally(files, accepted, suffix).render(suffix)}")
    return len(regressed) + len(unrecorded)


def check_negative(binary: Path) -> int:
    """Report every rejection case the parser accepts; the count of them."""
    if not NEGATIVE.is_dir():
        print(f"  no rejection set at {rel(NEGATIVE)} — the acceptance claim is weak")
        return 1
    leaked = [path for path in model_files(NEGATIVE) if accepts(binary, path)]
    for path in leaked:
        print(f"  should have been rejected but parsed: {rel(path)}")
    print(f"  rejection: {len(model_files(NEGATIVE)) - len(leaked)} caught / {len(leaked)} leaked")
    return len(leaked)


def sv2_binary() -> Path:
    """Where cargo left the `sv2` binary."""
    target = Path(os.environ.get("CARGO_TARGET_DIR") or ROOT / "target")
    return target / "debug" / "sv2"


def main(argv: list[str] | None = None) -> int:
    """Sweep both corpora; 1 when the ledger is wrong or a rejection case leaked."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--record", action="store_true", help="rewrite the ledger from this sweep")
    args = parser.parse_args(argv)

    binary = sv2_binary()
    if not binary.is_file():
        print(f"sv2 binary not at {binary}; sweep skipped")
        return 0

    roots = corpus_roots()
    files = [path for root in roots for path in model_files(root)]
    accepted = {rel(path) for path in files if accepts(binary, path)}

    if args.record:
        write_ledger(accepted)
        print(f"recorded {len(accepted)} accepted of {len(files)} corpus files")
        return 0

    failures = 0
    if roots:
        failures += check_positive(files, accepted)
    else:
        # Inert without a corpus, which is the vendorless state a fresh clone is in.
        # The ledger names vendor/corpus paths, so there is nothing here to check
        # until something is pinned — and the negative half still runs.
        print("  no positive corpus — the ledger is not checked")
    failures += check_negative(binary)

    print("corpus sweep: green" if failures == 0 else f"corpus sweep: {failures} failure(s)")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
