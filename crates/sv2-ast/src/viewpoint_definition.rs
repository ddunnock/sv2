// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! A viewpoint definition: its prefix, its `viewpoint def` keywords, its declaration and
//! its body.

use sv2_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

use crate::ast_node::AstNode;
use crate::occurrence_definition_prefix::OccurrenceDefinitionPrefix;

/// A viewpoint definition.
///
/// ```text
/// ViewpointDefinition = OccurrenceDefinitionPrefix 'viewpoint' 'def'
///                       DefinitionDeclaration RequirementBody
/// ```
///
/// `SysML` 8.2.2.26.3. Built by `viewpoint_definition`
/// (`crates/sv2-syntax/src/parser/view.rs`) on
/// [`RequirementDefinition`](crate::RequirementDefinition)'s spine: it takes the
/// declaration and the body directly, so there is **no `Definition` node** here, and the
/// two are this view's own children. The body is a `RequirementBody`, the body a
/// requirement definition takes (8.2.2.26.3, 8.2.2.21.1). 5 in the corpus, all under
/// `PackageMember`.
///
/// **Both keywords are always there.** `at_viewpoint_definition` begins a viewpoint
/// definition only on `viewpoint` followed by `def`, so `viewpoint A def B;` is a
/// `ViewpointUsage` and builds no viewpoint definition at all
/// (`tests/malformed/viewpoint-definition/def-missing.sysml`). A missing `def` is therefore
/// a missing node, never a node missing its keyword, and the keyword accessors answer
/// `None` on no tree this parser builds.
///
/// The prefix has a view and is returned typed. `DefinitionDeclaration` and
/// `RequirementBody` have kinds of their own and no views yet, so they are returned raw
/// until they do. The keywords are this node's own tokens.
// node: ViewpointDefinition
pub struct ViewpointDefinition(SyntaxNode);

impl AstNode for ViewpointDefinition {
    const KIND: SyntaxKind = SyntaxKind::ViewpointDefinition;

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

impl ViewpointDefinition {
    /// The `OccurrenceDefinitionPrefix`: `abstract` or `variation`, `individual`, and the
    /// user-defined keywords. Built even when nothing is written, so usually present and
    /// zero width (`OccurrenceDefinitionPrefix`).
    ///
    /// Read from the direct children. Reading descendants cannot be observed: the sweep
    /// asserts the prefix is the node's first element, so preorder reaches it first.
    pub fn prefix(&self) -> Option<OccurrenceDefinitionPrefix> {
        self.0.children().find_map(OccurrenceDefinitionPrefix::cast)
    }

    /// The `viewpoint` keyword.
    pub fn viewpoint_keyword(&self) -> Option<SyntaxToken> {
        self.token(SyntaxKind::KwViewpoint)
    }

    /// The `def` keyword: the one written straight after `viewpoint`. A second `def` is
    /// not this node's. It is left in an `Error` beside the member, as a name
    /// (`tests/malformed/viewpoint-definition/def-written-twice.sysml`).
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
    /// exactly `OccurrenceDefinitionPrefix 'viewpoint' 'def' DefinitionDeclaration
    /// RequirementBody`, one of each. Searching descendants is equivalent because it also
    /// asserts the prefix holds neither kind and the declaration holds no body, so each is
    /// the first of its kind in preorder; the body, which may nest another viewpoint
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
    /// only element before the keywords, holds neither; a viewpoint definition nested
    /// in the body comes after them in preorder. Taking the first own token whatever its
    /// kind is not equivalent, and is caught: it answers `viewpoint` for `def`.
    fn token(&self, kind: SyntaxKind) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == kind)
    }
}
