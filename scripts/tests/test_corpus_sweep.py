# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import pytest

import corpus_sweep
from corpus_sweep import Tally, check_positive, model_files, tally


@pytest.fixture
def corpus(tmp_path, monkeypatch):
    """A corpus root under ROOT, so the repository-relative paths in the ledger resolve."""
    monkeypatch.setattr(corpus_sweep, "ROOT", tmp_path)
    monkeypatch.setattr(corpus_sweep, "LEDGER", tmp_path / "tests" / "corpus-accepted.txt")
    root = tmp_path / "corpus"
    root.mkdir()
    return root


def plant(root, *names):
    for name in names:
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("package P;\n")
    return sorted(root.rglob("*.sysml")) + sorted(root.rglob("*.kerml"))


def test_model_files_takes_both_languages_and_nothing_else(corpus):
    plant(corpus, "a.sysml", "b.kerml")
    (corpus / "notes.md").write_text("x")
    assert [p.name for p in model_files(corpus)] == ["a.sysml", "b.kerml"]


def test_model_files_prunes_the_known_permissive_tree(corpus):
    plant(corpus, "a.sysml", "known-permissive/b.sysml")
    assert [p.name for p in model_files(corpus)] == ["a.sysml"]


def test_tally_counts_each_language_separately(corpus):
    plant(corpus, "a.sysml", "b.sysml", "c.kerml")
    files = model_files(corpus)
    accepted = {corpus_sweep.rel(files[0])}
    assert tally(files, accepted, ".sysml") == Tally(accepted=1, total=2)
    assert tally(files, accepted, ".kerml") == Tally(accepted=0, total=1)


def test_a_ledger_matching_the_sweep_has_no_failures(corpus):
    plant(corpus, "a.sysml")
    files = model_files(corpus)
    accepted = {corpus_sweep.rel(f) for f in files}
    corpus_sweep.write_ledger(accepted)
    assert check_positive(files, accepted) == 0


def test_a_file_that_stops_parsing_is_a_regression(corpus):
    """The set may grow and must never shrink."""
    files = plant(corpus, "a.sysml")
    corpus_sweep.write_ledger({corpus_sweep.rel(files[0])})
    assert check_positive(files, set()) == 1


def test_a_file_that_starts_parsing_fails_until_it_is_recorded(corpus):
    """A ledger that silently absorbs new acceptances cannot detect a rewrite over one."""
    files = plant(corpus, "a.sysml")
    corpus_sweep.write_ledger(set())
    assert check_positive(files, {corpus_sweep.rel(files[0])}) == 1


def test_recording_the_sweep_clears_the_failure(corpus):
    files = plant(corpus, "a.sysml", "b.sysml")
    accepted = {corpus_sweep.rel(f) for f in files}
    corpus_sweep.write_ledger(set())
    assert check_positive(files, accepted) == 2
    corpus_sweep.write_ledger(accepted)
    assert check_positive(files, accepted) == 0


@pytest.mark.usefixtures("corpus")
def test_the_ledger_round_trips_and_ignores_its_comment_header():
    corpus_sweep.write_ledger({"b.sysml", "a.sysml"})
    text = corpus_sweep.LEDGER.read_text()
    assert text.startswith("#")
    assert corpus_sweep.read_ledger() == {"a.sysml", "b.sysml"}
    # Sorted, so two machines that sweep the same corpus write the same file.
    assert text.splitlines()[-2:] == ["a.sysml", "b.sysml"]


@pytest.mark.usefixtures("corpus")
def test_an_absent_ledger_reads_as_empty_rather_than_failing():
    assert corpus_sweep.read_ledger() == set()


def test_corpus_roots_prefers_the_environment_override(corpus, monkeypatch):
    monkeypatch.setenv("SV2_CORPUS_DIR", str(corpus))
    assert corpus_sweep.corpus_roots() == [corpus]


@pytest.mark.usefixtures("corpus")
def test_corpus_roots_skips_a_root_that_does_not_exist(monkeypatch):
    monkeypatch.delenv("SV2_CORPUS_DIR", raising=False)
    (corpus_sweep.ROOT / "vendor" / "corpus").mkdir(parents=True)
    assert corpus_sweep.corpus_roots() == [corpus_sweep.ROOT / "vendor" / "corpus"]
