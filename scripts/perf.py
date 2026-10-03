# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Parser performance: measure it, ratchet it, and hold optimization work to it (ADR-0024).

    scripts/perf.sh measure                       print counters and timing for this tree
    scripts/perf.sh check                         the gate's ratchet
    scripts/perf.sh record --reason "..."         re-baseline, deliberately
    scripts/perf.sh series open NAME --primary M  start an optimization series
    scripts/perf.sh series step [--enabling WHY]  measure the change since the last step
    scripts/perf.sh series close                  end it; must be a net win
    scripts/perf.sh series abandon --reason WHY   end it without one

Two measurements, kept apart. The **counters** — allocations, bytes allocated, tokens
peeked — are identical on every run, so they can gate. The **timing** — median and p95
wall-clock — depends on the machine and its load, so it never gates on its own; it only
vetoes a series step that made the parser slower.

**The ratchet** runs for all work. `tests/perf-baseline.json` holds the counters the
corpus cost when it was recorded; the gate fails when the total of any counter grows more
than `RATCHET_TOLERANCE`, when any one file's grows more than `FILE_TOLERANCE`, or when the
scaling ratio — work per byte for the largest file repeated, against one copy — exceeds
`SCALING_LIMIT`, which is what a rescan does. Grammar work that legitimately costs more is
re-baselined with `record --reason`, a recorded act, never by widening a tolerance.

**A series** is optimization work. Every step must improve its primary metric, or be
declared *enabling* — a change that pays off only with later ones — and no more than
`MAX_ENABLING_RUN` of those may run in a row. No step may make any counter or the median
time worse past noise. The series closes only on a net win over where it opened, and the
close tightens the ratchet. While a series is open, the gate fails if the parser source
differs from the last step's, so no change goes unmeasured.

The shell side, `scripts/perf.sh`, builds the probe and hands over the binaries. The
logic is here because it is a list of records and it is tested.

Network: none.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from corpus_sweep import ROOT, corpus_roots, model_files, rel

BASELINE = ROOT / "tests" / "perf-baseline.json"
LEDGER = ROOT / ".claude" / "state" / "perf-series.json"
PARSER_SOURCE = ROOT / "crates" / "sv2-syntax" / "src"

# The ratchet. Loose, because grammar work adds real cost, and a ratchet that fires on
# every new production gets re-baselined until nobody reads the reasons.
RATCHET_TOLERANCE = 0.10
# One file may move more than the total, but a file that costs half again what it did is
# a construct that went pathological, whatever the rest of the corpus did.
FILE_TOLERANCE = 0.50
# Per-file breaches listed before the rest are counted: a broad regression breaches every
# file, and the first few say as much as all three hundred.
FILES_SHOWN = 10
# Linear work gives a ratio of 1.0. A rescan from the start of the input gives a ratio
# near the number of copies. Between the two, a little allowance for buffer growth.
SCALING_LIMIT = 1.25

# A series step. "Better" has to be more than rounding; a deterministic count can show
# a 1% gain exactly, while time needs more than its noise.
COUNTER_IMPROVEMENT = 0.01
TIME_IMPROVEMENT = 0.03
# "No worse" is tighter for counters, which do not wobble, than for time, which does.
COUNTER_REGRESSION = 0.02
TIME_REGRESSION = 0.05
# Enabling steps in a row before a step must show the improvement they were for.
MAX_ENABLING_RUN = 3

COUNTERS = ("allocations", "allocated_bytes", "peeked")
PRIMARIES = (*COUNTERS, "time")
# Timing runs per file, and how many of the largest files are timed.
TIMING_RUNS = 30
TIMING_FILES = 5
# ADR-0013 FIT-4's budget, against the largest file in the timed set (native, not wasm).
FIT4_P95_NS = 16_000_000

# Any: the measurements and ledger are deserialized JSON documents (STD-001-PY §6).
Doc = dict[str, Any]


