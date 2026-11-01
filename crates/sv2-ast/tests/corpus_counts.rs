// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! The corpus counts the crate's doc comments argue from, pinned.
//!
//! A count in a doc comment goes stale silently when the corpus is re-pinned or the parser
//! changes what it builds. Decision `ast-corpus-count-provenance` settled what to do about
//! that: a count that only describes the corpus is not written down at all, and a count a
//! doc comment draws a conclusion from is written here, beside the sentence that relies on
//! it. A figure that moves turns this test red, and the fix is to re-read that sentence,
//! not just to edit the number.
//!
//! Each figure is measured over the scope its sentence names: "in the corpus" is the 311
//! files under `vendor/corpus`, "the repository's trees" adds the rejection files under
//! `tests/rejection`, and "the rejection files" is those alone. The expected values are
//! the figures the doc comments stated before this test existed, each measured then by a
//! probe and none adjusted to fit. Its first run caught one scope slip: the feature and
//! non-feature member figures had been measured over every tree, rejection files included,
//! while two of their sentences said "in the corpus". Every figure matched over the scope
//! it had actually been taken over, so the sentences were corrected, not the numbers.

mod trees;

use std::collections::BTreeMap;
use std::path::PathBuf;

use sv2_syntax::{Language, SyntaxKind, SyntaxNode};

/// Every count, keyed `<doc site>: <what is counted>`.
type Counts = BTreeMap<&'static str, usize>;

/// The root of every file in `paths`, with its language.
#[expect(
    clippy::panic,
    reason = "test support only; an unreadable pinned file means this test cannot run at \
              all, and saying which file is the whole diagnosis"
)]
fn roots(paths: Vec<PathBuf>) -> Vec<(Language, SyntaxNode)> {
    paths
        .into_iter()
        .map(|path| {
            let source = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()));
            let language = trees::language_of(&path);
            (language, trees::tree(&source, language))
        })
        .collect()
}

/// Adds one to `key`, which must already be in `counts`: every key is listed up front with
/// zero, so a figure the corpus stops producing reads as 0 rather than going missing.
#[expect(
    clippy::panic,
    reason = "test support only; an unlisted key is a mistake in this file"
)]
fn bump(counts: &mut Counts, key: &'static str) {
    match counts.get_mut(key) {
        Some(count) => *count += 1,
        None => panic!("{key} is not listed in the expected table"),
    }
}

/// Whether `node` spans no text: a slot the parser built with nothing written in it.
fn is_empty(node: &SyntaxNode) -> bool {
    node.text_range().is_empty()
}

/// Whether `node`'s `MemberPrefix` child writes a visibility.
fn writes_visibility(member: &SyntaxNode) -> bool {
    member
        .children()
        .filter(|child| child.kind() == SyntaxKind::MemberPrefix)
        .any(|prefix| {
            prefix
                .children()
                .any(|below| below.kind() == SyntaxKind::VisibilityIndicator)
        })
}

/// The figures measured over the corpus alone.
fn measure_corpus(counts: &mut Counts) {
    for (_, root) in roots(trees::corpus_files()) {
        for node in root.descendants() {
            match node.kind() {
                // definition_prefix.rs: "Almost always empty ... 284 of the 291 are. The
                // other 7 write one slot each: `variation` (2) or a `#X` (5)."
                SyntaxKind::DefinitionPrefix => {
                    let slots = node.children().count();
                    let variation = node
                        .descendants_with_tokens()
                        .any(|element| element.kind() == SyntaxKind::KwVariation);
                    let extension = node
                        .children()
                        .any(|child| child.kind() == SyntaxKind::DefinitionExtensionKeyword);
                    bump(
                        counts,
                        match (is_empty(&node), slots, variation, extension) {
                            (true, _, _, _) => "definition_prefix: empty",
                            (false, 1, true, false) => "definition_prefix: `variation` alone",
                            (false, 1, false, true) => "definition_prefix: one `#X` alone",
                            _ => "definition_prefix: anything else",
                        },
                    );
                }
                // occurrence_definition_prefix.rs: "Almost always empty ... 970 of the
                // 1,020 are."
                SyntaxKind::OccurrenceDefinitionPrefix => bump(
                    counts,
                    if is_empty(&node) {
                        "occurrence_definition_prefix: empty"
                    } else {
                        "occurrence_definition_prefix: not empty"
                    },
                ),
                _ => {}
            }
        }
    }
}

