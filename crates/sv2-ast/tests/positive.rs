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

use sv2_ast::{
    AliasMember, AstNode, AttributeDefinition, ElementFilterMember, Import, MemberPrefix,
    PackageMember, PartDefinition,
};
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
        // `cast` guarantees the kind, so assert what it cannot: the alternative is
        // there, and the declaration spans something.
        assert!(
            declaration.imported().is_some(),
            "import {index} wraps an alternative"
        );
        assert!(
            declaration.syntax().text_range().len() > 0.into(),
            "import {index} names something, so its declaration cannot be zero width"
        );
        let body = import
            .body()
            .unwrap_or_else(|| panic!("import {index} must have a relationship body"));
        // `cast` guarantees the kind, so assert what it cannot: which alternative this
        // body is, read from the token that closed it, and that it owns nothing.
        assert_eq!(
            body.terminator().map(|token| token.kind()),
            Some(SyntaxKind::Semicolon),
            "every import in this file closes with a semicolon rather than a braced body"
        );
        assert_eq!(body.elements().count(), 0, "a `;` body owns nothing");
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

/// The corpus file written to exercise imports, and the only one that puts both
/// alternatives of `ImportDeclaration` in the same package.
const IMPORTS: &str = "vendor/corpus/kerml/src/examples/Simple Tests/Imports.kerml";

/// `Imports.kerml`, whose five imports are written:
///
/// ```text
///   line 27:  public import Q::*;    namespace
///   line 32:  public import P::*;    namespace
///   line 33:  public import Q::**;   MEMBERSHIP, recursive
///   line 41:  public import P::*;    namespace
///   line 42:  public import R::*;    namespace
/// ```
///
/// The third one is why this case exists. Read off the two productions (`KerML`
/// 8.2.3.4.2):
///
/// ```text
/// MembershipImport = [QualifiedName] ( '::' isRecursive ?= '**' )?
/// NamespaceImport  = [QualifiedName] '::' '*' ( '::' isRecursive ?= '**' )?
/// ```
///
/// what makes an import a `NamespaceImport` is the `::*`, and **not** the presence of
/// stars. `Q::**` has no `::*` in it, so it matches the first alternative with
/// `isRecursive` set: a recursive import of the membership `Q`, not of its namespace.
/// Anything deciding the alternative by looking for a `*` character gets it backwards,
/// and this file is the pinned input that says so.
///
/// `imported()` returns the raw node because the child is one of **two** kinds and
/// `AstNode` declares one `KIND` per view, so no typed signature admits both. The kind is
/// therefore the discriminator, which is what this case reads.
#[test]
fn a_double_star_without_a_single_one_is_a_membership_import() {
    let root = tree_of(IMPORTS);
    let declarations: Vec<sv2_ast::ImportDeclaration> = root
        .descendants()
        .filter_map(sv2_ast::ImportDeclaration::cast)
        .collect();

    let alternatives: Vec<SyntaxKind> = declarations
        .iter()
        .filter_map(sv2_ast::ImportDeclaration::imported)
        .map(|node| node.kind())
        .collect();
    assert_eq!(
        alternatives,
        [
            SyntaxKind::NamespaceImport,  // Q::*
            SyntaxKind::NamespaceImport,  // P::*
            SyntaxKind::MembershipImport, // Q::**  <- no `::*`, so the first alternative
            SyntaxKind::NamespaceImport,  // P::*
            SyntaxKind::NamespaceImport,  // R::*
        ],
        "the alternative each import matched, in source order"
    );
    assert_eq!(
        alternatives.len(),
        declarations.len(),
        "every declaration carries its alternative; the node is built at a checkpoint, \
         so there is no tree in which it is missing"
    );

    // The one that proves the rule: same trailing characters as a namespace import would
    // have, and a different alternative. Its name is `Q`, with the `**` outside the name.
    let recursive = declarations
        .iter()
        .find(|declaration| {
            declaration
                .imported()
                .is_some_and(|node| node.kind() == SyntaxKind::MembershipImport)
        })
        .expect("line 33 writes one");
    assert_eq!(
        recursive.syntax().text().to_string(),
        "Q::**",
        "the declaration spans the name and the recursive marker"
    );
    let membership = recursive
        .imported()
        .and_then(sv2_ast::MembershipImport::cast)
        .expect("the alternative found above is a membership import");
    assert!(
        membership.is_recursive(),
        "`Q::**` sets isRecursive on the membership import"
    );
    let name = membership
        .name()
        .expect("the alternative carries a QualifiedName");
    assert_eq!(
        name.segments()
            .map(|token| token.text().to_owned())
            .collect::<Vec<_>>(),
        ["Q"],
        "`**` is not a segment of the name; it is the alternative's own token"
    );
}

/// The corpus file that writes a braced relationship body among many terminated ones.
const PARTS_TREE: &str = "vendor/corpus/sysml/src/validation/01-Parts Tree/1a-Parts Tree.sysml";

/// `1a-Parts Tree.sysml` writes one braced relationship body and terminates every other
/// one with a semicolon:
///
/// ```text
///   lines 27-32:  private import Definitions::* {
///                     /* A "private" private import makes ... */
///                 }
/// ```
///
/// `RelationshipBody = ';' | '{' ( ownedRelationship += OwnedAnnotation )* '}'` (`SysML`
/// 8.2.2.2). Both alternatives end in a token, which is what `terminator()` returns, and
/// the token's kind is therefore the discriminator between them — there is deliberately no
/// second accessor saying which alternative was written.
///
/// What this case says that the sweep cannot: the two alternatives really do both occur in
/// one pinned file, the braced one owns the comment as an `OwnedAnnotation` rather than
/// skipping it as trivia, and every other body in the file is a bare `;` owning nothing.
#[test]
fn one_braced_body_among_semicolons_owns_its_comment() {
    let root = tree_of(PARTS_TREE);
    let bodies: Vec<sv2_ast::RelationshipBody> = root
        .descendants()
        .filter_map(sv2_ast::RelationshipBody::cast)
        .collect();
    assert!(
        bodies.len() > 1,
        "this file writes several relationships, or the contrast is not there: {}",
        bodies.len()
    );

    // Exactly one is braced, and it is the only one owning anything.
    let braced: Vec<&sv2_ast::RelationshipBody> = bodies
        .iter()
        .filter(|body| body.terminator().map(|token| token.kind()) == Some(SyntaxKind::RBrace))
        .collect();
    assert_eq!(
        braced.len(),
        1,
        "lines 27-32 write the file's only braced body"
    );
    assert_eq!(
        braced[0]
            .elements()
            .map(|node| node.kind())
            .collect::<Vec<_>>(),
        [SyntaxKind::OwnedAnnotation],
        "the comment inside the braces is owned as an annotation, not skipped as trivia"
    );

    // Every other body is a bare `;` owning nothing. Asserted as the complement, so a
    // body that stopped being terminated would land here rather than going unnoticed.
    for body in &bodies {
        if body.terminator().map(|token| token.kind()) == Some(SyntaxKind::RBrace) {
            continue;
        }
        assert_eq!(
            body.terminator().map(|token| token.kind()),
            Some(SyntaxKind::Semicolon),
            "every other relationship in this file closes with `;`, at {:?}",
            body.syntax().text_range()
        );
        assert_eq!(
            body.elements().count(),
            0,
            "a `;` body owns nothing, at {:?}",
            body.syntax().text_range()
        );
    }
}

/// The `KerML` corpus file whose package body holds both namespace member productions,
/// and whose one feature member owns something other than a `Feature`.
const BEHAVIORS: &str = "vendor/corpus/kerml/src/examples/Simple Tests/Behaviors.kerml";

/// `Behaviors.kerml`, whose package body holds four members:
///
/// ```text
///   line  2:  behavior A { ... }            NonFeatureMember
///   line  9:  behavior B specializes A {}   NonFeatureMember
///   line 13:  class C { ... }               NonFeatureMember
///   line 20:  abstract flow msg of C;       NamespaceFeatureMember
/// ```
///
/// `NamespaceFeatureMember = MemberPrefix ownedRelatedElement += FeatureElement` (`KerML`
/// 8.2.3.4.1), and a namespace's other members arrive through `NonFeatureMember`. The two
/// are told apart by what they own, not by how they are written, and this file is the
/// pinned input where both occur side by side.
///
/// The point of this case is the element. `FeatureElement` has ten alternatives and
/// `feature_element` dispatches to all ten, but 130 of the 133 feature members in the
/// corpus own a plain `Feature` — so a case reading any other file would not distinguish
/// `element()` from "the Feature child". Line 20 owns a `Flow`, which is one of the three
/// kinds that actually occur (`Feature` 130, `Expression` 2, `Flow` 1).
///
/// `KerML` only: ADR-0015 re-scoped this unit to the kerml grammar, and all 133 instances
/// in the repository come from `.kerml` files.
#[test]
fn the_one_feature_member_in_behaviors_owns_a_flow_not_a_feature() {
    let root = tree_of(BEHAVIORS);
    let members: Vec<sv2_ast::NamespaceFeatureMember> = root
        .descendants()
        .filter_map(sv2_ast::NamespaceFeatureMember::cast)
        .collect();
    assert_eq!(
        members.len(),
        1,
        "line 20 is the file's only namespace-level feature member"
    );

    // The element is a Flow, which is what makes this file worth reading: an accessor
    // that looked for a Feature child specifically would report nothing here.
    assert_eq!(
        members[0].element().map(|node| node.kind()),
        Some(SyntaxKind::Flow),
        "`abstract flow msg of C;` owns a Flow, not a Feature"
    );

    // No visibility is written on it, and that must read as none rather than as damage:
    // only one of the 133 feature members in the corpus writes one at all.
    let prefix = members[0]
        .prefix()
        .expect("MemberPrefix is built whether or not a visibility is written");
    assert!(
        prefix.visibility().is_none(),
        "line 20 writes no visibility"
    );
    assert_eq!(
        prefix.syntax().text_range().len(),
        0.into(),
        "so its prefix is zero width"
    );

    // And the sibling production is present in the same body, which is the contrast that
    // makes the kind mean something. Read off the file: three non-feature members.
    let body = members[0]
        .syntax()
        .parent()
        .expect("the member sits in the package body");
    let kinds: Vec<SyntaxKind> = body.children().map(|child| child.kind()).collect();
    assert_eq!(
        kinds
            .iter()
            .filter(|kind| **kind == SyntaxKind::NonFeatureMember)
            .count(),
        3,
        "two behaviors and a class, none of them feature members: {kinds:?}"
    );
    assert_eq!(
        kinds
            .iter()
            .filter(|kind| **kind == SyntaxKind::NamespaceFeatureMember)
            .count(),
        1,
        "and the one flow: {kinds:?}"
    );
}

