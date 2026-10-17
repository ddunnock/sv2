// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Every accessor, over every tree this repository holds (ADR-0025).
//!
//! The rejection corpus is malformed by construction, so the hardest inputs for this
//! layer arrive free and grow as the corpora grow.

mod trees;

use sv2_ast::{AstNode, Import, PackageMember, RootNamespace};
use sv2_syntax::{SyntaxElement, SyntaxKind, SyntaxNode};

/// The direct children of `node` that are of `kind`.
fn children_of_kind(node: &SyntaxNode, kind: SyntaxKind) -> Vec<SyntaxNode> {
    node.children()
        .filter(|child| child.kind() == kind)
        .collect()
}

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

/// What `Import` must satisfy at every node, whatever the tree:
///
/// (a) `cast` succeeds exactly when `node.kind()` is `SyntaxKind::Import`, and the node
///     survives the round trip through `syntax()`;
/// (b) the production states `VisibilityIndicator`, `ImportDeclaration` and
///     `RelationshipBody` **once each** (`SysML` 8.2.2.5.1, `KerML` 8.2.3.4.2), so no
///     import anywhere in the repository may carry two of any of them. This is the claim
///     `Import::child` relies on when it takes the first match and calls it the only one;
///     asserted here over all 610 trees rather than re-checked on every call;
/// (c) the differential: each accessor equals the child re-derived by walking the tree
///     directly. Equality, not containment — and against the re-derivation rather than
///     against a remembered answer, so an accessor that started returning a grandchild,
///     or a child of the wrong kind, fails;
/// (d) `is_import_all` equals whether a direct token child is `KwAll` — a token, so it is
///     invisible to every node-level assertion above. What the corpus cannot settle is
///     whether a DESCENDANT `all` is wrongly claimed, because no pinned file writes one
///     below an import without writing one above it; that case is hand written, in
///     `tests/positive.rs`;
/// (e) calling each accessor twice returns the same answer.
fn assert_import_accessor(node: &SyntaxNode) {
    let is_import = node.kind() == SyntaxKind::Import;
    let cast = Import::cast(node.clone());
    assert_eq!(
        cast.is_some(),
        is_import,
        "Import::cast disagreed with node.kind() at {:?} {:?}",
        node.kind(),
        node.text_range()
    );
    let Some(import) = cast else {
        return;
    };

    // (a) round trip: the accessor hands back the very node it was cast from.
    assert_eq!(
        import.syntax(),
        node,
        "Import::syntax did not return the node it was cast from, at {:?}",
        node.text_range()
    );

    // (b), then (c) per part. The length check is the grammar's claim; the equality is
    // the differential. They are asserted together because the accessor's correctness
    // depends on the claim: taking the first of two would be a silent choice.
    for (kind, accessor) in [
        (SyntaxKind::VisibilityIndicator, import.visibility()),
        (SyntaxKind::ImportDeclaration, import.declaration()),
        (SyntaxKind::RelationshipBody, import.body()),
    ] {
        let direct = children_of_kind(node, kind);
        assert!(
            direct.len() <= 1,
            "an import carries at most one {:?}, but this one has {} at {:?}",
            kind,
            direct.len(),
            node.text_range()
        );
        assert_eq!(
            accessor,
            direct.first().cloned(),
            "the {:?} accessor is not this node's {:?} child at {:?}",
            kind,
            kind,
            node.text_range()
        );
    }

    // (d) the token the node-level assertions cannot see.
    let all_written = node
        .children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .any(|token| token.kind() == SyntaxKind::KwAll);
    assert_eq!(
        import.is_import_all(),
        all_written,
        "is_import_all disagreed with the presence of a KwAll token child at {:?}",
        node.text_range()
    );

    // (e) idempotent: the same node answers the same way the second time.
    assert_eq!(
        (import.visibility(), import.declaration(), import.body()),
        (import.visibility(), import.declaration(), import.body()),
        "an Import accessor disagreed with itself between two calls on the same node"
    );
}

