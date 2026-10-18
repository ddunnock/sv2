// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! One element filter: a membership owning the condition a namespace filters by.

use sv2_syntax::{SyntaxKind, SyntaxNode};

use crate::ast_node::AstNode;
use crate::member_prefix::MemberPrefix;

/// A namespace's filter on what its imports bring in.
///
/// ```text
/// ElementFilterMember : ElementFilterMembership =
///     MemberPrefix 'filter' ownedRelatedElement += OwnedExpression ';'
/// ```
///
/// `SysML` 8.2.2.5.1 and `KerML` 8.2.5.13 — one of the productions both languages share
/// (`.claude/state/grammar/units/ElementFilterMember.json`), and the frozen unit records
/// the one way the two clauses differ: `SysML` writes
/// `ownedRelatedElement += OwnedExpression` and `KerML` writes `condition =
/// OwnedExpression`. Nothing about what parses turns on that — the text is the same
/// either way — and the difference is only which abstract-syntax slot the expression
/// lands in, which belongs to whoever builds the element rather than to this view.
///
/// The metaclass is `ElementFilterMembership`, so this node is the *relationship*: a
/// filter is not an element of the namespace but a condition on which elements the
/// namespace's imports contribute.
///
/// Where a filter is admitted is a narrower question than which bodies have members:
/// `Body::admits_filter` (`crates/sv2-syntax/src/parser/namespace.rs`) answers it, and a
/// `filter` in a `SysML` definition body or at a `KerML` root is reported rather than
/// accepted — `tests/rejection/element-filter-member-is-not-a-definition-body-item.sysml`
/// and `element-filter-member-is-not-a-kerml-root-element.kerml` hold those.
// node: ElementFilterMember
pub struct ElementFilterMember(SyntaxNode);

impl AstNode for ElementFilterMember {
    const KIND: SyntaxKind = SyntaxKind::ElementFilterMember;

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

impl ElementFilterMember {
    /// This member's prefix, which carries its visibility when one is written.
    ///
    /// The visibility is reached through the returned [`MemberPrefix`] and not from here:
    /// the production puts it in the prefix, so an accessor on this node that returned it
    /// would be answering for a node this is not a view of
    /// (`member.prefix().and_then(|prefix| prefix.visibility())`).
    ///
    /// Present and empty is the healthy shape. `MemberPrefix` is
    /// `( visibility = VisibilityIndicator )?` and this parser builds the node either way
    /// (`member_prefix`, `crates/sv2-syntax/src/parser/namespace.rs`); most filters in the
    /// corpus are written without a visibility, so most of these are zero width.
    pub fn prefix(&self) -> Option<MemberPrefix> {
        self.child_where(|kind| kind == SyntaxKind::MemberPrefix)
            .and_then(MemberPrefix::cast)
    }

    /// The condition this filter selects by: the `OwnedExpression` after the keyword.
    ///
    /// **Untyped on purpose, and the first accessor to be.** `OwnedExpression` gets no
    /// node of its own — it is the entry to the fifteen precedence tiers of `KerML`
    /// 8.2.5.8.1 and builds nothing — so there is no typed view to delegate to, and the
    /// kind that comes back is whichever concrete expression matched. Across the 25
    /// filter members in this repository's corpus and rejection sets that is three
    /// different kinds already: `ClassificationExpression` for the common
    /// `filter @Safety;`, `ConditionalBinaryOperatorExpression` once an `and` or an `or`
    /// is written, and `BinaryOperatorExpression` for a comparison. An accessor that
    /// promised any one of them would be wrong on the next file
    /// (`tests/positive.rs::the_two_filters_in_one_file_hold_conditions_of_different_kinds`
    /// pins two of the three in one corpus file).
    ///
    /// When the expression layer grows accessors of its own, the caller casts what this
    /// returns — `member.filter().and_then(ClassificationExpression::cast)` — and this
    /// signature does not change. Returning a typed umbrella instead would mean inventing
    /// one over 39 expression kinds before any of them has a view, which `AstNode` cannot
    /// express anyway: it declares one `KIND` per view.
    ///
    /// `None` means no condition was written, which is malformed — the slot is the one
    /// part of this production that is not optional. The parser reports it without
    /// consuming (`base_expression`'s final `else`,
    /// `crates/sv2-syntax/src/parser/primary.rs`) and builds nothing for the slot, so
    /// there is no `SyntaxKind::Error` standing in it either: read `Parse::errors()` to
    /// tell a missing condition from one that was never expected
    /// (`tests/malformed/element-filter-member/condition-missing.sysml`).
    ///
    /// Matched as "the child that is not the prefix", the same way
    /// [`PackageMember::element`](crate::PackageMember::element) matches its element,
    /// because the only other things the production states are the two literals and they
    /// are tokens.
    ///
    /// Unlike that element slot, **this one is never a `SyntaxKind::Error`**, and the
    /// difference is worth stating because the obvious assumption is that a damaged tree
    /// puts one here. The expression reader cannot produce one: a partial read still
    /// closes its own expression node — `filter @ ;` yields a `ClassificationExpression`
    /// with nothing to classify, `filter 1 + ;` a `BinaryOperatorExpression` with one
    /// operand — and a read that cannot start at all reports without consuming
    /// (`base_expression`, `crates/sv2-syntax/src/parser/primary.rs`), leaving the slot
    /// empty rather than filled with damage. So the two answers this returns are a real
    /// expression node and `None`, and a caller looking for an `Error` here to detect
    /// damage will not find one; `Parse::errors()` is where that lives.
    ///
    /// The kind-exclusion form is still the right one, because it does not depend on that
    /// holding for ever. An `Error` appearing in this slot after some later parser change
    /// would be returned as it is — validity gates writes, never reads (ADR-0002) — and
    /// `tests/sweep.rs` would say so, because its partition arithmetic counts every child
    /// exactly once and an unclaimed `Error` breaks it. Match on `kind()`.
    pub fn filter(&self) -> Option<SyntaxNode> {
        self.child_where(|kind| kind != SyntaxKind::MemberPrefix)
    }

    /// The first direct child whose kind satisfies `want`.
    ///
    /// First, not only: the production states one prefix and one condition, so a second
    /// of either would mean the tree disagrees with the grammar. `tests/sweep.rs` asserts
    /// that over every tree in the repository, which is the honest place for the claim —
    /// here it would cost a scan on every call to re-check what the parser owns.
    ///
    /// Taking the **last** match instead of the first is therefore an equivalent mutation,
    /// and the mutation run that closed this accessor confirmed it survives the whole
    /// suite. It is not a gap in the tests: `find` and `last` agree by construction while
    /// the sweep's "at most one prefix, at most one condition" holds, and if that
    /// assertion ever failed this mutation would become observable in the same run. That
    /// is the honest link between the two, and it is the same case `Import::child`
    /// records.
    fn child_where(&self, want: impl Fn(SyntaxKind) -> bool) -> Option<SyntaxNode> {
        self.0.children().find(|child| want(child.kind()))
    }
}