/// `Behaviors.kerml`'s non-feature members, in source order, and what each owns.
///
/// Read off the file:
///
/// ```text
///   line  1:  package Behaviors { ... }     the root's one member, owning a Package
///   line  2:  behavior A { ... }            owning a Behavior
///   line  9:  behavior B specializes A {}   owning a Behavior
///   line 13:  class C { ... }               owning a Class
/// ```
///
/// `NonFeatureMember = MemberPrefix ownedRelatedElement += MemberElement` (`KerML`
/// 8.2.3.4.1), and `MemberElement = AnnotatingElement | NonFeatureElement` (8.2.3.4.3) is
/// an alternation that builds no node, so the element is the concrete one. The case
/// is chosen for its behaviors: 243 of the 608 non-feature members in the corpus own a
/// `Class` or a `Package`, and a case reading only those could not tell `element()` from
/// an accessor that looked for one of them. Lines 2 and 9 own neither.
///
/// The members inside `class C { ... }` are features (`in x;`, `step a : A;`, ...) and
/// arrive through `FeatureMember`, and line 20 is the file's one `NamespaceFeatureMember`,
/// so none of them is counted here.
#[test]
fn the_non_feature_members_of_behaviors_own_a_package_two_behaviors_and_a_class() {
    let root = tree_of(BEHAVIORS);
    let members: Vec<sv2_ast::NonFeatureMember> = root
        .descendants()
        .filter_map(sv2_ast::NonFeatureMember::cast)
        .collect();
    assert_eq!(
        members
            .iter()
            .map(|member| member.element().map(|element| element.kind()))
            .collect::<Vec<_>>(),
        [
            Some(SyntaxKind::Package),
            Some(SyntaxKind::Behavior),
            Some(SyntaxKind::Behavior),
            Some(SyntaxKind::Class),
        ],
        "lines 1, 2, 9 and 13, in source order"
    );

    // The element is the member's own child, not something found further down: line 2's
    // Behavior holds a `composite step b`, and that must not stand in for it.
    for member in &members {
        assert_eq!(
            member
                .element()
                .and_then(|element| element.parent())
                .as_ref(),
            Some(member.syntax()),
            "each element is a direct child of its member"
        );
    }

    // No member in the file writes a visibility, and that reads as none, not as damage.
    for member in &members {
        let prefix = member
            .prefix()
            .expect("MemberPrefix is built whether or not a visibility is written");
        assert!(
            prefix.visibility().is_none(),
            "Behaviors.kerml writes no visibility"
        );
    }
}

/// Sixteen lines holding four packages, one with a braced body and three with `;`.
const DEPENDENCY_12A: &str = "vendor/corpus/sysml/src/validation/\
                              12-Dependency Relationships/12a-Dependency.sysml";

/// `12a-Dependency.sysml`'s packages, in source order, each with its own declaration and
/// body.
///
/// Read off the file:
///
/// ```text
///   line 1:  package '12a-Dependency' { ... }   braced body, holding the other three
///   line 3:  package 'Application Layer';       `;` body
///   line 4:  package 'Service Layer';           `;` body
///   line 5:  package 'Data Layer';              `;` body
/// ```
///
/// `Package = ( ownedRelationship += PrefixMetadataMember )* PackageDeclaration
/// PackageBody` and `PackageBody = ';' | '{' PackageBodyElement* '}'` (`SysML` 8.2.2.5.1).
/// The file is chosen because the outer package's body holds three packages of its own:
/// an accessor that reached into the body for a nested declaration, or that returned the
/// declaration for the body, reports a different name or a different first token here.
#[test]
fn the_packages_of_12a_dependency_each_report_their_own_declaration_and_body() {
    let root = tree_of(DEPENDENCY_12A);
    let packages: Vec<sv2_ast::Package> = root
        .descendants()
        .filter_map(sv2_ast::Package::cast)
        .collect();

    let declarations: Vec<Option<String>> = packages
        .iter()
        .map(|package| package.declaration().map(|d| d.text().to_string()))
        .collect();
    assert_eq!(
        declarations,
        [
            Some("package '12a-Dependency'".to_owned()),
            Some("package 'Application Layer'".to_owned()),
            Some("package 'Service Layer'".to_owned()),
            Some("package 'Data Layer'".to_owned()),
        ],
        "lines 1, 3, 4 and 5, in source order"
    );

    // Each body opens on the token the file writes: `{` for line 1, `;` for the rest.
    let openers: Vec<Option<SyntaxKind>> = packages
        .iter()
        .map(|package| {
            package
                .body()
                .and_then(|body| body.first_token())
                .map(|token| token.kind())
        })
        .collect();
    assert_eq!(
        openers,
        [
            Some(SyntaxKind::LBrace),
            Some(SyntaxKind::Semicolon),
            Some(SyntaxKind::Semicolon),
            Some(SyntaxKind::Semicolon),
        ],
        "one braced body, then three `;` bodies"
    );

    // And none of the four writes prefix metadata, which reads as none.
    assert!(
        packages
            .iter()
            .all(|package| package.prefix_metadata().next().is_none()),
        "12a-Dependency.sysml writes no `#` before any package"
    );
}

/// `#Safety #Approved package P;`, written by hand: two prefix metadata members, in
/// source order, then the declaration and the body.
///
/// `Package = ( ownedRelationship += PrefixMetadataMember )* PackageDeclaration
/// PackageBody` (`SysML` 8.2.2.5.1, and `KerML` 8.2.5.13 identically). Hand-written because
/// **no pinned file writes prefix metadata on a package** — none of the 666 packages in the
/// repository's 610 trees has one — so the corpus cannot tell `prefix_metadata()` from an
/// accessor that always returned nothing. Two, so that the order is observable too: the
/// clause's `ownedRelationship +=` is an ordered property, and source order is it.
#[test]
fn prefix_metadata_on_a_package_is_reported_in_source_order() {
    let root = trees::tree("#Safety #Approved package P;", sv2_syntax::Language::SysMl);
    let packages: Vec<sv2_ast::Package> = root
        .descendants()
        .filter_map(sv2_ast::Package::cast)
        .collect();
    assert_eq!(packages.len(), 1, "one package");
    let package = &packages[0];

    assert_eq!(
        package
            .prefix_metadata()
            .map(|member| member.text().to_string().trim().to_owned())
            .collect::<Vec<_>>(),
        ["#Safety", "#Approved"],
        "both members, in the order written"
    );
    assert_eq!(
        package
            .declaration()
            .map(|d| d.text().to_string())
            .as_deref(),
        Some("package P"),
        "the metadata is not the declaration"
    );
    assert_eq!(
        package.body().map(|b| b.text().to_string()).as_deref(),
        Some(";"),
        "and the body is the `;`"
    );
}

const LANGUAGE_EXTENSIONS_14C: &str =
    "vendor/corpus/sysml/src/validation/14-Language Extensions/14c-Language Extensions.sysml";

/// `14c-Language Extensions.sysml`, whose outer package holds three packages, two of them
/// library packages:
///
/// ```text
///   line   1:  package '14c-Language-Extensions' {
///   line   4:      library package FMEALibrary {       ... closed on line 75
///   line  77:      library package FMEAMetadata {      ... closed on line 139
///   line 141:      package FMEAUserModel {             a Package, not a LibraryPackage
/// ```
///
/// `LibraryPackage = ( isStandard ?= 'standard' ) 'library' ( ownedRelationship +=
/// PrefixMetadataMember )* PackageDeclaration PackageBody` (`SysML` 8.2.2.5.1, `'standard'`
/// optional by deviation `LibraryPackage`). Neither writes `standard` or prefix metadata,
/// and each body is braced. The lines are where each body's `}` is written: an accessor
/// that returned another package's body, or a nested one, closes on a different line.
#[test]
fn the_two_library_packages_of_14c_are_not_standard_and_close_their_own_bodies() {
    let path = trees::workspace().join(LANGUAGE_EXTENSIONS_14C);
    let source = std::fs::read_to_string(&path).expect("14c-Language Extensions.sysml is pinned");
    let root = trees::tree(&source, sv2_syntax::Language::SysMl);
    let libraries: Vec<sv2_ast::LibraryPackage> = root
        .descendants()
        .filter_map(sv2_ast::LibraryPackage::cast)
        .collect();

    assert_eq!(
        libraries
            .iter()
            .map(|library| library.declaration().map(|d| d.text().to_string()))
            .collect::<Vec<_>>(),
        [
            Some("package FMEALibrary".to_owned()),
            Some("package FMEAMetadata".to_owned()),
        ],
        "lines 4 and 77, in source order; FMEAUserModel is not a library package"
    );

    let closing_lines: Vec<Option<usize>> = libraries
        .iter()
        .map(|library| {
            library.body().and_then(|body| {
                let end = usize::from(body.text_range().end());
                let text = source.get(..end)?;
                text.ends_with('}')
                    .then(|| text.bytes().filter(|&b| b == b'\n').count() + 1)
            })
        })
        .collect();
    assert_eq!(
        closing_lines,
        [Some(75), Some(139)],
        "each body is braced and closes on the line the file closes it"
    );

    assert_eq!(
        libraries
            .iter()
            .map(|library| (library.is_standard(), library.prefix_metadata().count()))
            .collect::<Vec<_>>(),
        [(false, 0), (false, 0)],
        "the file writes `library package`, with no `standard` and no `#`"
    );
}

