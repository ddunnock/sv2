// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! The malformed case for `RootNamespace::members` (`.claude/rules/ast.md`).

mod trees;

use sv2_ast::{AstNode, RootNamespace};
use sv2_syntax::{Language, SyntaxKind};

const UNCLOSED_BODY: &str = include_str!("malformed/root-namespace/unclosed-body.sysml");

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
/// 3. the neighbouring accessors still answer — NOT asserted, and cannot be: this crate
///    has one accessor, so there is no neighbour to ask. The part is vacant, not
///    discharged. The second accessor to land must add it here, and must not read the
///    `kind()` check below as having covered it.
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
}
