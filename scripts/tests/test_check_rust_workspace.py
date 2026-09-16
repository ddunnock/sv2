# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import pytest

from check_rust_workspace import binary_findings, lib_findings, manifest_findings

GOOD = {
    "package": {
        "name": "sv2-x",
        "version": {"workspace": True},
        "edition": {"workspace": True},
        "rust-version": {"workspace": True},
        "license": {"workspace": True},
        "publish": {"workspace": True},
    },
    "dependencies": {
        "rowan": {"workspace": True},
        "miette": {"workspace": True, "features": ["fancy"]},
    },
    "lints": {"workspace": True},
}


def messages(findings):
    return [f.message for f in findings]


def test_conforming_manifest_has_no_findings():
    assert manifest_findings("sv2-x", GOOD) == []


@pytest.mark.parametrize(
    ("change", "expected"),
    [
        (lambda m: m.pop("lints"), "[lints] must be exactly"),
        (
            lambda m: m.update(lints={"workspace": True, "rust": {"unsafe_code": "allow"}}),
            "[lints] must be exactly",
        ),
        (lambda m: m["package"].update(edition="2021"), "`edition` must be"),
        (lambda m: m["package"].pop("publish"), "`publish` must be"),
        (lambda m: m.update(lib={"name": "renamed"}), "must not override the crate name"),
        (lambda m: m["dependencies"].update(serde="1"), "serde must be `{ workspace = true }`"),
        (
            lambda m: m["dependencies"].update(rowan={"workspace": True, "version": "0.16"}),
            "rowan also declares version",
        ),
        (
            lambda m: m.update({"dev-dependencies": {"insta": {"path": "../insta"}}}),
            "insta must be",
        ),
        (
            lambda m: m.update(target={"cfg(unix)": {"dependencies": {"libc": "0.2"}}}),
            "libc must be",
        ),
    ],
)
def test_manifest_violation_is_reported(change, expected):
    manifest = {k: (dict(v) if isinstance(v, dict) else v) for k, v in GOOD.items()}
    manifest["package"] = dict(GOOD["package"])
    manifest["dependencies"] = dict(GOOD["dependencies"])
    change(manifest)
    found = messages(manifest_findings("sv2-x", manifest))
    assert any(expected in m for m in found), found


def package(name, *kinds):
    return {"name": name, "targets": [{"name": name, "kind": [k]} for k in kinds]}


def test_binary_only_in_allowlisted_crates():
    packages = [
        package("sv2-cli", "lib", "bin"),
        package("sv2-syntax", "lib", "bin"),
        package("sv2-ast", "lib"),
    ]
    found = binary_findings(packages, {"sv2-cli": "the command line"})
    assert [f.crate for f in found] == ["sv2-syntax"]


LIB_OK = """//! Crate docs.
#![forbid(unsafe_code)]

// a comment
mod cli;
pub mod syntax;
#[cfg(test)]
mod tests;

pub use crate::cli::run;
pub use crate::syntax::{
    Node,
    Token,
};
"""


def test_declarations_and_reexports_are_allowed():
    assert lib_findings("sv2-x", LIB_OK) == []


@pytest.mark.parametrize(
    "extra",
    [
        "pub fn run() {}",
        "struct Parser;",
        "const LIMIT: usize = 4;",
        "use std::io;",
        "mod inline { pub fn f() {} }",
        "macro_rules! m { () => {} }",
    ],
)
def test_definitions_in_lib_rs_are_rejected(extra):
    assert lib_findings("sv2-x", LIB_OK + extra + "\n") != []