/// Written by hand: a library package holding a standard one, each with prefix metadata
/// after `library`.
///
/// ```text
/// library #Safety package Outer {
///     standard library #Approved #Reviewed package Inner;
/// }
/// ```
///
/// `LibraryPackage = ( isStandard ?= 'standard' ) 'library' ( ownedRelationship +=
/// PrefixMetadataMember )* PackageDeclaration PackageBody` (`SysML` 8.2.2.5.1, and `KerML`
/// 8.2.5.13 identically). Hand-written because **no pinned file writes `standard` or prefix
/// metadata on a library package** — none of the 10 in the repository's 610 trees has
/// either — so the corpus cannot tell `is_standard()` or `prefix_metadata()` from an
/// accessor that always answered "no". The nesting is the point: `Outer` is not standard
/// and owns one member, while the package inside its body is standard and owns two, so an
/// accessor that read past its own children answers `Outer` with `Inner`'s. Two members on
/// `Inner`, so that order is observable: `ownedRelationship +=` is ordered, and source
/// order is it.
#[test]
fn standard_and_prefix_metadata_are_each_library_packages_own() {
    let parse = trees::parsed(
        "library #Safety package Outer {\n\tstandard library #Approved #Reviewed package Inner;\n}",
        sv2_syntax::Language::SysMl,
    );
    let libraries: Vec<sv2_ast::LibraryPackage> = parse
        .syntax()
        .descendants()
        .filter_map(sv2_ast::LibraryPackage::cast)
        .collect();

    let slots: Vec<(bool, Vec<String>, Option<String>)> = libraries
        .iter()
        .map(|library| {
            (
                library.is_standard(),
                library
                    .prefix_metadata()
                    .map(|member| member.text().to_string().trim().to_owned())
                    .collect(),
                library.declaration().map(|d| d.text().to_string()),
            )
        })
        .collect();
    assert_eq!(
        slots,
        [
            (
                false,
                vec!["#Safety".to_owned()],
                Some("package Outer".to_owned())
            ),
            (
                true,
                vec!["#Approved".to_owned(), "#Reviewed".to_owned()],
                Some("package Inner".to_owned())
            ),
        ],
        "each package's own `standard` and metadata, in the order written"
    );
    assert_eq!(
        libraries
            .iter()
            .map(|library| library
                .body()
                .and_then(|body| body.first_token())
                .map(|t| t.kind()))
            .collect::<Vec<_>>(),
        [Some(SyntaxKind::LBrace), Some(SyntaxKind::Semicolon)],
        "Outer's body is braced, Inner's is the `;`"
    );
    assert!(
        parse.errors().is_empty(),
        "both are well-formed: {:?}",
        parse.errors()
    );
}

const SAFETY_AND_SECURITY_11B: &str = "vendor/corpus/sysml/src/validation/11-View and Viewpoint/\
                                       11b-Safety and Security Feature Views.sysml";

/// `11b-Safety and Security Feature Views.sysml`, whose three membership imports are
/// written:
///
/// ```text
///   line 49:  private import PartsTree::vehicle;     two segments, not recursive
///   line 52:  expose vehicle;                        a MembershipExpose, not recursive
///   line 61:  expose vehicle::**[@Safety and ...];   recursive, the head of a FilterPackage
/// ```
///
/// and one import that is **not** among them:
///
/// ```text
///   line 56:  expose vehicle::*::**;                 recursive, but a NamespaceImport
/// ```
///
/// `MembershipImport = importedMembership = [QualifiedName] ( '::' isRecursive ?= '**' )?`
/// and `NamespaceImport = importedNamespace = [QualifiedName] '::' '*' ( '::' isRecursive
/// ?= '**' )?` (`KerML` 8.2.3.4.2, `SysML` 8.2.2.5.1). The file is chosen for its spread:
/// a multi-segment name, where the `::` inside the name is not the suffix's; an `expose`,
/// which holds the alternative without an `ImportDeclaration` (`SysML` 8.2.2.26.2); a
/// recursive one inside a filter; and line 56, whose `**` belongs to a namespace import.
#[test]
fn the_membership_imports_of_11b_name_what_they_import_and_say_which_recurse() {
    let root = tree_of(SAFETY_AND_SECURITY_11B);
    let memberships: Vec<sv2_ast::MembershipImport> = root
        .descendants()
        .filter_map(sv2_ast::MembershipImport::cast)
        .collect();

    let read: Vec<(Vec<String>, bool, Option<SyntaxKind>)> = memberships
        .iter()
        .map(|membership| {
            (
                membership
                    .name()
                    .map(|name| name.segments().map(|t| t.text().to_owned()).collect())
                    .unwrap_or_default(),
                membership.is_recursive(),
                membership.syntax().parent().map(|parent| parent.kind()),
            )
        })
        .collect();
    assert_eq!(
        read,
        [
            (
                vec!["PartsTree".to_owned(), "vehicle".to_owned()],
                false,
                Some(SyntaxKind::ImportDeclaration)
            ),
            (
                vec!["vehicle".to_owned()],
                false,
                Some(SyntaxKind::MembershipExpose)
            ),
            (
                vec!["vehicle".to_owned()],
                true,
                Some(SyntaxKind::ImportDeclaration)
            ),
        ],
        "lines 49, 52 and 61, in source order; line 56 is a namespace import"
    );
}

/// Written by hand: two membership imports whose names are quoted and contain stars.
///
/// ```text
/// public import A::'**';      a name whose last segment is `'**'`: NOT recursive
/// public import 'b**'::**;    a name `'b**'`, then the suffix: recursive
/// ```
///
/// An `UnrestrictedName` is one `NAME` token whatever its quotes enclose (`KerML` 8.2.2.3),
/// so the stars inside one are a segment's text and never the `( '::' isRecursive ?= '**'
/// )?` of `MembershipImport` (8.2.3.4.2). Hand-written because no pinned file quotes a
/// name containing `*`, so the corpus cannot tell `is_recursive()` from an accessor that
/// looked for stars in the text — the same mistake `ImportDeclaration::imported` warns
/// against for telling the alternatives apart.
#[test]
fn stars_inside_a_quoted_name_are_not_the_recursive_suffix() {
    let parse = trees::parsed(
        "package P {\n\tpublic import A::'**';\n\tpublic import 'b**'::**;\n}",
        sv2_syntax::Language::SysMl,
    );
    let read: Vec<(Vec<String>, bool)> = parse
        .syntax()
        .descendants()
        .filter_map(sv2_ast::MembershipImport::cast)
        .map(|membership| {
            (
                membership
                    .name()
                    .map(|name| name.segments().map(|t| t.text().to_owned()).collect())
                    .unwrap_or_default(),
                membership.is_recursive(),
            )
        })
        .collect();
    assert_eq!(
        read,
        [
            (vec!["A".to_owned(), "'**'".to_owned()], false),
            (vec!["'b**'".to_owned()], true),
        ],
        "the quoted stars are segments; only the second writes the suffix"
    );
    assert!(
        parse.errors().is_empty(),
        "both are well-formed: {:?}",
        parse.errors()
    );
}

/// `11b-Safety and Security Feature Views.sysml` again, for its seven namespace imports:
///
/// ```text
///   line  1:  private import Views::*;                      at the root
///   line  3:  private import ScalarValues::*;
///   line 13:  public import AnnotationDefinitions::*;
///   line 34:  public import AnnotationDefinitions::*;
///   line 48:  private import ViewDefinitions::*;
///   line 56:  expose vehicle::*::**;                        recursive, in a NamespaceExpose
///   line 61:  expose vehicle::**[@Safety and ...];          the FilterPackage alternative
/// ```
///
/// ```text
/// NamespaceImport = importedNamespace = [QualifiedName] '::' '*' ( '::' isRecursive ?= '**' )?
///                 | importedNamespace = FilterPackage { ownedRelatedElement += importedNamespace }
/// ```
///
/// (`KerML` 8.2.3.4.2, `SysML` 8.2.2.5.1; the expose is `NamespaceExpose = NamespaceImport`,
/// 8.2.2.26.2). Line 61 is why the file was picked. Its second alternative names nothing
/// and assigns no `isRecursive`, but the `MembershipImport` *inside* its filter names
/// `vehicle` and writes `**`. So an accessor that reads this node's descendants rather
/// than its own children reports a name and a recursion that belong to the import it
/// filters.
#[test]
fn the_namespace_imports_of_11b_name_a_namespace_or_carry_a_filter_and_say_which_recurse() {
    let root = tree_of(SAFETY_AND_SECURITY_11B);
    let read: Vec<(Vec<String>, bool, bool, Option<SyntaxKind>)> = root
        .descendants()
        .filter_map(sv2_ast::NamespaceImport::cast)
        .map(|namespace| {
            (
                namespace
                    .name()
                    .map(|name| name.segments().map(|t| t.text().to_owned()).collect())
                    .unwrap_or_default(),
                namespace.is_recursive(),
                namespace.filter_package().is_some(),
                namespace.syntax().parent().map(|parent| parent.kind()),
            )
        })
        .collect();
    let plain = |name: &str| {
        (
            vec![name.to_owned()],
            false,
            false,
            Some(SyntaxKind::ImportDeclaration),
        )
    };
    assert_eq!(
        read,
        [
            plain("Views"),
            plain("ScalarValues"),
            plain("AnnotationDefinitions"),
            plain("AnnotationDefinitions"),
            plain("ViewDefinitions"),
            (
                vec!["vehicle".to_owned()],
                true,
                false,
                Some(SyntaxKind::NamespaceExpose)
            ),
            (vec![], false, true, Some(SyntaxKind::NamespaceExpose)),
        ],
        "lines 1, 3, 13, 34, 48, 56 and 61, in source order"
    );
}

const COMMENT_EXAMPLE: &str = "vendor/corpus/sysml/src/training/01. Packages/\
                               Comment Example.sysml";
const DOCUMENTATION_EXAMPLE: &str = "vendor/corpus/sysml/src/training/01. Packages/\
                                     Documentation Example.sysml";

/// `Comment Example.sysml` and `Documentation Example.sysml`, the two training siblings,
/// each owning exactly one annotation through a relationship and several more as members.
///
/// ```text
///   Comment Example        lines 2, 5, 7:   comments owned as members of the package
///                          lines 14-19:     alias Car for Automobile { /* ... */ }
///   Documentation Example  lines 2, 7:      documentation owned by its namespace
///                          lines 10-12:     alias Car for Automobile { doc /* ... */ }
/// ```
///
/// `OwnedAnnotation : Annotation = ownedRelatedElement += AnnotatingElement` and
/// `AnnotatingElement = Comment | Documentation | TextualRepresentation | MetadataFeature`
/// (`SysML` 8.2.2.4.1, `KerML` 8.2.3.3.1), reached from a braced `RelationshipBody`
/// (`SysML` 8.2.2.2). Only the alias bodies are relationship bodies. Everything else in
/// either file is a member, and is not an `OwnedAnnotation`.
///
/// Read together because no pinned file writes both alternatives. Comment Example alone
/// cannot fail an `element()` that looked only for a `Comment`, nor Documentation Example
/// one that looked only for a `Documentation`. Read together they fail both.
#[test]
fn the_alias_bodies_of_the_two_training_examples_each_own_one_annotation() {
    for (file, kind, written, annotating) in [
        (
            COMMENT_EXAMPLE,
            SyntaxKind::Comment,
            "/*\n\t\t * This is a comment annotating its owning\n\t\t * element.\n\t\t */",
            4,
        ),
        (
            DOCUMENTATION_EXAMPLE,
            SyntaxKind::Documentation,
            "doc /* This is documentation of the alias. */",
            3,
        ),
    ] {
        let root = tree_of(file);
        assert_eq!(
            root.descendants()
                .filter(|node| node.kind() == kind)
                .count(),
            annotating,
            "{file}: every {kind:?} the file writes, annotation or member"
        );
        let annotations: Vec<sv2_ast::OwnedAnnotation> = root
            .descendants()
            .filter_map(sv2_ast::OwnedAnnotation::cast)
            .collect();
        let read: Vec<(Option<SyntaxKind>, Option<String>)> = annotations
            .iter()
            .map(|annotation| {
                let element = annotation.element();
                (
                    element.as_ref().map(SyntaxNode::kind),
                    element.map(|node| node.text().to_string()),
                )
            })
            .collect();
        assert_eq!(
            read,
            [(Some(kind), Some(written.to_owned()))],
            "{file}: the alias body owns one annotation, and it is the one written there"
        );
        let alias = annotations[0]
            .syntax()
            .ancestors()
            .find_map(AliasMember::cast)
            .expect("the annotation is inside `alias Car`'s body");
        assert_eq!(
            alias.name().map(|token| token.text().to_owned()).as_deref(),
            Some("Car"),
            "{file}: the alias that owns it"
        );
    }
}

