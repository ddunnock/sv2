# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""The two values vendor_sync takes from a data file and uses to reach the network or disk.

No test here fetches anything (STD-001-PY §10.4): every case calls the resolver or the
path check directly.
"""

import hashlib

import pytest

import vendor_sync
from vendor_sync import _is_current, _resolve_url, _vendor_path

SHA = "5cca16d846016e62bb1e54e0e50e675254a022ef"
ENTRY = {"path": "vendor/pilot/x.xtext", "url": "https://host.invalid/{PILOT_SHA}/x.xtext"}


def test_a_full_commit_sha_is_substituted():
    assert _resolve_url(ENTRY, SHA) == f"https://host.invalid/{SHA}/x.xtext"


def test_a_url_without_the_placeholder_is_returned_unchanged():
    entry = {"path": "vendor/omg/a.xmi", "url": "https://host.invalid/a.xmi"}
    assert _resolve_url(entry, "UNSET") == "https://host.invalid/a.xmi"


@pytest.mark.parametrize(
    "revision",
    [
        "UNSET",
        "",
        "main",
        "v0.63.0",
        SHA.upper(),
        SHA[:-1],
        SHA + "0",
        f"{SHA}?ref=other",
        "../../../etc/passwd",
        f"{SHA}#fragment",
        f"{SHA}/../../other",
        "user@host:repo",
    ],
)
def test_a_revision_that_is_not_a_full_commit_sha_resolves_to_nothing(revision, capsys):
    assert _resolve_url(ENTRY, revision) is None
    assert "SKIP" in capsys.readouterr().out


def test_a_path_inside_vendor_resolves(tmp_path, monkeypatch):
    monkeypatch.setattr(vendor_sync, "ROOT", tmp_path)
    (tmp_path / "vendor").mkdir()
    assert _vendor_path("vendor/omg/a.xmi") == tmp_path / "vendor" / "omg" / "a.xmi"


@pytest.mark.parametrize(
    "entry_path",
    [
        "../.git/hooks/post-checkout",
        "vendor/../.git/config",
        "vendor/../../outside.txt",
        "/etc/passwd",
        "crates/sv2-syntax/src/lib.rs",
    ],
)
def test_a_path_outside_vendor_is_refused(entry_path, tmp_path, monkeypatch):
    monkeypatch.setattr(vendor_sync, "ROOT", tmp_path)
    (tmp_path / "vendor").mkdir()
    with pytest.raises(ValueError, match="outside vendor/"):
        _vendor_path(entry_path)


def test_a_file_matching_its_pin_is_current(tmp_path):
    path = tmp_path / "a.xmi"
    path.write_bytes(b"content")
    assert _is_current(path, hashlib.sha256(b"content").hexdigest())


def test_a_file_not_matching_its_pin_is_not_current(tmp_path):
    path = tmp_path / "a.xmi"
    path.write_bytes(b"other")
    assert not _is_current(path, hashlib.sha256(b"content").hexdigest())


def test_an_unpinned_entry_is_never_current(tmp_path):
    path = tmp_path / "a.xmi"
    path.write_bytes(b"content")
    assert not _is_current(path, "UNPINNED")


def test_a_missing_file_is_not_current(tmp_path):
    assert not _is_current(tmp_path / "absent.xmi", "f" * 64)
