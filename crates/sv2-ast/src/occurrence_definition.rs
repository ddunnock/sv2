// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! An occurrence definition: its prefix, its `occurrence def` keywords and its definition.

use sv2_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

use crate::ast_node::AstNode;
use crate::definition::Definition;
use crate::occurrence_definition_prefix::OccurrenceDefinitionPrefix;

/// An occurrence definition.
///
/// ```text
/// OccurrenceDefinition = OccurrenceDefinitionPrefix 'occurrence' 'def' Definition
/// ```
///
/// `SysML` 8.2.2.9.1. One of the eight productions `simple_definition`
/// (`crates/sv2-syntax/src/parser/definition.rs`) builds with one method, differing from
/// [`PartDefinition`](crate::PartDefinition) only in the first keyword. 17 in the corpus,
/// under `PackageMember` (15) and `DefinitionMember` (2).
///
/// **Not every `OccurrenceDefinition` is one of these.** `individual def A;` is an
/// `IndividualDefinition` (8.2.2.9.1), whose metaclass is `OccurrenceDefinition` too, but
/// it is another production, writing no `occurrence` and no prefix node, and
/// `individual_definition` builds it as another node kind. This view is of the node, not
/// the metaclass, so `cast` refuses it.
///
/// **Both keywords are always there.** `at_simple_definition` begins an occurrence
/// definition only on `occurrence` followed by `def`, so `occurrence A def B;` is an
/// `OccurrenceUsage` and builds no occurrence definition at all
/// (`tests/malformed/occurrence-definition/def-missing.sysml`). A missing `def` is
/// therefore a missing node, never a node missing its keyword, and the keyword accessors
/// answer `None` on no tree this parser builds.
///
/// Both node children have views, so both are returned typed. The keywords are this node's
/// own tokens.
// node: OccurrenceDefinition
pub struct OccurrenceDefinition(SyntaxNode);

impl AstNode for OccurrenceDefinition {
    const KIND: SyntaxKind = SyntaxKind::OccurrenceDefinition;

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

impl OccurrenceDefinition {
    /// The `OccurrenceDefinitionPrefix`: `abstract` or `variation`, `individual`, and the
    /// user-defined keywords. Built even when nothing is written, so usually present and
    /// zero width (`OccurrenceDefinitionPrefix`).
    ///
    /// Read from the direct children. Reading descendants cannot be observed: the sweep
    /// asserts the prefix is the node's first element, so preorder reaches it first.
    pub fn prefix(&self) -> Option<OccurrenceDefinitionPrefix> {
        self.0.children().find_map(OccurrenceDefinitionPrefix::cast)
    }

    /// The `occurrence` keyword.
    pub fn occurrence_keyword(&self) -> Option<SyntaxToken> {
        self.token(SyntaxKind::KwOccurrence)
    }

    /// The `def` keyword: the one written straight after `occurrence`. A second `def` is
    /// not this node's. It is left in an `Error` beside the member, as a name
    /// (`tests/malformed/occurrence-definition/def-written-twice.sysml`).
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
    /// 'occurrence' 'def' Definition`, one of each. Searching descendant tokens is
    /// equivalent because it also asserts the prefix, the only element before the
    /// keywords, holds neither; an occurrence definition nested in the body comes after
    /// them in preorder. Taking the first own token whatever its kind is not equivalent,
    /// and is caught: it answers `occurrence` for `def`.
    fn token(&self, kind: SyntaxKind) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == kind)
    }
}