/// Written by hand: an alias body owning each annotating element the corpus never owns
/// through a relationship, in both languages.
///
/// ```text
/// alias A for B { @M; }                            SysML: a MetadataUsage
/// alias A for B { language "ocl" /* x > 0 */ }     SysML: a TextualRepresentation
/// alias A for B { @M; }                            KerML: a MetadataFeature
/// ```
///
/// `AnnotatingElement = Comment | Documentation | TextualRepresentation |
/// MetadataFeature` (`SysML` 8.2.2.4.1, `KerML` 8.2.3.3.1). `SysML`'s fourth alternative
/// is a `MetadataUsage` by deviation `AnnotatingElement` (`follow_xtext`), and `KerML`'s is
/// the clause's own `MetadataFeature`. `TextualRepresentation = ( 'rep' Identification
/// )? 'language' language = STRING_VALUE body = REGULAR_COMMENT` (`SysML` 8.2.2.4.3).
/// Hand-written because every one of the corpus's 8 owned annotations is a `Comment` or
/// a `Documentation`. An `element()` that skipped the metadata alternative survived the
/// whole 610-tree sweep.
#[test]
fn an_owned_annotation_owns_every_alternative_of_annotating_element() {
    for (source, language, kind) in [
        (
            "package P {\n\talias A for B { @M; }\n}",
            sv2_syntax::Language::SysMl,
            SyntaxKind::MetadataUsage,
        ),
        (
            "package P {\n\talias A for B { language \"ocl\" /* x > 0 */ }\n}",
            sv2_syntax::Language::SysMl,
            SyntaxKind::TextualRepresentation,
        ),
        (
            "package P {\n\talias A for B { @M; }\n}",
            sv2_syntax::Language::KerMl,
            SyntaxKind::MetadataFeature,
        ),
    ] {
        let parse = trees::parsed(source, language);
        let read: Vec<Option<SyntaxKind>> = parse
            .syntax()
            .descendants()
            .filter_map(sv2_ast::OwnedAnnotation::cast)
            .map(|annotation| annotation.element().map(|element| element.kind()))
            .collect();
        assert_eq!(
            read,
            [Some(kind)],
            "{language:?} {source:?}: one annotation, owning the alternative written"
        );
        assert!(
            parse.errors().is_empty(),
            "{language:?} {source:?} is well-formed: {:?}",
            parse.errors()
        );
    }
}

const INDIVIDUAL_TEST: &str = "vendor/corpus/sysml/src/examples/Simple Tests/IndividualTest.sysml";

/// What one `OccurrenceDefinitionPrefix` says: its basic prefix's text, whether it is
/// individual, whether it carries the empty multiplicity, and its extension keywords.
type PrefixReading = (Option<String>, bool, bool, Vec<String>);

fn prefixes_of(relative: &str) -> Vec<PrefixReading> {
    tree_of(relative)
        .descendants()
        .filter_map(sv2_ast::OccurrenceDefinitionPrefix::cast)
        .map(|prefix| {
            (
                prefix.basic_prefix().map(|basic| basic.text().to_string()),
                prefix.is_individual(),
                prefix.multiplicity().is_some(),
                prefix
                    .extension_keywords()
                    .map(|keyword| keyword.text().to_string())
                    .collect(),
            )
        })
        .collect()
}

/// `14c-Language Extensions.sysml`, whose thirteen occurrence definitions write two
/// `abstract`s and one `#fmea`:
///
/// ```text
///   line   6:  abstract occurrence def Situation;
///   lines 10, 16, 22:  occurrence def Cause / FailureMode / Effect
///   line  28:  item def FMEAItem
///   lines 38, 45, 49, 55:  connection, requirement, requirement, connection
///   line  62:  abstract connection def ControllingMeasure
///   lines 67, 71:  connection def Prevention / Mitigation
///   line 161:  #fmea item def 'Glucose FMEA Item'
/// ```
///
/// `OccurrenceDefinitionPrefix = BasicDefinitionPrefix? ( isIndividual ?= 'individual'
/// ownedRelationship += EmptyMultiplicityMember )? DefinitionExtensionKeyword*` (`SysML`
/// 8.2.2.9.1), the prefix of the occurrence, item, connection and requirement definitions
/// (8.2.2.9.1, .10, .13.1, .21.1). The file's `enum def`s and `metadata def`s are not among
/// them: neither production opens with this prefix.
#[test]
fn the_occurrence_definition_prefixes_of_14c_write_abstract_twice_and_one_extension() {
    let plain: PrefixReading = (None, false, false, vec![]);
    let abstract_: PrefixReading = (Some("abstract".to_owned()), false, false, vec![]);
    let fmea: PrefixReading = (None, false, false, vec!["#fmea".to_owned()]);
    assert_eq!(
        prefixes_of(LANGUAGE_EXTENSIONS_14C),
        [
            abstract_.clone(), // 6
            plain.clone(),     // 10
            plain.clone(),     // 16
            plain.clone(),     // 22
            plain.clone(),     // 28
            plain.clone(),     // 38
            plain.clone(),     // 45
            plain.clone(),     // 49
            plain.clone(),     // 55
            abstract_,         // 62
            plain.clone(),     // 67
            plain,             // 71
            fmea,              // 161
        ],
        "the thirteen prefixes, in source order"
    );
}

/// `IndividualTest.sysml`, whose eight `individual`s belong to only seven prefixes:
///
/// ```text
///   line  2:  individual def IO1;              an IndividualDefinition: NO prefix
///   line  3:  individual occurrence def IO2    individual
///   line  7:  individual item def II1          individual
///   line 11:  item def I                       plain
///   line 14:  individual item def II2 :> I     individual
///   line 18:  individual part def IP1          individual
///   line 22:  part def P                       plain
///   line 25:  individual part def IP2 :> P     individual
///   line 29:  individual action def AP1        individual
///   line 33:  action def A                     plain
///   line 36:  individual action def IA2 :> A   individual
/// ```
///
/// Line 2 is why the file was picked. `IndividualDefinition : OccurrenceDefinition =
/// BasicDefinitionPrefix? isIndividual ?= 'individual' DefinitionExtensionKeyword* 'def'
/// Definition ownedRelationship += EmptyMultiplicityMember` (`SysML` 8.2.2.9.1) writes its
/// `individual` without an `OccurrenceDefinitionPrefix` around it. So a count of `individual`
/// keywords is not a count of individual prefixes. Each of the other seven carries
/// `ownedRelationship += EmptyMultiplicityMember` with its `individual`. The Pilot writes
/// `IndividualDefinition`'s multiplicity straight after `individual` (`SysML.xtext:816`),
/// not last; it consumes no tokens, so both accept the same text, and `individual_definition`
/// (`crates/sv2-syntax/src/parser/definition.rs`) records that no deviation is needed.
#[test]
fn the_individual_prefixes_of_individual_test_are_seven_of_its_eight_individuals() {
    let plain: PrefixReading = (None, false, false, vec![]);
    let individual: PrefixReading = (None, true, true, vec![]);
    assert_eq!(
        prefixes_of(INDIVIDUAL_TEST),
        [
            individual.clone(), // 3
            individual.clone(), // 7
            plain.clone(),      // 11
            individual.clone(), // 14
            individual.clone(), // 18
            plain.clone(),      // 22
            individual.clone(), // 25
            individual.clone(), // 29
            plain,              // 33
            individual,         // 36
        ],
        "ten prefixes, in source order; line 2's `individual` is not a prefix's"
    );
}

/// Written by hand: prefixes that write more than one slot, and more than one
/// user-defined keyword.
///
/// ```text
/// #A #B part def X;                        two extension keywords, in order
/// abstract individual #A #B part def Y;    every slot, in the production's order
/// ```
///
/// `OccurrenceDefinitionPrefix = BasicDefinitionPrefix? ( isIndividual ?= 'individual'
/// ownedRelationship += EmptyMultiplicityMember )? DefinitionExtensionKeyword*` (`SysML`
/// 8.2.2.9.1), and "It is also possible to include more than one user defined-keyword in
/// a declaration" (7.27.4, receipt 0f2c5bd1). Hand-written because no corpus prefix writes
/// two slots or two keywords: of 1,020, the 50 that write anything write exactly one. So
/// the corpus cannot tell `extension_keywords()` from one that reversed them or kept only
/// the first, and both survived the 610-tree sweep.
#[test]
fn a_prefix_writes_its_slots_together_and_its_keywords_in_order() {
    let parse = trees::parsed(
        "package P {\n\t#A #B part def X;\n\tabstract individual #A #B part def Y;\n}",
        sv2_syntax::Language::SysMl,
    );
    let read: Vec<PrefixReading> = parse
        .syntax()
        .descendants()
        .filter_map(sv2_ast::OccurrenceDefinitionPrefix::cast)
        .map(|prefix| {
            (
                prefix.basic_prefix().map(|basic| basic.text().to_string()),
                prefix.is_individual(),
                prefix.multiplicity().is_some(),
                prefix
                    .extension_keywords()
                    .map(|keyword| keyword.text().to_string())
                    .collect(),
            )
        })
        .collect();
    assert_eq!(
        read,
        [
            (None, false, false, vec!["#A".to_owned(), "#B".to_owned()]),
            (
                Some("abstract".to_owned()),
                true,
                true,
                vec!["#A".to_owned(), "#B".to_owned()]
            ),
        ],
        "both keywords in source order, and every slot of the second"
    );
    assert!(
        parse.errors().is_empty(),
        "both are well-formed: {:?}",
        parse.errors()
    );
}

