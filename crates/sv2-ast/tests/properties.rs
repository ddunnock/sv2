// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Accessor invariants over generated input: the cases nobody thought to write.

#![cfg(not(target_os = "wasi"))]

mod trees;

use proptest::prelude::*;
use sv2_syntax::Language;

/// A fragment alphabet that reaches real productions without being a grammar.
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

    #[test]
    fn truncating_at_any_byte_never_panics(source in fragment(), cut in 0usize..200) {
        let at = source.char_indices().map(|(i, _)| i).nth(cut).unwrap_or(source.len());
        let prefix = source.get(..at).unwrap_or(&source);
        let root = trees::tree(prefix, Language::SysMl);
        let limit = u32::try_from(source.len()).unwrap_or(u32::MAX);
        prop_assert!(root.text_range().len() <= limit.into());
    }
}