@dataclass(frozen=True)
class Verdict:
    """What a series step or close came to."""

    outcome: str  # "improved", "enabling", or "rejected"
    reasons: list[str]


# --- workload -----------------------------------------------------------------------


def workload() -> list[Path]:
    """The positive corpus the sweep reads, largest file first (the scaling case)."""
    files = [path for root in corpus_roots() for path in model_files(root)]
    return sorted(files, key=lambda p: (-p.stat().st_size, rel(p)))


def source_digest(root: Path = PARSER_SOURCE) -> str:
    """A digest of the parser source: what a series step was measured against."""
    digest = hashlib.sha256()
    for path in sorted(p for p in root.rglob("*") if p.is_file()):
        digest.update(path.relative_to(root).as_posix().encode())
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()


def shown(path: Path) -> str:
    """`path` for a message: repository-relative when it is inside the repository."""
    return rel(path) if path.resolve().is_relative_to(ROOT) else str(path)


def head() -> str:
    """The checked-out commit, for the record; empty outside a repository."""
    done = subprocess.run(
        ["git", "rev-parse", "--short", "HEAD"],
        capture_output=True,
        text=True,
        check=False,
        cwd=ROOT,
    )
    return done.stdout.strip()


# --- measuring ----------------------------------------------------------------------


def run_probe(probe: Path, args: list[str]) -> Doc:
    """The probe's JSON document for `args`; a failure is an error, never a zero."""
    done = subprocess.run(
        [str(probe), *args], capture_output=True, text=True, check=False, cwd=ROOT
    )
    if done.returncode != 0:
        msg = f"{probe.name} failed: {done.stderr.strip()}"
        raise RuntimeError(msg)
    doc: Doc = json.loads(done.stdout)
    return doc


def totals(files: Doc) -> dict[str, int]:
    """Each counter summed over the files."""
    return {name: sum(int(f[name]) for f in files.values()) for name in (*COUNTERS, "consumed")}


def measure_counters(probe: Path, files: list[Path]) -> Doc:
    """Counters per file and in total, and the scaling ratio."""
    doc = run_probe(probe, ["counters", *[rel(p) for p in files]])
    return {
        "files": doc["files"],
        "totals": totals(doc["files"]),
        "scaling": doc["scaling"]["ratio"],
    }


def measure_timing(probe: Path, files: list[Path]) -> Doc:
    """Median time summed over the largest files, and the largest one's p95."""
    timed = files[:TIMING_FILES]
    doc = run_probe(probe, ["timing", str(TIMING_RUNS), *[rel(p) for p in timed]])
    per_file = doc["files"]
    largest = per_file[rel(timed[0])] if timed else {"p95_ns": 0}
    return {
        "time_ns": sum(int(f["median_ns"]) for f in per_file.values()),
        "largest_p95_ns": int(largest["p95_ns"]),
    }


# --- the ratchet --------------------------------------------------------------------


def growth(before: float, after: float) -> float:
    """Fractional change from `before` to `after`; growth from nothing is unbounded."""
    if before == 0:
        return 0.0 if after == 0 else float("inf")
    return (after - before) / before


def ratchet_failures(baseline: Doc, current: Doc) -> list[str]:
    """Every way `current` breaches the ratchet `baseline` sets."""
    found = []
    for name in COUNTERS:
        g = growth(baseline["totals"][name], current["totals"][name])
        if g > RATCHET_TOLERANCE:
            found.append(
                f"total {name} grew {g:+.1%} (limit {RATCHET_TOLERANCE:.0%}): "
                f"{baseline['totals'][name]} -> {current['totals'][name]}"
            )
    per_file = [
        f"{path}: {name} grew {g:+.0%} (limit {FILE_TOLERANCE:.0%})"
        for path, now in sorted(current["files"].items())
        if path in baseline["files"]
        for name in COUNTERS
        if (g := growth(baseline["files"][path][name], now[name])) > FILE_TOLERANCE
    ]
    found += per_file[:FILES_SHOWN]
    if len(per_file) > FILES_SHOWN:
        found.append(f"... and {len(per_file) - FILES_SHOWN} more per-file breaches")
    if current["scaling"] > SCALING_LIMIT:
        found.append(
            f"scaling ratio {current['scaling']:.2f} exceeds {SCALING_LIMIT}: "
            "work per byte grows with the input, so something rescans"
        )
    return found