const VARIATION_DEFINITIONS: &str = "vendor/corpus/sysml/src/training/36. Variability/\
                                     Variation Definitions.sysml";
const AHF_CORE_LIB: &str =
    "vendor/corpus/sysml/src/examples/Arrowhead Framework Example/AHFCoreLib.sysml";

/// What one `DefinitionPrefix` says: its basic prefix's text and its extension keywords.
fn definition_prefixes_of(relative: &str) -> Vec<(Option<String>, Vec<String>)> {
    tree_of(relative)
        .descendants()
        .filter_map(sv2_ast::DefinitionPrefix::cast)
        .map(|prefix| {
            (
                prefix.basic_prefix().map(|basic| basic.text().to_string()),
                prefix
                    .extension_keywords()
                    .map(|keyword| keyword.text().to_string())
                    .collect(),
            )
        })
        .collect()
}

/// `Variation Definitions.sysml` and `AHFCoreLib.sysml`, two of the four corpus files that
/// write a `DefinitionPrefix` with something in it (the others are
/// `VehicleVariabilityModel.sysml` and `AHFNorwayTopics.sysml`, one prefix each):
///
/// ```text
///   Variation Definitions  line  5:  attribute def Diameter            plain
///                          line 25:  variation attribute def ...       variation
///                          line 30:  variation part def EngineChoices  NOT a DefinitionPrefix
///   AHFCoreLib             lines 7, 12, 15, 19:  #service port def ...
/// ```
///
/// `DefinitionPrefix : Definition = BasicDefinitionPrefix? DefinitionExtensionKeyword*`
/// (`SysML` 8.2.2.6.1), the prefix of `AttributeDefinition` (8.2.2.7) and `PortDefinition`
/// (8.2.2.12), the definitions that are not occurrences. Line 30 is why the first file
/// was picked. A `part def` is an occurrence definition and opens with an
/// `OccurrenceDefinitionPrefix` (8.2.2.11), so the file writes two `variation`s and only
/// one of them is a `DefinitionPrefix`'s.
#[test]
fn two_corpus_files_write_variation_once_and_service_four_times_in_definition_prefixes() {
    assert_eq!(
        definition_prefixes_of(VARIATION_DEFINITIONS),
        [(None, vec![]), (Some("variation".to_owned()), vec![])],
        "lines 5 and 25; line 30's `variation` is an occurrence definition's"
    );
    let service = (None, vec!["#service".to_owned()]);
    assert_eq!(
        definition_prefixes_of(AHF_CORE_LIB),
        [service.clone(), service.clone(), service.clone(), service],
        "lines 7, 12, 15 and 19"
    );
}

/// Written by hand: a `DefinitionPrefix` writing both slots and two user-defined keywords.
///
/// ```text
/// abstract #A #B attribute def X;
/// ```
///
/// `DefinitionPrefix = BasicDefinitionPrefix? DefinitionExtensionKeyword*` (`SysML`
/// 8.2.2.6.1), and "It is also possible to include more than one user defined-keyword in
/// a declaration" (7.27.4, receipt 0f2c5bd1). Hand-written because no corpus
/// `DefinitionPrefix` writes two slots or two keywords: of 291, the 7 that write anything
/// write exactly one.
#[test]
fn a_definition_prefix_writes_both_slots_and_its_keywords_in_order() {
    let parse = trees::parsed(
        "package P {\n\tabstract #A #B attribute def X;\n}",
        sv2_syntax::Language::SysMl,
    );
    let read: Vec<(Option<String>, Vec<String>)> = parse
        .syntax()
        .descendants()
        .filter_map(sv2_ast::DefinitionPrefix::cast)
        .map(|prefix| {
            (
                prefix.basic_prefix().map(|basic| basic.text().to_string()),
                prefix
                    .extension_keywords()
                    .map(|keyword| keyword.text().to_string())
                    .collect(),
            )
        })
        .collect();
    assert_eq!(
        read,
        [(
            Some("abstract".to_owned()),
            vec!["#A".to_owned(), "#B".to_owned()]
        )],
        "both slots, and both keywords in source order"
    );
    assert!(
        parse.errors().is_empty(),
        "well-formed: {:?}",
        parse.errors()
    );
}

const CART_SAMPLE: &str =
    "vendor/corpus/sysml/src/examples/State Space Representation Examples/CartSample.sysml";

/// `CartSample.sysml`, one of the four corpus files that nest a definition inside another
/// definition's body:
///
/// ```text
///   line  5:  part def Cart {                                    owns four
///   line  8:      attribute def CartInput :> Input { ... }
///   line 12:      attribute def CartOutput :> Output { ... }
///   line 16:      attribute def CartState :> StateSpace { ... }
///   line 20:      attribute def CartStateDerivative :> StateDerivative { ... }
///   line 26:  part def Pusher {                                  owns one
///   line 27:      attribute def PusherOutput :> Output { ... }
/// ```
///
/// `Definition = DefinitionDeclaration DefinitionBody` (`SysML` 8.2.2.6.1). Chosen for the
/// nesting: only four of the corpus's 311 files nest a definition in another's body, so a
/// `body()` that reached a nested definition's body, or a `declaration()` that reached a
/// nested declaration, would be invisible on almost every file. Here each definition's
/// declaration names it, and its body owns the definitions written in its braces, and no
/// others.
#[test]
fn the_definitions_of_cart_sample_declare_their_names_and_own_their_nested_definitions() {
    let read: Vec<(String, usize)> = tree_of(CART_SAMPLE)
        .descendants()
        .filter_map(sv2_ast::Definition::cast)
        .map(|definition| {
            let owned = definition.body().map_or(0, |body| {
                body.descendants()
                    .filter(|node| node.kind() == SyntaxKind::Definition)
                    .filter(|nested| {
                        nested
                            .ancestors()
                            .skip(1)
                            .find(|above| above.kind() == SyntaxKind::Definition)
                            .as_ref()
                            == Some(definition.syntax())
                    })
                    .count()
            });
            (
                definition
                    .declaration()
                    .map(|declaration| declaration.text().to_string())
                    .unwrap_or_default(),
                owned,
            )
        })
        .collect();
    assert_eq!(
        read,
        [
            ("Cart".to_owned(), 4),
            ("CartInput :> Input".to_owned(), 0),
            ("CartOutput :> Output".to_owned(), 0),
            ("CartState :> StateSpace".to_owned(), 0),
            ("CartStateDerivative :> StateDerivative".to_owned(), 0),
            ("Pusher".to_owned(), 1),
            ("PusherOutput :> Output".to_owned(), 0),
        ],
        "lines 5, 8, 12, 16, 20, 26 and 27, in source order"
    );
}

/// A part definition's two keywords, whether its prefix is `individual`, and what its
/// definition declares.
type PartDefinitionRead = (Option<String>, Option<String>, bool, Option<String>);

const INDIVIDUALS_AND_ROLES: &str =
    "vendor/corpus/sysml/src/training/28. Individuals/Individuals and Roles-1.sysml";

/// `Individuals and Roles-1.sysml`, three part definitions and a part usage that looks
/// like one:
///
/// ```text
///   line  4:  part def Wheel;                                    no prefix
///   line  6:  individual part def Vehicle_1 :> Vehicle { ... }   individual
///   line 11:  individual part def Wheel_1 :> Wheel;              individual
///   line 13:  individual part vehicle_1 : Vehicle_1 { ... }      a PartUsage: no `def`
/// ```
///
/// `PartDefinition = OccurrenceDefinitionPrefix 'part' 'def' Definition` (`SysML`
/// 8.2.2.11). Chosen because it writes the definition with and without a prefix side by
/// side, and line 13 beside them: `PartUsage` (8.2.2.11) writes `part` and no `def`, so it
/// is not one of these, however much it reads like one.
#[test]
fn the_part_definitions_of_individuals_and_roles_read_their_keywords_prefix_and_definition() {
    let read: Vec<PartDefinitionRead> = tree_of(INDIVIDUALS_AND_ROLES)
        .descendants()
        .filter_map(PartDefinition::cast)
        .map(|part| {
            (
                part.part_keyword().map(|token| token.text().to_owned()),
                part.def_keyword().map(|token| token.text().to_owned()),
                part.prefix().is_some_and(|prefix| prefix.is_individual()),
                part.definition()
                    .and_then(|definition| definition.declaration())
                    .map(|declaration| declaration.text().to_string()),
            )
        })
        .collect();
    let def = |individual: bool, declaration: &str| {
        (
            Some("part".to_owned()),
            Some("def".to_owned()),
            individual,
            Some(declaration.to_owned()),
        )
    };
    assert_eq!(
        read,
        [
            def(false, "Wheel"),
            def(true, "Vehicle_1 :> Vehicle"),
            def(true, "Wheel_1 :> Wheel"),
        ],
        "lines 4, 6 and 11, in source order, and not the usage on line 13"
    );
}

/// An attribute definition's two keywords, its prefix's basic prefix, and what its
/// definition declares.
type AttributeDefinitionRead = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// `Variation Definitions.sysml`, two attribute definitions and a part definition written
/// with the same prefix keyword:
///
/// ```text
///   line  5:  attribute def Diameter :> ISQ::LengthValue;          no prefix
///   line 25:  variation attribute def DiameterChoices :> Diameter  variation
///   line 30:  variation part def EngineChoices :> Engine           a PartDefinition
/// ```
///
/// `AttributeDefinition = DefinitionPrefix 'attribute' 'def' Definition` (`SysML`
/// 8.2.2.7). Chosen because it is one of the two corpus files whose attribute definition
/// writes a prefix, beside one that writes none, and because line 30 is the same shape
/// with another kind keyword: `PartDefinition` (8.2.2.11) is a sibling built by the same
/// parser method, and none of these.
#[test]
fn the_attribute_definitions_of_variation_definitions_read_their_keywords_prefix_and_definition() {
    let read: Vec<AttributeDefinitionRead> = tree_of(VARIATION_DEFINITIONS)
        .descendants()
        .filter_map(AttributeDefinition::cast)
        .map(|attribute| {
            (
                attribute
                    .attribute_keyword()
                    .map(|token| token.text().to_owned()),
                attribute.def_keyword().map(|token| token.text().to_owned()),
                attribute
                    .prefix()
                    .and_then(|prefix| prefix.basic_prefix())
                    .map(|basic| basic.text().to_string()),
                attribute
                    .definition()
                    .and_then(|definition| definition.declaration())
                    .map(|declaration| declaration.text().to_string()),
            )
        })
        .collect();
    let def = |basic: Option<&str>, declaration: &str| {
        (
            Some("attribute".to_owned()),
            Some("def".to_owned()),
            basic.map(str::to_owned),
            Some(declaration.to_owned()),
        )
    };
    assert_eq!(
        read,
        [
            def(None, "Diameter :> ISQ::LengthValue"),
            def(Some("variation"), "DiameterChoices :> Diameter"),
        ],
        "lines 5 and 25, in source order, and not the part definition on line 30"
    );
}

