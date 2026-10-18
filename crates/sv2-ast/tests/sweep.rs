// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Every accessor, over every tree this repository holds (ADR-0025).
//!
//! The rejection corpus is malformed by construction, so the hardest inputs for this
//! layer arrive free and grow as the corpora grow.

mod trees;

use sv2_ast::{
    AliasMember, AstNode, ElementFilterMember, Import, MemberPrefix, PackageMember, QualifiedName,
    RootNamespace, VisibilityIndicator,
};
use sv2_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

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
/// The three node children an import may carry, each at most once.
///
/// The length check is the grammar's claim; the equality is the differential. They are
/// asserted together because the accessor's correctness depends on the claim: taking the
/// first of two would be a silent choice. `visibility()` is typed, so it compares through
/// its node; the other two are still raw, their kinds having no view yet.
fn assert_import_children(node: &SyntaxNode, import: &Import) {
    for (kind, accessor) in [
        (
            SyntaxKind::VisibilityIndicator,
            import.visibility().map(|v| v.syntax().clone()),
        ),
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
}

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

    // (b) and (c): the three node children, one of each at most.
    assert_import_children(node, &import);

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
    let answers = || {
        (
            import.visibility().map(|v| v.syntax().clone()),
            import.declaration(),
            import.body(),
        )
    };
    assert_eq!(
        answers(),
        answers(),
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
        member.prefix().map(|prefix| prefix.syntax().clone()),
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
        assert_eq!(prefix.syntax().kind(), SyntaxKind::MemberPrefix);
        assert_ne!(
            Some(prefix.syntax()),
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
        (
            member.prefix().map(|prefix| prefix.syntax().clone()),
            member.element()
        ),
        (
            member.prefix().map(|prefix| prefix.syntax().clone()),
            member.element()
        ),
        "a PackageMember accessor disagreed with itself between two calls on the same node"
    );
}

/// The two name slots of `member`, checked against the positions the production states.
///
/// Re-derived from the TOKEN SEQUENCE rather than by repeating the accessor's walk, so
/// this is a differential and not a restatement:
///
/// - both slots, if filled, stand before the `for` keyword — everything after it belongs
///   to the target, and a name there is inside the `QualifiedName` node;
/// - a short name requires a `<` before it, because that is the only thing that opens the
///   slot;
/// - the two are never the same token;
/// - and the arithmetic: the number of name tokens standing before `for` equals how many
///   slots came back filled. This is the one that catches a name being silently dropped,
///   which neither "it is after the `<`" nor "they differ" can see.
fn assert_alias_member_names(node: &SyntaxNode, member: &AliasMember) {
    // Everything before `for`, taken as an iterator rather than sliced at a position:
    // slicing is a workspace lint (invariant 3), and `get(..n).unwrap_or(..)` would buy
    // the same result with a branch that can never run, which is its own defect.
    let before_for: Vec<SyntaxToken> = node
        .children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .take_while(|token| token.kind() != SyntaxKind::KwFor)
        .collect();
    let is_name = |token: &SyntaxToken| {
        matches!(
            token.kind(),
            SyntaxKind::BasicName | SyntaxKind::UnrestrictedName
        )
    };

    let names: Vec<&SyntaxToken> = before_for.iter().filter(|t| is_name(t)).collect();
    let filled = usize::from(member.short_name().is_some()) + usize::from(member.name().is_some());
    assert_eq!(
        names.len(),
        filled,
        "an alias has {} name token(s) before `for` but {filled} slot(s) came back filled, \
         at {:?} — a name is being dropped or invented",
        names.len(),
        node.text_range()
    );

    if let Some(short) = member.short_name() {
        let opener = before_for
            .iter()
            .position(|token| token.kind() == SyntaxKind::Lt);
        assert!(
            opener.is_some(),
            "a short name was reported with no `<` to open the slot, at {:?}",
            node.text_range()
        );
        let short_at = before_for
            .iter()
            .position(|token| token.text_range() == short.text_range());
        assert!(
            short_at > opener,
            "the short name must stand after the `<`, at {:?}",
            node.text_range()
        );
    }

    if let (Some(short), Some(name)) = (member.short_name(), member.name()) {
        assert_ne!(
            short.text_range(),
            name.text_range(),
            "the short name and the member name must be different tokens, at {:?}",
            node.text_range()
        );
    }
}

/// What `AliasMember` must satisfy at every node, whatever the tree:
///
/// (a) `cast` succeeds exactly when `node.kind()` is `SyntaxKind::AliasMember`, and the
///     node survives the round trip through `syntax()`;
/// (b) `MemberPrefix`, `QualifiedName` and `RelationshipBody` are stated once each
///     (`SysML` 8.2.2.5.1), so no alias may carry two of any of them, and each accessor
///     equals the child re-derived by walking the tree;
/// (c) the two name slots hold what their positions say — `assert_alias_member_names`;
/// (d) calling each accessor twice returns the same answer.
/// The three node children an alias may carry, each at most once.
///
/// `target()` returns a typed `QualifiedName` since that kind got a view of its own, so
/// it is compared through its node; `body()` is still raw, `RelationshipBody` having no
/// view yet. That asymmetry is the delegation rule mid-migration, not an inconsistency.
fn assert_alias_member_children(node: &SyntaxNode, member: &AliasMember) {
    assert_eq!(
        member.prefix().map(|prefix| prefix.syntax().clone()),
        children_of_kind(node, SyntaxKind::MemberPrefix)
            .first()
            .cloned(),
        "prefix() is not this node's MemberPrefix child at {:?}",
        node.text_range()
    );
    for (kind, accessor) in [
        (
            SyntaxKind::QualifiedName,
            member.target().map(|target| target.syntax().clone()),
        ),
        (SyntaxKind::RelationshipBody, member.body()),
    ] {
        let direct = children_of_kind(node, kind);
        assert!(
            direct.len() <= 1,
            "an alias carries at most one {:?}, but this one has {} at {:?}",
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
}

fn assert_alias_member_accessor(node: &SyntaxNode) {
    let is_alias = node.kind() == SyntaxKind::AliasMember;
    let cast = AliasMember::cast(node.clone());
    assert_eq!(
        cast.is_some(),
        is_alias,
        "AliasMember::cast disagreed with node.kind() at {:?} {:?}",
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
        "AliasMember::syntax did not return the node it was cast from, at {:?}",
        node.text_range()
    );

    // (b) one of each, and the differential against the re-derivation.
    assert_alias_member_children(node, &member);

    // (c) the token slots, which every node-level assertion above is blind to.
    assert_alias_member_names(node, &member);

    // (d) idempotent.
    assert_eq!(
        (
            member.short_name().map(|t| t.text_range()),
            member.name().map(|t| t.text_range())
        ),
        (
            member.short_name().map(|t| t.text_range()),
            member.name().map(|t| t.text_range())
        ),
        "an AliasMember accessor disagreed with itself between two calls on the same node"
    );
}

/// What `MemberPrefix` must satisfy at every node, whatever the tree:
///
/// (a) `cast` succeeds exactly when `node.kind()` is `SyntaxKind::MemberPrefix`, and the
///     node survives the round trip through `syntax()`;
/// (b) the production is `( visibility = VisibilityIndicator )?` and nothing else, so a
///     prefix's children are **zero or one `VisibilityIndicator` and nothing of any other
///     kind**. This is the claim that makes the accessor total: there is no damaged tree
///     in which a prefix means something this one method cannot say;
/// (c) the differential: `visibility()` equals the child re-derived by walking the tree;
/// (d) a zero-width prefix has no visibility, and one with a visibility is not zero
///     width — the two readings of "empty" cannot come apart;
/// (e) calling the accessor twice returns the same answer.
fn assert_member_prefix_accessor(node: &SyntaxNode) {
    let is_prefix = node.kind() == SyntaxKind::MemberPrefix;
    let cast = MemberPrefix::cast(node.clone());
    assert_eq!(
        cast.is_some(),
        is_prefix,
        "MemberPrefix::cast disagreed with node.kind() at {:?} {:?}",
        node.kind(),
        node.text_range()
    );
    let Some(prefix) = cast else {
        return;
    };

    // (a) round trip.
    assert_eq!(
        prefix.syntax(),
        node,
        "MemberPrefix::syntax did not return the node it was cast from, at {:?}",
        node.text_range()
    );

    // (b) nothing but visibilities, and at most one.
    let visibilities = children_of_kind(node, SyntaxKind::VisibilityIndicator);
    assert!(
        visibilities.len() <= 1,
        "a member prefix carries at most one VisibilityIndicator, but this one has {} at {:?}",
        visibilities.len(),
        node.text_range()
    );
    assert_eq!(
        node.children().count(),
        visibilities.len(),
        "a member prefix holds nothing but its visibility, yet this one has other \
         children ({:?}) at {:?}",
        node.children().map(|c| c.kind()).collect::<Vec<_>>(),
        node.text_range()
    );

    // (c) the differential.
    assert_eq!(
        prefix.visibility().map(|v| v.syntax().clone()),
        visibilities.first().cloned(),
        "visibility() is not this node's VisibilityIndicator child at {:?}",
        node.text_range()
    );

    // (d) the two readings of "empty" agree. A prefix is zero width exactly when no
    // visibility was written for it, so neither reading can drift from the other.
    assert_eq!(
        node.text_range().len() == 0.into(),
        prefix.visibility().is_none(),
        "a prefix is zero width exactly when it has no visibility, at {:?}",
        node.text_range()
    );

    // (e) idempotent.
    assert_eq!(
        prefix.visibility().map(|v| v.syntax().clone()),
        prefix.visibility().map(|v| v.syntax().clone()),
        "visibility() disagreed with itself between two calls on the same node"
    );
}

/// What `ElementFilterMember` must satisfy at every node, whatever the tree:
///
/// (a) `cast` succeeds exactly when `node.kind()` is `SyntaxKind::ElementFilterMember`,
///     and the node survives the round trip through `syntax()`;
/// (b) the production is `MemberPrefix 'filter' OwnedExpression ';'` (`SysML`
///     8.2.2.5.1, `KerML` 8.2.5.13): one prefix, one `filter` keyword, one condition.
///     So no filter member anywhere may carry two prefixes, two conditions, or two
///     keywords — the claim both accessors rely on when they take the first match;
/// (c) the differential: `prefix()` and `filter()` partition the node's children, and
///     together account for every one of them exactly once. Each equality taken alone
///     passes an accessor that returned the prefix for both, because `filter()` is
///     defined as "not the prefix" and a defect can satisfy that vacuously;
/// (d) the positional claim the node-level assertions are blind to: the prefix stands
///     BEFORE the `filter` keyword and the condition AFTER it. `filter()` finds the
///     condition by kind-exclusion, so nothing in it checks that the thing it found is
///     the expression the keyword introduces;
/// (e) calling each accessor twice returns the same answer.
fn assert_element_filter_member_accessor(node: &SyntaxNode) {
    let is_filter = node.kind() == SyntaxKind::ElementFilterMember;
    let cast = ElementFilterMember::cast(node.clone());
    assert_eq!(
        cast.is_some(),
        is_filter,
        "ElementFilterMember::cast disagreed with node.kind() at {:?} {:?}",
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
        "ElementFilterMember::syntax did not return the node it was cast from, at {:?}",
        node.text_range()
    );

    // (b), (c) and (d): the two accessors partition this node's children.
    assert_element_filter_member_partition(node, &member);

    // (e) the keyword, and the two slots it separates.
    assert_element_filter_member_keyword(node, &member);

    // (f) idempotent.
    assert_eq!(
        (
            member.prefix().map(|prefix| prefix.syntax().clone()),
            member.filter()
        ),
        (
            member.prefix().map(|prefix| prefix.syntax().clone()),
            member.filter()
        ),
        "an ElementFilterMember accessor disagreed with itself between two calls"
    );
}

/// `prefix()` and `filter()` partition `node`'s direct children.
///
/// The same shape as `assert_package_member_partition`, and for the same reason:
/// `filter()` is defined as "not the prefix", so each equality taken alone passes an
/// accessor that returned the prefix for both. The arithmetic is what rules that out.
fn assert_element_filter_member_partition(node: &SyntaxNode, member: &ElementFilterMember) {
    let prefixes = children_of_kind(node, SyntaxKind::MemberPrefix);
    let conditions: Vec<SyntaxNode> = node
        .children()
        .filter(|child| child.kind() != SyntaxKind::MemberPrefix)
        .collect();
    assert!(
        prefixes.len() <= 1,
        "a filter member carries at most one MemberPrefix, but this one has {} at {:?}",
        prefixes.len(),
        node.text_range()
    );
    assert!(
        conditions.len() <= 1,
        "a filter member carries at most one condition, but this one has {} ({:?}) at {:?}",
        conditions.len(),
        conditions.iter().map(SyntaxNode::kind).collect::<Vec<_>>(),
        node.text_range()
    );

    // (c) the differential, and then the arithmetic that ties the two halves together.
    assert_eq!(
        member.prefix().map(|prefix| prefix.syntax().clone()),
        prefixes.first().cloned(),
        "prefix() is not this node's MemberPrefix child at {:?}",
        node.text_range()
    );
    assert_eq!(
        member.filter(),
        conditions.first().cloned(),
        "filter() is not this node's non-prefix child at {:?}",
        node.text_range()
    );
    assert_eq!(
        usize::from(member.prefix().is_some()) + usize::from(member.filter().is_some()),
        node.children().count(),
        "prefix() and filter() must account for every direct child exactly once, at {:?}",
        node.text_range()
    );
    if let (Some(prefix), Some(condition)) = (member.prefix(), member.filter()) {
        assert_ne!(
            prefix.syntax(),
            &condition,
            "filter() returned the prefix at {:?}",
            node.text_range()
        );
        assert_ne!(
            condition.kind(),
            SyntaxKind::MemberPrefix,
            "filter() returned a MemberPrefix at {:?}",
            node.text_range()
        );
    }
}

/// The `filter` keyword, and the two slots it separates.
///
/// `filter` is a reserved word in both languages (`RESERVED_KERML` and `RESERVED_SYSML`
/// in `crates/sv2-syntax/src/generated/kinds.rs`), so the token is a `KwFilter` and never
/// a name. The positional claim is the one nothing in either accessor checks: `filter()`
/// finds the condition by excluding the prefix's kind, so no part of it knows that what
/// it found is the expression the keyword introduces.
fn assert_element_filter_member_keyword(node: &SyntaxNode, member: &ElementFilterMember) {
    let keywords: Vec<SyntaxToken> = node
        .children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .filter(|token| token.kind() == SyntaxKind::KwFilter)
        .collect();
    assert_eq!(
        keywords.len(),
        1,
        "the production states one `filter` keyword, but this node has {} at {:?}",
        keywords.len(),
        node.text_range()
    );
    // Taken as a loop rather than by index: indexing is a workspace lint (invariant 3),
    // and a fallback branch would be one that can never run, given the length assertion
    // immediately above. The assertion makes this exactly one iteration.
    for keyword in keywords.iter().map(SyntaxToken::text_range) {
        if let Some(prefix) = member.prefix() {
            assert!(
                prefix.syntax().text_range().end() <= keyword.start(),
                "the prefix stands before the `filter` keyword, at {:?}",
                node.text_range()
            );
        }
        if let Some(condition) = member.filter() {
            assert!(
                condition.text_range().start() >= keyword.end(),
                "the condition stands after the `filter` keyword, at {:?}",
                node.text_range()
            );
        }
    }
}

/// What `QualifiedName` must satisfy at every node, whatever the tree:
///
/// (a) `cast` succeeds exactly when `node.kind()` is `SyntaxKind::QualifiedName`, and the
///     node survives the round trip through `syntax()`;
/// (b) the production is `( '$' '::' )? ( NAME '::' )* NAME` (`KerML` 8.2.3.4.1), which
///     holds **tokens only** — so this node has no child nodes anywhere in the
///     repository, and every token in it is a name, a `::` or the one `$`;
/// (c) the differential: `segments()` equals the name tokens re-derived by walking the
///     node's own tokens. Equality, not containment — a subset check passes an accessor
///     that yields nothing, one that reverses the order, and one that drops a segment;
/// (d) `name()` is the LAST of `segments()` and nothing else, and `is_global()` equals
///     the presence of a `Dollar` token child. The `Dollar` is what tells the qualifier
///     from a segment named `$`, which the corpus writes both of (`Scoping.kerml`);
/// (e) calling each accessor twice returns the same answer.
fn assert_qualified_name_accessor(node: &SyntaxNode) {
    let is_name = node.kind() == SyntaxKind::QualifiedName;
    let cast = QualifiedName::cast(node.clone());
    assert_eq!(
        cast.is_some(),
        is_name,
        "QualifiedName::cast disagreed with node.kind() at {:?} {:?}",
        node.kind(),
        node.text_range()
    );
    let Some(name) = cast else {
        return;
    };

    // (a) round trip.
    assert_eq!(
        name.syntax(),
        node,
        "QualifiedName::syntax did not return the node it was cast from, at {:?}",
        node.text_range()
    );

    // (b) tokens only, and only the three kinds the production states. Trivia is skipped
    // because `qualified_name` eats it before the node opens, so any that lands inside is
    // between segments and belongs to no slot.
    assert_eq!(
        node.children().count(),
        0,
        "a qualified name holds tokens only, yet this one has child nodes ({:?}) at {:?}",
        node.children()
            .map(|child| child.kind())
            .collect::<Vec<_>>(),
        node.text_range()
    );

    // (c) and (d): the segments, and the two accessors derived from them.
    assert_qualified_name_segments(node, &name);

    // (e) idempotent.
    assert_eq!(
        name.segments()
            .map(|token| token.text_range())
            .collect::<Vec<_>>(),
        name.segments()
            .map(|token| token.text_range())
            .collect::<Vec<_>>(),
        "segments() disagreed with itself between two calls on the same node"
    );
}

/// `segments()`, and the `name()` and `is_global()` that are read off it.
///
/// Re-derived from the TOKEN SEQUENCE rather than by repeating the accessor's own filter,
/// so this is a differential and not a restatement.
fn assert_qualified_name_segments(node: &SyntaxNode, name: &QualifiedName) {
    let written: Vec<SyntaxToken> = node
        .children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .filter(|token| {
            matches!(
                token.kind(),
                SyntaxKind::BasicName | SyntaxKind::UnrestrictedName
            )
        })
        .collect();
    let reported: Vec<SyntaxToken> = name.segments().collect();
    assert_eq!(
        reported
            .iter()
            .map(SyntaxToken::text_range)
            .collect::<Vec<_>>(),
        written
            .iter()
            .map(SyntaxToken::text_range)
            .collect::<Vec<_>>(),
        "segments() is not exactly this node's name tokens, in order, at {:?}",
        node.text_range()
    );

    // (d) `name()` is the last segment, and the qualifier is the `$` TOKEN rather than
    // anything about the text: a segment may itself be named `$`, written `'$'`, and
    // then it is an UnrestrictedName and not the qualifier (`Scoping.kerml` lines 32
    // and 35 write both).
    assert_eq!(
        name.name().map(|token| token.text_range()),
        written.last().map(SyntaxToken::text_range),
        "name() is not the last segment at {:?}",
        node.text_range()
    );
    let dollars = node
        .children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .filter(|token| token.kind() == SyntaxKind::Dollar)
        .count();
    assert!(
        dollars <= 1,
        "the production states one `$` at most, but this name has {dollars} at {:?}",
        node.text_range()
    );
    assert_eq!(
        name.is_global(),
        dollars == 1,
        "is_global disagreed with the presence of a Dollar token child at {:?}",
        node.text_range()
    );

    // (e) idempotent.
    assert_eq!(
        name.segments()
            .map(|token| token.text_range())
            .collect::<Vec<_>>(),
        name.segments()
            .map(|token| token.text_range())
            .collect::<Vec<_>>(),
        "segments() disagreed with itself between two calls on the same node"
    );
}

/// What `VisibilityIndicator` must satisfy at every node, whatever the tree:
///
/// (a) `cast` succeeds exactly when `node.kind()` is `SyntaxKind::VisibilityIndicator`,
///     and the node survives the round trip through `syntax()`;
/// (b) the production is `'public' | 'private' | 'protected'` (`KerML` 8.2.3.4.1), an
///     alternation of three terminals, so the node holds **one token and no child node**
///     — and the token is one of those three keywords and never anything else. That is
///     the claim that makes the accessor total;
/// (c) the differential: `keyword()` equals the token re-derived by walking the node's
///     own tokens;
/// (d) the node's width is the keyword's width. A zero-width indicator would mean the
///     parser took `visibility_indicator`'s error path, which nothing reaches — asserting
///     it here is what would notice if that changed.
fn assert_visibility_indicator_accessor(node: &SyntaxNode) {
    let is_indicator = node.kind() == SyntaxKind::VisibilityIndicator;
    let cast = VisibilityIndicator::cast(node.clone());
    assert_eq!(
        cast.is_some(),
        is_indicator,
        "VisibilityIndicator::cast disagreed with node.kind() at {:?} {:?}",
        node.kind(),
        node.text_range()
    );
    let Some(indicator) = cast else {
        return;
    };

    // (a) round trip.
    assert_eq!(
        indicator.syntax(),
        node,
        "VisibilityIndicator::syntax did not return the node it was cast from, at {:?}",
        node.text_range()
    );

    // (b), (c) and (d): the one token, and the two accessors read off it.
    assert_visibility_indicator_token(node, &indicator);
}

/// The one keyword token an indicator holds, and what the accessor makes of it.
///
/// Re-derived from the token sequence rather than by calling the accessor twice, so this
/// is a differential. The kind check is the alternation itself: three terminals, so a
/// token of any other kind here would mean the tree disagrees with the grammar.
fn assert_visibility_indicator_token(node: &SyntaxNode, indicator: &VisibilityIndicator) {
    assert_eq!(
        node.children().count(),
        0,
        "an indicator is one terminal, yet this one has child nodes ({:?}) at {:?}",
        node.children()
            .map(|child| child.kind())
            .collect::<Vec<_>>(),
        node.text_range()
    );
    let written: Vec<SyntaxToken> = node
        .children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .filter(|token| !token.text().trim().is_empty())
        .collect();
    assert!(
        written.len() <= 1,
        "the production is one terminal, but this indicator holds {} ({:?}) at {:?}",
        written.len(),
        written.iter().map(SyntaxToken::kind).collect::<Vec<_>>(),
        node.text_range()
    );
    for token in &written {
        assert!(
            matches!(
                token.kind(),
                SyntaxKind::KwPublic | SyntaxKind::KwPrivate | SyntaxKind::KwProtected
            ),
            "an indicator holds one of the three visibility keywords, not {:?}, at {:?}",
            token.kind(),
            node.text_range()
        );
    }

    // (c) the differential.
    assert_eq!(
        indicator.keyword().map(|token| token.text_range()),
        written.first().map(SyntaxToken::text_range),
        "keyword() is not this node's own keyword token at {:?}",
        node.text_range()
    );

    // (d) the node is exactly as wide as the keyword it holds. An indicator with no
    // keyword would be zero width, which is the shape `visibility_indicator`'s
    // unreachable error path would produce.
    assert_eq!(
        indicator.keyword().is_some(),
        node.text_range().len() > 0.into(),
        "an indicator is non-empty exactly when it holds a keyword, at {:?}",
        node.text_range()
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
            assert_alias_member_accessor(&node);
            assert_member_prefix_accessor(&node);
            assert_element_filter_member_accessor(&node);
            assert_qualified_name_accessor(&node);
            assert_visibility_indicator_accessor(&node);
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
