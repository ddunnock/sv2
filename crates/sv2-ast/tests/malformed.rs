// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! One malformed case per accessor (`.claude/rules/ast.md`).

mod trees;

use sv2_ast::{AstNode, Import, PackageMember, RootNamespace};
use sv2_syntax::{Language, SyntaxKind, SyntaxNode};

const UNCLOSED_BODY: &str = include_str!("malformed/root-namespace/unclosed-body.sysml");
const DECLARATION_NAMES_NOTHING: &str =
    include_str!("malformed/import/declaration-names-nothing.sysml");
const ORPHANED_VISIBILITY: &str =
    include_str!("malformed/package-member/orphaned-visibility.sysml");

/// Every `Import` in `root`, in document order.
fn imports(root: &SyntaxNode) -> Vec<Import> {
    root.descendants().filter_map(Import::cast).collect()
}

/// Every `PackageMember` in `root`, in document order.
fn members(root: &SyntaxNode) -> Vec<PackageMember> {
    root.descendants().filter_map(PackageMember::cast).collect()
}

/// `root-namespace/unclosed-body.sysml`: the first package's body is never closed, so
/// the parser's recovery reads to end of file, and the second `package` becomes one of
/// the first package's OWN members rather than a sibling of it at the root.
///
/// The source for that outcome is the parser, and only the parser: `body_elements` in
/// `crates/sv2-syntax/src/parser/namespace.rs` loops `while !self.at_end() && !at(until)`,
/// so a body whose terminator never arrives consumes the rest of the file. No ADR states
/// it and no clause states it — the specification says a package body closes with `}`
/// and stops there. What ADR-0004 supports is the shape, not the outcome: an error node
/// is just another node, so recovery keeps the partial tree instead of discarding it.
///
/// Of the three things a malformed case must prove (`.claude/rules/ast.md`):
///
/// 1. the parse still yields a tree — asserted;
/// 2. the accessor reports absence rather than guessing — asserted in both halves:
///    exactly one top-level member rather than the two the source text names, *and* a
///    diagnostic saying why, because absence that nothing reported is absence guessed;
/// 3. the neighbouring accessors still answer — asserted as far as THIS fixture allows,
///    which is not far: it contains no import, so the only thing `Import` can be asked
///    is whether it invents one, and it must not. The substantive form of part 3 lives
///    in the import case below, where the damaged subject is the import and the
///    neighbour asked is `RootNamespace`, which does have something to say about it.
#[test]
fn unclosed_body_nests_the_next_package_rather_than_dropping_it() {
    let parse = trees::parsed(UNCLOSED_BODY, Language::SysMl);

    // 1. the parse still yields a tree: the root is still `RootNamespace`.
    let namespace =
        RootNamespace::cast(parse.syntax()).expect("the root node is always RootNamespace");

    // 2a. absence, not a guess: the second package was swallowed into the first's
    // unclosed body, so the ROOT has exactly one member, not the two the text names.
    let members: Vec<_> = namespace.members().collect();
    assert_eq!(
        members.len(),
        1,
        "the root must not be reported as having two members when the recovery nested \
         the second package inside the first"
    );

    // 2b. and the absence was reported: recovery that silently accepted an unclosed
    // body at end of file would leave 2a green while telling the caller nothing.
    assert!(
        !parse.errors().is_empty(),
        "an unclosed package body must produce a diagnostic, not a quietly short tree"
    );

    // The one member is a genuine node, and it is the FIRST package carrying the second
    // inside it -- `len() == 1` alone holds just as well in a world where the second
    // package was dropped from the tree entirely, which is the opposite claim.
    let only_member = &members[0];
    assert_eq!(only_member.kind(), SyntaxKind::PackageMember);
    assert!(
        only_member.text().to_string().contains("Formed"),
        "the swallowed package must be nested inside the unclosed one, not lost: \
         the surviving member's text does not mention it"
    );

    // 3. the neighbour, for what it is worth here: this fixture declares no import, so
    // `Import` must report none. An accessor that cast something else to `Import`
    // because the tree was damaged would fail this.
    assert!(
        imports(&parse.syntax()).is_empty(),
        "no import is written in this fixture, so none may be reported"
    );
}

