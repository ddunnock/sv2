// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! One import relationship.

use sv2_syntax::{SyntaxElement, SyntaxKind, SyntaxNode};

use crate::ast_node::AstNode;
use crate::import_declaration::ImportDeclaration;
use crate::relationship_body::RelationshipBody;
use crate::visibility_indicator::VisibilityIndicator;

/// An import of one membership or one namespace into the enclosing namespace.
///
/// ```text
/// Import = visibility = VisibilityIndicator 'import' ( isImportAll ?= 'all' )?
///          ImportDeclaration RelationshipBody
/// ```
///
/// `SysML` 8.2.2.5.1, `KerML` 8.2.3.4.2.
///
/// **The visibility is mandatory**, which is the one thing about this production most
/// easily got wrong: everywhere else in the language visibility arrives through the
/// nullable `MemberPrefix`, but `Import` states `visibility = VisibilityIndicator` with no
/// `?`. `import A::*;` with no leading keyword is therefore not an `Import` at all — this
/// parser builds an `Error` node for it, because `at_import` requires the visibility
/// (`crates/sv2-syntax/src/parser/import.rs`, and `tests/rejection/`
/// `import-without-visibility.sysml`). The Pilot's Xtext omits the whole prefix and
/// handles it at its call sites, an LL factoring this project does not follow; the
/// reasoning is in `.claude/state/grammar/units/Import.json`.
// node: Import
pub struct Import(SyntaxNode);

impl AstNode for Import {
    const KIND: SyntaxKind = SyntaxKind::Import;

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

impl Import {
    /// This import's visibility indicator, holding `public`, `private` or `protected`.
    ///
    /// The production requires one, so a well-formed import always has it. It is still an
    /// `Option`, because validity gates writes and never reads (ADR-0002): a tree that
    /// parsed badly enough still enters the IR, and an accessor that promised a node the
    /// tree does not carry would have to panic to keep the promise.
    /// Returns the typed [`VisibilityIndicator`] rather than the raw node, under the
    /// delegation rule in `.claude/rules/ast.md`: the child has a kind of its own, so
    /// which keyword it is belongs to it and not to this node
    /// (`import.visibility().and_then(|visibility| visibility.keyword())`). It returned a
    /// raw `SyntaxNode` until that kind had a view to delegate to.
    pub fn visibility(&self) -> Option<VisibilityIndicator> {
        self.child(SyntaxKind::VisibilityIndicator)
            .and_then(VisibilityIndicator::cast)
    }

    /// Whether `all` is written, which is `isImportAll` in the abstract syntax.
    ///
    /// A direct token child, not a descendant: `all` belongs to this production, and a
    /// nested `FilterPackage` brings its own `ImportDeclaration` whose tokens are not
    /// this import's.
    pub fn is_import_all(&self) -> bool {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .any(|token| token.kind() == SyntaxKind::KwAll)
    }

    /// What is imported: an `ImportDeclaration` wrapping a `MembershipImport` or a
    /// `NamespaceImport`.
    ///
    /// **Present does not mean populated.** When the name is missing, this parser opens
    /// the declaration's nodes before looking for it and closes them empty, so this
    /// returns `Some` for a node of zero width — `public import ;` is the pinned case
    /// (`tests/malformed/import/declaration-names-nothing.sysml`). A caller that reads
    /// `is_some()` as "the import names something" is wrong in exactly the way an
    /// absence check cannot see. Ask the node for its range or its text.
    ///
    /// Returns the typed [`ImportDeclaration`] rather than the raw node, under the
    /// delegation rule in `.claude/rules/ast.md`: which alternative was written is that
    /// node's own question (`import.declaration().and_then(|d| d.imported())`). It
    /// returned a raw `SyntaxNode` until that kind had a view to delegate to.
    pub fn declaration(&self) -> Option<ImportDeclaration> {
        self.child(SyntaxKind::ImportDeclaration)
            .and_then(ImportDeclaration::cast)
    }

    /// The relationship body: `;`, or braces carrying annotations.
    ///
    /// Empty when the import is unterminated at end of file, for the same reason
    /// `declaration` can be: the node is opened before the terminator is sought.
    /// Returns the typed [`RelationshipBody`] rather than the raw node, under the
    /// delegation rule in `.claude/rules/ast.md`: what the body owns, and which of its two
    /// alternatives was written, are that node's own questions
    /// (`import.body().and_then(|body| body.terminator())`). It returned a raw
    /// `SyntaxNode` until that kind had a view to delegate to.
    pub fn body(&self) -> Option<RelationshipBody> {
        self.child(SyntaxKind::RelationshipBody)
            .and_then(RelationshipBody::cast)
    }

    /// The first direct child of `kind`.
    ///
    /// First, not only: the production states each of these once, so a second would mean
    /// the tree disagrees with the grammar. `tests/sweep.rs` asserts over every tree in
    /// the repository that there is never more than one, which is the honest place for
    /// that claim — here it would cost a scan on every call to re-check something the
    /// parser is responsible for.
    fn child(&self, kind: SyntaxKind) -> Option<SyntaxNode> {
        self.0.children().find(|child| child.kind() == kind)
    }
}