/// An occurrence definition's two keywords, whether its prefix is `individual`, and what
/// its definition declares.
type OccurrenceDefinitionRead = (Option<String>, Option<String>, bool, Option<String>);

const OCCURRENCE_TEST: &str = "vendor/corpus/sysml/src/examples/Simple Tests/OccurrenceTest.sysml";

/// Every `OccurrenceDefinition` in the corpus file at `relative`, read.
fn occurrence_definitions_of(relative: &str) -> Vec<OccurrenceDefinitionRead> {
    tree_of(relative)
        .descendants()
        .filter_map(sv2_ast::OccurrenceDefinition::cast)
        .map(|occurrence| {
            (
                occurrence
                    .occurrence_keyword()
                    .map(|token| token.text().to_owned()),
                occurrence
                    .def_keyword()
                    .map(|token| token.text().to_owned()),
                occurrence
                    .prefix()
                    .is_some_and(|prefix| prefix.is_individual()),
                occurrence
                    .definition()
                    .and_then(|definition| definition.declaration())
                    .map(|declaration| declaration.text().to_string()),
            )
        })
        .collect()
}

/// `OccurrenceTest.sysml` and `IndividualTest.sysml`, read for their occurrence
/// definitions:
///
/// ```text
///   OccurrenceTest   line  2:  occurrence def Occ { ... }             no prefix
///                    line 13:  occurrence occ : Occ { ... }           an OccurrenceUsage
///                    line 19:  individual occurrence def Ind { ... }  individual
///                    line 23:  individual occurrence ind : ...        an OccurrenceUsage
///   IndividualTest   line  2:  individual def IO1;                    an IndividualDefinition
///                    line  3:  individual occurrence def IO2 { ... }  individual
/// ```
///
/// `OccurrenceDefinition = OccurrenceDefinitionPrefix 'occurrence' 'def' Definition`
/// (`SysML` 8.2.2.9.1). The first file writes the definition with and without a prefix
/// beside usages of the same keyword (`OccurrenceUsage`, 8.2.2.9.2), which write no `def`.
/// The second is chosen for line 2: `IndividualDefinition` (8.2.2.9.1) has the metaclass
/// `OccurrenceDefinition` too ("individual may be used in place of the kind keyword",
/// 7.9.4, receipt 8c84370d), but it is another production, with no `occurrence` keyword
/// and no `OccurrenceDefinitionPrefix`, and this parser builds it as another node.
#[test]
fn the_occurrence_definitions_of_two_simple_tests_read_their_keywords_prefix_and_definition() {
    let def = |individual: bool, declaration: &str| {
        (
            Some("occurrence".to_owned()),
            Some("def".to_owned()),
            individual,
            Some(declaration.to_owned()),
        )
    };
    assert_eq!(
        occurrence_definitions_of(OCCURRENCE_TEST),
        [def(false, "Occ"), def(true, "Ind")],
        "lines 2 and 19, in source order, and neither usage"
    );
    assert_eq!(
        occurrence_definitions_of(INDIVIDUAL_TEST),
        [def(true, "IO2")],
        "line 3, and not the individual definition on line 2"
    );
}

/// A `simple_definition` production's two keywords, its prefix's basic keyword, and what its
/// definition declares: what the item and later definitions' positive cases compare.
type SimpleDefinitionRead = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// `ItemTest.sysml`, read for its item definitions:
///
/// ```text
///   line  3:  item f: A;                 an ItemUsage
///   line  5:  public item def A { ... }  no prefix: `public` is the member's
///   line  6:    item b: B;               an ItemUsage
///   line 10:  abstract item def B { ... } abstract
///   lines 19, 20:  in item a1 / out item a2   ItemUsages
/// ```
///
/// `ItemDefinition = OccurrenceDefinitionPrefix 'item' 'def' Definition` (`SysML`
/// 8.2.2.10). The file writes the definition with and without `abstract`, beside usages of
/// the same keyword (`ItemUsage = OccurrenceUsagePrefix 'item' Usage`, 8.2.2.10), which
/// write no `def`. `public` is a visibility, which the member's prefix owns, not the
/// definition's (8.2.2.5.1).
#[test]
fn the_item_definitions_of_item_test_read_their_keywords_prefix_and_definition() {
    let read: Vec<SimpleDefinitionRead> = tree_of(ITEM_TEST)
        .descendants()
        .filter_map(sv2_ast::ItemDefinition::cast)
        .map(|item| {
            (
                item.item_keyword().map(|token| token.text().to_owned()),
                item.def_keyword().map(|token| token.text().to_owned()),
                item.prefix()
                    .and_then(|prefix| prefix.basic_prefix())
                    .map(|basic| basic.text().to_string()),
                item.definition()
                    .and_then(|definition| definition.declaration())
                    .map(|declaration| declaration.text().to_string()),
            )
        })
        .collect();
    let def = |basic: Option<&str>, declaration: &str| {
        (
            Some("item".to_owned()),
            Some("def".to_owned()),
            basic.map(str::to_owned),
            Some(declaration.to_owned()),
        )
    };
    assert_eq!(
        read,
        [def(None, "A"), def(Some("abstract"), "B")],
        "lines 5 and 10, in source order, and none of the usages"
    );
}

const CONNECTION_TEST: &str = "vendor/corpus/sysml/src/examples/Simple Tests/ConnectionTest.sysml";

/// `ConnectionTest.sysml`, read for its connection definitions:
///
/// ```text
///   lines 12, 16, 17:  connect p to y; ...       ConnectionUsages, with no `connection`
///   line 31:  abstract connection def C { ... }  abstract
///   line 43:  connection bus : C connect (...);  a ConnectionUsage
///   lines 45, 51:  connection : C { ... }, connection { ... }   ConnectionUsages
///   line 57:  abstract flow def F;               a FlowDefinition, a sibling
///   line 67:  connection def AB { ... }          no prefix
/// ```
///
/// `ConnectionDefinition = OccurrenceDefinitionPrefix 'connection' 'def' Definition`
/// (`SysML` 8.2.2.13.1). The file writes the definition with and without `abstract`,
/// beside usages that open with the same keyword and write no `def` (`ConnectionUsage`,
/// 8.2.2.13.1), and beside a flow definition, which `simple_definition` builds with the same
/// method and another node kind.
#[test]
fn the_connection_definitions_of_connection_test_read_their_keywords_prefix_and_definition() {
    let read: Vec<SimpleDefinitionRead> = tree_of(CONNECTION_TEST)
        .descendants()
        .filter_map(sv2_ast::ConnectionDefinition::cast)
        .map(|connection| {
            (
                connection
                    .connection_keyword()
                    .map(|token| token.text().to_owned()),
                connection
                    .def_keyword()
                    .map(|token| token.text().to_owned()),
                connection
                    .prefix()
                    .and_then(|prefix| prefix.basic_prefix())
                    .map(|basic| basic.text().to_string()),
                connection
                    .definition()
                    .and_then(|definition| definition.declaration())
                    .map(|declaration| declaration.text().to_string()),
            )
        })
        .collect();
    let def = |basic: Option<&str>, declaration: &str| {
        (
            Some("connection".to_owned()),
            Some("def".to_owned()),
            basic.map(str::to_owned),
            Some(declaration.to_owned()),
        )
    };
    assert_eq!(
        read,
        [def(Some("abstract"), "C"), def(None, "AB")],
        "lines 31 and 67, in source order, and neither the usages nor the flow definition"
    );
}

const FLOW_DEFINITION_EXAMPLE: &str =
    "vendor/corpus/sysml/src/training/13. Flows/Flow Definition Example.sysml";

/// Every `FlowDefinition` in the corpus file at `relative`, read.
fn flow_definitions_of(relative: &str) -> Vec<SimpleDefinitionRead> {
    tree_of(relative)
        .descendants()
        .filter_map(sv2_ast::FlowDefinition::cast)
        .map(|flow| {
            (
                flow.flow_keyword().map(|token| token.text().to_owned()),
                flow.def_keyword().map(|token| token.text().to_owned()),
                flow.prefix()
                    .and_then(|prefix| prefix.basic_prefix())
                    .map(|basic| basic.text().to_string()),
                flow.definition()
                    .and_then(|definition| definition.declaration())
                    .map(|declaration| declaration.text().to_string()),
            )
        })
        .collect()
}

/// The corpus's two flow definitions, one in each of two files:
///
/// ```text
///   ConnectionTest            line 57:  abstract flow def F;             abstract
///                             line 59:  message : F from p to p;         a Message
///   Flow Definition Example   line  6:  flow def FuelFlow { ... }        no prefix
///                             line 16:  flow : FuelFlow of Fuel ...;     a FlowUsage
/// ```
///
/// `FlowDefinition = OccurrenceDefinitionPrefix 'flow' 'def' Definition` (`SysML`
/// 8.2.2.16). Each file writes the definition beside a usage of it that writes no `def`:
/// a `FlowUsage`, or a `Message`, whose metaclass is `FlowUsage` (8.2.2.16). And
/// `ConnectionTest` writes it among connection definitions, which `simple_definition`
/// builds with the same method and another node kind.
#[test]
fn the_flow_definitions_of_two_files_read_their_keywords_prefix_and_definition() {
    let def = |basic: Option<&str>, declaration: &str| {
        (
            Some("flow".to_owned()),
            Some("def".to_owned()),
            basic.map(str::to_owned),
            Some(declaration.to_owned()),
        )
    };
    assert_eq!(
        flow_definitions_of(CONNECTION_TEST),
        [def(Some("abstract"), "F")],
        "line 57, and none of the connection definitions or usages"
    );
    assert_eq!(
        flow_definitions_of(FLOW_DEFINITION_EXAMPLE),
        [def(None, "FuelFlow")],
        "line 6, and not the flow usage on line 16"
    );
}

