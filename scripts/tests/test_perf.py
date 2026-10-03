# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""The ratchet holds for all work, and an optimization series holds each step to a win."""

from __future__ import annotations

from pathlib import Path
from typing import Any

import pytest

import perf

# Any: measurements are JSON documents (STD-001-PY §6).
Doc = dict[str, Any]


def counted(allocations: int = 1000, allocated: int = 50_000, peeked: int = 10_000) -> Doc:
    """A counters measurement over one file, with the given totals."""
    file = {"allocations": allocations, "allocated_bytes": allocated, "peeked": peeked}
    totals = {**file, "consumed": 100}
    return {"files": {"a.sysml": {**file, "consumed": 100}}, "totals": totals, "scaling": 1.0}


DEFAULTS: dict[str, int | str] = {
    "peeked": 10_000,
    "allocations": 1000,
    "time_ns": 1_000_000,
    "source": "s",
    "outcome": "improved",
    "fingerprint": "f0",
}


def step(**changes: int | str) -> Doc:
    """A series measurement (or recorded step): the defaults, with `changes` over them."""
    v = {**DEFAULTS, **changes}
    return {
        "totals": {
            "allocations": v["allocations"],
            "allocated_bytes": 50_000,
            "peeked": v["peeked"],
        },
        "time_ns": v["time_ns"],
        "source": v["source"],
        "fingerprint": v["fingerprint"],
        "outcome": v["outcome"],
        "notes": [],
        "commit": "abc",
    }


# --- the ratchet --------------------------------------------------------------------


def test_the_ratchet_passes_within_tolerance():
    assert perf.ratchet_failures(counted(), counted(peeked=10_900)) == []


def test_the_ratchet_fails_when_a_total_grows_past_tolerance():
    found = perf.ratchet_failures(counted(), counted(peeked=11_500))
    assert any(f.startswith("total peeked grew") for f in found)


def test_the_ratchet_does_not_fire_on_a_shrinking_total():
    assert perf.ratchet_failures(counted(), counted(peeked=2_000)) == []


def test_one_file_going_pathological_fails_even_inside_the_total():
    baseline = counted()
    baseline["files"]["b.sysml"] = {"allocations": 10, "allocated_bytes": 10, "peeked": 10}
    current = counted()
    current["files"]["b.sysml"] = {"allocations": 10, "allocated_bytes": 10, "peeked": 100}
    found = perf.ratchet_failures(baseline, current)
    assert found == ["b.sysml: peeked grew +900% (limit 50%)"]


def test_a_broad_regression_lists_a_few_files_and_counts_the_rest():
    baseline, current = counted(), counted()
    for i in range(perf.FILES_SHOWN + 5):
        baseline["files"][f"f{i:02}.sysml"] = {"allocations": 1, "allocated_bytes": 1, "peeked": 1}
        current["files"][f"f{i:02}.sysml"] = {"allocations": 1, "allocated_bytes": 1, "peeked": 9}
    found = perf.ratchet_failures(baseline, current)
    assert len(found) == perf.FILES_SHOWN + 1
    assert found[-1] == "... and 5 more per-file breaches"


def test_a_new_file_is_not_compared_against_nothing():
    current = counted()
    current["files"]["new.sysml"] = {"allocations": 1, "allocated_bytes": 1, "peeked": 1}
    assert perf.ratchet_failures(counted(), current) == []


def test_superlinear_scaling_fails_whatever_the_baseline():
    current = counted()
    current["scaling"] = 3.9
    found = perf.ratchet_failures(counted(), current)
    assert len(found) == 1
    assert "something rescans" in found[0]


def test_linear_scaling_passes():
    current = counted()
    current["scaling"] = 1.02
    assert perf.ratchet_failures(counted(), current) == []


def test_growth_from_zero_is_unbounded_and_zero_to_zero_is_none():
    assert perf.growth(0, 0) == 0.0
    assert perf.growth(0, 1) == float("inf")


# --- series steps -------------------------------------------------------------------


def test_a_step_that_improves_the_primary_is_improved():
    verdict = perf.step_verdict(step(), step(peeked=9_000), "peeked", None, 0)
    assert verdict.outcome == "improved"


def test_a_step_short_of_an_improvement_is_rejected_without_enabling():
    verdict = perf.step_verdict(step(), step(peeked=9_950), "peeked", None, 0)
    assert verdict.outcome == "rejected"
    assert "--enabling" in verdict.reasons[0]


