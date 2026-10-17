// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Well-formed cases: one read from the pinned corpus, one written to isolate a
//! distinction the corpus does not happen to contain.
//!
//! Expectations come from the corpus file's own text or from a cited clause, never from
//! what an accessor printed (`.claude/rules/tests.md`). The sweep already asserts every
//! accessor's differential over all 610 trees; what a case here adds is a named,
//! human-checkable statement, so a defect reads as "`PrivateImportTest` has four imports
//! and we found three" rather than as a failure at tree 412.
//!
//! A hand-written case earns its place only when no pinned file distinguishes the two
//! behaviours. `an_import_does_not_claim_an_all_from_its_filter_expression` is here
//! because mutating `is_import_all` to read descendants instead of direct children
//! passed the whole sweep: nothing in 610 trees writes `all` below an import without
//! writing one above it, so the corpus cannot tell a correct accessor from that one.

mod trees;

use sv2_ast::{AstNode, Import, PackageMember};
use sv2_syntax::{SyntaxKind, SyntaxNode};

/// The file the frozen grammar unit cites as evidence for the optional `all`
/// (`.claude/state/grammar/units/Import.json`, evidence[2]).
const PRIVATE_IMPORT_TEST: &str = "vendor/corpus/sysml/src/examples/Import Tests/\
                                   PrivateImportTest.sysml";

/// Eight lines holding a `PackageMember` beside the two other memberships that stand at
/// the same position in a package body, which is what makes it useful.
const PACKAGE_EXAMPLE: &str = "vendor/corpus/sysml/src/training/01. Packages/\
                               Package Example.sysml";

#[expect(
    clippy::panic,
    reason = "test support only; an unreadable pinned corpus file means this test \
              cannot run at all, and saying which file is the whole diagnosis"
)]
fn tree_of(relative: &str) -> SyntaxNode {
    let path = trees::workspace().join(relative);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("the pinned corpus file {relative} must be readable: {e}"));
    trees::tree(&source, trees::language_of(&path))
}

/// `PrivateImportTest.sysml` writes four imports, and exactly one of them is an
/// `import all`.
///
/// Read off the file, in source order:
///
/// ```text
///   package P2 { private import P1::*; }        private, not all
///   public import P2::*;                        public,  not all
///   private import P3::*;                       private, not all
///   package P4 { public import all P2::*; }     public,  ALL
/// ```
///
/// This is the shape the clause states — `visibility = VisibilityIndicator 'import'
/// ( isImportAll ?= 'all' )? ImportDeclaration RelationshipBody`, `SysML` 8.2.2.5.1
/// and `KerML` 8.2.3.4.2 — with the visibility written every time, which makes it
/// evidence that the visibility is mandatory rather than merely common.
#[test]
fn private_import_test_writes_four_imports_one_of_them_import_all() {
    let root = tree_of(PRIVATE_IMPORT_TEST);
    let imports: Vec<Import> = root.descendants().filter_map(Import::cast).collect();

    assert_eq!(imports.len(), 4, "the file writes four imports");

    let visibilities: Vec<String> = imports
        .iter()
        .map(|import| {
            import
                .visibility()
                .map_or_else(|| "<none>".to_owned(), |node| node.text().to_string())
        })
        .collect();
    assert_eq!(
        visibilities,
        ["private", "public", "private", "public"],
        "the visibilities, in source order, as the file writes them"
    );

    let all: Vec<bool> = imports.iter().map(Import::is_import_all).collect();
    assert_eq!(
        all,
        [false, false, false, true],
        "only the import in package P4 is written `import all`"
    );

    // Every import here is well formed, so every part the production states is present
    // and carries text. This is the half the malformed case cannot assert: there, the
    // declaration is present and empty, and a test that only ever saw that state would
    // not notice an accessor which returned an empty node for every import in the repository.
    for (index, import) in imports.iter().enumerate() {
        let declaration = import
            .declaration()
            .unwrap_or_else(|| panic!("import {index} must have a declaration"));
        assert_eq!(declaration.kind(), SyntaxKind::ImportDeclaration);
        assert!(
            declaration.text_range().len() > 0.into(),
            "import {index} names something, so its declaration cannot be zero width"
        );
        let body = import
            .body()
            .unwrap_or_else(|| panic!("import {index} must have a relationship body"));
        assert_eq!(body.kind(), SyntaxKind::RelationshipBody);
        assert_eq!(
            body.text().to_string(),
            ";",
            "every import in this file closes with a semicolon rather than a braced body"
        );
    }
}

