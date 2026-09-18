# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import json

import pytest

from check_web_dependencies import Manifest, allowlist, check, is_allowed, is_exact, manifest

ALLOWED = frozenset({"react", "@codemirror/*", "sv2-wasm", "typescript"})
CLEAN = Manifest(
    declared={"react": "19.3.0"},
    trusted=(),
    package_manager="bun@1.4.2",
)


def test_clean_manifest_has_no_findings():
    assert check(CLEAN, ALLOWED, {"react": "19.3.0"}) == []


@pytest.mark.parametrize(
    ("name", "expected"),
    [
        ("react", True),
        ("@codemirror/state", True),
        ("@codemirror/view", True),
        # The glob is scoped. A package that merely starts the same way is not in it.
        ("@codemirrorx/state", False),
        ("lodash", False),
    ],
)
def test_allowlist_globs_are_scoped(name, expected):
    assert is_allowed(name, ALLOWED) is expected


@pytest.mark.parametrize(
    ("spec", "expected"),
    [
        ("19.3.0", True),
        # §13.1 declares sv2-wasm by path; a path names no version and cannot drift.
        ("file:../crates/sv2-wasm/pkg", True),
        # §3.1 rule 2: no ranges, in any of their spellings.
        ("^7", False),
        ("^19.3.0", False),
        ("~19.3.0", False),
        (">=19.3.0", False),
        ("19.x", False),
        ("19.3", False),
        ("latest", False),
        ("*", False),
    ],
)
def test_only_an_exact_version_or_a_path_is_accepted(spec, expected):
    assert is_exact(spec) is expected


def test_peer_dependencies_are_read_like_every_other_field():
    """Bun resolves and installs a peer dependency, so a range there is a real version."""
    pkg = manifest(json.dumps({"peerDependencies": {"typescript": "^7"}}))
    assert pkg.declared == {"typescript": "^7"}


@pytest.mark.parametrize(
    ("pkg", "present", "expected"),
    [
        (
            Manifest({"lodash": "4.17.21"}, (), "bun@1.4.2"),
            {"lodash": "4.17.21"},
            "not in [allowed]",
        ),
        (
            Manifest({"react": "^19.3.0"}, (), "bun@1.4.2"),
            {"react": "^19.3.0"},
            "forbids ranges",
        ),
        (
            Manifest({"react": "19.3.0"}, (), "bun@1.4.2"),
            {},
            "missing from [present]",
        ),
        (
            Manifest({"react": "19.3.0"}, (), "bun@1.4.2"),
            {"react": "19.2.0"},
            "[present] says",
        ),
        (
            Manifest({}, (), "bun@1.4.2"),
            {"react": "19.3.0"},
            "not declared in package.json",
        ),
        (
            Manifest({"react": "19.3.0"}, ("esbuild",), "bun@1.4.2"),
            {"react": "19.3.0"},
            "trustedDependencies",
        ),
        (
            Manifest({"react": "19.3.0"}, (), "bun@^1.4.2"),
            {"react": "19.3.0"},
            "exact release",
        ),
        (
            Manifest({"react": "19.3.0"}, (), ""),
            {"react": "19.3.0"},
            "exact release",
        ),
    ],
)
def test_disagreement_is_reported(pkg, present, expected):
    found = check(pkg, ALLOWED, present)
    assert any(expected in f for f in found), found


def test_present_is_checked_against_allowed_as_well():
    """[present] is a subset of [allowed] by construction; a stray entry is a defect."""
    found = check(CLEAN, ALLOWED, {"react": "19.3.0", "lodash": "4.17.21"})
    assert any("in [present] but not in [allowed]" in f for f in found), found


def test_allowlist_reads_both_tables():
    allowed, present = allowlist(
        '[allowed.react]\nkind = "runtime"\nwhy = "the UI framework"\n\n'
        '[present]\nreact = "19.3.0"\n'
    )
    assert allowed == frozenset({"react"})
    assert present == {"react": "19.3.0"}