# --- series verdicts ----------------------------------------------------------------


def metric(measurement: Doc, name: str) -> float:
    """One metric of a series measurement: a counter total or the summed median time."""
    return float(measurement["time_ns"] if name == "time" else measurement["totals"][name])


def regressions(before: Doc, after: Doc) -> list[str]:
    """Every counter or time that got worse past its noise from `before` to `after`."""
    found = []
    for name in COUNTERS:
        g = growth(metric(before, name), metric(after, name))
        if g > COUNTER_REGRESSION:
            found.append(f"{name} worse by {g:+.1%} (limit {COUNTER_REGRESSION:.0%})")
    g = growth(metric(before, "time"), metric(after, "time"))
    if g > TIME_REGRESSION:
        found.append(f"median time worse by {g:+.1%} (limit {TIME_REGRESSION:.0%})")
    return found


def improved(before: Doc, after: Doc, primary: str) -> tuple[bool, float]:
    """Whether `primary` improved past its threshold, and by how much (negative is better)."""
    g = growth(metric(before, primary), metric(after, primary))
    threshold = TIME_IMPROVEMENT if primary == "time" else COUNTER_IMPROVEMENT
    return g <= -threshold, g


def step_verdict(
    before: Doc, after: Doc, primary: str, enabling: str | None, enabling_run: int
) -> Verdict:
    """Judge one step of a series against the step before it.

    `enabling_run` is how many enabling steps immediately precede this one.
    """
    worse = regressions(before, after)
    if worse:
        return Verdict("rejected", worse)
    better, g = improved(before, after, primary)
    if better:
        return Verdict("improved", [f"{primary} {g:+.1%}"])
    if enabling is None:
        return Verdict(
            "rejected",
            [
                (
                    f"{primary} moved {g:+.1%}, short of an improvement; "
                    "if this change enables a later one, say so with --enabling"
                )
            ],
        )
    if enabling_run >= MAX_ENABLING_RUN:
        return Verdict(
            "rejected",
            [
                (
                    f"{MAX_ENABLING_RUN} enabling steps already in a row; "
                    "this step has to show the improvement they were for"
                )
            ],
        )
    return Verdict("enabling", [f"{primary} {g:+.1%}", enabling])


def close_verdict(opening: Doc, final: Doc, primary: str) -> Verdict:
    """Whether a series may close: a net win on `primary`, nothing else worse."""
    worse = regressions(opening, final)
    better, g = improved(opening, final, primary)
    if worse:
        return Verdict("rejected", worse)
    if not better:
        return Verdict("rejected", [f"{primary} moved {g:+.1%} over the series, not a net win"])
    return Verdict("improved", [f"{primary} {g:+.1%} over the series"])


def enabling_run(steps: list[Doc]) -> int:
    """How many enabling steps end the list."""
    run = 0
    for step in reversed(steps):
        if step["outcome"] != "enabling":
            break
        run += 1
    return run


def unmeasured(ledger: Doc, digest: str) -> str | None:
    """Why the gate must fail while a series is open over unmeasured source, or None."""
    series = ledger.get("open")
    if not series:
        return None
    steps = series["steps"]
    last = steps[-1]["source"] if steps else series["source"]
    if last == digest:
        return None
    return (
        f"series {series['name']!r} is open and the parser source changed since its last "
        "measurement; record it with `scripts/perf.sh series step`"
    )


# --- files --------------------------------------------------------------------------


def load(path: Path, default: Doc) -> Doc:
    """A JSON document, or `default` when the file does not exist."""
    if not path.is_file():
        return default
    doc: Doc = json.loads(path.read_text(encoding="utf-8"))
    return doc


