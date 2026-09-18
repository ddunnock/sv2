# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""The decisions index refuses to name one decision twice, or to disagree with itself."""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

import index_decisions
from index_decisions import (
    all_records,
    build_index,
    duplicate_ids,
    lifecycle_problems,
    link_problems,
    record,
)

if TYPE_CHECKING:
    from pathlib import Path

ADR = """---
status: "accepted"
date: 2026-09-16
---

# ADR-{num}: {title}
"""


def write_adr(directory: Path, name: str, title: str = "a decision") -> Path:
    """Write one ADR file under `directory` and return its path."""
    path = directory / name
    path.write_text(ADR.format(num=name.split("-", maxsplit=1)[0], title=title))
    return path


def test_the_id_is_the_leading_number_of_the_filename(tmp_path):
    adr = write_adr(tmp_path, "0011-specification-bnf-as-a-pinned-input.md")
    assert record(adr)["id"] == "0011"


def test_two_files_numbered_the_same_are_a_duplicate(monkeypatch, tmp_path):
    # The case that actually happened: an ADR written as 0011 landed beside the
    # specification-BNF ADR, which already held 0011. Both index under one id, so
    # every citation of ADR-0011 becomes ambiguous.
    adrs = tmp_path / "docs/adr"
    adrs.mkdir(parents=True)
    write_adr(adrs, "0011-specification-bnf-as-a-pinned-input.md")
    write_adr(adrs, "0011-rust-cst-via-webassembly.md")
    monkeypatch.chdir(tmp_path)

    clashes = duplicate_ids(all_records())

    assert list(clashes) == ["0011"]
    # The report has to name both files; naming the id alone leaves the reader
    # grepping for which two collided.
    assert len(clashes["0011"]) == 2
    assert all("0011-" in f for f in clashes["0011"])


def test_distinct_numbers_are_not_a_duplicate(monkeypatch, tmp_path):
    # The positive case: a checker that reported every ADR as a clash would be
    # just as useless as one that reported none.
    adrs = tmp_path / "docs/adr"
    adrs.mkdir(parents=True)
    write_adr(adrs, "0011-specification-bnf-as-a-pinned-input.md")
    write_adr(adrs, "0013-rust-cst-via-webassembly.md")
    monkeypatch.chdir(tmp_path)

    assert duplicate_ids(all_records()) == {}


@pytest.mark.parametrize("names", [[], ["0001-only-one.md"]])
def test_nothing_to_collide_is_not_a_duplicate(monkeypatch, tmp_path, names):
    adrs = tmp_path / "docs/adr"
    adrs.mkdir(parents=True)
    for name in names:
        write_adr(adrs, name)
    monkeypatch.chdir(tmp_path)

    assert duplicate_ids(all_records()) == {}


# -- the status lifecycle ---------------------------------------------------------
#
# The case that actually happened: ADR-0009 and ADR-0016 both decided element
# identity, both said `proposed`, and neither named the other. Nothing was wrong with
# either file on its own, which is why only a check across all of them finds it.


def entry(adr_id, status, **extra):
    """One index record, as `record` would build it."""
    return {"id": adr_id, "title": "t", "status": status, "date": "", "file": "f", **extra}


def test_a_matched_supersession_is_not_a_problem():
    records = [
        entry("0009", "superseded", **{"superseded-by": "0016"}),
        entry("0016", "accepted", supersedes="0009"),
    ]
    assert lifecycle_problems(records) == []


def test_a_superseded_record_must_name_its_successor():
    problems = lifecycle_problems([entry("0009", "superseded")])
    assert len(problems) == 1
    assert "names no superseded-by" in problems[0]


def test_the_successor_must_say_so_too():
    # A one-sided claim is the whole failure mode: 0009 points at 0016 and 0016 has
    # never heard of it, so a reader of 0016 alone still thinks it is one of two.
    records = [
        entry("0009", "superseded", **{"superseded-by": "0016"}),
        entry("0016", "accepted"),
    ]
    problems = lifecycle_problems(records)
    assert len(problems) == 1
    assert "does not say it supersedes 0009" in problems[0]


def test_a_successor_that_does_not_exist_is_a_problem():
    problems = lifecycle_problems([entry("0009", "superseded", **{"superseded-by": "9999"})])
    assert len(problems) == 1
    assert "does not exist" in problems[0]


def test_naming_a_successor_without_the_status_is_a_problem():
    records = [
        entry("0009", "proposed", **{"superseded-by": "0016"}),
        entry("0016", "accepted", supersedes="0009"),
    ]
    assert any("its status is 'proposed'" in p for p in lifecycle_problems(records))


def test_an_unknown_status_is_a_problem():
    problems = lifecycle_problems([entry("0001", "mostly-decided")])
    assert len(problems) == 1
    assert "is not one of" in problems[0]


def test_only_a_proposed_decision_is_open():
    # Superseded and rejected are answers, not open questions. Counting them as open
    # made the index report four outstanding decisions when two had been settled.
    records = [
        entry("0001", "accepted"),
        entry("0009", "superseded", **{"superseded-by": "0016"}),
        entry("0013", "proposed"),
        entry("0016", "accepted", supersedes="0009"),
    ]
    assert build_index(records)["open"] == ["0013"]


# -- cross-reference integrity ----------------------------------------------------


def adr_dir(monkeypatch, tmp_path):
    """An empty docs/adr that `link_problems` will read."""
    adrs = tmp_path / "docs/adr"
    adrs.mkdir(parents=True)
    monkeypatch.setattr(index_decisions, "ADR_DIR", adrs)
    return adrs


def test_a_link_to_a_file_that_does_not_exist_is_a_problem(monkeypatch, tmp_path):
    # ADR-0017 linked 0016-element-ids-in-inline-notes.md, twice. That filename has
    # never existed.
    adrs = adr_dir(monkeypatch, tmp_path)
    (adrs / "0017-sidecar.md").write_text("see [ADR-0016](0016-element-ids-in-inline-notes.md)")
    problems = link_problems()
    assert len(problems) == 1
    assert "does not exist" in problems[0]


def test_a_link_naming_one_adr_and_pointing_at_another_is_a_problem(monkeypatch, tmp_path):
    # The worse half: ADR-0017 cited ADR-0011 with ADR-0013's filename. The prose
    # reads correctly and the link goes somewhere, so nothing looks wrong.
    adrs = adr_dir(monkeypatch, tmp_path)
    (adrs / "0013-rust-cst.md").write_text("# 13")
    (adrs / "0017-sidecar.md").write_text("see [ADR-0011](0013-rust-cst.md)")
    problems = link_problems()
    assert len(problems) == 1
    assert "link says ADR-0011" in problems[0]


def test_a_correct_link_is_not_a_problem(monkeypatch, tmp_path):
    adrs = adr_dir(monkeypatch, tmp_path)
    (adrs / "0013-rust-cst.md").write_text("# 13")
    (adrs / "0017-sidecar.md").write_text("see [ADR-0013](0013-rust-cst.md)")
    assert link_problems() == []
