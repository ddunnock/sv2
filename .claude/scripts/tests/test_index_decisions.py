# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""The decisions index refuses to name one decision twice."""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

from index_decisions import all_records, duplicate_ids, record

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