def save(path: Path, doc: Doc) -> None:
    """Write `doc` as stable, reviewable JSON."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(doc, indent=1, sort_keys=True) + "\n", encoding="utf-8")


def empty_ledger() -> Doc:
    """A ledger with no series, open or closed."""
    return {"open": None, "history": []}


# --- commands -----------------------------------------------------------------------


@dataclass(frozen=True)
class Probes:
    """The two probe builds: counters (any profile, `counters` on) and timing (release)."""

    counters: Path
    timing: Path


def series_measurement(probes: Probes, files: list[Path]) -> Doc:
    """What a series records per step: counter totals, scaling, time, and the source."""
    counted = measure_counters(probes.counters, files)
    return {
        "totals": counted["totals"],
        "scaling": counted["scaling"],
        **measure_timing(probes.timing, files),
        "source": source_digest(),
        "commit": head(),
    }


def cmd_measure(probes: Probes) -> int:
    """Print the counters and timing for this tree."""
    files = workload()
    counted = measure_counters(probes.counters, files)
    timed = measure_timing(probes.timing, files)
    p95 = timed["largest_p95_ns"]
    print(f"workload: {len(files)} files, largest {rel(files[0])}")
    for name, value in counted["totals"].items():
        print(f"  {name:16} {value:>14,}")
    print(f"  {'scaling':16} {counted['scaling']:>14.3f}")
    print(
        f"  {'median time':16} {timed['time_ns'] / 1e6:>11.2f} ms  ({TIMING_FILES} largest files)"
    )
    print(f"  {'largest p95':16} {p95 / 1e6:>11.2f} ms  (FIT-4 budget {FIT4_P95_NS / 1e6:.0f} ms)")
    return 0


def cmd_check(probes: Probes) -> int:
    """The gate: the ratchet, and no unmeasured change inside an open series."""
    failures = []
    reason = unmeasured(load(LEDGER, empty_ledger()), source_digest())
    if reason:
        failures.append(reason)
    if not BASELINE.is_file():
        failures.append(
            f"no baseline at {shown(BASELINE)}; record one with `perf.sh record --reason`"
        )
    else:
        failures += ratchet_failures(
            load(BASELINE, {}), measure_counters(probes.counters, workload())
        )
    for line in failures:
        print(f"  {line}")
    if failures:
        return 1
    print("perf ratchet: green")
    return 0


def cmd_record(probes: Probes, reason: str) -> int:
    """Re-baseline the ratchet from this tree, with the reason on the record."""
    if load(LEDGER, empty_ledger()).get("open"):
        print("a series is open; close or abandon it, which records the baseline itself")
        return 1
    write_baseline(probes, reason)
    print(f"baseline recorded at {shown(BASELINE)}")
    return 0


def write_baseline(probes: Probes, reason: str) -> None:
    """Measure the counters and write them as the ratchet's baseline."""
    counted = measure_counters(probes.counters, workload())
    save(BASELINE, {"reason": reason, "commit": head(), **counted})


def cmd_open(probes: Probes, name: str, primary: str) -> int:
    """Open a series at this tree, measured, as its baseline."""
    ledger = load(LEDGER, empty_ledger())
    if ledger.get("open"):
        print(f"series {ledger['open']['name']!r} is already open")
        return 1
    ledger["open"] = {
        "name": name,
        "primary": primary,
        **series_measurement(probes, workload()),
        "steps": [],
    }
    save(LEDGER, ledger)
    print(f"series {name!r} open; primary metric {primary}")
    return 0


