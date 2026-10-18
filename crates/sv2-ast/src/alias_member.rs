// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! One alias: a membership naming an element owned somewhere else.

use sv2_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

use crate::ast_node::AstNode;
use crate::member_prefix::MemberPrefix;
use crate::qualified_name::QualifiedName;

/// The two name slots while `AliasMember::names` walks the tokens.
///
/// A named state rather than three locals, so the one rule that matters is written once,
/// in [`fill`](Slots::fill), instead of being spelled out inside a `match` arm.
#[derive(Default)]
struct Slots {
    /// Whether a `<` has been seen and no `>` has closed it yet.
    open: bool,
    short: Option<SyntaxToken>,
    name: Option<SyntaxToken>,
}

impl Slots {
    /// Put `token` in the slot its position earns.
    ///
    /// The short-name slot takes it only while that slot is open **and still empty**; a
    /// second name in an open slot therefore falls through to the member name, which is
    /// what keeps `alias <c Car for X;` right when the `>` was never written. A third
    /// name would be dropped, and `tests/sweep.rs` asserts over every tree that no alias
    /// has one, so the drop is a proven impossibility rather than an assumed one.
    fn fill(&mut self, token: SyntaxToken) {
        let slot = if self.open && self.short.is_none() {
            &mut self.short
        } else {
            &mut self.name
        };
        if slot.is_none() {
            *slot = Some(token);
        }
    }
}

/// An extra name, in this namespace, for an element declared elsewhere.
///
/// ```text
/// AliasMember : Membership =
///     MemberPrefix
///     'alias' ( '<' memberShortName = NAME '>' )?
///     ( memberName = NAME )?
///     'for' memberElement = [QualifiedName]
///     RelationshipBody
/// ```
///
/// `SysML` 8.2.2.5.1.
///
/// The metaclass is `Membership`, **not** `OwningMembership`, and that distinction is what
/// the production exists for: `KerML` 8.3.2.4.3 says a `Membership` which does not own its
/// `memberElement` makes its member names "effectively aliases within the
/// `membershipOwningNamespace` for an Element with a separate `OwningMembership`". So
/// [`target`](Self::target) is a *reference* to an element declared elsewhere — a
/// `QualifiedName` — and not a nested element the way `PackageMember`'s is.
// node: AliasMember
pub struct AliasMember(SyntaxNode);

impl AstNode for AliasMember {
    const KIND: SyntaxKind = SyntaxKind::AliasMember;

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

impl AliasMember {
    /// This member's prefix, which carries its visibility when one is written.
    ///
    /// Present and empty is the healthy shape, for the reason [`MemberPrefix`] gives:
    /// the parser builds the node whether or not a visibility is written. The visibility
    /// is reached through the returned prefix —
    /// `alias.prefix().and_then(|prefix| prefix.visibility())`.
    pub fn prefix(&self) -> Option<MemberPrefix> {
        self.0
            .children()
            .find(|child| child.kind() == SyntaxKind::MemberPrefix)
            .and_then(MemberPrefix::cast)
    }

    /// The short name, written `< name >` — `memberShortName`.
    pub fn short_name(&self) -> Option<SyntaxToken> {
        self.names().0
    }

    /// The declared name — `memberName`.
    pub fn name(&self) -> Option<SyntaxToken> {
        self.names().1
    }

    /// Both name slots, told apart by position.
    ///
    /// Nothing about the two tokens' KIND distinguishes them: `memberShortName` and
    /// `memberName` are both `NAME`, which reaches the tree as `BasicName` or
    /// `UnrestrictedName`. Only where they stand says which is which, so this walks the
    /// direct token children in order and keeps one rule:
    ///
    /// **`<` opens the short-name slot, `>` closes it, and the slot holds at most one
    /// name.** A second name while the slot is still open therefore falls through to
    /// `memberName`.
    ///
    /// That last clause is not decoration. `expect` reports a missing `>` without
    /// consuming anything (`crates/sv2-syntax/src/parser/namespace.rs`), so
    /// `alias <c Car for X;` puts both names in the tree as adjacent siblings with no
    /// delimiter between them. A rule phrased as "the name *between* `<` and `>`" finds
    /// no short name there and then reports `c` as the member name — wrong about both
    /// slots from one missing character
    /// (`tests/malformed/alias-member/short-name-unclosed.sysml`). The fall-through is
    /// also what keeps `alias <> Car for X;` right, where the slot opens and closes empty
    /// and `Car` is the member name.
    ///
    /// The walk stops at `for`: everything after it belongs to the target.
    ///
    /// That `break` is **defensive and currently unobservable**, and it is left in with
    /// its reason rather than removed as dead. Dropping it changes no answer on any tree,
    /// because after `for` this node's direct children are nodes — `qualified_name` and
    /// `relationship_body` both open one — so no name token ever reaches the walk from
    /// there. That was checked rather than assumed, against nine damaged spellings
    /// including `alias Car for X Y;`, `alias Car for X Y for Z;` and a target left
    /// unterminated at end of file. The `break` states which tokens this walk is about,
    /// so the answer does not quietly depend on that structural accident holding.
    fn names(&self) -> (Option<SyntaxToken>, Option<SyntaxToken>) {
        let mut slots = Slots::default();
        for token in self
            .0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
        {
            match token.kind() {
                SyntaxKind::Lt => slots.open = true,
                SyntaxKind::Gt => slots.open = false,
                SyntaxKind::KwFor => break,
                SyntaxKind::BasicName | SyntaxKind::UnrestrictedName => slots.fill(token),
                _ => {}
            }
        }
        (slots.short, slots.name)
    }

    /// The element this alias names: a `QualifiedName` reference, never a nested element.
    ///
    /// Empty when nothing was written after `for`: the node is opened before the name is
    /// sought, so this returns `Some` for a node of zero width. Presence is not evidence
    /// that anything was named, the same trap `Import::declaration` documents — and with
    /// the typed view, a caller can now tell the two apart without reading text, because
    /// a zero-width target has no segments.
    ///
    /// Returns the typed [`QualifiedName`] rather than the raw node, under the delegation
    /// rule in `.claude/rules/ast.md`: the child has a kind of its own, so the segments
    /// and the global qualifier are its business and not this node's
    /// (`alias.target().and_then(|target| target.name())`). It returned a raw
    /// `SyntaxNode` until `QualifiedName` had a view to delegate to.
    pub fn target(&self) -> Option<QualifiedName> {
        self.0
            .children()
            .find(|child| child.kind() == SyntaxKind::QualifiedName)
            .and_then(QualifiedName::cast)
    }

    /// The relationship body: `;`, or braces carrying annotations.
    pub fn body(&self) -> Option<SyntaxNode> {
        self.0
            .children()
            .find(|child| child.kind() == SyntaxKind::RelationshipBody)
    }
}
