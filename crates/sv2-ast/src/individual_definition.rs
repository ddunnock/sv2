// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! An individual definition: `individual def`, its prefix slots, its definition and its
//! empty multiplicity.

use sv2_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

use crate::ast_node::AstNode;
use crate::definition::Definition;

/// An individual definition.
///
/// ```text
/// IndividualDefinition : OccurrenceDefinition =
///     BasicDefinitionPrefix? isIndividual ?= 'individual'
///     DefinitionExtensionKeyword* 'def' Definition
///     ownedRelationship += EmptyMultiplicityMember
/// ```
///
/// `SysML` 8.2.2.9.1: "individual may be used in place of the kind keyword, in which case
/// the declaration is equivalent to individual occurrence" (7.9.4, receipt 8c84370d).
/// Built by `individual_definition` (`crates/sv2-syntax/src/parser/definition.rs`). 31 in
/// the corpus, all under `PackageMember`.
///
/// **Its prefix has no node.** The metaclass is `OccurrenceDefinition`, but the slots an
/// [`OccurrenceDefinitionPrefix`](crate::OccurrenceDefinitionPrefix) would hold are this
/// node's own children, written inline, so they are read here, with that prefix's names.
/// An occurrence definition whose prefix is `individual`, `individual occurrence def X`,
/// is an [`OccurrenceDefinition`](crate::OccurrenceDefinition), and `cast` refuses it.
///
/// **`individual` and `def` are always there.** `at_individual_definition` begins one only
/// on `individual` followed, past its `#` keywords, by `def`. `individual abstract def A;`
/// begins none and is an `Error`
/// (`tests/malformed/individual-definition/basic-prefix-after-individual.sysml`), so a
/// misplaced word is a missing node, and those two accessors answer `None` on no tree this
/// parser builds.
///
/// `Definition` has a view and is returned typed. The basic prefix, the extension keywords
/// and the empty multiplicity have kinds of their own and no views yet, so they are
/// returned raw until they do. The keywords are this node's own tokens.
// node: IndividualDefinition
pub struct IndividualDefinition(SyntaxNode);

impl AstNode for IndividualDefinition {
    const KIND: SyntaxKind = SyntaxKind::IndividualDefinition;

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

impl IndividualDefinition {
    /// The `BasicDefinitionPrefix`, `abstract` or `variation`, when one is written.
    ///
    /// A raw `SyntaxNode` of kind `BasicDefinitionPrefix`, because that kind has no view
    /// yet. Which keyword it is belongs to that node: ask it for its token.
    pub fn basic_prefix(&self) -> Option<SyntaxNode> {
        self.child(SyntaxKind::BasicDefinitionPrefix)
    }

    /// The `individual` keyword: `isIndividual`, always true of this production.
    pub fn individual_keyword(&self) -> Option<SyntaxToken> {
        self.token(SyntaxKind::KwIndividual)
    }

    /// Each `DefinitionExtensionKeyword` written between `individual` and `def`, a `#X`
    /// user-defined keyword, in source order.
    ///
    /// Raw nodes of kind `DefinitionExtensionKeyword`, because that kind has no view yet.
    /// Each holds the `PrefixMetadataMember` that names the metadata. No corpus file writes
    /// one here, so `tests/positive.rs` writes two and pins their order.
    ///
    /// Read from the direct children. Reading descendants would also yield the keywords of
    /// a definition nested in the body, and `tests/positive.rs` writes one that would.
    pub fn extension_keywords(&self) -> impl Iterator<Item = SyntaxNode> + '_ {
        self.0
            .children()
            .filter(|child| child.kind() == SyntaxKind::DefinitionExtensionKeyword)
    }

    /// The `def` keyword: the one written after `individual` and its `#` keywords. A
    /// second `def` is not this node's. It is left in an `Error` beside the member, as a
    /// name (`tests/malformed/individual-definition/def-written-twice.sysml`).
    pub fn def_keyword(&self) -> Option<SyntaxToken> {
        self.token(SyntaxKind::KwDef)
    }

    /// The `Definition`: the declaration and the body.
    ///
    /// Read from the direct children. Two mutations cannot be observed, each under a
    /// `tests/sweep.rs` assertion. Taking the last is equivalent because the node has one.
    /// Reading descendants is equivalent because the sweep asserts no prefix slot holds a
    /// `Definition`, and the slots and keywords are all that precede it in preorder.
    pub fn definition(&self) -> Option<Definition> {
        self.0.children().find_map(Definition::cast)
    }

    /// The `EmptyMultiplicityMember`: `ownedRelationship += EmptyMultiplicityMember`, last,
    /// where the clause writes it. The sweep asserts it is on every individual definition.
    ///
    /// It spans no text. It is the element the abstract syntax says an individual
    /// definition owns (`empty_multiplicity_member`,
    /// `crates/sv2-syntax/src/parser/multiplicity.rs`), so it is returned as a node, raw,
    /// because that kind has no view yet.
    pub fn multiplicity(&self) -> Option<SyntaxNode> {
        self.child(SyntaxKind::EmptyMultiplicityMember)
    }

    /// The first direct child of `kind`. First, not only: the production states at most
    /// one of each, and `tests/sweep.rs` asserts the node's elements are in the
    /// production's order with one of each, which is what makes taking the last an
    /// equivalent mutation. Searching descendants is NOT equivalent: an absent basic prefix
    /// would be answered by one in the body, and the multiplicity, last, comes after the
    /// body in preorder. `tests/positive.rs` nests a definition that catches both.
    fn child(&self, kind: SyntaxKind) -> Option<SyntaxNode> {
        self.0.children().find(|child| child.kind() == kind)
    }

    /// The first of this node's own tokens of `kind`.
    ///
    /// Two mutations of this cannot be observed, each under a `tests/sweep.rs`
    /// assertion: taking the last token of the kind, because the node has one of each;
    /// and searching descendant tokens, because nothing beneath a prefix slot is either
    /// keyword, and an individual definition nested in the body comes after both in
    /// preorder. Taking the first own token whatever its kind is not
    /// equivalent, and is caught: it answers `individual` for `def`.
    fn token(&self, kind: SyntaxKind) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == kind)
    }
}