def cmd_step(probes: Probes, enabling: str | None) -> int:
    """Measure this tree against the series' last step and record it if it passes."""
    ledger = load(LEDGER, empty_ledger())
    series = ledger.get("open")
    if not series:
        print("no series is open; `series open` first")
        return 1
    before = series["steps"][-1] if series["steps"] else series
    after = series_measurement(probes, workload())
    if after["source"] == before["source"]:
        print("the parser source has not changed since the last step; nothing to measure")
        return 1
    verdict = step_verdict(
        before, after, series["primary"], enabling, enabling_run(series["steps"])
    )
    for line in verdict.reasons:
        print(f"  {line}")
    if verdict.outcome == "rejected":
        print("step rejected and not recorded: rework the change, or revert it")
        return 1
    # The verdict last, so nothing in the measurement can overwrite it.
    series["steps"].append({**after, "outcome": verdict.outcome, "notes": verdict.reasons})
    save(LEDGER, ledger)
    print(f"step recorded: {verdict.outcome}")
    return 0


def cmd_close(probes: Probes) -> int:
    """Close the open series on a net win, and tighten the ratchet to where it ended."""
    ledger = load(LEDGER, empty_ledger())
    series = ledger.get("open")
    if not series:
        print("no series is open")
        return 1
    if not series["steps"] or series["steps"][-1]["source"] != source_digest():
        print("record the last change with `series step` before closing")
        return 1
    verdict = close_verdict(series, series["steps"][-1], series["primary"])
    for line in verdict.reasons:
        print(f"  {line}")
    if verdict.outcome == "rejected":
        print("not closed: keep going, or `series abandon --reason` and revert the series")
        return 1
    finish(ledger, "closed", verdict.reasons)
    write_baseline(probes, f"series {series['name']!r} closed: {'; '.join(verdict.reasons)}")
    print(f"series {series['name']!r} closed; baseline tightened")
    return 0


def cmd_abandon(reason: str) -> int:
    """Close the open series without a win; print the commits to revert."""
    ledger = load(LEDGER, empty_ledger())
    series = ledger.get("open")
    if not series:
        print("no series is open")
        return 1
    finish(ledger, "abandoned", [reason])
    print(f"series {series['name']!r} abandoned; revert {series['commit']}..HEAD")
    return 0


def finish(ledger: Doc, outcome: str, notes: list[str]) -> None:
    """Move the open series into history with its outcome, and save the ledger."""
    series = ledger["open"]
    ledger["history"].append(
        {
            "name": series["name"],
            "primary": series["primary"],
            "outcome": outcome,
            "notes": notes,
            "opened_at": series["commit"],
            "closed_at": head(),
            "steps": [{"outcome": s["outcome"], "notes": s["notes"]} for s in series["steps"]],
        }
    )
    ledger["open"] = None
    save(LEDGER, ledger)


def arguments(argv: list[str] | None) -> argparse.Namespace:
    """The command line."""
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawTextHelpFormatter
    )
    parser.add_argument("--counters-probe", type=Path, required=True)
    parser.add_argument("--timing-probe", type=Path, required=True)
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("measure")
    sub.add_parser("check")
    record = sub.add_parser("record")
    record.add_argument("--reason", required=True)
    series = sub.add_parser("series").add_subparsers(dest="action", required=True)
    opened = series.add_parser("open")
    opened.add_argument("name")
    opened.add_argument("--primary", choices=PRIMARIES, required=True)
    step = series.add_parser("step")
    step.add_argument("--enabling", metavar="WHY")
    series.add_parser("close")
    abandon = series.add_parser("abandon")
    abandon.add_argument("--reason", required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    """Dispatch the command; 1 when a check fails or a step is refused."""
    args = arguments(argv)
    probes = Probes(args.counters_probe, args.timing_probe)
    try:
        if args.command == "measure":
            return cmd_measure(probes)
        if args.command == "check":
            return cmd_check(probes)
        if args.command == "record":
            return cmd_record(probes, args.reason)
        actions = {
            "open": lambda: cmd_open(probes, args.name, args.primary),
            "step": lambda: cmd_step(probes, args.enabling),
            "close": lambda: cmd_close(probes),
            "abandon": lambda: cmd_abandon(args.reason),
        }
        return actions[args.action]()
    except RuntimeError as error:
        print(error, file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