def test_a_step_short_of_an_improvement_is_enabling_when_declared():
    verdict = perf.step_verdict(step(), step(peeked=10_000), "peeked", "builds the index", 0)
    assert verdict.outcome == "enabling"
    assert verdict.reasons[-1] == "builds the index"


def test_a_fourth_enabling_step_in_a_row_is_rejected():
    verdict = perf.step_verdict(step(), step(), "peeked", "still building", perf.MAX_ENABLING_RUN)
    assert verdict.outcome == "rejected"


def test_a_third_enabling_step_in_a_row_is_accepted():
    run = perf.MAX_ENABLING_RUN - 1
    verdict = perf.step_verdict(step(), step(), "peeked", "nearly there", run)
    assert verdict.outcome == "enabling"


def test_a_step_that_worsens_another_counter_is_rejected_even_if_it_improves_the_primary():
    verdict = perf.step_verdict(step(), step(peeked=5_000, allocations=1100), "peeked", None, 0)
    assert verdict.outcome == "rejected"
    assert verdict.reasons[0].startswith("allocations worse")


def test_a_step_that_slows_the_parser_past_noise_is_rejected():
    verdict = perf.step_verdict(step(), step(peeked=5_000, time_ns=1_100_000), "peeked", None, 0)
    assert verdict.outcome == "rejected"
    assert verdict.reasons[0].startswith("median time worse")


def test_a_step_that_changes_the_parse_output_is_rejected_however_fast():
    verdict = perf.step_verdict(step(), step(peeked=1_000, fingerprint="f1"), "peeked", None, 0)
    assert verdict.outcome == "rejected"
    assert verdict.reasons[0].startswith("the parse output changed")


def test_a_series_whose_output_drifted_does_not_close():
    verdict = perf.close_verdict(step(), step(peeked=1_000, fingerprint="f1"), "peeked")
    assert verdict.outcome == "rejected"


def test_time_within_noise_does_not_veto_a_counter_win():
    verdict = perf.step_verdict(step(), step(peeked=5_000, time_ns=1_040_000), "peeked", None, 0)
    assert verdict.outcome == "improved"


def test_a_time_primary_needs_more_than_its_noise_to_count():
    assert perf.step_verdict(step(), step(time_ns=980_000), "time", None, 0).outcome == "rejected"
    assert perf.step_verdict(step(), step(time_ns=960_000), "time", None, 0).outcome == "improved"


def test_the_enabling_run_counts_only_the_trailing_enabling_steps():
    steps = [step(outcome="enabling"), step(outcome="improved"), step(outcome="enabling")]
    assert perf.enabling_run(steps) == 1
    assert perf.enabling_run([]) == 0
    assert perf.enabling_run([step(outcome="enabling")] * 2) == 2


# --- closing ------------------------------------------------------------------------


def test_a_series_closes_on_a_net_win():
    assert perf.close_verdict(step(), step(peeked=8_000), "peeked").outcome == "improved"


def test_a_series_without_a_net_win_does_not_close():
    assert perf.close_verdict(step(), step(peeked=9_990), "peeked").outcome == "rejected"


def test_a_series_that_traded_one_counter_for_another_does_not_close():
    verdict = perf.close_verdict(step(), step(peeked=5_000, allocations=1500), "peeked")
    assert verdict.outcome == "rejected"


# --- the unmeasured-change guard ----------------------------------------------------


def test_no_open_series_never_blocks():
    assert perf.unmeasured({"open": None, "history": []}, "x") is None


def test_an_open_series_with_unchanged_source_does_not_block():
    ledger = {"open": {"name": "n", "source": "s0", "steps": [step(source="s1")]}}
    assert perf.unmeasured(ledger, "s1") is None


def test_an_open_series_with_changed_source_blocks():
    ledger = {"open": {"name": "n", "source": "s0", "steps": [step(source="s1")]}}
    assert "series step" in (perf.unmeasured(ledger, "s2") or "")


def test_a_series_with_no_steps_compares_against_its_opening():
    ledger = {"open": {"name": "n", "source": "s0", "steps": []}}
    assert perf.unmeasured(ledger, "s0") is None
    assert perf.unmeasured(ledger, "s1") is not None


