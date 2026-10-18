// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! How a relationship ends: a semicolon, or braces owning annotations.

use sv2_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

use crate::ast_node::AstNode;

/// The tail of a relationship: `;`, or `{ ... }` owning what the relationship annotates.
///
/// ```text
/// RelationshipBody = ';' | '{' ( ownedRelationship += OwnedAnnotation )* '}'
///                                                           (SysML 8.2.2.2)
///
/// RelationshipBody : Relationship = ';' | '{' RelationshipOwnedElement* '}'
///                                                           (KerML 8.2.3.1)
/// ```
///
/// **The most shared child in the crate so far**, by a wide margin: across this
/// repository's 610 trees there are 857 of these nodes under **fourteen** different
/// parent productions — `Import` 744, `AliasMember` 29, `Dependency` 22,
/// `InitialNodeMember` 20, `Expose` 10, `Redefinition` 5, `Subclassification` 5,
/// `FeatureInverting` 4, `Specialization` 4, `Subsetting` 4, `Conjugation` 3,
/// `FeatureTyping` 3, `Disjoining` 2 and `TypeFeaturing` 2. Two of those parents already
/// shipped an accessor returning this kind raw, which is why it goes before the kinds
/// listed above it on the worklist (`.claude/rules/ast.md`: a shared child is worth
/// taking out of order).
///
/// **Three reachable shapes, where the production states two.** 827 of the 857 are a bare
/// `;`, 16 are braced, and 14 are **zero width** — a relationship unterminated at end of
/// input, where `relationship_body` opens the node and finds neither alternative
/// (`crates/sv2-syntax/src/parser/annotation.rs`). The third shape is not in the grammar
/// and no clause describes it; [`terminator`](Self::terminator) is how a caller tells it
/// from the other two.
// node: RelationshipBody
pub struct RelationshipBody(SyntaxNode);

impl AstNode for RelationshipBody {
    const KIND: SyntaxKind = SyntaxKind::RelationshipBody;

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

impl RelationshipBody {
    /// What the braces own, in order — **unfiltered**.
    ///
    /// Empty for the `;` alternative, and for empty braces: `{ }` is written in the corpus
    /// and owns nothing, so an empty iterator does not mean the body was a semicolon. Ask
    /// [`terminator`](Self::terminator) which alternative this is.
    ///
    /// Unfiltered, and deliberately not an `annotations()` that keeps only
    /// `OwnedAnnotation`, because filtering would drop two of the three things that
    /// legitimately land here:
    ///
    /// - `SysML` admits `OwnedAnnotation` alone (8.2.2.2), but `KerML`'s
    ///   `RelationshipOwnedElement` is `OwnedRelatedElement | OwnedAnnotation`, and
    ///   `OwnedRelatedElement` is `NonFeatureElement | FeatureElement` (8.2.3.1). Neither
    ///   alternation builds a node, so what arrives is the concrete element — one body in
    ///   the pinned corpus owns a `Feature`;
    /// - recovery contributes `Error` nodes. The braced run reads an annotation when one
    ///   starts and otherwise recovers **one token at a time**, so a stray `part def X;`
    ///   arrives as four separate `Error` children, not one
    ///   (`sysml_relationship_body`, `crates/sv2-syntax/src/parser/annotation.rs`;
    ///   `tests/malformed/relationship-body/non-annotation-in-braces.sysml`). They are
    ///   returned as they are: elements with failed constraints still enter the IR
    ///   carrying their diagnostics, and validity gates writes, never reads (ADR-0002).
    ///
    /// So the count is not an annotation count, and the kinds are the only structural
    /// signal of what is really here. Match on `kind()`.
    pub fn elements(&self) -> impl Iterator<Item = SyntaxNode> + '_ {
        self.0.children()
    }

    /// The token that closed this body: the `;`, or the `}`.
    ///
    /// Its `kind()` is the discriminator between the two alternatives, which is why there
    /// is no second accessor asking which one was written — two ways to say one thing are
    /// two things that can disagree. `Semicolon` is the first alternative, `RBrace` the
    /// second.
    ///
    /// `None` means the body never closed, and that is reachable in two ways: the
    /// relationship ran out of input before either alternative began, leaving a zero-width
    /// body (14 of these are in the pinned trees), or a `{` was written and no `}`
    /// arrived, leaving one that is non-empty and still unterminated. Found by **position**
    /// — the last token of the node — rather than by searching for a kind, because that is
    /// what "closed" means here and it keeps the unclosed-brace case honest: a body holding
    /// `{` alone has a token but no terminator.
    ///
    /// **Closed is not healthy.** A body can return its `}` and still own four recovery
    /// fragments; a caller reading `terminator().is_some()` as well-formedness would pass
    /// the malformed case cited above. `Parse::errors()` is what says something failed.
    ///
    /// Two mutations are worth recording, because they came out differently. Accepting
    /// whatever token came last, rather than only a closer, survived the whole suite at
    /// first: with no unclosed body in any of the 610 trees, "the last token" and "the
    /// last token if it closes" agree everywhere, so nothing distinguished it. That was a
    /// gap rather than an equivalence, and
    /// `tests/malformed/relationship-body/unclosed-braces.sysml` was written to close it —
    /// it now fails that mutation. Searching for a closer **anywhere** in the node instead
    /// of taking the last token does survive, and is equivalent: the node holds only `;`,
    /// `{` and `}` directly and never a `;` beside a brace, both asserted in
    /// `tests/sweep.rs` over every tree, so a closer can only ever be last. If either
    /// assertion stopped holding, that mutation would become observable in the same run.
    pub fn terminator(&self) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .filter(|token| !token.text().trim().is_empty())
            .last()
            .filter(|token| matches!(token.kind(), SyntaxKind::Semicolon | SyntaxKind::RBrace))
    }
}
