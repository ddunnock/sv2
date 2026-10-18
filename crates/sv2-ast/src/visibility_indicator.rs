// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! One visibility: `public`, `private` or `protected`.

use sv2_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

use crate::ast_node::AstNode;

/// The visibility written on a membership or an import.
///
/// ```text
/// VisibilityIndicator = 'public' | 'private' | 'protected'
/// ```
///
/// `KerML` 8.2.3.4.1 and `SysML` 8.2.2.5.1. An alternation of three terminals and nothing
/// else, which makes this the smallest node in the language that still gets a kind of its
/// own — and the second accessor here, after [`QualifiedName`](crate::QualifiedName), to
/// hold tokens only.
///
/// **It does not return a `Visibility` enum, and that is a citation rather than a
/// preference.** The Xtext writes this as `enum VisibilityIndicator returns
/// SysML::VisibilityKind`, binding each keyword to a literal, and the frozen unit
/// (`.claude/state/grammar/units/VisibilityIndicator.json`) records what that binding is:
/// "abstract-syntax assignment, not concrete syntax, and it is not carried across: what
/// parses is the keyword, and which enum literal it becomes belongs to whoever builds the
/// element". Constructing a metaclass is explicitly not this crate's work
/// (`.claude/rules/ast.md`). So this view hands back the keyword token, whose `kind()`
/// already distinguishes the three without anyone comparing a string.
///
/// A shared child, like `MemberPrefix`: across this repository's 610 trees there are 821
/// indicators, 744 of them under `Import` and 77 under `MemberPrefix`. `Import` states
/// its visibility where `MemberPrefix` leaves it optional, which is why the counts are so
/// uneven and why the corpus exercises this production mostly through imports.
// node: VisibilityIndicator
pub struct VisibilityIndicator(SyntaxNode);

impl AstNode for VisibilityIndicator {
    const KIND: SyntaxKind = SyntaxKind::VisibilityIndicator;

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

impl VisibilityIndicator {
    /// The keyword this indicator is: a `KwPublic`, `KwPrivate` or `KwProtected` token.
    ///
    /// Match on `kind()` to tell them apart. The three kinds are the alternation, so a
    /// caller needs no text and no enum of its own:
    ///
    /// ```text
    /// match indicator.keyword().map(|token| token.kind()) {
    ///     Some(SyntaxKind::KwPublic) => ...,
    ///     Some(SyntaxKind::KwPrivate) => ...,
    ///     Some(SyntaxKind::KwProtected) => ...,
    ///     _ => ...,
    /// }
    /// ```
    ///
    /// `Option` because the node can in principle be empty, and **not because any input
    /// makes it so**. `visibility_indicator`
    /// (`crates/sv2-syntax/src/parser/import.rs`) opens the node before it looks, and its
    /// `None` arm reports "`public`, `private` or `protected`" without consuming — but
    /// **no** caller can reach that arm, and the whole call graph says so rather than
    /// one entry point. It has exactly two callers: `member_prefix` (`namespace.rs`),
    /// guarded by `at_visibility()`, and `import()`, whose own three call sites
    /// (`namespace.rs`, and two in `metadata.rs`) are each guarded by `at_import()` —
    /// which is `at_visibility() && nth_is_keyword(1, "import")`. Every path in
    /// therefore already has a visibility keyword at the cursor, which is also why a
    /// bare `import A::*;` comes back as "unexpected `import`": it never reaches
    /// `import()` at all. Measured: all 821 indicators in the repository
    /// hold exactly one keyword, and the rejection corpus produces no empty one either.
    /// The arm exists so that a future caller cannot silently lose a token, and this
    /// signature exists so that such a caller would not get a wrong answer instead of
    /// none. `tests/sweep.rs` ties the two readings together — an indicator is non-empty
    /// exactly when it holds a keyword — so if the error path ever became reachable, that
    /// is where it would show.
    ///
    /// **Most mutations of this method are equivalent, and that is a property of the
    /// production rather than a gap in the tests.** Of seven tried, three were caught —
    /// reporting only `public`, reporting nothing, and the delegation returning nothing —
    /// and the other four cannot be observed, each under an invariant `tests/sweep.rs`
    /// asserts over all 610 trees:
    ///
    /// - taking the first token of **any** kind: the node's only token is the keyword,
    ///   because `visibility_indicator` eats trivia before it opens the node and bumps
    ///   exactly one token inside it;
    /// - taking the **last** matching token: there is at most one;
    /// - reading **descendant** tokens: the node has no child nodes for one to hide in;
    /// - and in the delegation, `MemberPrefix::visibility` reading descendants rather
    ///   than children: a prefix holds nothing but its visibility, so there is nothing
    ///   deeper to reach.
    ///
    /// Each becomes observable the moment its invariant stops holding, and the assertion
    /// that proves the invariant fails first. An alternation of three terminals leaves
    /// very little room to be wrong in, which is the honest reason the count is what it
    /// is.
    pub fn keyword(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| {
                matches!(
                    token.kind(),
                    SyntaxKind::KwPublic | SyntaxKind::KwPrivate | SyntaxKind::KwProtected
                )
            })
    }
}
