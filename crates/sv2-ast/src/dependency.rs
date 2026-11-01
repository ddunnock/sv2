// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! A dependency: its prefix annotations, its keyword, its declaration or the parts the
//! declaration would hold, and its body.

use sv2_syntax::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

use crate::ast_node::AstNode;
use crate::qualified_name::QualifiedName;
use crate::relationship_body::RelationshipBody;

/// A dependency.
///
/// ```text
/// SysML:  Dependency = PrefixMetadataAnnotation* 'dependency'
///                      DependencyDeclaration RelationshipBody          (SysML 8.2.2.3)
/// KerML:  Dependency = PrefixMetadataAnnotation* 'dependency'
///                      ( Identification? 'from' )?
///                      client += [QualifiedName] ( ',' [QualifiedName] )* 'to'
///                      supplier += [QualifiedName] ( ',' [QualifiedName] )*
///                      RelationshipBody                                 (KerML 8.2.3.2)
/// ```
///
/// Built by `dependency` (`crates/sv2-syntax/src/parser/definition.rs`). **The two
/// languages build different trees from the same text** (ADR-0015). `SysML` names the
/// declaration as a production, so a `.sysml` dependency has a `DependencyDeclaration`
/// child holding the identification, `from`, the clients, `to` and the suppliers. `KerML`
/// writes the same parts inline, so a `.kerml` dependency owns them directly.
///
/// This view answers for its own children only, as every view in this crate does
/// (`.claude/rules/ast.md`). So [`declaration`](Self::declaration) answers in `SysML` and
/// is `None` in `KerML`, and the inline parts — [`identification`](Self::identification),
/// [`from_keyword`](Self::from_keyword), [`clients`](Self::clients),
/// [`to_keyword`](Self::to_keyword) and [`suppliers`](Self::suppliers) — answer in
/// `KerML` and are empty in `SysML`, where they are the declaration's. A caller holding a
/// `SysML` dependency asks the declaration, which has no view yet and is returned raw.
///
/// The body has a view and is returned typed, as are the clients and suppliers.
/// `PrefixMetadataAnnotation`, `DependencyDeclaration` and `Identification` have kinds of
/// their own and no views yet, so they are returned raw until they do. The keywords are
/// this node's own tokens.
// node: Dependency
pub struct Dependency(SyntaxNode);

impl AstNode for Dependency {
    const KIND: SyntaxKind = SyntaxKind::Dependency;

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

impl Dependency {
    /// The `PrefixMetadataAnnotation`s written before `dependency`, in source order: each
    /// a `#` and the metadata it names (`#refinement dependency ...`). Usually none.
    ///
    /// Raw `SyntaxNode`s of kind `PrefixMetadataAnnotation`, because that kind has no view
    /// yet.
    pub fn prefix_annotations(&self) -> impl Iterator<Item = SyntaxNode> + '_ {
        self.0
            .children()
            .filter(|child| child.kind() == SyntaxKind::PrefixMetadataAnnotation)
    }

    /// The `dependency` keyword.
    pub fn dependency_keyword(&self) -> Option<SyntaxToken> {
        self.token(SyntaxKind::KwDependency)
    }

    /// The `DependencyDeclaration`: in `SysML` only, where it holds the identification,
    /// `from`, the clients, `to` and the suppliers. `None` in `KerML`, which writes those
    /// inline.
    ///
    /// A raw `SyntaxNode` of kind `DependencyDeclaration`, because that kind has no view
    /// yet.
    pub fn declaration(&self) -> Option<SyntaxNode> {
        self.child(SyntaxKind::DependencyDeclaration)
    }

    /// The `Identification` before `from`: in `KerML` only, and only when the `( ... 'from'
    /// )?` group is written. Present and zero width for `dependency from a to b;`, which
    /// names nothing but writes `from`.
    ///
    /// A raw `SyntaxNode` of kind `Identification`, because that kind has no view yet.
    pub fn identification(&self) -> Option<SyntaxNode> {
        self.child(SyntaxKind::Identification)
    }

    /// The `from` keyword, in `KerML` only, and only when the group is written.
    pub fn from_keyword(&self) -> Option<SyntaxToken> {
        self.token(SyntaxKind::KwFrom)
    }

