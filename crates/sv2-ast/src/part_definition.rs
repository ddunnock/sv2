// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! A part definition: its prefix, its `part def` keywords and its definition.

use sv2_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

use crate::ast_node::AstNode;
use crate::definition::Definition;
use crate::occurrence_definition_prefix::OccurrenceDefinitionPrefix;

/// A part definition.
///
/// ```text
/// PartDefinition = OccurrenceDefinitionPrefix 'part' 'def' Definition
/// ```
///
/// `SysML` 8.2.2.11. One of the eight productions `simple_definition`
/// (`crates/sv2-syntax/src/parser/definition.rs`) builds with one method; the other seven
/// differ from it only in the first keyword, and `AttributeDefinition` in taking a
/// `DefinitionPrefix`. 544 in the corpus, every one under a `PackageMember`.
///
/// **Both keywords are always there.** `at_simple_definition` begins a part definition
/// only on `part` followed by `def`, so `part A def B;` is a `PartUsage` and builds no
/// part definition at all (`tests/malformed/part-definition/def-missing.sysml`). A
/// missing `def` is therefore a missing node, never a node missing its keyword, and the
/// keyword accessors answer `None` on no tree this parser builds.
///
/// Both node children have views, so both are returned typed. The keywords are this node's
/// own tokens.
// node: PartDefinition
pub struct PartDefinition(SyntaxNode);

impl AstNode for PartDefinition {
    const KIND: SyntaxKind = SyntaxKind::PartDefinition;

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

impl PartDefinition {
    /// The `OccurrenceDefinitionPrefix`: `abstract` or `variation`, `individual`, and the
    /// user-defined keywords. Built even when nothing is written, so usually present and
    /// zero width (`OccurrenceDefinitionPrefix`).
    ///
    /// Read from the direct children. Reading descendants cannot be observed: the sweep
    /// asserts the prefix is the node's first element, so preorder reaches it first.
    pub fn prefix(&self) -> Option<OccurrenceDefinitionPrefix> {
        self.0.children().find_map(OccurrenceDefinitionPrefix::cast)
    }

    /// The `part` keyword.
    pub fn part_keyword(&self) -> Option<SyntaxToken> {
        self.token(SyntaxKind::KwPart)
    }

    /// The `def` keyword: the one written straight after `part`. A second `def` is not
    /// this node's. It is left in an `Error` beside the member, as a name
    /// (`tests/malformed/part-definition/def-written-twice.sysml`).
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
    /// the sweep asserts the node's elements are exactly `OccurrenceDefinitionPrefix
    /// 'part' 'def' Definition`, one of each. Searching descendant tokens is equivalent
    /// because it also asserts the prefix, the only element before the keywords, holds
    /// neither; a part definition nested in the body comes after them in preorder. Taking
    /// the first own token whatever its kind is not equivalent, and is caught: it answers
    /// `part` for `def`.
    fn token(&self, kind: SyntaxKind) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == kind)
    }
}
