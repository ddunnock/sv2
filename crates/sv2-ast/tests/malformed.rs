// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! One malformed case per accessor (`.claude/rules/ast.md`).

mod trees;

use sv2_ast::{
    AliasMember, AstNode, ElementFilterMember, Import, MemberPrefix, PackageMember, QualifiedName,
    RelationshipBody, RootNamespace,
};
use sv2_syntax::{Language, SyntaxKind, SyntaxNode};

const UNCLOSED_BODY: &str = include_str!("malformed/root-namespace/unclosed-body.sysml");
const DECLARATION_NAMES_NOTHING: &str =
    include_str!("malformed/import/declaration-names-nothing.sysml");
const ORPHANED_VISIBILITY: &str =
    include_str!("malformed/package-member/orphaned-visibility.sysml");
const SHORT_NAME_UNCLOSED: &str = include_str!("malformed/alias-member/short-name-unclosed.sysml");

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
    assert_eq!(
        visibility.keyword().map(|token| token.kind()),
        Some(SyntaxKind::KwPublic),
        "the indicator is reached as a typed view and reports `public` by token kind"
    );
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
    // The kind no longer needs asserting -- `cast` guarantees it -- so the typed view
    // buys a stronger statement instead: the alternative inside is present and is itself
    // empty, which is what "opened before the name was sought" means one level down.
    assert_eq!(
        declaration.imported().map(|node| node.kind()),
        Some(SyntaxKind::MembershipImport),
        "no `*` was written, so the checkpoint closed as a membership import"
    );
    assert_eq!(
        declaration.syntax().text_range().len(),
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
    assert_eq!(prefix.syntax().kind(), SyntaxKind::MemberPrefix);
    assert_eq!(
        prefix.syntax().text_range().len(),
        0.into(),
        "the member was written with no visibility of its own, so its prefix is empty"
    );
    assert!(
        prefix.visibility().is_none(),
        "an empty prefix reports no visibility -- asked of MemberPrefix itself, which is \
         the accessor that owns the question"
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

/// `alias-member/short-name-unclosed.sysml`: `alias <c Car for Automobile;`. The `>` is
/// not written, so both names sit in the tree as adjacent `BasicName` siblings.
///
/// The source is the parser, and only the parser: `alias_member`
/// (`crates/sv2-syntax/src/parser/namespace.rs`) reads `<`, takes a name, then calls
/// `expect(SyntaxKind::Gt, ...)`, which reports a diagnostic **without consuming**, and
/// carries on to read the optional member name. The clause (`SysML` 8.2.2.5.1) says only
/// that a short name is written `'<' NAME '>'`.
///
/// What this case pins that the first three could not: both slots hold the same TOKEN
/// KIND, so only position tells them apart, and the delimiter a positional rule would
/// key on is the one character missing. A rule phrased "the name between `<` and `>`"
/// reports no short name and hands `c` back as the member name — wrong about both slots
/// at once. The rule that survives is `<` opens the slot, `>` closes it, and the slot
/// holds at most one name, so the second falls through.
#[test]
fn an_unclosed_short_name_does_not_swallow_the_member_name() {
    let parse = trees::parsed(SHORT_NAME_UNCLOSED, Language::SysMl);
    let root = parse.syntax();

    // 1. the parse still yields a tree, and the alias is in it.
    let found: Vec<AliasMember> = root.descendants().filter_map(AliasMember::cast).collect();
    assert_eq!(found.len(), 1, "the fixture writes one alias");
    let alias = &found[0];

    // 2a. both slots are filled, and filled with the right token each. This is the whole
    // case: `c` opened the short-name slot, and `Car` fell through to the member name
    // because the slot was never closed and holds at most one.
    assert_eq!(
        alias.short_name().map(|token| token.text().to_owned()),
        Some("c".to_owned()),
        "`c` stands after the `<`, so it is the short name even with no `>` to close it"
    );
    assert_eq!(
        alias.name().map(|token| token.text().to_owned()),
        Some("Car".to_owned()),
        "`Car` is the member name; a rule keyed on the missing `>` would report it as \
         the short name, or report it as nothing at all"
    );
    assert_ne!(
        alias.short_name().map(|t| t.text_range()),
        alias.name().map(|t| t.text_range()),
        "the two slots must never be the same token"
    );

    // 2b. the rest of the member is intact and must read as intact, or an accessor that
    // gave up on the whole node would pass 2a's shape by accident.
    assert_eq!(
        alias
            .target()
            .map(|target| target.syntax().text().to_string())
            .unwrap_or_default(),
        "Automobile",
        "the target after `for` is unaffected by the missing `>`"
    );
    assert_eq!(
        alias
            .body()
            .and_then(|body| body.terminator())
            .map(|token| token.text().to_owned()),
        Some(";".to_owned())
    );

    // 2c. and the missing `>` was reported. A parser that accepted it in silence would
    // leave 2a green while telling the caller the text was fine.
    assert!(
        parse
            .errors()
            .iter()
            .any(|diagnostic| diagnostic.message().contains('>')),
        "the missing `>` must be diagnosed, not quietly tolerated: {:?}",
        parse.errors()
    );

    // 3. the neighbouring accessors still answer. The alias is the root's one member,
    // and `PackageMember` — which shares the MemberPrefix shape — must not claim it.
    let namespace = RootNamespace::cast(root.clone()).expect("the root is always RootNamespace");
    let top: Vec<_> = namespace.members().collect();
    assert_eq!(top.len(), 1, "the alias is the root's one member");
    assert_eq!(top[0], *alias.syntax());
    assert!(
        members(&root).is_empty(),
        "an alias is a Membership, not an OwningMembership, so it is not a PackageMember"
    );
}

const SHORT_NAME_EMPTY: &str = include_str!("malformed/alias-member/short-name-empty.sysml");

/// `alias-member/short-name-empty.sysml`: `alias <> Car for Automobile;`. The slot opens
/// and closes with nothing in it, so `Car` is the MEMBER name, not the short name.
///
/// The second fixture for one kind, and deliberately so: this is the branch of the
/// positional rule that the unclosed case cannot distinguish. "The first name after `<`"
/// satisfies `alias <c Car ...` and inverts both slots here. Only "`<` opens the slot,
/// `>` closes it, the slot holds at most one name" is right in both, and it takes both
/// fixtures to say so. `ast_coverage.py` allows several conditions per kind for this.
#[test]
fn an_empty_short_name_slot_leaves_the_member_name_where_it_is() {
    let parse = trees::parsed(SHORT_NAME_EMPTY, Language::SysMl);
    let root = parse.syntax();

    let found: Vec<AliasMember> = root.descendants().filter_map(AliasMember::cast).collect();
    assert_eq!(found.len(), 1, "the fixture writes one alias");
    let alias = &found[0];

    assert_eq!(
        alias.short_name().map(|token| token.text().to_owned()),
        None,
        "the slot was closed empty, so there is no short name to report"
    );
    assert_eq!(
        alias.name().map(|token| token.text().to_owned()),
        Some("Car".to_owned()),
        "`Car` stands after a CLOSED slot, so it is the member name"
    );
    assert!(
        parse
            .errors()
            .iter()
            .any(|diagnostic| diagnostic.message().contains("short name")),
        "the empty slot must be diagnosed: {:?}",
        parse.errors()
    );

    // 3. the neighbour still answers.
    let namespace = RootNamespace::cast(root.clone()).expect("the root is always RootNamespace");
    assert_eq!(namespace.members().count(), 1);
}

const VISIBILITIES_LOST: &str =
    include_str!("malformed/member-prefix/visibilities-lost-to-recovery.sysml");

/// `member-prefix/visibilities-lost-to-recovery.sysml`:
/// `private protected part def A;`. Two visibilities, both recovered into one `Error`,
/// and the surviving prefix reports neither.
///
/// `MemberPrefix` is `( visibility = VisibilityIndicator )?`, so its own shape cannot be
/// wrong — every one in every tree holds nothing or one `VisibilityIndicator`, which
/// `tests/sweep.rs` asserts over all 610 trees. There is no violated shape to pin, so
/// this case pins the one thing that can go wrong instead: **attribution**.
///
/// The source is the parser: `recover_statement`
/// (`crates/sv2-syntax/src/parser/recovery.rs`) takes the first token always and runs to
/// a member keyword, so `private protected` lands in a single `Error` and `part def A;`
/// survives as its own member. The clause (`SysML` 8.2.2.5.1) says only that a visibility
/// in front of a member belongs to it.
///
/// Why two *disagreeing* words rather than `private private`: an accessor that went
/// looking sideways for "the visibility the author meant" finds two candidates and no
/// principled way to choose. There is nothing to invent, which states the rule more
/// sharply than a case where guessing would have happened to be right.
#[test]
fn a_prefix_does_not_claim_a_visibility_recovery_took() {
    let parse = trees::parsed(VISIBILITIES_LOST, Language::SysMl);
    let root = parse.syntax();

    // 1. the parse still yields a tree, with the surviving member in it.
    let found = members(&root);
    assert_eq!(found.len(), 1, "`part def A;` survives as the one member");
    let prefix = found[0]
        .prefix()
        .expect("MemberPrefix is built whether or not a visibility is written");

    // 2a. attribution: the prefix reports no visibility, although two were written
    // immediately before it.
    assert!(
        prefix.visibility().is_none(),
        "neither lost visibility is this member's, so none may be reported"
    );
    assert_eq!(
        prefix.syntax().text_range().len(),
        0.into(),
        "a prefix with no visibility of its own is zero width"
    );

    // 2b. and both words really are in the tree, in one Error node — so this is a
    // statement about attribution and not merely about an empty prefix, which is the
    // ordinary shape of most members in the corpus.
    let orphan = found[0]
        .syntax()
        .prev_sibling()
        .expect("the Error recovery built stands immediately before the member");
    assert_eq!(orphan.kind(), SyntaxKind::Error);
    assert_eq!(
        orphan.text().to_string().trim(),
        "private protected",
        "recovery took both words into one node, not one each"
    );

    // 2c. one diagnostic for two lost words: the count is not a measure of how much was
    // lost, and a caller using it that way would under-count here.
    assert_eq!(
        parse.errors().len(),
        1,
        "the pair is reported once: {:?}",
        parse.errors()
    );

    // 3. the neighbours still answer. Every `MemberPrefix` in this tree is empty — the
    // outer member has none either — and no accessor invents one.
    let prefixes: Vec<MemberPrefix> = root.descendants().filter_map(MemberPrefix::cast).collect();
    assert!(
        !prefixes.is_empty(),
        "the surviving member has a prefix, so there is at least one to check"
    );
    assert!(
        prefixes.iter().all(|prefix| prefix.visibility().is_none()),
        "no prefix anywhere in this tree was written a visibility"
    );
    assert!(
        imports(&root).is_empty() && root.descendants().filter_map(AliasMember::cast).count() == 0,
        "no import and no alias is written, so neither may be reported"
    );
}

const CONDITION_MISSING: &str =
    include_str!("malformed/element-filter-member/condition-missing.sysml");

/// `element-filter-member/condition-missing.sysml`: `private filter ;`. The one slot of
/// this production that is not optional is left empty, and the slot comes back EMPTY
/// rather than holding an `Error`.
///
/// `OwnedExpression` is not optional in `ElementFilterMember` (`SysML` 8.2.2.5.1,
/// `KerML` 8.2.5.13), and no file in the corpus omits it — 25 filter members, each with
/// exactly one condition — so this is a hand-written case for a distinction the pinned
/// corpus does not contain (`.claude/rules/tests.md`).
///
/// The source for the outcome is the parser: `owned_expression` climbs to
/// `base_expression`, whose final `else` calls `error_expected("an expression")`
/// (`crates/sv2-syntax/src/parser/primary.rs`), and `error_expected` reports **without
/// consuming**. So nothing is built for the slot, the `;` survives for the member's own
/// `expect`, and the member closes normally. No clause states that and no ADR does;
/// ADR-0004 supports only the shape, that every byte still reaches the tree.
///
/// The three things a malformed case must prove (`.claude/rules/ast.md`):
///
/// 1. the parse still yields a tree, with the filter member in it;
/// 2. the accessor reports absence rather than guessing — `filter()` is `None`, *and* a
///    diagnostic says why. Both halves matter more here than usual: the slot holds no
///    `Error` node, so the diagnostic is the only record that anything was written
///    there at all;
/// 3. the neighbours still answer, and substantively: the member's own prefix still
///    reports `private`, and the import in the same body comes back whole. The damage is
///    one slot wide, and an accessor that spread it would lose facts that are still
///    there.
#[test]
fn a_filter_with_no_condition_reports_none_rather_than_an_error_node() {
    let parse = trees::parsed(CONDITION_MISSING, Language::SysMl);
    let root = parse.syntax();

    // 1. the parse still yields a tree, and the member is in it.
    let filters: Vec<ElementFilterMember> = root
        .descendants()
        .filter_map(ElementFilterMember::cast)
        .collect();
    assert_eq!(filters.len(), 1, "the fixture writes one filter member");
    let filter = &filters[0];

    // 2a. absence, not a guess: the condition slot is empty.
    assert_eq!(
        filter.filter(),
        None,
        "no condition was written, so none may be reported"
    );

    // 2b. and empty means EMPTY — there is no `Error` node standing in the slot, which is
    // what a caller would otherwise look for. The member's only node child is its prefix.
    let node_children: Vec<SyntaxKind> = filter.syntax().children().map(|c| c.kind()).collect();
    assert_eq!(
        node_children,
        [SyntaxKind::MemberPrefix],
        "the prefix is the only node child; nothing stands where the condition goes"
    );

    // 2c. something reported it. Absence that nothing reported is absence guessed, and
    // here the diagnostic is the ONLY record that a condition was expected at all.
    assert!(
        parse
            .errors()
            .iter()
            .any(|diagnostic| diagnostic.message().contains("an expression")),
        "the empty condition must be diagnosed: {:?}",
        parse.errors()
    );

    // 2d. the member still closed: the `;` was not consumed by the failed read, so the
    // damage is one slot wide and not a run of recovery.
    assert!(
        filter.syntax().text().to_string().trim_end().ends_with(';'),
        "the member closes with its own semicolon: {:?}",
        filter.syntax().text().to_string()
    );

    // 3a. the neighbour that shares this node: the prefix keeps its visibility.
    let prefix = filter
        .prefix()
        .expect("MemberPrefix is built whether or not a visibility is written");
    assert_eq!(
        prefix
            .visibility()
            .and_then(|visibility| visibility.keyword())
            .map(|token| token.text().to_string())
            .as_deref(),
        Some("private"),
        "`private` is this member's own, and the damaged condition does not cost it"
    );

    // 3b. the neighbour in the same body: the import is whole, visibility and all.
    let found = imports(&root);
    assert_eq!(found.len(), 1, "the fixture writes one import");
    assert_eq!(
        found[0]
            .visibility()
            .and_then(|visibility| visibility.keyword())
            .map(|token| token.text().to_string())
            .as_deref(),
        Some("public"),
        "the import above the filter is untouched"
    );
    assert!(
        found[0].declaration().is_some(),
        "and it still names what it imports"
    );
}

const GLOBAL_QUALIFIER_NAMES_NOTHING: &str =
    include_str!("malformed/qualified-name/global-qualifier-names-nothing.sysml");
const SEGMENTS_LOST: &str =
    include_str!("malformed/qualified-name/segments-lost-to-recovery.sysml");

/// Every `QualifiedName` in `root`, in document order.
fn qualified_names(root: &SyntaxNode) -> Vec<QualifiedName> {
    root.descendants().filter_map(QualifiedName::cast).collect()
}

/// `qualified-name/global-qualifier-names-nothing.sysml`: `public import $::;`. The final
/// `NAME` is not optional (`KerML` 8.2.3.4.1), and here nothing was written for it.
///
/// `expect_name` reports without consuming (`crates/sv2-syntax/src/parser/recovery.rs`),
/// so the node is built holding both punctuation tokens and no name token at all. The
/// three accessors must then disagree in exactly the right way: the `$` really is there,
/// and no segment is.
///
/// The three things a malformed case must prove (`.claude/rules/ast.md`):
///
/// 1. the parse still yields a tree, with the qualified name node in it;
/// 2. absence, reported: `segments()` is empty and `name()` is `None`, *and* a diagnostic
///    says a name was expected. `is_global()` stays true, which is the part worth
///    asserting rather than assuming — the qualifier is punctuation about where
///    resolution starts, not a name to resolve, so it does not go away with the segments;
/// 3. the neighbour still answers: `Import` reports its visibility and its declaration,
///    because the damage is inside the name and not to the import around it.
#[test]
fn a_global_qualifier_with_no_segment_names_nothing() {
    let parse = trees::parsed(GLOBAL_QUALIFIER_NAMES_NOTHING, Language::SysMl);
    let root = parse.syntax();

    // 1. the parse still yields a tree, and the node is in it.
    let names = qualified_names(&root);
    assert_eq!(names.len(), 1, "the fixture writes one qualified name");
    let name = &names[0];

    // 2a. absence: no segment was written, so none may be reported, and there is no
    // final segment to identify either.
    assert_eq!(
        name.segments().count(),
        0,
        "`$::` writes no segment name: {:?}",
        name.segments()
            .map(|token| token.text().to_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        name.name().map(|token| token.text().to_owned()),
        None,
        "with no segments there is no final segment to report"
    );

    // 2b. and the qualifier does NOT go away with them: it is punctuation saying where
    // resolution starts (8.2.3.4.1 Note 3), not a segment that could be missing.
    assert!(
        name.is_global(),
        "the `$` is written, so the qualifier is there however empty the name is"
    );

    // 2c. something reported it.
    assert!(
        parse
            .errors()
            .iter()
            .any(|diagnostic| diagnostic.message().contains("a name")),
        "the missing segment must be diagnosed: {:?}",
        parse.errors()
    );

    // 3. the neighbour still answers: the import around the damaged name is whole.
    let found = imports(&root);
    assert_eq!(found.len(), 1, "the fixture writes one import");
    assert_eq!(
        found[0]
            .visibility()
            .and_then(|visibility| visibility.keyword())
            .map(|token| token.text().to_string())
            .as_deref(),
        Some("public"),
        "the import's own visibility is untouched by the damage inside the name"
    );
    assert!(
        found[0].declaration().is_some(),
        "and it still has the declaration the name sits in"
    );
}

/// `qualified-name/segments-lost-to-recovery.sysml`: `alias X for A::::B;`. The name ends
/// at `A`, and `B` — the last name a reader sees — is not a segment of it.
///
/// The second fixture for this kind: the first pins absence, this one pins
/// **attribution**, which is the other way a token accessor goes wrong. The source is the
/// parser: `qualified_name` loops `while self.at(ColonColon) &&
/// self.name_follows_separator()` (`crates/sv2-syntax/src/parser/import.rs`), so a `::`
/// with no name after it is not part of the name, and the loop never runs.
///
/// 1. the parse still yields a tree, with a one-segment name in it;
/// 2. attribution, not a guess: `name()` is `A`, not `B`, although `B` is written last.
///    An accessor reading the node's text, or scanning past the node's own tokens, would
///    hand back `B`. The lost run really is in the tree, in one `Error`, so this is a
///    statement about attribution and not merely about a short name;
/// 3. the neighbour still answers: the alias still reports the target it does have.
#[test]
fn a_name_does_not_claim_a_segment_recovery_took() {
    let parse = trees::parsed(SEGMENTS_LOST, Language::SysMl);
    let root = parse.syntax();

    // 1. one qualified name, one segment, though the text writes three names.
    let names = qualified_names(&root);
    assert_eq!(names.len(), 1, "the fixture writes one qualified name");
    let name = &names[0];
    assert_eq!(
        name.segments()
            .map(|token| token.text().to_owned())
            .collect::<Vec<_>>(),
        ["A"],
        "the name ends at the separator that has no name after it"
    );

    // 2a. attribution: the final segment is `A`. `B` is the last name written, and it is
    // not this name's.
    assert_eq!(
        name.name().map(|token| token.text().to_owned()),
        Some("A".to_owned()),
        "`B` is not a segment of this name, so it may not be reported as its final one"
    );
    assert!(
        !name.is_global(),
        "no `$` is written, so the qualifier is not there"
    );

    // 2b. and `B` really is in the tree, in one Error node with the rest of the run —
    // so this says something about attribution, not merely that the name is short.
    let orphan = root
        .descendants()
        .find(|node| node.kind() == SyntaxKind::Error)
        .expect("recovery built an Error node for the run it could not read");
    assert_eq!(
        orphan.text().to_string().trim(),
        "::::B;",
        "recovery took the whole run into one node, the closing `;` included"
    );

    // 2c. one diagnostic for the whole run: the count is not a measure of how many
    // segments were lost.
    assert_eq!(
        parse.errors().len(),
        1,
        "the run is reported once: {:?}",
        parse.errors()
    );

    // 3. the neighbour still answers: the alias reports the target it does have, and its
    // own name slot is undamaged.
    let aliases: Vec<AliasMember> = root.descendants().filter_map(AliasMember::cast).collect();
    assert_eq!(aliases.len(), 1, "the fixture writes one alias");
    assert_eq!(
        aliases[0].name().map(|token| token.text().to_owned()),
        Some("X".to_owned()),
        "the alias's own name stands before the damage and is untouched"
    );
    assert!(
        aliases[0].target().is_some(),
        "and it still reports the target it does have"
    );
}

const FIRST_OF_TWO_LOST: &str =
    include_str!("malformed/visibility-indicator/first-of-two-lost-to-recovery.sysml");

/// `visibility-indicator/first-of-two-lost-to-recovery.sysml`:
/// `public private import A::*;`. The import's indicator is `private` — the SECOND word
/// — and `public` belongs to nobody.
///
/// Like `MemberPrefix`, this kind's own shape cannot be wrong: it is three keywords and
/// nothing else, every one of the 821 in the repository holds exactly one token, and an
/// empty one is unreachable from either caller (`body_elements` enters only on
/// `at_visibility`; a bare `import A::*;` is reported as "unexpected `import`" and never
/// reaches `import()`). So there is no violated shape to pin and no reachable absence
/// either, and this case pins **attribution** instead.
///
/// The source is the parser: `at_import` wants a visibility then `import`, which
/// `public private` is not, so `recover_statement`
/// (`crates/sv2-syntax/src/parser/recovery.rs`) takes `public` into an `Error` and
/// `private import A::*;` parses as an import of its own.
///
/// 1. the parse still yields a tree, with one import in it;
/// 2. attribution, not a guess: the indicator reports `private`. Anything that took the
///    first visibility in the statement, or looked leftwards past the node, would answer
///    `public`. The two disagree, so no wrong answer here is accidentally right;
/// 3. the neighbours still answer: the import reports its declaration, and no
///    `MemberPrefix` anywhere claims the lost word.
#[test]
fn an_indicator_reports_its_own_keyword_not_the_one_recovery_took() {
    let parse = trees::parsed(FIRST_OF_TWO_LOST, Language::SysMl);
    let root = parse.syntax();

    // 1. one import, and it has an indicator.
    let found = imports(&root);
    assert_eq!(found.len(), 1, "`private import A::*;` is the one import");
    let indicator = found[0]
        .visibility()
        .expect("the import states its visibility and this one is written");

    // 2a. attribution: the keyword is `private`, the second word. Asserted by TOKEN KIND
    // rather than by text, because the kind is what tells the three apart without a
    // string comparison (the point of returning the token).
    assert_eq!(
        indicator.keyword().map(|token| token.kind()),
        Some(SyntaxKind::KwPrivate),
        "the indicator's own keyword is `private`, not the `public` before it"
    );
    assert_ne!(
        indicator.keyword().map(|token| token.kind()),
        Some(SyntaxKind::KwPublic),
        "`public` is not this import's visibility"
    );

    // 2b. and `public` really is in the tree, in its own Error node — so the word was
    // not dropped, and this is a statement about attribution rather than about loss.
    let orphan = root
        .descendants()
        .find(|node| node.kind() == SyntaxKind::Error)
        .expect("recovery built an Error node for the word it could not place");
    assert_eq!(
        orphan.text().to_string().trim(),
        "public",
        "recovery took the first word alone, not the pair"
    );
    assert!(
        orphan.text_range().end() <= indicator.syntax().text_range().start(),
        "the lost word stands before the indicator, which is why it looks like its own"
    );

    // 2c. something reported it.
    assert_eq!(
        parse.errors().len(),
        1,
        "the stray visibility is reported once: {:?}",
        parse.errors()
    );

    // 3. the neighbours still answer.
    assert!(
        found[0].declaration().is_some(),
        "the import still names what it imports"
    );
    let prefixes: Vec<MemberPrefix> = root.descendants().filter_map(MemberPrefix::cast).collect();
    assert!(
        prefixes.iter().all(|prefix| prefix.visibility().is_none()),
        "no member prefix in this tree was written a visibility, so none may claim one"
    );
}

const TRAILING_SEPARATOR: &str =
    include_str!("malformed/import-declaration/trailing-separator.sysml");

/// `import-declaration/trailing-separator.sysml`: `public import A::;`. Every accessor
/// answers plausibly and the tree looks undamaged — which is the point.
///
/// An `ImportDeclaration`'s own shape cannot be wrong (one alternative, no tokens of its
/// own, asserted over all 756 in `tests/sweep.rs`), and the empty-but-present case is
/// already pinned for the `Import` kind by `import/declaration-names-nothing.sysml`. What
/// is left to get wrong is the **alternative**: `qualified_name` takes a `::` into the
/// name only when a name follows, so the name ends at `A`, `import_suffix` finds no `*`,
/// and `imported` closes the checkpoint as a `MembershipImport`
/// (`crates/sv2-syntax/src/parser/import.rs`). That is the alternative which parsed, not
/// the one a trailing `::` suggests was intended.
///
/// 1. the parse still yields a tree, with one declaration in it;
/// 2. the accessor reports what is there rather than guessing what was meant: the child
///    is a `MembershipImport`, its name is `A`, and **only** `Parse::errors()` says
///    anything is wrong. Asserted together, because each half alone reads as health;
/// 3. the neighbours still answer, down the whole delegation chain: the import reports
///    `public`, and the `QualifiedName` inside reports exactly one segment — the `::` is
///    a token of the alternative, not of the name and not of the declaration.
#[test]
fn a_trailing_separator_leaves_an_import_that_looks_undamaged() {
    let parse = trees::parsed(TRAILING_SEPARATOR, Language::SysMl);
    let root = parse.syntax();

    // 1. one import, one declaration.
    let found = imports(&root);
    assert_eq!(found.len(), 1, "the fixture writes one import");
    let declaration = found[0]
        .declaration()
        .expect("the node is opened before the name is sought, so it is always there");

    // 2a. the alternative that parsed, reported as itself. `MembershipImport` and
    // `NamespaceImport` have no views yet, so the kind is read from the node.
    let imported = declaration
        .imported()
        .expect("the alternative node is built at a checkpoint and is always there");
    assert_eq!(
        imported.kind(),
        SyntaxKind::MembershipImport,
        "no `*` follows the separator, so what parsed is a membership import"
    );

    // 2b. and it is not merely present: it names `A`, so nothing here reads as damage.
    let name = QualifiedName::cast(
        imported
            .children()
            .find(|child| child.kind() == SyntaxKind::QualifiedName)
            .expect("the alternative always carries its QualifiedName"),
    )
    .expect("that child is a QualifiedName");
    assert_eq!(
        name.segments()
            .map(|token| token.text().to_owned())
            .collect::<Vec<_>>(),
        ["A"],
        "the name ends at the separator, so one segment and not two"
    );

    // 2c. the only thing that says otherwise. Without this the fixture would be a
    // well-formed import, which is exactly why a caller needs the diagnostics.
    assert_eq!(
        parse.errors().len(),
        1,
        "the stranded separator is reported once: {:?}",
        parse.errors()
    );

    // 3a. width is not evidence: the declaration is WIDER than the name it carries,
    // where `import/declaration-names-nothing.sysml` is zero width with a child. Neither
    // direction tells a caller what was named.
    assert!(
        declaration.syntax().text_range().len() > name.syntax().text_range().len(),
        "the declaration spans `A::` and the name only `A`"
    );

    // 3b. the separator belongs to the alternative, not to the declaration or the name.
    assert_eq!(
        declaration.syntax().children_with_tokens().count(),
        1,
        "the declaration holds its alternative and nothing else"
    );

    // 3c. the neighbour above still answers.
    assert_eq!(
        found[0]
            .visibility()
            .and_then(|visibility| visibility.keyword())
            .map(|token| token.kind()),
        Some(SyntaxKind::KwPublic),
        "the import's visibility is untouched by the damage inside the declaration"
    );
}

const NON_ANNOTATION_IN_BRACES: &str =
    include_str!("malformed/relationship-body/non-annotation-in-braces.sysml");

/// The five children of the braced body in `non-annotation-in-braces.sysml`.
///
/// Split out of the test only to stay inside clippy's function-length lint; the three
/// claims belong together, because each alone reads as health. One real annotation, four
/// one-token recovery fragments, and the count that a caller would misread as four extra
/// annotations.
fn assert_braced_body_children(body: &RelationshipBody) {
    assert_eq!(
        body.elements().map(|node| node.kind()).collect::<Vec<_>>(),
        [
            SyntaxKind::OwnedAnnotation,
            SyntaxKind::Error,
            SyntaxKind::Error,
            SyntaxKind::Error,
            SyntaxKind::Error,
        ],
        "the comment is an annotation; the four tokens of `part def X;` are not"
    );
    assert_eq!(
        body.elements()
            .filter(|node| node.kind() == SyntaxKind::OwnedAnnotation)
            .count(),
        1,
        "exactly one real annotation was written"
    );
    assert_eq!(
        body.elements().count(),
        5,
        "and five children came back, so a count is not an annotation count"
    );
    assert_eq!(
        body.elements()
            .filter(|node| node.kind() == SyntaxKind::Error)
            .map(|node| node.text().to_string())
            .collect::<Vec<_>>(),
        ["part", "def", "X", ";"],
        "recovery took one token at a time, not the statement as a whole"
    );
}

/// `relationship-body/non-annotation-in-braces.sysml`:
/// `public import A::* { /* a */ part def X; }`. The body is braced, closed and
/// non-empty — which is what a healthy annotated relationship looks like — and four of
/// its five children are recovery fragments.
///
/// `RelationshipBody` admits `OwnedAnnotation` and nothing else in `SysML` 8.2.2.2. The
/// braced run reads an annotation when one starts and otherwise recovers one token at a
/// time, reporting each (`sysml_relationship_body`,
/// `crates/sv2-syntax/src/parser/annotation.rs`), so `part def X;` becomes four `Error`
/// nodes rather than one.
///
/// 1. the parse still yields a tree, with the body in it;
/// 2. the accessor reports what is there rather than what it means: `elements()` yields
///    all five children in order, the `Error`s included (ADR-0002 — elements with failed
///    constraints still enter the IR carrying their diagnostics), and `Parse::errors()`
///    is the only thing that says anything failed. A caller counting `elements()` as
///    annotations is wrong by four;
/// 3. the neighbour still answers, and the subtler claim: `terminator()` returns the `}`,
///    so **closed is not healthy**. The opposite case — never closed — is the fourteen
///    zero-width bodies already in the corpus, where `terminator()` is `None`.
#[test]
fn a_braced_body_reports_recovery_fragments_as_the_children_they_are() {
    let parse = trees::parsed(NON_ANNOTATION_IN_BRACES, Language::SysMl);
    let root = parse.syntax();

    let found = imports(&root);
    assert_eq!(found.len(), 1, "the fixture writes one import");
    let body = found[0]
        .body()
        .expect("the body node is built whichever alternative is written");

    // 2a, 2b and 2c: the children, the count that misleads, and the fragments.
    assert_braced_body_children(&body);

    assert_eq!(
        parse.errors().len(),
        4,
        "one diagnostic per recovered token: {:?}",
        parse.errors()
    );

    // 3a. closed is not healthy: the `}` is there, and the body is damaged anyway.
    assert_eq!(
        body.terminator().map(|token| token.kind()),
        Some(SyntaxKind::RBrace),
        "the body closed with its brace, which says nothing about what is inside it"
    );

    // 3b. the neighbour above still answers.
    assert_eq!(
        found[0]
            .visibility()
            .and_then(|visibility| visibility.keyword())
            .map(|token| token.kind()),
        Some(SyntaxKind::KwPublic),
        "the import's visibility is untouched by the damage inside the body"
    );
    assert!(
        found[0]
            .declaration()
            .and_then(|declaration| declaration.imported())
            .is_some(),
        "and it still reports what it imports"
    );
}

const UNCLOSED_BRACES: &str = include_str!("malformed/relationship-body/unclosed-braces.sysml");

/// `relationship-body/unclosed-braces.sysml`: `public import A::* {` and nothing more. The
/// body holds a token, is not zero width, and is **not terminated**.
///
/// The second fixture for this kind, and hand written because the corpus cannot settle the
/// question: no pinned tree holds a body whose last token is a `{` — 827 are `;`, 16 are
/// braced with both braces, 14 are zero width. Mutating `terminator()` to accept whatever
/// token came last survived the whole suite for that reason, which is the first case
/// `.claude/rules/ast.md` describes for a surviving mutation: nothing distinguished it, so
/// write the case.
///
/// The source for the outcome is the parser: `sysml_relationship_body`
/// (`crates/sv2-syntax/src/parser/annotation.rs`) closes the node with `expect(RBrace)`,
/// which reports without consuming when the brace never arrives.
///
/// 1. the parse still yields a tree, with the body in it;
/// 2. absence, reported: `terminator()` is `None` although a token is present, *and* a
///    diagnostic says the `}` was expected. This is the one tree where the two readings of
///    "unterminated" come apart — the 14 zero-width bodies are empty *and* unterminated,
///    so an accessor answering from emptiness would be right about all of them and wrong
///    here;
/// 3. the neighbour still answers: the import above reports its visibility and what it
///    imports, both of which were read before the brace.
#[test]
fn an_unclosed_brace_is_a_token_but_not_a_terminator() {
    let parse = trees::parsed(UNCLOSED_BRACES, Language::SysMl);
    let root = parse.syntax();

    let found = imports(&root);
    assert_eq!(found.len(), 1, "the fixture writes one import");
    let body = found[0].body().expect("the body node is always built");

    // 2a. unterminated, although a token is there.
    assert_eq!(
        body.terminator(),
        None,
        "a `{{` opens the body and does not close it"
    );
    assert!(
        body.syntax().text_range().len() > 0.into(),
        "the body is not zero width -- it holds the `{{`, which is what makes this case \
         different from the fourteen unterminated-at-EOF bodies in the corpus"
    );
    assert_eq!(
        body.syntax().text().to_string().trim(),
        "{",
        "the brace is the only thing in it"
    );

    // 2b. emptiness and closure are independent: this one is empty and unterminated,
    // where the corpus's `{ }` is empty and terminated.
    assert_eq!(
        body.elements().count(),
        0,
        "nothing was written inside the brace"
    );

    // 2c. something reported it.
    assert!(
        parse
            .errors()
            .iter()
            .any(|diagnostic| diagnostic.message().contains('}')),
        "the missing brace must be diagnosed: {:?}",
        parse.errors()
    );

    // 3. the neighbour still answers, for the part read before the brace.
    assert_eq!(
        found[0]
            .visibility()
            .and_then(|visibility| visibility.keyword())
            .map(|token| token.kind()),
        Some(SyntaxKind::KwPublic),
        "the visibility stands before the damage"
    );
    assert!(
        found[0]
            .declaration()
            .and_then(|declaration| declaration.imported())
            .is_some(),
        "and so does what it imports"
    );
}