const ALLOCATION_TEST: &str = "vendor/corpus/sysml/src/examples/Simple Tests/AllocationTest.sysml";

/// `AllocationTest.sysml`, read for its allocation definitions:
///
/// ```text
///   line 19:  allocate l.component to assembly.element;          an AllocationUsage
///   line 22:  allocation def A;                                  no prefix
///   line 24:  allocation def Logical_to_Physical :> A { ... }    no prefix
///   lines 29, 30:  allocation allocation1 / allocation2 : ...    AllocationUsages
///   line 35:  allocate l.component to p.assembly.element;        an AllocationUsage
/// ```
///
/// `AllocationDefinition = OccurrenceDefinitionPrefix 'allocation' 'def' Definition`
/// (`SysML` 8.2.2.15). The file writes the definition with a body and without, beside
/// usages that open with the same keyword or with `allocate` and write no `def`
/// (`AllocationUsage`, 8.2.2.15).
///
/// No corpus file writes a prefix on an allocation definition (all six are listed by
/// `grep -rnE '\ballocation\s+def\b' vendor/corpus`), so this case cannot tell a `prefix()`
/// that answers nothing from one that answers an empty prefix. The sweep can: it asserts
/// the prefix node is there on every allocation definition in every tree.
#[test]
fn the_allocation_definitions_of_allocation_test_read_their_keywords_prefix_and_definition() {
    let read: Vec<SimpleDefinitionRead> = tree_of(ALLOCATION_TEST)
        .descendants()
        .filter_map(sv2_ast::AllocationDefinition::cast)
        .map(|allocation| {
            (
                allocation
                    .allocation_keyword()
                    .map(|token| token.text().to_owned()),
                allocation
                    .def_keyword()
                    .map(|token| token.text().to_owned()),
                allocation
                    .prefix()
                    .and_then(|prefix| prefix.basic_prefix())
                    .map(|basic| basic.text().to_string()),
                allocation
                    .definition()
                    .and_then(|definition| definition.declaration())
                    .map(|declaration| declaration.text().to_string()),
            )
        })
        .collect();
    let def = |declaration: &str| {
        (
            Some("allocation".to_owned()),
            Some("def".to_owned()),
            None,
            Some(declaration.to_owned()),
        )
    };
    assert_eq!(
        read,
        [def("A"), def("Logical_to_Physical :> A")],
        "lines 22 and 24, in source order, and none of the usages"
    );
}

const VIEW_TEST: &str = "vendor/corpus/sysml/src/examples/Simple Tests/ViewTest.sysml";

/// `ViewTest.sysml`, read for its one rendering definition, the corpus's only one:
///
/// ```text
///   line 23:  rendering def R;                  no prefix
///   line 25:  rendering r : R;                  a RenderingUsage
///   line 32:  render rendering r1: R[0..1];     a rendering usage, under `render`
///   line 36:  render r;                         a reference, under `render`
///   line 38:  rendering r2;                     a RenderingUsage
/// ```
///
/// `RenderingDefinition = OccurrenceDefinitionPrefix 'rendering' 'def' Definition`
/// (`SysML` 8.2.2.26.4). The file writes it beside four usages, opening with the same
/// keyword or with `render` and writing no `def` (`RenderingUsage`, 8.2.2.26.4;
/// `ViewRenderingMember`, 8.2.2.26.1), and beside the concern, viewpoint and view
/// definitions, none of which `simple_definition` builds.
///
/// The one rendering definition writes no prefix, so this case cannot tell a `prefix()`
/// that answers nothing from one that answers an empty prefix. The sweep can: it asserts
/// the prefix node is there on every rendering definition in every tree.
#[test]
fn the_rendering_definition_of_view_test_reads_its_keywords_prefix_and_definition() {
    let read: Vec<SimpleDefinitionRead> = tree_of(VIEW_TEST)
        .descendants()
        .filter_map(sv2_ast::RenderingDefinition::cast)
        .map(|rendering| {
            (
                rendering
                    .rendering_keyword()
                    .map(|token| token.text().to_owned()),
                rendering.def_keyword().map(|token| token.text().to_owned()),
                rendering
                    .prefix()
                    .and_then(|prefix| prefix.basic_prefix())
                    .map(|basic| basic.text().to_string()),
                rendering
                    .definition()
                    .and_then(|definition| definition.declaration())
                    .map(|declaration| declaration.text().to_string()),
            )
        })
        .collect();
    assert_eq!(
        read,
        [(
            Some("rendering".to_owned()),
            Some("def".to_owned()),
            None,
            Some("R".to_owned()),
        )],
        "line 23, and none of the four usages"
    );
}

/// A port definition's two keywords, its prefix's `#` keywords, and what its definition
/// declares.
type PortDefinitionRead = (Option<String>, Option<String>, Vec<String>, Option<String>);

const CONJUGATION_TEST: &str =
    "vendor/corpus/sysml/src/examples/Simple Tests/ConjugationTest.sysml";

/// Every `PortDefinition` in the corpus file at `relative`, read.
fn port_definitions_of(relative: &str) -> Vec<PortDefinitionRead> {
    tree_of(relative)
        .descendants()
        .filter_map(sv2_ast::PortDefinition::cast)
        .map(|port| {
            (
                port.port_keyword().map(|token| token.text().to_owned()),
                port.def_keyword().map(|token| token.text().to_owned()),
                port.prefix()
                    .map(|prefix| {
                        prefix
                            .extension_keywords()
                            .map(|keyword| keyword.text().to_string())
                            .collect()
                    })
                    .unwrap_or_default(),
                port.definition()
                    .and_then(|definition| definition.declaration())
                    .map(|declaration| declaration.text().to_string()),
            )
        })
        .collect()
}

/// `ConjugationTest.sysml` and `AHFCoreLib.sysml`, read for their port definitions:
///
/// ```text
///   ConjugationTest   line  2:  port def P;                              no prefix
///                     lines 5, 6, 10, 11, ...:  port p1: P; port p2: ~P; PortUsages
///   AHFCoreLib        line  7:  #service port def ServiceDiscovery { }   #service
///                     line 12:  #service port def ServiceDiscoveryDD :> ServiceDiscovery
///                     line 15:  #service port def Authorisation { ... }
///                     line 19:  #service port def AuthorisationDD :> Authorisation
///                     line 28:  #service serviceDiscovery : ServiceDiscovery;   no port
/// ```
///
/// `PortDefinition = DefinitionPrefix 'port' 'def' Definition ownedRelationship +=
/// ConjugatedPortDefinitionMember` (`SysML` 8.2.2.12). The first file writes the
/// definition beside usages of it and of its conjugate, which write no `def` (`PortUsage`,
/// 8.2.2.12). No corpus file writes `abstract` or `variation` before `port def`; the
/// second file is chosen because it writes a prefix all the same, a user-defined keyword
/// (`DefinitionExtensionKeyword`, 8.2.2.6.1), and writes it on non-port elements too.
#[test]
fn the_port_definitions_of_two_files_read_their_keywords_prefix_and_definition() {
    let def = |extensions: &[&str], declaration: &str| {
        (
            Some("port".to_owned()),
            Some("def".to_owned()),
            extensions
                .iter()
                .map(|&keyword| keyword.to_owned())
                .collect::<Vec<_>>(),
            Some(declaration.to_owned()),
        )
    };
    assert_eq!(
        port_definitions_of(CONJUGATION_TEST),
        [def(&[], "P")],
        "line 2, and none of the usages"
    );
    assert_eq!(
        port_definitions_of(AHF_CORE_LIB),
        [
            def(&["#service"], "ServiceDiscovery"),
            def(&["#service"], "ServiceDiscoveryDD :> ServiceDiscovery"),
            def(&["#service"], "Authorisation"),
            def(&["#service"], "AuthorisationDD :> Authorisation"),
        ],
        "lines 7, 12, 15 and 19, in source order"
    );
}

/// An individual definition's basic prefix, `individual`, `#` keywords and `def` as
/// written, what its definition declares, and whether it owns its empty multiplicity.
type IndividualDefinitionRead = (
    Option<String>,
    Option<String>,
    Vec<String>,
    Option<String>,
    Option<String>,
    bool,
);

/// Every `IndividualDefinition` under `root`, read.
fn individual_definitions_in(root: &SyntaxNode) -> Vec<IndividualDefinitionRead> {
    root.descendants()
        .filter_map(sv2_ast::IndividualDefinition::cast)
        .map(|individual| {
            (
                individual
                    .basic_prefix()
                    .map(|basic| basic.text().to_string()),
                individual
                    .individual_keyword()
                    .map(|token| token.text().to_owned()),
                individual
                    .extension_keywords()
                    .map(|keyword| keyword.text().to_string())
                    .collect(),
                individual
                    .def_keyword()
                    .map(|token| token.text().to_owned()),
                individual
                    .definition()
                    .and_then(|definition| definition.declaration())
                    .map(|declaration| declaration.text().to_string()),
                individual.multiplicity().is_some(),
            )
        })
        .collect()
}

/// An individual definition with nothing before `individual` and nothing between it and
/// `def`, declaring `declaration`: the only kind the corpus writes.
fn bare_individual(declaration: &str) -> IndividualDefinitionRead {
    (
        None,
        Some("individual".to_owned()),
        vec![],
        Some("def".to_owned()),
        Some(declaration.to_owned()),
        true,
    )
}

const VERIFICATION_CASE_USAGE_EXAMPLE: &str =
    "vendor/corpus/sysml/src/training/34. Verification/Verification Case Usage Example.sysml";

