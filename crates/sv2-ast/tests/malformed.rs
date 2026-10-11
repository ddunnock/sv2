// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! The malformed case for `RootNamespace::members` (`.claude/rules/ast.md`).

mod trees;

use sv2_ast::{AstNode, RootNamespace};
use sv2_syntax::{Language, SyntaxKind};

const UNCLOSED_BODY: &str = include_str!("malformed/root-namespace-unclosed-body.sysml");

/// `root-namespace-unclosed-body.sysml`: the first package's body is never closed, so
/// the parser's recovery reads to end of file, and the second `package` becomes one of
/// the first package's OWN members rather than a sibling of it at the root
/// (`crates/sv2-syntax/src/parser/body.rs`). Proves the three things a malformed case
/// must (`.claude/rules/ast.md`):
///
/// 1. the parse still yields a tree;
/// 2. the accessor reports absence rather than guessing — exactly one top-level
///    member, not the two the source text names;
/// 3. the neighbouring accessor still answers — the one member that survived is a
///    genuine, well-formed node.
#[test]
fn unclosed_body_swallows_the_next_package_instead_of_losing_the_tree() {
    let root = trees::tree(UNCLOSED_BODY, Language::SysMl);

    // 1. the parse still yields a tree: the root is still `RootNamespace`.
    let namespace = RootNamespace::cast(root).expect("the root node is always RootNamespace");

    // 2. absence, not a guess: the second package was swallowed into the first's
    // unclosed body, so there is exactly one top-level member, not two.
    let members: Vec<_> = namespace.members().collect();
    assert_eq!(
        members.len(),
        1,
        "the unclosed package must not be reported as having a sibling that the \
         recovery actually nested inside it"
    );

    // 3. the neighbouring accessor still answers: the one member that does exist is a
    // genuine node, not a casualty of the recovery.
    let only_member = &members[0];
    assert_eq!(only_member.kind(), SyntaxKind::PackageMember);
}
