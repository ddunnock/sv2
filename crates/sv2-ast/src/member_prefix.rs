// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! The prefix a membership may carry: its visibility, if one is written.

use sv2_syntax::{SyntaxKind, SyntaxNode};

use crate::ast_node::AstNode;

/// The optional visibility in front of a member.
///
/// ```text
/// MemberPrefix : Membership = ( visibility = VisibilityIndicator )?
/// ```
///
/// `SysML` 8.2.2.5.1. Shared by every membership production that admits a visibility —
/// `PackageMember` and `AliasMember` have one, and so do `ElementFilterMember`,
/// `NonFeatureMember` and the usage memberships. That breadth is why it is worth its own
/// accessor out of worklist order: without it, each of those has to reach into this node
/// itself, and reaching across a production boundary is how an accessor starts answering
/// for a node it is not a view of.
///
/// **The node is built whether or not a visibility is written, and an empty one is the
/// honest shape, not a defect.** The slot exists in the production, and a tree that
/// omitted the node when the slot was empty would make every consumer handle two shapes
/// for one construct (`member_prefix`, `crates/sv2-syntax/src/parser/namespace.rs`). Most
/// members in the pinned corpus carry no visibility, so most of these are zero width.
///
/// Nothing can make this production's own shape wrong: every part of it is optional, so
/// every `MemberPrefix` in any tree holds either nothing or one `VisibilityIndicator`.
/// `tests/sweep.rs` asserts exactly that over all 610 trees. What *can* go wrong is
/// attribution — see [`visibility`](Self::visibility).
// node: MemberPrefix
pub struct MemberPrefix(SyntaxNode);

impl AstNode for MemberPrefix {
    const KIND: SyntaxKind = SyntaxKind::MemberPrefix;

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

impl MemberPrefix {
    /// The visibility written for this member, or `None` when none was.
    ///
    /// `None` means **this member has no visibility of its own**. It does not mean none
    /// was written near it. Recovery can take one or more visibilities into an `Error`
    /// node and leave the member that follows with an empty prefix:
    /// `private protected part def A;` puts both words in one `Error` and raises a single
    /// diagnostic, and the surviving prefix reports neither
    /// (`tests/malformed/member-prefix/visibilities-lost-to-recovery.sysml`). An accessor
    /// that looked at a sibling or at preceding text would have to invent an answer, and
    /// there would be no principled way to choose between the two words it found.
    ///
    /// The `VisibilityIndicator` node comes back rather than its keyword token: that
    /// production is a node of its own, with its own accessor still on the worklist.
    ///
    /// Written as "the child of that kind" although `children().next()` would return the
    /// same node on every tree that exists, and so would a search over `descendants()`.
    /// Both were tried as mutations and neither can be observed, because `tests/sweep.rs`
    /// asserts over all 610 trees that a prefix's children are **nothing but** zero or
    /// one `VisibilityIndicator` — so there is no other child for `next()` to reach and
    /// nothing nested for `descendants()` to find. That is equivalence under a verified
    /// invariant rather than an untested guess, and it is stated here so the next edit
    /// knows which assertion is holding it up: weaken that one and these stop being the
    /// same function.
    pub fn visibility(&self) -> Option<SyntaxNode> {
        self.0
            .children()
            .find(|child| child.kind() == SyntaxKind::VisibilityIndicator)
    }
}
