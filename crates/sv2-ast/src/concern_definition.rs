// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! A concern definition: its prefix, its `concern def` keywords, its declaration and its
//! body.

use sv2_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

use crate::ast_node::AstNode;
use crate::occurrence_definition_prefix::OccurrenceDefinitionPrefix;

/// A concern definition.
///
/// ```text
/// ConcernDefinition = OccurrenceDefinitionPrefix 'concern' 'def'
///                     DefinitionDeclaration RequirementBody
/// ```
///
/// `SysML` 8.2.2.21.3: "A concern definition or usage is declared as a requirement
/// definition or usage ... using the kind keyword concern instead of requirement" (7.21.3,
/// receipt 0e50a373). Built by `concern_definition`
/// (`crates/sv2-syntax/src/parser/requirement.rs`) on the same spine as
/// [`RequirementDefinition`](crate::RequirementDefinition): **no `Definition` node**, the
/// declaration and the `RequirementBody` are this view's own children. 3 in the corpus,
/// all under `PackageMember`.
///
/// **Both keywords are always there.** `at_concern_definition` begins a concern definition
/// only on `concern` followed by `def`, so `concern A def B;` is a `ConcernUsage` and builds
/// no concern definition at all (`tests/malformed/concern-definition/def-missing.sysml`). A
/// missing `def` is therefore a missing node, never a node missing its keyword, and the
/// keyword accessors answer `None` on no tree this parser builds.
///
/// The prefix has a view and is returned typed. `DefinitionDeclaration` and
/// `RequirementBody` have kinds of their own and no views yet, so they are returned raw
/// until they do. The keywords are this node's own tokens.
// node: ConcernDefinition
pub struct ConcernDefinition(SyntaxNode);

impl AstNode for ConcernDefinition {
    const KIND: SyntaxKind = SyntaxKind::ConcernDefinition;

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

impl ConcernDefinition {
    /// The `OccurrenceDefinitionPrefix`: `abstract` or `variation`, `individual`, and the
    /// user-defined keywords. Built even when nothing is written, so usually present and
    /// zero width (`OccurrenceDefinitionPrefix`).
    ///
    /// Read from the direct children. Reading descendants cannot be observed: the sweep
    /// asserts the prefix is the node's first element, so preorder reaches it first.
    pub fn prefix(&self) -> Option<OccurrenceDefinitionPrefix> {
        self.0.children().find_map(OccurrenceDefinitionPrefix::cast)
    }

    /// The `concern` keyword.
    pub fn concern_keyword(&self) -> Option<SyntaxToken> {
        self.token(SyntaxKind::KwConcern)
    }

    /// The `def` keyword: the one written straight after `concern`. A second `def` is
    /// not this node's. It is left in an `Error` beside the member, as a name
    /// (`tests/malformed/concern-definition/def-written-twice.sysml`).
    pub fn def_keyword(&self) -> Option<SyntaxToken> {
        self.token(SyntaxKind::KwDef)
    }

    /// The `DefinitionDeclaration`: the identification and the subclassification part.
    /// Built even when nothing is written, so present and zero width after a `def` with
    /// no name.
    ///
    /// A raw `SyntaxNode` of kind `DefinitionDeclaration`, because that kind has no view
    /// yet.
    pub fn declaration(&self) -> Option<SyntaxNode> {
        self.child(SyntaxKind::DefinitionDeclaration)
    }

    /// The `RequirementBody`: `;`, or `{` and its items and `}`. Zero width when neither
    /// `;` nor `{` follows the declaration, which the parser reports.
    ///
    /// A raw `SyntaxNode` of kind `RequirementBody`, because that kind has no view yet.
    pub fn body(&self) -> Option<SyntaxNode> {
        self.child(SyntaxKind::RequirementBody)
    }

    /// The first direct child of `kind`.
    ///
    /// Two mutations of this cannot be observed, each under a `tests/sweep.rs` assertion.
    /// Taking the last is equivalent because the sweep asserts the node's elements are
    /// exactly `OccurrenceDefinitionPrefix 'concern' 'def' DefinitionDeclaration
    /// RequirementBody`, one of each. Searching descendants is equivalent because it also
    /// asserts the prefix holds neither kind and the declaration holds no body, so each is
    /// the first of its kind in preorder; the body, which may nest another concern
    /// definition, comes after both.
    fn child(&self, kind: SyntaxKind) -> Option<SyntaxNode> {
        self.0.children().find(|child| child.kind() == kind)
    }

    /// The first of this node's own tokens of `kind`.
    ///
    /// Two mutations of this cannot be observed, and each names the `tests/sweep.rs`
    /// assertion that holds it up. Taking the last token of the kind is equivalent because
    /// the sweep asserts the node's elements are exactly the production's, one of each.
    /// Searching descendant tokens is equivalent because it also asserts the prefix, the
    /// only element before the keywords, holds neither; a concern definition nested
    /// in the body comes after them in preorder. Taking the first own token whatever its
    /// kind is not equivalent, and is caught: it answers `concern` for `def`.
    fn token(&self, kind: SyntaxKind) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == kind)
    }
}
