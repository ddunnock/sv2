# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
from pathlib import Path

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


# -- rejection provenance -------------------------------------------------------------


@pytest.fixture
def rejection(tmp_path, monkeypatch):
    """A rejection set and a pending ledger under ROOT, both writable by a test."""
    monkeypatch.setattr(corpus_sweep, "ROOT", tmp_path)
    negative = tmp_path / "tests" / "rejection"
    negative.mkdir(parents=True)
    monkeypatch.setattr(corpus_sweep, "NEGATIVE", negative)
    monkeypatch.setattr(
        corpus_sweep, "PENDING", tmp_path / "tests" / "rejection-provenance-pending.txt"
    )
    return negative


def test_declared_reads_the_code_and_the_line_it_is_about(rejection):
    path = rejection / "a.sysml"
    path.write_text(
        "// why this is rejected\n"
        "// rejects: PARSE-UNEXPECTED on: merge m;\n"
        "part def P { merge m; }\n"
    )
    assert corpus_sweep.declared(path) == ("PARSE-UNEXPECTED", "merge m;")


def test_declared_is_none_when_the_header_says_nothing(rejection):
    path = rejection / "a.sysml"
    path.write_text("// REJECTED BY RULE, but which one is not stated\npart def P { merge m; }\n")
    assert corpus_sweep.declared(path) is None


def test_a_declaration_must_be_a_comment(rejection):
    # The pattern anchors on `//`, so a line of MODEL text that happens to read like a
    # declaration is not one. A file could otherwise declare its own provenance in the
    # code under test, which is the text the check exists to be independent of.
    path = rejection / "a.sysml"
    path.write_text("rejects: PARSE-UNEXPECTED on: merge m;\n")
    assert corpus_sweep.declared(path) is None


def test_first_diagnostic_resolves_the_line_to_its_text(rejection, monkeypatch):
    path = rejection / "a.sysml"
    path.write_text("// header\npart def P {\n\tmerge m;\n}\n")

    class Done:
        stderr = (
            "  3:2: error[PARSE-UNEXPECTED]: unexpected `merge`\n  4:1: error[PARSE-EXPECTED]: x\n"
        )

    monkeypatch.setattr(corpus_sweep.subprocess, "run", lambda *_args, **_kwargs: Done())
    # The FIRST diagnostic only, and its line as text rather than as a number: a header
    # edit moves every number in the file and no body line's text.
    assert corpus_sweep.first_diagnostic(Path("sv2"), path) == ("PARSE-UNEXPECTED", "merge m;")


def test_provenance_fails_a_file_that_neither_declares_nor_is_pending(rejection, monkeypatch):
    (rejection / "a.sysml").write_text("// nothing declared\npart def P { merge m; }\n")
    monkeypatch.setattr(corpus_sweep, "first_diagnostic", lambda *_: ("PARSE-UNEXPECTED", "x"))
    assert corpus_sweep.check_provenance(Path("sv2")) == 1


def test_provenance_passes_a_pending_file_with_no_declaration(rejection, monkeypatch):
    (rejection / "a.sysml").write_text("// nothing declared\npart def P { merge m; }\n")
    corpus_sweep.PENDING.write_text("tests/rejection/a.sysml\n")
    monkeypatch.setattr(corpus_sweep, "first_diagnostic", lambda *_: ("PARSE-UNEXPECTED", "x"))
    assert corpus_sweep.check_provenance(Path("sv2")) == 0


def test_provenance_fails_when_the_reason_moved(rejection, monkeypatch):
    # The case this check exists for: the file still rejects, and for something else.
    (rejection / "a.sysml").write_text(
        "// rejects: PARSE-UNEXPECTED on: merge m;\npart def P { merge m; }\n"
    )
    monkeypatch.setattr(
        corpus_sweep, "first_diagnostic", lambda *_: ("PARSE-EXPECTED", "part def P { merge m; }")
    )
    assert corpus_sweep.check_provenance(Path("sv2")) == 1


def test_provenance_fails_a_declared_file_that_is_still_pending(rejection, monkeypatch):
    # Otherwise the backlog would never be worked off: a file could declare its reason and
    # stay on the list, and the list's count would stop meaning anything.
    (rejection / "a.sysml").write_text(
        "// rejects: PARSE-UNEXPECTED on: merge m;\npart def P { merge m; }\n"
    )
    corpus_sweep.PENDING.write_text("tests/rejection/a.sysml\n")
    monkeypatch.setattr(
        corpus_sweep, "first_diagnostic", lambda *_: ("PARSE-UNEXPECTED", "merge m;")
    )
    assert corpus_sweep.check_provenance(Path("sv2")) == 1


def test_provenance_fails_a_pending_entry_whose_file_is_gone(rejection, monkeypatch):
    # `rejection` is required: it is what puts NEGATIVE and PENDING under a temporary ROOT.
    assert rejection.is_dir()
    corpus_sweep.PENDING.write_text("tests/rejection/gone.sysml\n")
    monkeypatch.setattr(corpus_sweep, "first_diagnostic", lambda *_: None)
    assert corpus_sweep.check_provenance(Path("sv2")) == 1


def test_provenance_fails_a_declaration_the_parser_contradicts_with_silence(rejection, monkeypatch):
    (rejection / "a.sysml").write_text(
        "// rejects: PARSE-UNEXPECTED on: merge m;\npart def P { merge m; }\n"
    )
    monkeypatch.setattr(corpus_sweep, "first_diagnostic", lambda *_: None)
    assert corpus_sweep.check_provenance(Path("sv2")) == 1
