// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! One namespace member that owns something other than a feature.

use sv2_syntax::{SyntaxKind, SyntaxNode};

use crate::ast_node::AstNode;
use crate::member_prefix::MemberPrefix;

/// A namespace's ownership of one element that is not a feature.
///
/// ```text
/// NonFeatureMember : OwningMembership =
///     MemberPrefix ownedRelatedElement += MemberElement          (KerML 8.2.3.4.1)
/// MemberElement = AnnotatingElement | NonFeatureElement            (KerML 8.2.3.4.3)
/// ```
///
/// **`KerML` only.** ADR-0015 scopes this unit to the kerml grammar — a `SysML` namespace
/// owns its members through `PackageMember` — and all 608 of these nodes across the
/// repository's 610 trees come from `.kerml` files. With its sibling
/// [`NamespaceFeatureMember`](crate::NamespaceFeatureMember) it is the whole of `KerML`'s
/// `NamespaceMember`, and a type body reaches it too, `TypeBodyElement` naming it as its
/// first alternative (8.2.4.1.1).
///
/// The metaclass is `OwningMembership`, so this node is the *relationship* and not the
/// thing related. It is shape-identical to `PackageMember` and `NamespaceFeatureMember` —
/// a prefix and one owned element — and one parser method builds all three, differing only
/// in which elements it may own (`membership`, `crates/sv2-syntax/src/parser/namespace.rs`).
///
/// Both slots are mandatory, and every one of the 608 holds exactly one of each, which
/// `tests/sweep.rs` asserts as equality rather than as "at most one".
// node: NonFeatureMember
pub struct NonFeatureMember(SyntaxNode);

impl AstNode for NonFeatureMember {
    const KIND: SyntaxKind = SyntaxKind::NonFeatureMember;

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

impl NonFeatureMember {
    /// This member's prefix, which carries its visibility when one is written.
    ///
    /// Almost always empty: 6 of the 608 non-feature members in the corpus write a
    /// visibility. The node is built either way (`member_prefix`,
    /// `crates/sv2-syntax/src/parser/namespace.rs`), so an empty prefix is the ordinary
    /// shape rather than a defect, and the visibility is reached through the returned
    /// [`MemberPrefix`] rather than from here
    /// (`member.prefix().and_then(|prefix| prefix.visibility())`).
    ///
    /// Read from the direct children. Reading descendants instead cannot be observed —
    /// the mutation survives the sweep — because the prefix is the first child, which
    /// `tests/sweep.rs` asserts as the production's order, and so preorder reaches it
    /// before any prefix nested in the element.
    pub fn prefix(&self) -> Option<MemberPrefix> {
        self.child_where(|kind| kind == SyntaxKind::MemberPrefix)
            .and_then(MemberPrefix::cast)
    }

    /// The element this membership owns: the `MemberElement`.
    ///
    /// Untyped because `MemberElement` is an alternation that builds **no node of its
    /// own**, the second of the three cases in `.claude/rules/ast.md`, as
    /// [`PackageMember::element`](crate::PackageMember::element) is. What comes back is
    /// the concrete element, and twenty-eight kinds occur in the corpus — `Class`,
    /// `Package`, `Classifier`, `Association`, `Structure`, `Behavior`, `Comment`,
    /// `Documentation` and the relationship declarations among them — so match on
    /// `kind()`.
    ///
    /// **Never an `Error`, and `None` only on a tree the parser did not build.** A member
    /// is opened only when `at_kerml_non_feature_element` or `at_annotating_member` holds,
    /// and `kerml_non_feature_element` reads the element by the same list
    /// (`crates/sv2-syntax/src/parser/annotation.rs`), so the slot is never empty. Where
    /// the head is not recognised no member is built at all: the text goes to a
    /// `SyntaxKind::Error` beside the members, which no `NonFeatureMember` reports
    /// (`tests/malformed/non-feature-member/library-package-head-cut-short.kerml`). That
    /// is the opposite of the sibling, whose total fallback builds a member for anything;
    /// either way, read `Parse::errors()` rather than this slot to learn what was damaged.
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