/// `prefix()` and `element()` partition `node`'s direct children.
///
/// (b) the production is `MemberPrefix` then **one** element, so neither half may have
///     two members;
/// (c) each accessor equals its re-derived half, and then the arithmetic that ties the
///     two together: the counts must add up to every child, exactly once. Each equality
///     taken alone passes an accessor that returned the prefix for both, because
///     `element()` is defined as "not the prefix" and a defect can satisfy that vacuously;
/// (d) the prefix is a `MemberPrefix`, the element never is, and they are never the same
///     node.
fn assert_package_member_partition(node: &SyntaxNode, member: &PackageMember) {
    let prefixes = children_of_kind(node, SyntaxKind::MemberPrefix);
    let elements: Vec<SyntaxNode> = node
        .children()
        .filter(|child| child.kind() != SyntaxKind::MemberPrefix)
        .collect();
    assert!(
        prefixes.len() <= 1,
        "a package member carries at most one MemberPrefix, but this one has {} at {:?}",
        prefixes.len(),
        node.text_range()
    );
    assert!(
        elements.len() <= 1,
        "a package member owns at most one element, but this one has {} ({:?}) at {:?}",
        elements.len(),
        elements.iter().map(SyntaxNode::kind).collect::<Vec<_>>(),
        node.text_range()
    );

    assert_eq!(
        member.prefix(),
        prefixes.first().cloned(),
        "prefix() is not this node's MemberPrefix child at {:?}",
        node.text_range()
    );
    assert_eq!(
        member.element(),
        elements.first().cloned(),
        "element() is not this node's non-prefix child at {:?}",
        node.text_range()
    );
    assert_eq!(
        usize::from(member.prefix().is_some()) + usize::from(member.element().is_some()),
        node.children().count(),
        "prefix() and element() must account for every direct child exactly once, at {:?}",
        node.text_range()
    );

    if let Some(prefix) = member.prefix() {
        assert_eq!(prefix.kind(), SyntaxKind::MemberPrefix);
        assert_ne!(
            Some(&prefix),
            member.element().as_ref(),
            "element() returned the prefix at {:?}",
            node.text_range()
        );
    }
    if let Some(element) = member.element() {
        assert_ne!(
            element.kind(),
            SyntaxKind::MemberPrefix,
            "element() returned a MemberPrefix at {:?}",
            node.text_range()
        );
    }
}

/// What `PackageMember` must satisfy at every node, whatever the tree:
///
/// (a) `cast` succeeds exactly when `node.kind()` is `SyntaxKind::PackageMember`, and the
///     node survives the round trip through `syntax()`;
/// (b) the production is `MemberPrefix` then **one** element (`SysML` 8.2.2.5.1), so no
///     member anywhere may carry two prefixes or two elements. This is the claim
///     `PackageMember::child_where` relies on when it takes the first match;
/// (c) the differential, and the one that matters here: `prefix()` and `element()`
///     partition the direct children. `element()` is defined as "not the prefix", so the
///     assertion re-derives both halves independently and demands that together they
///     account for every child and never for the same one — an accessor that returned
///     the prefix as the element would satisfy each half alone;
/// (d) the prefix is a `MemberPrefix` and the element never is;
/// (e) calling each accessor twice returns the same answer.
fn assert_package_member_accessor(node: &SyntaxNode) {
    let is_member = node.kind() == SyntaxKind::PackageMember;
    let cast = PackageMember::cast(node.clone());
    assert_eq!(
        cast.is_some(),
        is_member,
        "PackageMember::cast disagreed with node.kind() at {:?} {:?}",
        node.kind(),
        node.text_range()
    );
    let Some(member) = cast else {
        return;
    };

    // (a) round trip.
    assert_eq!(
        member.syntax(),
        node,
        "PackageMember::syntax did not return the node it was cast from, at {:?}",
        node.text_range()
    );

    // (b), (c) and (d): the two accessors partition this node's children.
    assert_package_member_partition(node, &member);

    // (e) idempotent.
    assert_eq!(
        (member.prefix(), member.element()),
        (member.prefix(), member.element()),
        "a PackageMember accessor disagreed with itself between two calls on the same node"
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
            assert_import_accessor(&node);
            assert_package_member_accessor(&node);
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