/// `namespace_feature_member.rs`'s and `non_feature_member.rs`'s figures, at one member of
/// either kind. Measured over every tree in the repository, which is what both files'
/// figures were taken over, the rejection files included.
fn measure_member(counts: &mut Counts, language: Language, member: &SyntaxNode) {
    let feature = member.kind() == SyntaxKind::NamespaceFeatureMember;
    // Both files: "KerML only ... and the corpus agrees", every one from a `.kerml` file.
    if language == Language::SysMl {
        bump(
            counts,
            if feature {
                "namespace_feature_member: in a .sysml file"
            } else {
                "non_feature_member: in a .sysml file"
            },
        );
    }
    // Both files: "Almost always empty", 1 of the 133 and 6 of the 608 writing a
    // visibility, so an empty prefix is the ordinary shape.
    bump(
        counts,
        match (feature, writes_visibility(member)) {
            (true, true) => "namespace_feature_member: prefix writes a visibility",
            (true, false) => "namespace_feature_member: prefix writes none",
            (false, true) => "non_feature_member: prefix writes a visibility",
            (false, false) => "non_feature_member: prefix writes none",
        },
    );
    if !feature {
        // tests/positive.rs, the `Behaviors.kerml` case: "243 of the 608 non-feature
        // members ... own a `Class` or a `Package`, and a case reading only those could not
        // tell `element()` from an accessor that looked for one of them."
        let element = member
            .children()
            .find(|child| child.kind() != SyntaxKind::MemberPrefix)
            .map(|element| element.kind());
        if matches!(element, Some(SyntaxKind::Class | SyntaxKind::Package)) {
            bump(counts, "non_feature_member: owns a Class or a Package");
        }
        return;
    }
    // namespace_feature_member.rs: "`Feature` 130 times, `Expression` twice and `Flow`
    // once -- which is why `tests/positive.rs` reads the `Flow`."
    let element = member
        .children()
        .find(|child| child.kind() != SyntaxKind::MemberPrefix)
        .map(|element| element.kind());
    bump(
        counts,
        match element {
            Some(SyntaxKind::Feature) => "namespace_feature_member: owns a Feature",
            Some(SyntaxKind::Expression) => "namespace_feature_member: owns an Expression",
            Some(SyntaxKind::Flow) => "namespace_feature_member: owns a Flow",
            _ => "namespace_feature_member: owns anything else",
        },
    );
}

/// The figures measured over every tree in the repository.
fn measure_all_trees(counts: &mut Counts) {
    let mut body_parents = std::collections::BTreeSet::new();
    let mut every = trees::corpus_files();
    every.extend(trees::rejection_files());
    for (language, root) in roots(every) {
        for node in root.descendants() {
            match node.kind() {
                SyntaxKind::NamespaceFeatureMember | SyntaxKind::NonFeatureMember => {
                    measure_member(counts, language, &node);
                }
                SyntaxKind::RelationshipBody => {
                    body_parents.extend(node.parent().map(|parent| parent.kind()));
                    bump(counts, relationship_body_shape(&node));
                }
                SyntaxKind::VisibilityIndicator => measure_visibility(counts, &node),
                _ => {}
            }
        }
    }
    // relationship_body.rs: "The most shared child in the crate so far ... under fourteen
    // different parent productions".
    counts.insert("relationship_body: parent kinds", body_parents.len());
}

