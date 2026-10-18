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

use sv2_ast::{AliasMember, AstNode, ElementFilterMember, Import, MemberPrefix, PackageMember};
use sv2_syntax::{SyntaxKind, SyntaxNode};

/// The file the frozen grammar unit cites as evidence for the optional `all`
/// (`.claude/state/grammar/units/Import.json`, evidence[2]).
const PRIVATE_IMPORT_TEST: &str = "vendor/corpus/sysml/src/examples/Import Tests/\
                                   PrivateImportTest.sysml";

/// Eight lines holding a `PackageMember` beside the two other memberships that stand at
/// the same position in a package body, which is what makes it useful.
const PACKAGE_EXAMPLE: &str = "vendor/corpus/sysml/src/training/01. Packages/\
                               Package Example.sysml";

/// Twenty-two lines carrying all three visibility keywords, under several different
/// membership productions.
const ITEM_TEST: &str = "vendor/corpus/sysml/src/examples/Simple Tests/ItemTest.sysml";

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
                .and_then(|visibility| visibility.keyword())
                .map_or_else(|| "<none>".to_owned(), |token| token.text().to_string())
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
            member.prefix().map_or_else(
                || "<none>".to_owned(),
                |prefix| prefix.syntax().text().to_string(),
            )
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

/// The two aliases `Package Example.sysml` writes, one with a visibility and one without,
/// one target qualified and one not.
///
/// ```text
///   public alias Car for Automobile;      public,  target Automobile
///   alias Torque for ISQ::TorqueValue;    no visibility, target ISQ::TorqueValue
/// ```
///
/// Both name slots behave the same way in the pinned corpus — every alias in all 311
/// files writes a member name and no short name — so this file pins the ordinary shape
/// and `an_alias_short_name_is_the_one_inside_the_brackets` pins the rest.
#[test]
fn package_example_writes_two_aliases_one_qualified() {
    let root = tree_of(PACKAGE_EXAMPLE);
    let aliases: Vec<AliasMember> = root.descendants().filter_map(AliasMember::cast).collect();

    assert_eq!(aliases.len(), 2, "the file writes two aliases");

    let named: Vec<String> = aliases
        .iter()
        .map(|alias| {
            alias
                .name()
                .map_or_else(|| "<none>".to_owned(), |token| token.text().to_owned())
        })
        .collect();
    assert_eq!(
        named,
        ["Car", "Torque"],
        "the member names, in source order"
    );

    assert!(
        aliases.iter().all(|alias| alias.short_name().is_none()),
        "no alias in the pinned corpus writes a short name"
    );

    let targets: Vec<String> = aliases
        .iter()
        .map(|alias| {
            alias.target().map_or_else(
                || "<none>".to_owned(),
                |target| target.syntax().text().to_string(),
            )
        })
        .collect();
    assert_eq!(
        targets,
        ["Automobile", "ISQ::TorqueValue"],
        "the second target is qualified, so a target accessor that stopped at the first \
         name would read `ISQ` and not notice"
    );

    let prefixes: Vec<String> = aliases
        .iter()
        .map(|alias| {
            alias.prefix().map_or_else(
                || "<none>".to_owned(),
                |prefix| prefix.syntax().text().to_string(),
            )
        })
        .collect();
    assert_eq!(
        prefixes,
        ["public", ""],
        "one is written public, one is bare"
    );
}

/// The short name is the one inside the brackets, and neither slot stands in for the
/// other when it is empty.
///
/// Hand written, and it has to be: the short-name slot is **not exercised by the pinned
/// corpus at all**. The only two occurrences of `alias <` in 311 files are inside `//`
/// comments (`SimpleVehicleModel.sysml:458`, `// with alias <fm>`), which the parser in
/// this project reads as trivia, not as a short name. The parser says so at
/// `alias_member` too. So no corpus file can tell a correct accessor from one that
/// confuses the two slots, and the expectation comes from the clause instead:
/// `'alias' ( '<' memberShortName = NAME '>' )? ( memberName = NAME )?`, `SysML`
/// 8.2.2.5.1. Every input here parses with no diagnostic — the damaged spellings of the
/// same slot are pinned in `tests/malformed/alias-member/`.
#[test]
fn an_alias_short_name_is_the_one_inside_the_brackets() {
    // (source, short name, member name) — read straight off the production.
    let cases = [
        ("alias <c> Car for Automobile;", Some("c"), Some("Car")),
        ("alias <c> for Automobile;", Some("c"), None),
        ("alias Car for Automobile;", None, Some("Car")),
        // Both slots are optional and both are 0..1 on the metaclass, so this is well
        // formed as far as the syntax goes — the parser's own note says as much.
        ("alias for Automobile;", None, None),
    ];

    for (source, short, name) in cases {
        let parse = trees::parsed(source, sv2_syntax::Language::SysMl);
        assert!(
            parse.errors().is_empty(),
            "the clause admits {source:?}, so a diagnostic here is the parser \
             disagreeing with it, not the test being wrong: {:?}",
            parse.errors()
        );
        let root = parse.syntax();
        let aliases: Vec<AliasMember> = root.descendants().filter_map(AliasMember::cast).collect();
        assert_eq!(aliases.len(), 1, "{source:?} writes one alias");
        let alias = &aliases[0];

        assert_eq!(
            alias.short_name().map(|token| token.text().to_owned()),
            short.map(str::to_owned),
            "short name of {source:?}"
        );
        assert_eq!(
            alias.name().map(|token| token.text().to_owned()),
            name.map(str::to_owned),
            "member name of {source:?}"
        );
    }
}

