// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! A qualified name: the segments that identify an element from somewhere else.

use sv2_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

use crate::ast_node::AstNode;

/// A name written to refer to an element declared elsewhere.
///
/// ```text
/// QualifiedName = ( '$' '::' )? ( NAME '::' )* NAME
/// ```
///
/// `KerML` 8.2.3.4.1, and Note 3 of that clause is what this view is shaped by: a
/// qualified name is "a sequence of segment names separated by `::` punctuation,
/// optionally with the global scope qualifier `$` as an initial segment", and "an
/// unqualified name can be considered the degenerate case of a qualified name with a
/// single segment name". So a bare `Boolean` is a qualified name of one segment, not a
/// different thing.
///
/// **The only node in this crate so far that holds nothing but tokens.** There are no
/// child nodes to delegate to: the segments are `BasicName` or `UnrestrictedName` tokens,
/// the separators are `ColonColon`, and the qualifier is a single `Dollar`
/// (`qualified_name`, `crates/sv2-syntax/src/parser/import.rs`). `tests/sweep.rs` asserts
/// the absence of child nodes over every tree in the repository.
///
/// What this view does **not** do is resolve the name. Note 3 says the segments before
/// the last "identify a sequence of nested Namespaces that provide the context for
/// resolving the final segment name", and that resolution needs more than this file —
/// it belongs to `sv2-resolve` (`.claude/rules/ast.md`). This view says which segments
/// were written and which is final; it never says what any of them means.
// node: QualifiedName
pub struct QualifiedName(SyntaxNode);

impl AstNode for QualifiedName {
    const KIND: SyntaxKind = SyntaxKind::QualifiedName;

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

impl QualifiedName {
    /// The segment names, in the order they are written.
    ///
    /// The names only: the `::` separators and the `$` qualifier are punctuation and are
    /// not segments (`KerML` 8.2.3.4.1 Note 3). A one-segment iterator is the ordinary
    /// case, not a degenerate one — most references in the corpus are a single name.
    ///
    /// Empty is possible and means malformed: `expect_name` reports without consuming
    /// (`crates/sv2-syntax/src/parser/recovery.rs`), so `$::` builds a node holding two
    /// punctuation tokens and no name at all
    /// (`tests/malformed/qualified-name/global-qualifier-names-nothing.sysml`).
    ///
    /// Both name tokens are included, because the two spellings are the same thing: a
    /// `BasicName` is written bare and an `UnrestrictedName` in single quotes, and the
    /// corpus writes a segment whose name is `$` as `'$'` (`Scoping.kerml` line 32). That
    /// segment is an `UnrestrictedName` and is **not** [`is_global`](Self::is_global) —
    /// see there.
    ///
    /// Reads this node's own tokens, not its descendants'. Reading descendants is an
    /// equivalent mutation today, because a qualified name has no child nodes for a
    /// descendant token to hide in — `tests/sweep.rs` asserts that over every tree in the
    /// repository, and if it ever stopped holding, that assertion would fail before this
    /// accessor silently started reporting someone else's segments.
    pub fn segments(&self) -> impl Iterator<Item = SyntaxToken> + '_ {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .filter(|token| {
                matches!(
                    token.kind(),
                    SyntaxKind::BasicName | SyntaxKind::UnrestrictedName
                )
            })
    }

    /// The final segment: the name this qualified name identifies.
    ///
    /// The last of [`segments`](Self::segments), by the clause rather than by convenience
    /// — Note 3 singles it out, the segments before it being the context it is resolved
    /// in. The caller takes those from `segments()`; there is deliberately no second
    /// accessor for them, because two ways to say one thing are two things that can
    /// disagree.
    ///
    /// `None` when no segment was written at all, which is malformed. It is **not** "the
    /// last name written nearby": where recovery ends a name early, the names after it
    /// belong to no segment of this one, and this returns the last segment that is
    /// actually here (`tests/malformed/qualified-name/segments-lost-to-recovery.sysml`,
    /// where `A::::B` reports `A`).
    pub fn name(&self) -> Option<SyntaxToken> {
        self.segments().last()
    }

    /// Whether the global scope qualifier `$` opens this name.
    ///
    /// True exactly when a `Dollar` token is written here. That is a stronger statement
    /// than it looks, because a segment may itself be named `$`: `Scoping.kerml` declares
    /// `package '$'` and then refers to it as `'$'::Objects::Object` on line 32, two
    /// lines above `$::Objects::Object` on line 35. The quoted spelling lexes as an
    /// `UnrestrictedName` and is a segment; the bare one lexes as `Dollar` and is not.
    /// `tests/positive.rs::scoping_tells_a_package_named_dollar_from_the_global_qualifier`
    /// holds that pair, and it catches an accessor that counts the segment named `$` as
    /// the qualifier — measured, not assumed.
    ///
    /// What it does **not** catch, because nothing can: testing the node's TEXT for a
    /// leading `$` instead of looking for the token. That mutation survives the whole
    /// suite, and the first version of this comment claimed the opposite. It is an
    /// equivalent mutation rather than an untested one — an `UnrestrictedName` is always
    /// quoted, so the quoted segment's text begins with `'` and not `$`, and the
    /// production puts the `$` first or nowhere, so the token is present exactly when
    /// the trimmed text starts with `$`. The token-kind test is still the one to keep:
    /// it needs no string, and it does not rest on how the quoting happens to lex.
    ///
    /// The qualifier says where resolution starts, so it is independent of the segments
    /// and does not go away with them: `$::` is global and has no segments at all.
    pub fn is_global(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .any(|token| token.kind() == SyntaxKind::Dollar)
    }
}