/// `IndividualTest.sysml` and `Verification Case Usage Example.sysml`, read for their
/// individual definitions:
///
/// ```text
///   IndividualTest   line  2:  individual def IO1;                    an IndividualDefinition
///                    lines 3, 7, 14, 18, 25, 29, 36:  individual <kind> def ...
///                              occurrence, item, part and action definitions, each
///                              with an `individual` OccurrenceDefinitionPrefix
///   Verification     line 30:  individual def TestSystem :> MassVerificationSystem;
///                    lines 32, 33:  individual def TestVehicle1 / TestVehicle2 :> Vehicle;
///                    line 35:  individual testSystem : TestSystem ...  a usage, no `def`
/// ```
///
/// `IndividualDefinition : OccurrenceDefinition = BasicDefinitionPrefix? isIndividual ?=
/// 'individual' DefinitionExtensionKeyword* 'def' Definition ownedRelationship +=
/// EmptyMultiplicityMember` (`SysML` 8.2.2.9.1): "individual may be used in place of the
/// kind keyword" (7.9.4, receipt 8c84370d). The first file is chosen because it writes
/// this production once beside seven definitions whose `individual` stands before a kind
/// keyword, which are other productions; the second because it writes it beside an
/// individual usage. Each owns its `EmptyMultiplicityMember`, which the clause puts there
/// and which spans no text.
#[test]
fn the_individual_definitions_of_two_files_read_their_keywords_and_definition() {
    assert_eq!(
        individual_definitions_in(&tree_of(INDIVIDUAL_TEST)),
        [bare_individual("IO1")],
        "line 2, and none of the definitions whose individual is a prefix's"
    );
    assert_eq!(
        individual_definitions_in(&tree_of(VERIFICATION_CASE_USAGE_EXAMPLE)),
        [
            bare_individual("TestSystem :> MassVerificationSystem"),
            bare_individual("TestVehicle1 :> Vehicle"),
            bare_individual("TestVehicle2 :> Vehicle"),
        ],
        "lines 30, 32 and 33, in source order, and not the usage on line 35"
    );
}

/// An individual definition's prefix slots, which no corpus file writes.
///
/// `BasicDefinitionPrefix? 'individual' DefinitionExtensionKeyword* 'def'` (`SysML`
/// 8.2.2.9.1), with `BasicDefinitionPrefix = isAbstract ?= 'abstract' | isVariation ?=
/// 'variation'` and `DefinitionExtensionKeyword = ownedRelationship +=
/// PrefixMetadataMember`, a `#` and a name (8.2.2.6.1). Every individual definition in the
/// corpus writes `individual def` and nothing else before its name (`grep -rnE
/// '\b(abstract|variation)\s+individual\s+def\b|\bindividual\s+#\S+\s+def\b'
/// vendor/corpus` finds none), so `basic_prefix()` and `extension_keywords()` would answer
/// nothing on every corpus tree whatever they did. This text writes each slot: one with
/// `abstract` and one keyword, one with `variation` and two, whose order is the source's.
#[test]
fn an_individual_definition_reads_the_prefix_slots_the_corpus_never_writes() {
    const PREFIXED: &str = "package P {\n\
                            \tmetadata def M;\n\
                            \tmetadata def N;\n\
                            \tabstract individual #M def X;\n\
                            \tvariation individual #N #M def Y;\n\
                            }\n";

    let parse = trees::parsed(PREFIXED, sv2_syntax::Language::SysMl);
    assert!(
        parse.errors().is_empty(),
        "the clauses admit this text, so a diagnostic here is the parser disagreeing \
         with them, not the test being wrong: {:?}",
        parse.errors()
    );
    let written = |basic: &str, keywords: &[&str], declaration: &str| {
        (
            Some(basic.to_owned()),
            Some("individual".to_owned()),
            keywords
                .iter()
                .map(|&keyword| keyword.to_owned())
                .collect::<Vec<_>>(),
            Some("def".to_owned()),
            Some(declaration.to_owned()),
            true,
        )
    };
    assert_eq!(
        individual_definitions_in(&parse.syntax()),
        [
            written("abstract", &["#M"], "X"),
            written("variation", &["#N", "#M"], "Y"),
        ],
        "X and Y, each slot as written, in source order"
    );
}

/// An individual definition's slots and multiplicity are its own, not those of one nested
/// in its body.
///
/// `IndividualDefinition = BasicDefinitionPrefix? 'individual'
/// DefinitionExtensionKeyword* 'def' Definition EmptyMultiplicityMember` (`SysML`
/// 8.2.2.9.1), and a `Definition`'s body admits another definition (`DefinitionBodyItem`,
/// 8.2.2.6.1). Here X writes no basic prefix and no `#` keyword, and Y, in X's body, writes
/// both. X's multiplicity is the last element X owns, after its body, so in preorder Y's
/// comes first. Reading descendants rather than children would hand X Y's `abstract`, Y's
/// `#M` and Y's multiplicity. No corpus file nests an individual definition (none writes
/// a prefix slot on one at all), so only this text can tell the two apart.
#[test]
fn an_individual_definition_does_not_claim_the_slots_of_one_in_its_body() {
    const NESTED: &str = "package P {\n\
                          \tmetadata def M;\n\
                          \tindividual def X {\n\
                          \t\tabstract individual #M def Y;\n\
                          \t}\n\
                          }\n";

    let parse = trees::parsed(NESTED, sv2_syntax::Language::SysMl);
    assert!(
        parse.errors().is_empty(),
        "the clauses admit this text, so a diagnostic here is the parser disagreeing \
         with them, not the test being wrong: {:?}",
        parse.errors()
    );
    let root = parse.syntax();
    let individuals: Vec<sv2_ast::IndividualDefinition> = root
        .descendants()
        .filter_map(sv2_ast::IndividualDefinition::cast)
        .collect();
    assert_eq!(
        individuals
            .iter()
            .map(|individual| (
                individual
                    .definition()
                    .and_then(|definition| definition.declaration())
                    .map(|declaration| declaration.text().to_string()),
                individual
                    .basic_prefix()
                    .map(|basic| basic.text().to_string()),
                individual
                    .extension_keywords()
                    .map(|keyword| keyword.text().to_string())
                    .collect::<Vec<_>>(),
                individual
                    .multiplicity()
                    .and_then(|multiplicity| multiplicity.parent())
                    .as_ref()
                    == Some(individual.syntax()),
            ))
            .collect::<Vec<_>>(),
        [
            (Some("X".to_owned()), None, vec![], true),
            (
                Some("Y".to_owned()),
                Some("abstract".to_owned()),
                vec!["#M".to_owned()],
                true
            ),
        ],
        "X has no slot written and owns its own multiplicity; Y has both slots and its own"
    );
}

/// A requirement or concern definition's two keywords, what its declaration declares, and
/// the first token of its body: `;` or `{`.
type RequirementShapedRead = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// The first token of `body`, as written.
fn opening(body: &SyntaxNode) -> Option<String> {
    body.first_token().map(|token| token.text().to_owned())
}

const REQUIREMENT_TEST: &str =
    "vendor/corpus/sysml/src/examples/Simple Tests/RequirementTest.sysml";

/// `RequirementTest.sysml`, read for its requirement definitions:
///
/// ```text
///   line  5:  requirement def R { ... }               R
///   line  9:    requirement;                          a RequirementUsage
///   line 10:    requirement def <'1'> A { ... }       nested in R, with a short name
///   line 15:  requirement def R1 { ... }              R1
///   lines 20, 25:  requirement r : R; / r1 : R1;      RequirementUsages
///   line 33:  requirement def R2 { ... }              R2
/// ```
///
/// `RequirementDefinition = OccurrenceDefinitionPrefix 'requirement' 'def'
/// DefinitionDeclaration RequirementBody` (`SysML` 8.2.2.21.1): no `Definition` node, the
/// declaration and the body are the definition's own. The file writes the definition at
/// the top and nested in another's body, with a short name, beside usages of the same
/// keyword that write no `def` (`RequirementUsage`, 8.2.2.21.2). In preorder the nested
/// one comes before R's later siblings.
///
/// No corpus file writes a prefix on a requirement definition (`grep -rnE
/// '\b(abstract|variation|individual)\s+requirement\s+def\b|#\S+\s+requirement\s+def'
/// vendor/corpus` finds none), so this case cannot tell a `prefix()` that answers nothing
/// from one that answers an empty prefix. The sweep can: it asserts the prefix node is
/// there on every requirement definition in every tree.
#[test]
fn the_requirement_definitions_of_requirement_test_read_their_keywords_declaration_and_body() {
    let read: Vec<RequirementShapedRead> = tree_of(REQUIREMENT_TEST)
        .descendants()
        .filter_map(sv2_ast::RequirementDefinition::cast)
        .map(|requirement| {
            (
                requirement
                    .requirement_keyword()
                    .map(|token| token.text().to_owned()),
                requirement
                    .def_keyword()
                    .map(|token| token.text().to_owned()),
                requirement
                    .declaration()
                    .map(|declaration| declaration.text().to_string()),
                requirement.body().as_ref().and_then(opening),
            )
        })
        .collect();
    let def = |declaration: &str| {
        (
            Some("requirement".to_owned()),
            Some("def".to_owned()),
            Some(declaration.to_owned()),
            Some("{".to_owned()),
        )
    };
    assert_eq!(
        read,
        [def("R"), def("<'1'> A"), def("R1"), def("R2")],
        "lines 5, 10, 15 and 33, in preorder, and none of the usages"
    );
}

/// `ViewTest.sysml`, read for its concern definitions:
///
/// ```text
///   line  9:  concern def C { subject; stakeholder s : S; }   C
///   line 14:  concern c : C { ... }                           a ConcernUsage
///   line 29:  frame concern c1;                               a concern usage, under `frame`
///   line 30:  concern c2;                                     a ConcernUsage
/// ```
///
/// `ConcernDefinition = OccurrenceDefinitionPrefix 'concern' 'def' DefinitionDeclaration
/// RequirementBody` (`SysML` 8.2.2.21.3): "specified exactly like a regular requirement
/// definition" with the kind keyword `concern` (7.21.3, receipt 0e50a373), so no
/// `Definition` node. The file writes one with a body holding a subject and a
/// stakeholder, which are `RequirementBody` items and no definition body's, beside usages
/// of the same keyword that write no `def` (`ConcernUsage`, 8.2.2.21.3).
///
/// No corpus file writes a prefix on a concern definition (all three are listed by
/// `grep -rnE '\bconcern\s+def\b' vendor/corpus`), so this case cannot tell a `prefix()`
/// that answers nothing from one that answers an empty prefix. The sweep can: it asserts
/// the prefix node is there on every concern definition in every tree.
#[test]
fn the_concern_definition_of_view_test_reads_its_keywords_declaration_and_body() {
    let read: Vec<RequirementShapedRead> = tree_of(VIEW_TEST)
        .descendants()
        .filter_map(sv2_ast::ConcernDefinition::cast)
        .map(|concern| {
            (
                concern
                    .concern_keyword()
                    .map(|token| token.text().to_owned()),
                concern.def_keyword().map(|token| token.text().to_owned()),
                concern
                    .declaration()
                    .map(|declaration| declaration.text().to_string()),
                concern.body().as_ref().and_then(opening),
            )
        })
        .collect();
    assert_eq!(
        read,
        [(
            Some("concern".to_owned()),
            Some("def".to_owned()),
            Some("C".to_owned()),
            Some("{".to_owned()),
        )],
        "line 9, and none of the three usages"
    );
}
