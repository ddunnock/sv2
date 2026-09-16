// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! The losslessness invariant: `parse(s).text() == s`, for every `s`.
//!
//! This is the most important test in the workspace (`.claude/rules/syntax.md`). A
//! graphical edit downstream has to produce a minimal text delta, and it cannot if
//! the tree threw any byte away — so the property is asserted over generated input,
//! not over a handful of hand-written strings, and it must hold for malformed text
//! as much as for valid text.

use std::path::{Path, PathBuf};

use proptest::prelude::*;
use sv2_syntax::parse;

/// The alphabet a `SysML` file is actually made of, so the generator spends its
/// budget on comment delimiters, quotes and punctuation rather than on arbitrary
/// Unicode that no lexer branch looks at.
const FRAGMENTS: &[&str] = &[
    "package",
    "Vehicle",
    " ",
    "\n",
    "\t",
    // Form feed: white space per KerML 8.2.2.1 and absent from the Pilot's WS
    // terminal. Nothing in the corpus will exercise it.
    "\u{000C}",
    "{",
    "}",
    ";",
    "<",
    ">",
    "::",
    "//",
    "//*",
    "/*",
    "*/",
    "'",
    "\"",
    "\\",
    "12",
    "1e",
    "1e-3",
    // Multi-byte, to catch a slice landing inside a character.
    "é",
    "→",
    "\u{1F600}",
];

fn source_text() -> impl Strategy<Value = String> {
    // The weights keep the mix each fragment had when every one was its own branch:
    // the 25 fragments together are 25 times as likely as a generated identifier.
    let part = prop_oneof![
        25 => proptest::sample::select(FRAGMENTS).prop_map(str::to_owned),
        1 => "[a-zA-Z_][a-zA-Z0-9_]{0,4}",
    ];
    proptest::collection::vec(part, 0..24).prop_map(|parts| parts.concat())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]

    /// The invariant, over generated input.
    #[test]
    fn parse_round_trips_every_input(source in source_text()) {
        prop_assert_eq!(parse(&source).text(), source);
    }

    /// Arbitrary Unicode, including text no branch of the lexer was written for.
    #[test]
    fn parse_round_trips_arbitrary_text(source in ".{0,120}") {
        prop_assert_eq!(parse(&source).text(), source);
    }

    /// Truncation is the editor's normal state: a file is incomplete for most of
    /// the seconds it is open. Every prefix must round-trip too.
    #[test]
    fn every_prefix_round_trips(source in source_text()) {
        for end in 0..=source.len() {
            if let Some(prefix) = source.get(..end) {
                prop_assert_eq!(parse(prefix).text(), prefix.to_owned());
            }
        }
    }
}

/// Every `.sysml` and `.kerml` file under `dir`, found without recursing in Rust.
fn model_files(dir: PathBuf) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![dir];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path
                .extension()
                .is_some_and(|e| e == "sysml" || e == "kerml")
            {
                found.push(path);
            }
        }
    }
    found
}

#[test]
fn round_trips_the_corpus() {
    // The pinned corpus is the real evidence: 311 files a conformant tool accepts.
    // Losslessness is asserted over all of them even though almost none parses yet,
    // because the round-trip does not depend on the parser understanding the text.
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/corpus")
        .canonicalize();
    let Ok(root) = root else {
        // Inert when the corpus is not vendored, like every other pinned-input check.
        return;
    };

    let mut checked = 0_usize;
    for path in model_files(root) {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        assert_eq!(
            parse(&source).text(),
            source,
            "round-trip failed for {}",
            path.display()
        );
        checked += 1;
    }
    assert!(
        checked > 0,
        "corpus is present but no model files were read"
    );
}
