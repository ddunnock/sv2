// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
#![allow(
    dead_code,
    reason = "this module is compiled into every test binary that declares `mod trees;`; \
              a helper used by only one of those binaries is unused in the others"
)]
//! Shared support for sv2-ast's integration tests: where the trees come from.
//!
//! Not shipped: this lives under `tests/`, so it is compiled only for them.

use std::path::{Path, PathBuf};

use sv2_syntax::{Language, SyntaxNode, parse};

/// The workspace root, from this crate's manifest directory.
#[expect(
    clippy::expect_used,
    reason = "test support only; a missing workspace root means the test binary cannot run at all"
)]
pub(crate) fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root is two levels above the crate manifest")
}

fn model_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("sysml" | "kerml")
            ) {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// Every positive-corpus file. These parse; the accessors see well-formed trees.
pub(crate) fn corpus_files() -> Vec<PathBuf> {
    model_files(&workspace().join("vendor/corpus"))
}

/// Every rejection case. These do NOT parse cleanly, which is the point: the trees are
/// partial, and a partial tree is the normal case for an editor (ADR-0002).
pub(crate) fn rejection_files() -> Vec<PathBuf> {
    model_files(&workspace().join("tests/rejection"))
}

/// The language a file's suffix names.
pub(crate) fn language_of(path: &Path) -> Language {
    match path.extension().and_then(|e| e.to_str()) {
        Some("kerml") => Language::KerMl,
        _ => Language::SysMl,
    }
}

/// The root node of `source`.
pub(crate) fn tree(source: &str, language: Language) -> SyntaxNode {
    parse(source, language).syntax()
}