/// Every visibility keyword the language has, across four different membership
/// productions, in one twenty-two-line file.
///
/// `ItemTest.sysml`, read in source order — the non-empty prefixes only:
///
/// ```text
///   public item def A {            public
///       protected ref part c: C;   protected
///   abstract item def B {
///       public abstract part a: A; public
///   private part def C {           private
///       private in ref y: A, B;    private
/// ```
///
/// This is the file that shows `MemberPrefix` is **shared**, which is the whole reason it
/// was taken out of worklist order: its parents here are `PackageMember`,
/// `OccurrenceUsageMember` and `NonOccurrenceUsageMember`, so a visibility accessor bolted
/// onto `PackageMember` alone would have answered for one of three callers and left the
/// others to reach into the node themselves.
#[test]
fn item_test_writes_every_visibility_across_several_memberships() {
    let root = tree_of(ITEM_TEST);
    let prefixes: Vec<MemberPrefix> = root.descendants().filter_map(MemberPrefix::cast).collect();

    let written: Vec<String> = prefixes
        .iter()
        .filter_map(MemberPrefix::visibility)
        .filter_map(|visibility| visibility.keyword())
        .map(|token| token.text().to_string())
        .collect();
    assert_eq!(
        written,
        ["public", "protected", "public", "private", "private"],
        "the visibilities this file writes, in source order"
    );

    // Most members carry none, and that must read as none rather than as damage.
    assert!(
        prefixes.len() > written.len(),
        "this file also writes members with no visibility, and they must still have a \
         prefix node: {} prefixes, {} of them written",
        prefixes.len(),
        written.len()
    );

    // The visibility is a VisibilityIndicator node, not a bare keyword token: that
    // production has its own kind, and now its own view, which `visibility()` returns.
    assert!(
        prefixes
            .iter()
            .filter_map(MemberPrefix::visibility)
            .all(|visibility| visibility.syntax().kind() == SyntaxKind::VisibilityIndicator),
        "every visibility comes back as its own node"
    );

    // Shared across membership productions, which is the claim that justified the
    // reordering. Asserted as a set, so adding a membership kind to the parser does not
    // break it, but dropping below two would.
    let mut parents: Vec<SyntaxKind> = prefixes
        .iter()
        .filter_map(|prefix| prefix.syntax().parent())
        .map(|parent| parent.kind())
        .collect();
    parents.sort_unstable();
    parents.dedup();
    assert!(
        parents.len() >= 2,
        "MemberPrefix must appear under more than one membership production here, or the \
         case does not show what it claims: {parents:?}"
    );
    assert!(
        parents.contains(&SyntaxKind::PackageMember),
        "PackageMember is one of them: {parents:?}"
    );
}

/// The training file for filtering: two filter members, one condition each, and the two
/// conditions are of DIFFERENT expression kinds — which is the whole reason `filter()`
/// returns a raw node.
const FILTERING_EXAMPLE: &str = "vendor/corpus/sysml/src/training/40. Filtering/\
                                 Filtering Example-1.sysml";