/// `import/declaration-names-nothing.sysml`: `public import ;` names nothing, so the
/// declaration's nodes are present and **zero width**.
///
/// The source is the parser, and only the parser: `import_declaration` opens
/// `ImportDeclaration`, `imported` takes a checkpoint and calls `qualified_name`, and
/// `qualified_name` opens its own node before calling `expect_name`, which reports a
/// diagnostic rather than consuming anything (`crates/sv2-syntax/src/parser/import.rs`).
/// No clause states this and no ADR states it; the specification says only what an
/// import must contain (`SysML` 8.2.2.5.1, `KerML` 8.2.3.4.2).
///
/// This is the inverse of the root-namespace case, and the reason it was chosen. There,
/// the damage removed a member and the risk was reporting too many. Here nothing is
/// missing from the tree at all: `declaration()` returns `Some`, and a caller reading
/// that as "the import names something" is **confidently wrong**. Presence is not
/// evidence, which is precisely what an absence check cannot catch.
///
/// Parts 1, 2 and 3 of `.claude/rules/ast.md` are all discharged here — see the comments
/// at each assertion. Part 3 is no longer vacant: `RootNamespace` is a real neighbour
/// with a real answer about this tree.
#[test]
fn an_import_naming_nothing_still_reports_a_declaration_that_names_nothing() {
    let parse = trees::parsed(DECLARATION_NAMES_NOTHING, Language::SysMl);
    let root = parse.syntax();

    // 1. the parse still yields a tree, and the import is in it.
    let found = imports(&root);
    assert_eq!(
        found.len(),
        1,
        "the fixture writes one import, and a missing name must not cost the import node"
    );
    let import = &found[0];

    // 2a. the half that is intact must read as intact: visibility is mandatory in this
    // production and is written here, so an accessor that reported the whole import as
    // damaged would be as wrong as one that reported it as fine.
    let visibility = import
        .visibility()
        .expect("`public` is written, so the VisibilityIndicator is there");
    assert_eq!(visibility.kind(), SyntaxKind::VisibilityIndicator);
    assert_eq!(visibility.text().to_string(), "public");
    assert!(
        !import.is_import_all(),
        "no `all` is written, so none may be reported"
    );

    // 2b. the damaged half, and the whole point of this case: the declaration node is
    // PRESENT, so `is_some()` proves nothing, and it is EMPTY, which is the only thing
    // that tells the caller the import names nobody. Both halves are asserted, because
    // either alone is satisfied by a defect: `is_some()` alone passes an accessor that
    // reports a name it invented, and emptiness alone passes one that dropped the node.
    let declaration = import
        .declaration()
        .expect("recovery opens the declaration before it looks for a name, so it is there");
    assert_eq!(declaration.kind(), SyntaxKind::ImportDeclaration);
    assert_eq!(
        declaration.text_range().len(),
        0.into(),
        "the declaration names nothing, so it must be zero width -- a non-empty range \
         here would mean the accessor is reporting text the author never wrote"
    );

    // 2c. and the emptiness was reported: a parser that accepted `import ;` in silence
    // would leave 2b green while telling the caller nothing was wrong.
    assert!(
        !parse.errors().is_empty(),
        "an import that names nothing must produce a diagnostic, not a quietly empty node"
    );

    // 3. the neighbouring accessor still answers, and answers correctly: the root holds
    // exactly this import as its one member. This is the part the root-namespace case
    // above could not discharge, because it had no neighbour to ask.
    let namespace = RootNamespace::cast(root.clone()).expect("the root is always RootNamespace");
    let members: Vec<_> = namespace.members().collect();
    assert_eq!(
        members.len(),
        1,
        "the damaged import is still the root's one member; a comment is trivia, not a member"
    );
    assert_eq!(members[0], *import.syntax());
}