/// An import's `all` is its own, and an `all` written inside its filter expression is
/// not it.
///
/// Two clauses, two different `all`s, and the accessor must not confuse them:
///
/// - `Import = visibility = VisibilityIndicator 'import' ( isImportAll ?= 'all' )?
///   ImportDeclaration RelationshipBody` — `SysML` 8.2.2.5.1, `KerML` 8.2.3.4.2. The
///   `all` that sets `isImportAll` stands between `import` and the declaration;
/// - `ExtentExpression : OperatorExpression = operator = 'all'
///   ownedRelationship += TypeReferenceMember` — `KerML` 8.2.5.8.1. Here `all` is an
///   operator inside an expression, and a `FilterPackageMember` is
///   `'[' ownedRelatedElement += OwnedExpression ']'` (`SysML` 8.2.2.5.1), so such an
///   expression is reachable **inside** an import's own declaration.
///
/// `public import A::*[all B];` therefore writes exactly one `all`, and it belongs to the
/// extent expression, not to the import. `isImportAll` is false. Reading the import's
/// descendants rather than its direct children inverts that, and the pinned corpus
/// cannot catch it, because no file there writes this shape.
#[test]
fn an_import_does_not_claim_an_all_from_its_filter_expression() {
    const FILTERED: &str = "public import A::*[all B];";

    let parse = trees::parsed(FILTERED, sv2_syntax::Language::SysMl);
    assert!(
        parse.errors().is_empty(),
        "the clauses admit this text, so a diagnostic here is the parser disagreeing \
         with them, not the test being wrong: {:?}",
        parse.errors()
    );
    let root = parse.syntax();

    let imports: Vec<Import> = root.descendants().filter_map(Import::cast).collect();
    assert_eq!(imports.len(), 1, "one import is written");
    let import = &imports[0];

    // The premise: there IS an `all` inside this import, so the test is not vacuous. If
    // the parser ever stopped putting the extent expression under the import, this would
    // fail and say so, rather than the case silently proving nothing.
    let all_below = import
        .syntax()
        .descendants_with_tokens()
        .filter_map(sv2_syntax::SyntaxElement::into_token)
        .any(|token| token.kind() == SyntaxKind::KwAll);
    assert!(
        all_below,
        "the extent expression's `all` must be inside the import, or this case cannot \
         distinguish direct children from descendants"
    );

    // And the claim: it is not the import's own.
    assert!(
        !import.is_import_all(),
        "the `all` here is the extent expression's operator (KerML 8.2.5.8.1), not the \
         import's isImportAll (SysML 8.2.2.5.1)"
    );
}

/// `Package Example.sysml` writes two package members, and four things that are not
/// package members although they sit in the same position and share its prefix shape.
///
/// Read off the file, which is eight lines whole:
///
/// ```text
///   package 'Package Example' {            PackageMember, no visibility
///       public import ISQ::TorqueValue;    Import
///       private import ScalarValues::*;    Import
///       private part def Automobile;       PackageMember, private
///       public alias Car for Automobile;   AliasMember
///       alias Torque for ISQ::TorqueValue; AliasMember
///   }
/// ```
///
/// `PackageBodyElement` admits all three kinds (`SysML` 8.2.2.5.1), and `Import`,
/// `AliasMember` and `PackageMember` each open with a `MemberPrefix` or a visibility — so
/// a cast that went by shape rather than by kind would collect six "members" here instead
/// of two. That is what this file is for, and no hand-written input would do it as well:
/// these are the real neighbours, in a real file, in the order a real author wrote them.
///
/// It also pins both prefix states against one another. The outer package is written with
/// no visibility, so its prefix is present and **zero width**; `Automobile` is written
/// `private`, so its prefix carries a `VisibilityIndicator`. A test that saw only one of
/// those would not notice an accessor that always returned the same thing.
#[test]
fn package_example_writes_two_members_beside_other_memberships() {
    let root = tree_of(PACKAGE_EXAMPLE);
    let members: Vec<PackageMember> = root.descendants().filter_map(PackageMember::cast).collect();

    assert_eq!(
        members.len(),
        2,
        "the outer package and `private part def Automobile;` are the package members; \
         the imports and aliases are not, although they stand in the same place"
    );

    // The elements, by the kind each alternation resolved to. `DefinitionElement` and
    // `UsageElement` build no node, so these are the concrete elements.
    let elements: Vec<SyntaxKind> = members
        .iter()
        .map(|member| {
            member
                .element()
                .map_or(SyntaxKind::Error, |element| element.kind())
        })
        .collect();
    assert_eq!(
        elements,
        [SyntaxKind::Package, SyntaxKind::PartDefinition],
        "the outer member owns the package; the inner one owns the part definition"
    );

    // Both prefix states, in one file: empty for the package, `private` for the part
    // definition. The node is present in both cases, by deliberate choice in the parser.
    let prefixes: Vec<String> = members
        .iter()
        .map(|member| {
            member
                .prefix()
                .map_or_else(|| "<none>".to_owned(), |node| node.text().to_string())
        })
        .collect();
    assert_eq!(
        prefixes,
        ["", "private"],
        "a member written without a visibility has an empty prefix, not an absent one"
    );

    // And the neighbours really are there, so the count above is a discrimination rather
    // than an accident of a file that happens to hold two of something.
    let kinds: Vec<SyntaxKind> = root.descendants().map(|node| node.kind()).collect();
    let count = |want: SyntaxKind| kinds.iter().filter(|kind| **kind == want).count();
    assert_eq!(count(SyntaxKind::Import), 2, "two imports are written");
    assert_eq!(count(SyntaxKind::AliasMember), 2, "two aliases are written");
}
