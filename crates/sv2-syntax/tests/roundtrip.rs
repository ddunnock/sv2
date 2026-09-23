// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! The losslessness invariant: `parse(s, l).text() == s`, for every `s` and every `l`.
//!
//! This is the most important test in the workspace (`.claude/rules/syntax.md`). A
//! graphical edit downstream has to produce a minimal text delta, and it cannot if
//! the tree threw any byte away — so the property is asserted over generated input,
//! not over a handful of hand-written strings, and it must hold for malformed text
//! as much as for valid text.

use std::path::{Path, PathBuf};

use proptest::prelude::*;
use sv2_syntax::{Language, parse};

/// Both grammars. Losslessness is a property of the tree, not of understanding the
/// text, so it holds for every input under every grammar — and asserting it under only
/// one would leave the `KerML` start symbol (ADR-0014) untested for it.
const LANGUAGES: [Language; 2] = [Language::KerMl, Language::SysMl];

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
        for language in LANGUAGES {
            prop_assert_eq!(parse(&source, language).text(), source.clone());
        }
    }

    /// Every diagnostic points somewhere inside the text it is about.
    ///
    /// A range past the end, or reversed, is a crash or a silent mis-highlight in
    /// whatever underlines it. This is the invariant the typed diagnostic exists for,
    /// so it is asserted over generated input rather than over examples.
    #[test]
    fn every_diagnostic_range_lies_within_the_source(source in source_text()) {
        for language in LANGUAGES {
            for diagnostic in parse(&source, language).errors() {
                let range = diagnostic.range();
                prop_assert!(range.start() <= range.end(), "{:?}", diagnostic);
                prop_assert!(
                    usize::from(range.end()) <= source.len(),
                    "{:?} past the end of {} bytes",
                    diagnostic,
                    source.len()
                );
                // And it must not split a character, or slicing it panics.
                prop_assert!(source.is_char_boundary(usize::from(range.start())), "{:?}", diagnostic);
                prop_assert!(source.is_char_boundary(usize::from(range.end())), "{:?}", diagnostic);
            }
        }
    }

    /// Arbitrary Unicode, including text no branch of the lexer was written for.
    #[test]
    fn parse_round_trips_arbitrary_text(source in ".{0,120}") {
        for language in LANGUAGES {
            prop_assert_eq!(parse(&source, language).text(), source.clone());
        }
    }

    /// Truncation is the editor's normal state: a file is incomplete for most of
    /// the seconds it is open. Every prefix must round-trip too.
    #[test]
    fn every_prefix_round_trips(source in source_text()) {
        for language in LANGUAGES {
            for end in 0..=source.len() {
                if let Some(prefix) = source.get(..end) {
                    prop_assert_eq!(parse(prefix, language).text(), prefix.to_owned());
                }
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
        // Each file against its own grammar, exactly as the sweep reads it.
        let Some(language) = Language::from_path(&path) else {
            continue;
        };
        assert_eq!(
            parse(&source, language).text(),
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

#[test]
fn deeply_nested_input_is_reported_and_not_a_stack_overflow() {
    // INVARIANT 3: the parser does not die on any input. A recursive-descent parser
    // dies on deeply nested input by overflowing the stack, and a stack overflow
    // ABORTS the process — it is not a panic, so `catch_unwind` would not save it
    // and no test that only checks for panics would catch it either.
    //
    // Measured before the depth guard existed: parenthesised expressions overflowed
    // between 5000 and 10000 levels, unary operators between 2000 and 5000, and
    // nested bodies between 5000 and 10000. Each construct below is an order of
    // magnitude past its own former limit.
    //
    // Losslessness is asserted with them, because the guard recovers rather than
    // truncating: the tokens past the limit still enter the tree as error nodes.
    for (what, source) in [
        (
            "parenthesised expressions",
            format!(
                "attribute x = {}1{};",
                "(".repeat(50_000),
                ")".repeat(50_000)
            ),
        ),
        (
            "unary operators",
            format!("attribute x = {}1;", "-".repeat(50_000)),
        ),
        (
            "nested bodies",
            format!(
                "{}package P;{}",
                "package P { ".repeat(50_000),
                "}".repeat(50_000)
            ),
        ),
        // Feature chains are the fourth, and the one that shows depth is a property of
        // the TREE and not of the parser's recursion. The fold that builds them is a
        // loop and uses no stack at all, and a 50000-link chain still overflowed a test
        // thread — because each link wraps the last, so the tree is as deep as the
        // chain is long. It is counted against the same budget for that reason.
        (
            "feature chains",
            format!("attribute x = a{};", ".b".repeat(50_000)),
        ),
        // The `->` operation folds as a chain link does, and for the same reason.
        (
            "function operations",
            format!("attribute x = a{};", "->f()".repeat(50_000)),
        ),
        // An index folds as a chain link does.
        (
            "index expressions",
            format!("attribute x = a{};", "#(1)".repeat(50_000)),
        ),
        // A body expression recurses through a calculation body back to an expression.
        (
            "expression bodies",
            format!(
                "attribute x = {}1{};",
                "{".repeat(50_000),
                "}".repeat(50_000)
            ),
        ),
    ] {
        // SysML: `attribute` and `part` are usages, which KerML has none of.
        let parsed = parse(&source, Language::SysMl);
        assert_eq!(parsed.text(), source, "{what} lost bytes");
        assert!(
            !parsed.errors().is_empty(),
            "{what} past the depth limit must be reported"
        );
    }
}