    /// The clients: the names before `to`, in source order. In `KerML` only.
    ///
    /// Split from the suppliers by the two lists the parser read, not by where `to` is:
    /// without a `to`, `dependency a b;` still reads `a` as a client and `b` as a supplier
    /// (`tests/malformed/dependency/to-missing.kerml`).
    pub fn clients(&self) -> impl Iterator<Item = QualifiedName> {
        self.lists().0.into_iter()
    }

    /// The `to` keyword, in `KerML` only. `None` when it was not written, which the parser
    /// reports.
    pub fn to_keyword(&self) -> Option<SyntaxToken> {
        self.token(SyntaxKind::KwTo)
    }

    /// The suppliers: the names after `to`, in source order. In `KerML` only. Split as
    /// [`clients`](Self::clients) says.
    pub fn suppliers(&self) -> impl Iterator<Item = QualifiedName> {
        self.lists().1.into_iter()
    }

    /// The `RelationshipBody`: `;`, or `{` and its annotations (and, in `KerML`, owned
    /// related elements) and `}`.
    pub fn body(&self) -> Option<RelationshipBody> {
        self.child(SyntaxKind::RelationshipBody)
            .and_then(RelationshipBody::cast)
    }

    /// This node's own names, split into the client list and the supplier list.
    ///
    /// The grammar writes each list as `[QualifiedName] ( ',' [QualifiedName] )*`, and
    /// `dependency_declaration_parts` (`crates/sv2-syntax/src/parser/definition.rs`) reads
    /// one, expects `to`, and reads the other. So the client list is the run of names
    /// joined by commas that the node's names begin with, and the supplier list is every
    /// name after it -- whether or not a `to` stands between them.
    ///
    /// The elements to walk are this node's own children and tokens, trivia aside:
    /// `QualifiedName` nodes, `Comma` and `KwTo` tokens, and the rest (the keyword, any
    /// annotations, the identification, `from`, the body), which are no part of either
    /// list.
    ///
    /// Two plausible alternatives are wrong, and each is caught by a case written for it.
    /// Splitting at `to` makes `b` a client in `dependency a b;`
    /// (`tests/malformed/dependency/to-missing.kerml`). Ending the client list after its
    /// first name passes every tree in the repository, none of which writes two `KerML`
    /// clients, and is caught by the hand-written `dependency a, b to c, d;` in
    /// `tests/positive.rs`.
    fn lists(&self) -> (Vec<QualifiedName>, Vec<QualifiedName>) {
        let (mut clients, mut suppliers) = (Vec::new(), Vec::new());
        let mut after_comma = false;
        for element in self.0.children_with_tokens() {
            if element.kind() == SyntaxKind::Comma {
                after_comma = true;
                continue;
            }
            let Some(name) = element.into_node().and_then(QualifiedName::cast) else {
                continue;
            };
            // A name not joined to the one before it by a comma begins the next list, so
            // once any client is read, it begins the suppliers.
            let list = if suppliers.is_empty() && (clients.is_empty() || after_comma) {
                &mut clients
            } else {
                &mut suppliers
            };
            list.push(name);
            after_comma = false;
        }
        (clients, suppliers)
    }

    /// The first direct child of `kind`.
    ///
    /// Taking the last instead cannot be observed: `tests/sweep.rs` asserts a dependency
    /// holds at most one declaration, identification and body of its own. Searching
    /// descendants is not equivalent, and is caught: a `SysML` declaration holds an
    /// identification that is not this node's.
    fn child(&self, kind: SyntaxKind) -> Option<SyntaxNode> {
        self.0.children().find(|child| child.kind() == kind)
    }

    /// The first of this node's own tokens of `kind`.
    ///
    /// Taking the last instead cannot be observed: `tests/sweep.rs` asserts a dependency
    /// holds at most one `dependency`, `from` and `to` of its own. Searching descendant
    /// tokens is not equivalent, and is caught: a `SysML` declaration holds a `from` and a
    /// `to` that are not this node's.
    fn token(&self, kind: SyntaxKind) -> Option<SyntaxToken> {
        self.0
            .children_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(|token| token.kind() == kind)
    }
}