def test_the_source_digest_follows_content_and_names(tmp_path: Path):
    (tmp_path / "a.rs").write_text("fn a() {}")
    first = perf.source_digest(tmp_path)
    assert perf.source_digest(tmp_path) == first
    (tmp_path / "a.rs").write_text("fn a() { }")
    second = perf.source_digest(tmp_path)
    assert second != first
    (tmp_path / "a.rs").rename(tmp_path / "b.rs")
    assert perf.source_digest(tmp_path) != second


# --- the commands, over a fake probe ------------------------------------------------


@pytest.fixture
def series_env(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> dict[str, Any]:
    """A ledger and baseline in `tmp_path`, and a probe whose next numbers the test sets."""
    env: dict[str, Any] = {"next": step(source="s0")}
    monkeypatch.setattr(perf, "LEDGER", tmp_path / "series.json")
    monkeypatch.setattr(perf, "BASELINE", tmp_path / "baseline.json")
    monkeypatch.setattr(perf, "workload", list)
    monkeypatch.setattr(perf, "head", lambda: "abc")
    monkeypatch.setattr(perf, "series_measurement", lambda _p, _f: dict(env["next"]))
    monkeypatch.setattr(perf, "source_digest", lambda: env["next"]["source"])
    monkeypatch.setattr(perf, "measure_counters", lambda _p, _f: counted())
    return env


PROBES = perf.Probes(Path("c"), Path("t"))


def test_a_rejected_step_is_not_recorded(series_env: dict[str, Any]):
    assert perf.cmd_open(PROBES, "lookahead", "peeked") == 0
    series_env["next"] = step(peeked=9_990, source="s1")
    assert perf.cmd_step(PROBES, None) == 1
    assert perf.load(perf.LEDGER, {})["open"]["steps"] == []


def test_an_improved_step_is_recorded_and_the_series_closes_tightening_the_baseline(
    series_env: dict[str, Any],
):
    perf.cmd_open(PROBES, "lookahead", "peeked")
    series_env["next"] = step(peeked=10_000, source="s1")
    assert perf.cmd_step(PROBES, "adds the index") == 0
    series_env["next"] = step(peeked=7_000, source="s2")
    assert perf.cmd_step(PROBES, None) == 0
    assert perf.cmd_close(PROBES) == 0
    ledger = perf.load(perf.LEDGER, {})
    assert ledger["open"] is None
    assert [s["outcome"] for s in ledger["history"][0]["steps"]] == ["enabling", "improved"]
    assert "closed" in perf.load(perf.BASELINE, {})["reason"]


def test_a_series_cannot_close_over_an_unmeasured_change(series_env: dict[str, Any]):
    perf.cmd_open(PROBES, "lookahead", "peeked")
    series_env["next"] = step(peeked=7_000, source="s1")
    perf.cmd_step(PROBES, None)
    series_env["next"] = step(peeked=7_000, source="s2")
    assert perf.cmd_close(PROBES) == 1


@pytest.mark.usefixtures("series_env")
def test_a_step_with_no_source_change_measures_nothing():
    perf.cmd_open(PROBES, "lookahead", "peeked")
    assert perf.cmd_step(PROBES, None) == 1


@pytest.mark.usefixtures("series_env")
def test_a_second_series_cannot_open_over_the_first():
    assert perf.cmd_open(PROBES, "one", "peeked") == 0
    assert perf.cmd_open(PROBES, "two", "peeked") == 1


@pytest.mark.usefixtures("series_env")
def test_an_abandoned_series_leaves_the_baseline_alone():
    perf.cmd_open(PROBES, "lookahead", "peeked")
    assert perf.cmd_abandon("no win in it") == 0
    assert perf.load(perf.LEDGER, {})["history"][0]["outcome"] == "abandoned"
    assert not perf.BASELINE.exists()


@pytest.mark.usefixtures("series_env")
def test_record_refuses_while_a_series_is_open():
    perf.cmd_open(PROBES, "lookahead", "peeked")
    assert perf.cmd_record(PROBES, "grammar work") == 1


def test_check_fails_inside_an_open_series_over_an_unmeasured_change(series_env: dict[str, Any]):
    perf.cmd_record(PROBES, "initial")
    perf.cmd_open(PROBES, "lookahead", "peeked")
    assert perf.cmd_check(PROBES) == 0
    series_env["next"] = step(source="s1")
    assert perf.cmd_check(PROBES) == 1


@pytest.mark.usefixtures("series_env")
def test_check_fails_without_a_baseline():
    assert perf.cmd_check(PROBES) == 1
