// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! An attribute definition: its prefix, its `attribute def` keywords and its definition.

use sv2_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

use crate::ast_node::AstNode;
use crate::definition::Definition;
use crate::definition_prefix::DefinitionPrefix;

/// An attribute definition.
///
/// ```text
/// AttributeDefinition = DefinitionPrefix 'attribute' 'def' Definition
/// ```
///
/// `SysML` 8.2.2.7. One of the eight productions `simple_definition`
/// (`crates/sv2-syntax/src/parser/definition.rs`) builds with one method, and the one of
/// them that is not an occurrence: its prefix is a [`DefinitionPrefix`], with no
/// `individual`, where the other seven take an
/// [`OccurrenceDefinitionPrefix`](crate::OccurrenceDefinitionPrefix). So `individual
/// attribute def A;` leaves the `individual` in an `Error` and begins the definition at
/// `attribute` (`tests/malformed/attribute-definition/individual-before-attribute.sysml`).
/// 171 in the corpus, under `PackageMember` (153) and `DefinitionMember` (18).
///
/// **Both keywords are always there.** `at_simple_definition` begins an attribute
/// definition only on `attribute` followed by `def`, so `attribute A def B;` is an
/// `AttributeUsage` and builds no attribute definition at all
/// (`tests/malformed/attribute-definition/def-missing.sysml`). A missing `def` is
/// therefore a missing node, never a node missing its keyword, and the keyword accessors
/// answer `None` on no tree this parser builds.
///
/// Both node children have views, so both are returned typed. The keywords are this node's
/// own tokens.
// node: AttributeDefinition
pub struct AttributeDefinition(SyntaxNode);

impl AstNode for AttributeDefinition {
    const KIND: SyntaxKind = SyntaxKind::AttributeDefinition;

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

impl AttributeDefinition {
    /// The `DefinitionPrefix`: `abstract` or `variation`, and the user-defined keywords.
    /// Built even when nothing is written, so usually present and zero width
    /// (`DefinitionPrefix`).
    ///
    /// Read from the direct children. Reading descendants cannot be observed: the sweep
    /// asserts the prefix is the node's first element, so preorder reaches it first.
    pub fn prefix(&self) -> Option<DefinitionPrefix> {
        self.0.children().find_map(DefinitionPrefix::cast)
    }

    /// The `attribute` keyword.
    pub fn attribute_keyword(&self) -> Option<SyntaxToken> {
        self.token(SyntaxKind::KwAttribute)
    }

    /// The `def` keyword: the one written straight after `attribute`. A second `def` is
    /// not this node's. It is left in an `Error` beside the member, as a name
    /// (`tests/malformed/attribute-definition/def-written-twice.sysml`).
    pub fn def_keyword(&self) -> Option<SyntaxToken> {
        self.token(SyntaxKind::KwDef)
    }

    /// The `Definition`: the declaration and the body.
    ///
    /// Read from the direct children. Two mutations cannot be observed, each under a
    /// `tests/sweep.rs` assertion. Taking the last is equivalent because the definition is
    /// the node's only child of the kind. Reading descendants is equivalent because the
    /// prefix, the one element before it in preorder, is asserted to hold no `Definition`.
    pub fn definition(&self) -> Option<Definition> {
        self.0.children().find_map(Definition::cast)
    }

    /// The first of this node's own tokens of `kind`.
    ///
    /// Two mutations of this cannot be observed, and each names the `tests/sweep.rs`
    /// assertion that holds it up. Taking the last token of the kind is equivalent because
    /// the sweep asserts the node's elements are exactly `DefinitionPrefix 'attribute'
    /// 'def' Definition`, one of each. Searching descendant tokens is equivalent because it
    /// also asserts the prefix, the only element before the keywords, holds neither; an
    /// attribute definition nested in the body comes after them in preorder. Taking the
    /// first own token whatever its kind is not equivalent, and is caught: it answers
    /// `attribute` for `def`.
    fn token(&self, kind: SyntaxKind) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == kind)
    }
}
