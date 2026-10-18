// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! One package member: a membership owning one element.

use sv2_syntax::{SyntaxKind, SyntaxNode};

use crate::ast_node::AstNode;

/// A package's ownership of one element.
///
/// ```text
/// PackageMember : OwningMembership =
///     MemberPrefix ( ownedRelatedElement += DefinitionElement
///                  | ownedRelatedElement = UsageElement )
/// ```
///
/// `SysML` 8.2.2.5.1. `SysML` only, not `KerML` (ADR-0015): a `KerML` namespace owns its
/// members through `NonFeatureMember` and `NamespaceFeatureMember` instead.
///
/// The metaclass is `OwningMembership`, so this node is the *relationship*, not the thing
/// related — the package element itself is [`element`](Self::element). Two notational
/// slips in the clause are recorded in
/// `.claude/state/grammar/units/PackageMember@sysml.json` and change nothing that parses:
/// the production is written with no `=`, filed as SYSML21-335, and its two alternatives
/// assign with `+=` and `=` respectively where the Xtext uses `+=` for both.
// node: PackageMember
pub struct PackageMember(SyntaxNode);

impl AstNode for PackageMember {
    const KIND: SyntaxKind = SyntaxKind::PackageMember;

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

impl PackageMember {
    /// This member's prefix, which carries its visibility when one is written.
    ///
    /// **Present and empty is the healthy shape, not a defect.** `MemberPrefix` is
    /// `( visibility = VisibilityIndicator )?`, and this parser builds the node either
    /// way, deliberately: the slot exists in the production, and omitting the node when
    /// the slot is empty would make every consumer handle two shapes for one construct
    /// (`member_prefix`, `crates/sv2-syntax/src/parser/namespace.rs`). Most members in
    /// the corpus are written without a visibility, so most prefixes are zero width.
    ///
    /// An empty prefix therefore says only that *this member* has no visibility of its
    /// own. It does not mean none was written nearby: recovery can leave an orphaned
    /// `private` in an `Error` node immediately before a member whose prefix is empty,
    /// and that visibility is not this member's
    /// (`tests/malformed/package-member/orphaned-visibility.sysml`).
    ///
    /// The visibility inside is reached through `MemberPrefix`'s own accessor, which is
    /// still on the worklist. It is deliberately not reached from here: the production
    /// puts the visibility in the prefix, so an accessor on this node that returned it
    /// would be reaching across a production boundary — the same move that, done by
    /// mistake, is how an accessor starts answering for a node it is not a view of.
    pub fn prefix(&self) -> Option<SyntaxNode> {
        self.child_where(|kind| kind == SyntaxKind::MemberPrefix)
    }

    /// The element this membership owns: the `DefinitionElement` or `UsageElement`.
    ///
    /// Matched as "the child that is not the prefix" rather than against a list of kinds,
    /// because `DefinitionElement` and `UsageElement` get **no node of their own** — they
    /// are alternations, and the element that matched already says which alternative was
    /// taken, so a node there would carry nothing (`crates/sv2-syntax/src/parser/`
    /// `feature.rs`). The kind that comes back is therefore the concrete element:
    /// `Package`, `PartDefinition`, `PartUsage`, and so on across the two alternations.
    ///
    /// A `SyntaxKind::Error` can be one of these on a damaged tree, and is returned as
    /// it is. Validity gates writes, never reads (ADR-0002): a caller that cannot see the
    /// damage cannot report it. Match on `kind()`.
    pub fn element(&self) -> Option<SyntaxNode> {
        self.child_where(|kind| kind != SyntaxKind::MemberPrefix)
    }

    /// The first direct child whose kind satisfies `want`.
    ///
    /// First, not only: the production states one prefix and one element, so a second of
    /// either would mean the tree disagrees with the grammar. `tests/sweep.rs` asserts
    /// that over every tree in the repository, which is the honest place for the claim —
    /// here it would cost a scan on every call to re-check what the parser owns.
    fn child_where(&self, want: impl Fn(SyntaxKind) -> bool) -> Option<SyntaxNode> {
        self.0.children().find(|child| want(child.kind()))
    }
}