/// `Filtering Example-1.sysml`, whose two filters are written (lines 29 and 35):
///
/// ```text
///   filter @Safety;
///   filter @Safety and (as Safety).isMandatory;
/// ```
///
/// Read off the production, `MemberPrefix 'filter' OwnedExpression ';'` (`SysML`
/// 8.2.2.5.1, `KerML` 8.2.5.13): each member has one condition, and its text is the
/// text between the keyword and the semicolon.
///
/// What this case says that the sweep cannot is that the two conditions are **different
/// kinds of node**. `@Safety` alone is a `ClassificationExpression` — the operand of a
/// classification test is optional, which is what lets `@Safety` stand by itself inside
/// a filter (`KerML` 8.2.5.8.1, and the frozen unit's own decision says so). Put `and`
/// between two of them and the top of the expression is the conditional operator
/// instead. `OwnedExpression` builds no node of its own, so what arrives in the slot is
/// whichever concrete expression matched, and an accessor that promised one type would
/// be wrong on the next corpus file.
#[test]
fn the_two_filters_in_one_file_hold_conditions_of_different_kinds() {
    let root = tree_of(FILTERING_EXAMPLE);
    let filters: Vec<ElementFilterMember> = root
        .descendants()
        .filter_map(ElementFilterMember::cast)
        .collect();
    assert_eq!(filters.len(), 2, "this file writes two filters");

    // The conditions, as the file writes them.
    let conditions: Vec<String> = filters
        .iter()
        .filter_map(ElementFilterMember::filter)
        .map(|node| node.text().to_string())
        .collect();
    assert_eq!(
        conditions,
        ["@Safety", "@Safety and (as Safety).isMandatory"],
        "each filter's condition is the text between `filter` and `;`"
    );

    // And they are different kinds, which is the claim. Asserted as "not equal" and then
    // by name: the inequality is the part that stays true as the expression layer grows,
    // the names are what make a failure readable.
    let kinds: Vec<SyntaxKind> = filters
        .iter()
        .filter_map(ElementFilterMember::filter)
        .map(|node| node.kind())
        .collect();
    assert_ne!(
        kinds[0], kinds[1],
        "the two conditions must not be the same kind, or this case shows nothing: \
         {kinds:?}"
    );
    assert_eq!(
        kinds[0],
        SyntaxKind::ClassificationExpression,
        "`@Safety` standing alone is a classification test with no operand \
         (KerML 8.2.5.8.1)"
    );
    assert_eq!(
        kinds[1],
        SyntaxKind::ConditionalBinaryOperatorExpression,
        "`and` is the conditional operator, so it is the top of the second condition \
         (KerML 8.2.5.8.1, tier table row for `and`)"
    );

    // Neither filter is written a visibility here, and that must read as none rather
    // than as damage: the prefix node is still built (`member_prefix`,
    // crates/sv2-syntax/src/parser/namespace.rs).
    for filter in &filters {
        let prefix = filter.prefix().expect("the prefix node is always built");
        assert!(
            prefix.visibility().is_none(),
            "neither filter in this file is written a visibility"
        );
    }
}

/// The corpus file that tells the global scope qualifier apart from a package NAMED `$`.
/// `KerML`, because the qualifier is stated in `KerML` 8.2.3.4.1.
const SCOPING: &str = "vendor/corpus/kerml/src/examples/Simple Tests/Scoping.kerml";

/// `Scoping.kerml` writes a package named `'$'` (line 21) and then refers both to it and
/// to the global scope — which is the one distinction this accessor can get wrong.
///
/// ```text
///   line 32:  class D :> '$'::Objects::Object   a package NAMED `$`: three segments
///   line 35:  class E :>  $ ::Objects::Object   the QUALIFIER: two segments, global
/// ```
///
/// `KerML` 8.2.3.4.1, Note 3: a qualified name is "a sequence of segment names separated
/// by `::` punctuation, optionally with the global scope qualifier `$` as an initial
/// segment", and "an unqualified name can be considered the degenerate case of a
/// qualified name with a single segment name". So the quoted `'$'` is a segment name like
/// any other and the bare `$` is not a segment at all — it is punctuation about where
/// resolution starts.
///
/// The two spellings differ by two characters of quoting and by nothing else, which is
/// why no hand-written case is needed here: the pinned corpus already contains the pair,
/// and the file exists to exercise exactly this.
///
/// Every qualified name in the file, in source order, read off the text above and the
/// production `( '$' '::' )? ( NAME '::' )* NAME`.
#[test]
fn scoping_tells_a_package_named_dollar_from_the_global_qualifier() {
    let root = tree_of(SCOPING);
    let names: Vec<sv2_ast::QualifiedName> = root
        .descendants()
        .filter_map(sv2_ast::QualifiedName::cast)
        .collect();

    let segments: Vec<Vec<String>> = names
        .iter()
        .map(|name| {
            name.segments()
                .map(|token| token.text().to_owned())
                .collect()
        })
        .collect();
    assert_eq!(
        segments,
        [
            vec!["A"],                        // line 11, `class B :> A`
            vec!["g"],                        // line 12, `feature :>> g`
            vec!["Objects", "Object"],        // line 29
            vec!["test1"],                    // line 30
            vec!["'$'", "Objects", "Object"], // line 32, a package NAMED `$`
            vec!["test2"],                    // line 33
            vec!["Objects", "Object"],        // line 35, the QUALIFIER, not a segment
            vec!["subobjects"],               // line 36
        ],
        "every qualified name this file writes, in source order"
    );

    // Exactly one of them is written with the qualifier, and it is the one on line 35.
    let global: Vec<bool> = names
        .iter()
        .map(sv2_ast::QualifiedName::is_global)
        .collect();
    assert_eq!(
        global,
        [false, false, false, false, false, false, true, false],
        "only line 35 writes the bare `$`"
    );

    // The pair that makes this file worth reading: same trailing segments, and the
    // quoted one is NOT global. An accessor testing the node's text for a leading `$`
    // would call both of them global.
    let quoted = &names[4];
    let qualified = &names[6];
    assert!(
        !quoted.is_global(),
        "`'$'` is a segment name, not the qualifier (8.2.3.4.1 Note 3)"
    );
    assert!(qualified.is_global(), "`$::` is the qualifier");
    assert_eq!(
        quoted.segments().count(),
        qualified.segments().count() + 1,
        "the quoted spelling has one more segment, which is the `$` itself"
    );

    // And the final segment is the same name in both: what differs is where resolution
    // starts, not what is being identified.
    assert_eq!(
        quoted.name().map(|token| token.text().to_owned()),
        qualified.name().map(|token| token.text().to_owned()),
        "both identify a final segment called `Object`"
    );
    assert_eq!(
        qualified.name().map(|token| token.text().to_owned()),
        Some("Object".to_owned()),
        "the last segment written is the one the name identifies"
    );
}

