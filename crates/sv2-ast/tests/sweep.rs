// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Every accessor, over every tree this repository holds (ADR-0025).
//!
//! The rejection corpus is malformed by construction, so the hardest inputs for this
//! layer arrive free and grow as the corpora grow.

mod trees;

use sv2_ast::{AstNode, RootNamespace};
use sv2_syntax::{SyntaxKind, SyntaxNode};

/// What every accessor must satisfy at every node, whatever the tree.
fn assert_node_invariants(node: &SyntaxNode) {
    for child in node.children() {
        assert!(
            node.text_range().contains_range(child.text_range()),
            "child {:?} at {:?} is outside its parent {:?} at {:?}",
            child.kind(),
            child.text_range(),
            node.kind(),
            node.text_range()
        );
    }
}

/// What `RootNamespace` must satisfy at every node, whatever the tree:
///
/// (a) `cast` succeeds exactly when `node.kind()` is `SyntaxKind::RootNamespace`, and
///     returns `None` otherwise, and the node survives the round trip;
/// (b) the differential assertion: `members()` yields the node's direct children,
///     **exactly** — same members, same order, no more and no fewer. Equality, not
///     containment: a subset check passes an accessor that yields nothing, one that
///     reverses the order, and one that silently filters a kind out, which are the
///     defects most likely to be introduced by a later edit.
/// (c) calling the accessor twice returns the same answer.
fn assert_root_namespace_accessor(node: &SyntaxNode) {
    let is_root_namespace = node.kind() == SyntaxKind::RootNamespace;
    let cast = RootNamespace::cast(node.clone());
    assert_eq!(
        cast.is_some(),
        is_root_namespace,
        "RootNamespace::cast disagreed with node.kind() at {:?} {:?}",
        node.kind(),
        node.text_range()
    );
    let Some(root_namespace) = cast else {
        return;
    };

    // (a) round trip: the accessor hands back the very node it was cast from. This is the
    // one contract `AstNode` itself states, and nothing else asserts it.
    assert_eq!(
        root_namespace.syntax(),
        node,
        "RootNamespace::syntax did not return the node it was cast from, at {:?}",
        node.text_range()
    );

    // (b) differential: re-derive the children by walking the tree directly, then demand
    // that `members()` equal them. Equality is the honest assertion here because
    // `members()` claims every direct child -- recovery `Error` nodes included (ADR-0002).
    // An accessor that filters a kind out must change this line and say why.
    let children_by_range: Vec<(SyntaxKind, sv2_syntax::TextRange)> = node
        .children()
        .map(|child| (child.kind(), child.text_range()))
        .collect();
    let first: Vec<(SyntaxKind, sv2_syntax::TextRange)> = root_namespace
        .members()
        .map(|member| (member.kind(), member.text_range()))
        .collect();
    assert_eq!(
        first,
        children_by_range,
        "members() is not exactly this node's direct children, in order, at {:?}",
        node.text_range()
    );

    // (c) idempotent: the same node answers the same way the second time.
    let second: Vec<(SyntaxKind, sv2_syntax::TextRange)> = root_namespace
        .members()
        .map(|member| (member.kind(), member.text_range()))
        .collect();
    assert_eq!(
        first, second,
        "members() disagreed with itself between two calls on the same node"
    );
}

fn sweep(paths: Vec<std::path::PathBuf>) -> usize {
    let mut visited = 0;
    for path in paths {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let root = trees::tree(&source, trees::language_of(&path));
        for node in root.descendants() {
            assert_node_invariants(&node);
            assert_root_namespace_accessor(&node);
            visited += 1;
        }
    }
    visited
}

#[test]
fn every_corpus_tree_holds_the_node_invariants() {
    let visited = sweep(trees::corpus_files());
    assert!(
        visited > 0,
        "the corpus produced no nodes; is vendor/corpus populated?"
    );
}

#[test]
fn every_rejection_tree_holds_the_node_invariants() {
    let visited = sweep(trees::rejection_files());
    assert!(visited > 0, "the rejection set produced no nodes");
}
