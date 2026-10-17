// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Accessor invariants over generated input: the cases nobody thought to write.

#![cfg(not(target_os = "wasi"))]

mod trees;

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use sv2_ast::{AstNode, RootNamespace};
use sv2_syntax::Language;

/// A fragment alphabet that reaches real productions without being a grammar.
///
/// The last three are 2-, 3- and 4-byte characters. They are here so a cut lands
/// immediately before and after a multi-byte sequence, and so the byte length of a
/// fragment is routinely not its character count — the arithmetic a `text_range()` is
/// built from.
fn fragment() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        prop_oneof![
            Just("package ".to_owned()),
            Just("part ".to_owned()),
            Just("def ".to_owned()),
            Just("x".to_owned()),
            Just("{".to_owned()),
            Just("}".to_owned()),
            Just(";".to_owned()),
            Just("// note\n".to_owned()),
            Just("é".to_owned()),
            Just("→".to_owned()),
            Just("𝛼".to_owned()),
        ],
        0..40,
    )
    .prop_map(|parts| parts.concat())
}

proptest! {
    #[test]
    fn a_tree_from_any_fragment_holds_its_ranges(source in fragment()) {
        let root = trees::tree(&source, Language::SysMl);
        for node in root.descendants() {
            for child in node.children() {
                prop_assert!(node.text_range().contains_range(child.text_range()));
            }
        }
    }

    /// The root accessor over a truncated tree, which is an editor's normal state
    /// (ADR-0002): `members()` still answers, and still answers correctly.
    ///
    /// The cut is at a character boundary because the parser takes `&str`, and no `&str`
    /// can end inside a multi-byte sequence — a mid-character prefix is unrepresentable
    /// at this layer, not merely untested. The byte-level question belongs to the layer
    /// that does the slicing and is answered there: `every_prefix_round_trips` walks
    /// `0..=len` and `source.get(..end)` yields `None` at a non-boundary, so every byte
    /// position is considered (`crates/sv2-syntax/tests/roundtrip.rs`).
    ///
    /// The differential is re-derived here rather than calling the sweep's assertion:
    /// over generated input the point is a second, independent derivation, and a call
    /// into the sweep would inherit any defect the sweep's version has.
    #[test]
    fn the_root_accessor_holds_over_any_truncation(source in fragment(), cut in 0usize..1000) {
        // `cut` is taken modulo the character count so no draw is wasted past the end,
        // and `take` cannot split a character.
        let characters = source.chars().count();
        let prefix: String = source.chars().take(cut % (characters + 1)).collect();
        let root = trees::tree(&prefix, Language::SysMl);

        let Some(namespace) = RootNamespace::cast(root.clone()) else {
            return Err(TestCaseError::fail(
                "every tree has exactly one RootNamespace at its root",
            ));
        };

        let members: Vec<_> = namespace
            .members()
            .map(|member| (member.kind(), member.text_range()))
            .collect();
        let children: Vec<_> = root
            .children()
            .map(|child| (child.kind(), child.text_range()))
            .collect();
        prop_assert_eq!(
            members,
            children,
            "members() is not exactly the root's direct children for prefix {:?}",
            prefix
        );
    }
}