/// All three visibility keywords come back as the token kind that names them, under both
/// of the productions that carry an indicator.
///
/// `VisibilityIndicator = 'public' | 'private' | 'protected'` (`KerML` 8.2.3.4.1, `SysML`
/// 8.2.2.5.1). `ItemTest.sysml` writes `public`, `protected` and `private`, which is all
/// three, and it writes them under `MemberPrefix`; `PrivateImportTest.sysml` writes them
/// under `Import`. Both files are already read above for other claims.
///
/// What this case adds is that the accessor hands back the **token**, so a caller tells
/// the three apart by `kind()` and never by comparing text. That is the whole reason it
/// does not return an enum: the frozen unit
/// (`.claude/state/grammar/units/VisibilityIndicator.json`) records that the Xtext's
/// `enum VisibilityIndicator returns SysML::VisibilityKind` binding "is abstract-syntax
/// assignment, not concrete syntax, and it is not carried across: what parses is the
/// keyword, and which enum literal it becomes belongs to whoever builds the element".
/// Constructing a metaclass is not this crate's work (`.claude/rules/ast.md`).
#[test]
fn every_visibility_keyword_comes_back_as_its_own_token_kind() {
    let root = tree_of(ITEM_TEST);
    let indicators: Vec<sv2_ast::VisibilityIndicator> = root
        .descendants()
        .filter_map(sv2_ast::VisibilityIndicator::cast)
        .collect();

    let kinds: Vec<SyntaxKind> = indicators
        .iter()
        .filter_map(sv2_ast::VisibilityIndicator::keyword)
        .map(|token| token.kind())
        .collect();
    assert_eq!(
        kinds,
        [
            SyntaxKind::KwPublic,    // public item def A
            SyntaxKind::KwProtected, // protected ref part c: C
            SyntaxKind::KwPublic,    // public abstract part a: A
            SyntaxKind::KwPrivate,   // private part def C
            SyntaxKind::KwPrivate,   // private in ref y: A, B
        ],
        "the visibilities this file writes, in source order, by token kind"
    );
    assert_eq!(
        kinds.len(),
        indicators.len(),
        "every indicator in the file reported a keyword; an empty one is unreachable \
         (`at_visibility` gates the only caller that could build one)"
    );

    // All three keywords are exercised, which is the whole alternation. Asserted as a
    // set so that reading a different file would still satisfy it.
    let mut distinct = kinds.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(
        distinct.len(),
        3,
        "this file writes all three keywords: {kinds:?}"
    );

    // The other production that carries an indicator. Two parents is what makes this a
    // shared child, the same reason `MemberPrefix` was taken out of worklist order.
    let under_import = tree_of(PRIVATE_IMPORT_TEST)
        .descendants()
        .filter_map(sv2_ast::VisibilityIndicator::cast)
        .filter_map(|indicator| indicator.syntax().parent().map(|parent| parent.kind()))
        .collect::<Vec<_>>();
    assert_eq!(
        under_import,
        [SyntaxKind::Import; 4],
        "all four of that file's indicators sit under an Import, not a MemberPrefix"
    );
}