/// `package-member/orphaned-visibility.sysml`: `package P { private ; part def A; }`.
/// The author's `private` is recovered into an `Error` **sibling**, and the member that
/// follows it has an empty prefix.
///
/// The source is the parser, and only the parser: `recover_statement`
/// (`crates/sv2-syntax/src/parser/recovery.rs`) opens a `SyntaxKind::Error`, always takes
/// the first token, and stops at a `;` at the depth it began from, so both `private` and
/// `;` land in that node; `at_member_keyword` is what then ends the run, which is why
/// `part def A;` survives as a member of its own rather than being swallowed. The clause
/// (`SysML` 8.2.2.5.1) states only what a member is.
///
/// This is the third distinct failure mode of the three accessors so far, and that is why
/// this condition was chosen over re-pinning either of the others:
///
/// | case | the damage |
/// |---|---|
/// | `root-namespace/unclosed-body` | a member MOVED — reporting too many |
/// | `import/declaration-names-nothing` | a node PRESENT but empty — presence is not evidence |
/// | this one | the text is in the tree but attached to the WRONG node — attribution |
///
/// Nothing is missing here and nothing is wrongly empty: `prefix()` is present and empty,
/// which for a member written without a visibility is also the perfectly healthy shape
/// (`MemberPrefix`'s node is built either way, by deliberate choice — see
/// `member_prefix` in `crates/sv2-syntax/src/parser/namespace.rs`). The defect an
/// accessor could introduce is therefore not emptiness but *reaching*: looking at a
/// sibling, or at preceding text, to find the visibility the author clearly intended.
#[test]
fn an_orphaned_visibility_does_not_belong_to_the_member_after_it() {
    let parse = trees::parsed(ORPHANED_VISIBILITY, Language::SysMl);
    let root = parse.syntax();

    // 1. the parse still yields a tree. The outer package is a member, and the inner
    // `part def A;` is another, so the file holds exactly two.
    let found = members(&root);
    assert_eq!(
        found.len(),
        2,
        "the outer package and the surviving `part def A;` are both PackageMembers"
    );
    let surviving = &found[1];
    assert_eq!(
        surviving.syntax().text().to_string(),
        "part def A;",
        "the second member is the item that survived the recovery run"
    );

    // 2a. attribution, the point of this case: the surviving member's prefix is present
    // and EMPTY. The `private` the author wrote is not its visibility.
    let prefix = surviving
        .prefix()
        .expect("MemberPrefix is built whether or not a visibility is written");
    assert_eq!(prefix.kind(), SyntaxKind::MemberPrefix);
    assert_eq!(
        prefix.text_range().len(),
        0.into(),
        "the member was written with no visibility of its own, so its prefix is empty"
    );
    assert!(
        prefix.first_child().is_none(),
        "an empty prefix holds no VisibilityIndicator"
    );

    // 2b. and the `private` really is elsewhere in the tree, as a sibling Error — so
    // this test is not merely observing that a prefix is empty, which it would be for
    // any ordinary member. The whole trap exists because the text IS present.
    let orphan = surviving
        .syntax()
        .prev_sibling()
        .expect("the Error node recovery built stands immediately before this member");
    assert_eq!(orphan.kind(), SyntaxKind::Error);
    assert_eq!(
        orphan.text().to_string().trim(),
        "private ;",
        "recovery took the first token and stopped at the `;` at its own depth"
    );

    // 2c. and it was reported. Recovery that silently dropped the visibility would leave
    // 2a and 2b green while telling the caller nothing was wrong.
    assert!(
        !parse.errors().is_empty(),
        "an orphaned visibility must produce a diagnostic, not a quietly prefixless member"
    );

    // 3. the neighbouring accessors still answer. `RootNamespace` still reports its one
    // member, and `Import` — which also carries a visibility, required rather than
    // optional — must not claim anything here.
    let namespace = RootNamespace::cast(root.clone()).expect("the root is always RootNamespace");
    let top: Vec<_> = namespace.members().collect();
    assert_eq!(
        top.len(),
        1,
        "the root holds the outer package and nothing else"
    );
    assert_eq!(top[0], *found[0].syntax());
    assert!(
        imports(&root).is_empty(),
        "no import is written, so none may be reported from a stray visibility"
    );
}
