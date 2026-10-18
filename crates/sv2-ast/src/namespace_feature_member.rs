// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! One namespace member that owns a feature.

use sv2_syntax::{SyntaxKind, SyntaxNode};

use crate::ast_node::AstNode;
use crate::member_prefix::MemberPrefix;

/// A namespace's ownership of one feature.
///
/// ```text
/// NamespaceFeatureMember : OwningMembership =
///     MemberPrefix ownedRelatedElement += FeatureElement        (KerML 8.2.3.4.1)
/// ```
///
/// **`KerML` only.** ADR-0015 re-scoped this unit to the kerml grammar because no `SysML`
/// grammar reaches it — a `SysML` namespace owns its members through `PackageMember`
/// instead — and the corpus agrees: all 133 of these nodes across the repository's 610
/// trees come from `.kerml` files. Its sibling for everything that is not a feature is
/// `NonFeatureMember`, and the two occur side by side in `Behaviors.kerml`.
///
/// The metaclass is `OwningMembership`, so this node is the *relationship* and not the
/// thing related. It is shape-identical to `PackageMember` — a prefix and one owned
/// element — and the frozen unit records that it is also shape-identical to
/// `OwnedFeatureMember` (`KerML` 8.2.4.1.6): the two are told apart by context rather than
/// by text, this one reached through `NamespaceMember` in a namespace body and that one
/// through `FeatureMember` in a type body, with only the membership metaclass differing.
///
/// Both slots are mandatory, and every one of the 133 holds exactly one of each, which
/// `tests/sweep.rs` asserts as equality rather than as "at most one".
// node: NamespaceFeatureMember
pub struct NamespaceFeatureMember(SyntaxNode);

impl AstNode for NamespaceFeatureMember {
    const KIND: SyntaxKind = SyntaxKind::NamespaceFeatureMember;

    fn cast(node: SyntaxNode) -> Option<Self> {
        if node.kind() == Self::KIND {
            Some(Self(node))
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

impl NamespaceFeatureMember {
    /// This member's prefix, which carries its visibility when one is written.
    ///
    /// Almost always empty here: of the 133 feature members in the corpus, **one** writes
    /// a visibility. The node is built either way (`member_prefix`,
    /// `crates/sv2-syntax/src/parser/namespace.rs`), so an empty prefix is the ordinary
    /// shape rather than a defect, and the visibility is reached through the returned
    /// [`MemberPrefix`] rather than from here
    /// (`member.prefix().and_then(|prefix| prefix.visibility())`).
    pub fn prefix(&self) -> Option<MemberPrefix> {
        self.child_where(|kind| kind == SyntaxKind::MemberPrefix)
            .and_then(MemberPrefix::cast)
    }

    /// The feature this membership owns: the `FeatureElement`.
    ///
    /// Untyped because `FeatureElement` is an alternation that builds **no node of its
    /// own**, the same case as [`PackageMember::element`](crate::PackageMember::element)
    /// and the second of the three in `.claude/rules/ast.md`. What comes back is the
    /// concrete element, and `feature_element`
    /// (`crates/sv2-syntax/src/parser/feature.rs`) dispatches to all ten of the
    /// alternatives: `Feature`, `Step`, `Expression`, `BooleanExpression`, `Invariant`,
    /// `Connector`, `BindingConnector`, `Succession`, `Flow` and `SuccessionFlow`.
    ///
    /// Three of the ten actually occur as a namespace member in the pinned corpus —
    /// `Feature` 130 times, `Expression` twice and `Flow` once — which is why
    /// `tests/positive.rs` reads the `Flow` rather than any of the 130: a case built on a
    /// `Feature` could not tell this accessor from one that looked for a `Feature` child
    /// specifically.
    ///
    /// **Never `None`, and never an `Error`.** `feature_element`'s dispatch ends in a
    /// total fallback to `self.feature()`, a `KerML` `Feature`'s keyword being optional,
    /// so there is no input for which the slot is empty or holds recovery damage. That
    /// makes `is_some()` worthless as evidence that anything usable was written — the
    /// pinned case is a `Feature` with no name, no typing and no terminator
    /// (`tests/malformed/namespace-feature-member/feature-with-nothing-after-it.kerml`).
    /// It is the trap `Import::declaration` documents, reached the other way round: there
    /// the node is opened before the name is sought, here the dispatch cannot fail to pick
    /// an alternative. Read `Parse::errors()`.
    pub fn element(&self) -> Option<SyntaxNode> {
        self.child_where(|kind| kind != SyntaxKind::MemberPrefix)
    }

    /// The first direct child whose kind satisfies `want`.
    ///
    /// First, not only: the production states one prefix and one element, so a second of
    /// either would mean the tree disagrees with the grammar. `tests/sweep.rs` asserts
    /// exactly one of each over every tree in the repository, which is the honest place
    /// for the claim — and which also makes taking the last match rather than the first an
    /// equivalent mutation, the same case `Import::child` records.
    fn child_where(&self, want: impl Fn(SyntaxKind) -> bool) -> Option<SyntaxNode> {
        self.0.children().find(|child| want(child.kind()))
    }
}