/// `visibility_indicator.rs`'s figures, at one indicator.
fn measure_visibility(counts: &mut Counts, indicator: &SyntaxNode) {
    // "744 indicators sit under `Import` and 77 under `MemberPrefix` ... which is why the
    // counts are so uneven".
    bump(
        counts,
        match indicator.parent().map(|parent| parent.kind()) {
            Some(SyntaxKind::Import) => "visibility_indicator: under an Import",
            Some(SyntaxKind::MemberPrefix) => "visibility_indicator: under a MemberPrefix",
            _ => "visibility_indicator: under anything else",
        },
    );
    // `visibility()`: "no indicator in any of the repository's trees is empty", which is
    // what says the parser's error arm is unreachable.
    if is_empty(indicator) {
        bump(counts, "visibility_indicator: empty");
    }
}

/// `relationship_body.rs`: "Three reachable shapes, where the production states two. 827
/// of the 857 are a bare `;`, 16 are braced, and 14 are zero width"; and of `terminator`,
/// a mutation survived the sweep because there is "no unclosed body in any of the 610
/// trees" -- no `{` without its `}`.
fn relationship_body_shape(body: &SyntaxNode) -> &'static str {
    let first = body.first_token().map(|token| token.kind());
    let last = body.last_token().map(|token| token.kind());
    match (is_empty(body), first, last) {
        (true, _, _) => "relationship_body: zero width",
        (false, Some(SyntaxKind::Semicolon), _) => "relationship_body: a bare `;`",
        (false, Some(SyntaxKind::LBrace), Some(SyntaxKind::RBrace)) => {
            "relationship_body: braced and closed"
        }
        (false, Some(SyntaxKind::LBrace), _) => "relationship_body: braced and unclosed",
        _ => "relationship_body: anything else",
    }
}

/// The figure measured over the rejection files alone.
fn measure_rejection_files(counts: &mut Counts) {
    for (_, root) in roots(trees::rejection_files()) {
        // namespace.rs: an Error "appears here whenever the parser could not make sense of
        // top-level text -- 64 times across the 299 rejection files".
        for member in root.children() {
            if member.kind() == SyntaxKind::Error {
                bump(
                    counts,
                    "root_namespace: an Error member in a rejection file",
                );
            }
        }
    }
}

#[test]
fn the_corpus_counts_the_doc_comments_argue_from_still_hold() {
    let expected: Counts = [
        ("definition_prefix: empty", 284),
        ("definition_prefix: `variation` alone", 2),
        ("definition_prefix: one `#X` alone", 5),
        ("definition_prefix: anything else", 0),
        ("occurrence_definition_prefix: empty", 970),
        ("occurrence_definition_prefix: not empty", 50),
        ("namespace_feature_member: prefix writes a visibility", 1),
        ("namespace_feature_member: prefix writes none", 132),
        ("namespace_feature_member: owns a Feature", 130),
        ("namespace_feature_member: owns an Expression", 2),
        ("namespace_feature_member: owns a Flow", 1),
        ("namespace_feature_member: owns anything else", 0),
        ("non_feature_member: prefix writes a visibility", 6),
        ("non_feature_member: prefix writes none", 602),
        ("non_feature_member: owns a Class or a Package", 243),
        ("namespace_feature_member: in a .sysml file", 0),
        ("non_feature_member: in a .sysml file", 0),
        ("relationship_body: a bare `;`", 827),
        ("relationship_body: braced and closed", 16),
        ("relationship_body: zero width", 14),
        ("relationship_body: braced and unclosed", 0),
        ("relationship_body: anything else", 0),
        ("relationship_body: parent kinds", 14),
        ("visibility_indicator: under an Import", 744),
        ("visibility_indicator: under a MemberPrefix", 77),
        ("visibility_indicator: under anything else", 0),
        ("visibility_indicator: empty", 0),
        ("root_namespace: an Error member in a rejection file", 64),
    ]
    .into_iter()
    .collect();

    let mut counts: Counts = expected.keys().map(|key| (*key, 0)).collect();
    measure_corpus(&mut counts);
    measure_all_trees(&mut counts);
    measure_rejection_files(&mut counts);
    assert_eq!(
        counts, expected,
        "a figure a doc comment argues from has moved: re-read the sentence beside it \
         (the key names the file) before changing the number here"
    );
}
