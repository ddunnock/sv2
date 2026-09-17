// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! The package declaration, checked against `SysML` 8.2.2.5.1.
//!
//! ```text
//! Package            = PrefixMetadataMember* PackageDeclaration PackageBody
//! PackageDeclaration = 'package' Identification
//! PackageBody        = ';' | '{' PackageBodyElement* '}'
//! Identification     = ( '<' declaredShortName = NAME '>' )? ( declaredName = NAME )?
//! ```

use std::fmt::Write as _;

use sv2_syntax::{Parse, SyntaxElement, SyntaxNode, parse};

/// Parse text the grammar accepts, asserting that nothing was reported.
fn parse_accepted(source: &str) -> Parse {
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    parsed
}

/// Parse text the grammar rejects, asserting that it is reported and that the tree
/// still carries every byte.
fn parse_rejected(source: &str) -> Parse {
    let parsed = parse(source);
    assert!(
        !parsed.errors().is_empty(),
        "{source:?} must be reported as an error"
    );
    assert_eq!(parsed.text(), source, "the tree is still lossless");
    parsed
}

/// The tree as indented text, so a shape change shows up as a snapshot diff.
fn render(node: &SyntaxNode) -> String {
    let mut out = String::new();
    write_element(&mut out, node.clone().into(), 0);
    out
}

/// The lines of the first `kind` node in `rendered`, with its children.
///
/// A child is any following line indented deeper than the node itself; a sibling at
/// the same depth ends it. Getting that wrong is how a test reads a neighbour's
/// tokens as its own.
fn subtree(rendered: &str, kind: &str) -> String {
    let mut lines = rendered.lines().skip_while(|l| l.trim_start() != kind);
    let Some(head) = lines.next() else {
        return String::new();
    };
    let depth = head.len() - head.trim_start().len();
    let kids = lines.take_while(|l| l.len() - l.trim_start().len() > depth);
    std::iter::once(head)
        .chain(kids)
        .collect::<Vec<_>>()
        .join("\n")
}

fn write_element(out: &mut String, element: SyntaxElement, depth: usize) {
    let indent = "  ".repeat(depth);
    match element {
        SyntaxElement::Node(node) => {
            let _ = writeln!(out, "{indent}{:?}", node.kind());
            for child in node.children_with_tokens() {
                write_element(out, child, depth + 1);
            }
        }
        SyntaxElement::Token(token) => {
            let _ = writeln!(out, "{indent}{:?} {:?}", token.kind(), token.text());
        }
    }
}

#[test]
fn an_empty_package_body_is_a_semicolon() {
    let parsed = parse_accepted("package Vehicle;");
    insta::assert_snapshot!(render(&parsed.syntax()));
}

#[test]
fn a_braced_package_body_holds_its_elements() {
    let parsed = parse_accepted("package Outer { package Inner; }");
    insta::assert_snapshot!(render(&parsed.syntax()));
}

#[test]
fn trivia_is_attached_to_the_tree_not_skipped() {
    // The losslessness invariant made visible in the tree shape: the comment and
    // every space are tokens, not gaps between them.
    let parsed = parse_accepted("// note\npackage  P ;\n");
    insta::assert_snapshot!(render(&parsed.syntax()));
}

#[test]
fn a_short_name_is_parsed_before_the_declared_name() {
    // Identification = ( '<' declaredShortName '>' )? ( declaredName )?
    let parsed = parse_accepted("package <V> Vehicle;");
    insta::assert_snapshot!(render(&parsed.syntax()));
}

#[test]
fn an_unrestricted_name_may_be_the_declared_name() {
    // NAME = BASIC_NAME | UNRESTRICTED_NAME (KerML 8.2.2.3).
    parse_accepted("package 'Vehicle Model';");
}

#[test]
fn a_package_need_not_declare_a_name() {
    // Identification = ( '<' declaredShortName = NAME '>' )? ( declaredName = NAME )?
    // BOTH parts are optional (SysML 8.2.2.2), so an anonymous package is well
    // formed and the parser must not report anything. No constraint in KerML
    // requires a Namespace to carry a declared name; if one is wanted it belongs to
    // a later layer, not to the syntax.
    //
    // This case previously sat under the negative-case banner named
    // `..._is_reported_without_losing_text` while asserting nothing about
    // diagnostics — the name claimed a diagnostic the grammar does not license.
    let parsed = parse_accepted("package ;");
    assert_eq!(parsed.text(), "package ;");
}

// -- negative cases -------------------------------------------------------------

#[test]
fn a_package_with_no_body_is_an_error_but_still_a_tree() {
    // PackageBody is not optional: it is ';' or '{...}'.
    parse_rejected("package Vehicle");
}

#[test]
fn an_unclosed_brace_is_reported_and_the_contents_are_kept() {
    parse_rejected("package Outer { package Inner;");
}

#[test]
fn text_that_is_not_a_package_becomes_an_error_node() {
    // part def is not implemented yet and must not parse silently.
    parse_rejected("part def Engine;");
}

#[test]
fn one_bad_element_does_not_discard_the_enclosing_body() {
    // Error recovery is a first-class path: an editor reparses invalid text
    // constantly, and a body that vanishes on one bad token blanks the diagram.
    let parsed = parse_rejected("package Outer { @ package Inner; }");
    let rendered = render(&parsed.syntax());
    assert!(
        rendered.contains("Package") && rendered.matches("PackageDeclaration").count() == 2,
        "the inner package must still parse:\n{rendered}"
    );
}

#[test]
fn parsing_never_panics_on_truncated_input() {
    let source = "package <V> Outer { package 'q\\' Inner; } /* c";
    for end in 0..=source.len() {
        if let Some(prefix) = source.get(..end) {
            let parsed = parse(prefix);
            assert_eq!(parsed.text(), prefix);
        }
    }
}

// -- reserved words are not names (KerML 8.2.2.6) ---------------------------------

#[test]
fn a_reserved_word_cannot_be_a_declared_name() {
    // "A reserved keyword is a token that has the lexical structure of a basic name
    // but cannot actually be used as a basic name" (KerML 8.2.2.6), and `package` is
    // on that list. It lexes as BASIC_NAME, so only the pinned keyword table can tell
    // the parser that Identification has no declaredName here.
    parse_rejected("package package;");
}

#[test]
fn a_reserved_word_cannot_be_a_short_name() {
    // Identification = ( '<' declaredShortName = NAME '>' )? ( declaredName = NAME )?
    // Both slots are NAME, so 8.2.2.6 governs both.
    parse_rejected("package <package> Vehicle;");
}

#[test]
fn a_name_that_merely_contains_a_reserved_word_is_a_name() {
    // The positive case for the same rule: the exclusion is by whole token, not by
    // substring. `packages` is not `package`, and a parser that rejected it would
    // reject most of the corpus.
    parse_accepted("package packages;");
    parse_accepted("package MyPackage;");
}

#[test]
fn a_reserved_word_in_quotes_is_a_name() {
    // UNRESTRICTED_NAME = single_quote ( NAME_CHARACTER | ESCAPE_SEQUENCE )*
    // single_quote (KerML 8.2.2.3). Quoting is exactly the escape hatch 8.2.2.6
    // leaves open: the represented name is the characters within the quotes, and
    // nothing says those characters may not spell a keyword.
    parse_accepted("package 'package';");
}

// -- comments must be closed (KerML 8.2.2.2) --------------------------------------

#[test]
fn an_unterminated_regular_comment_is_reported() {
    // REGULAR_COMMENT = '/*' COMMENT_TEXT '*/'. Text that runs to end of input
    // matches neither that nor anything else.
    parse_rejected("package Vehicle; /* never closed");
}

#[test]
fn an_unclosed_multiline_note_opener_is_a_single_line_note() {
    // MULTILINE_NOTE = '//*' COMMENT_TEXT '*/' requires the terminator, so an unclosed
    // `//*` is not one. It is still '//' LINE_TEXT, a SINGLE_LINE_NOTE, and the line
    // after it is ordinary text (KerML 8.2.2.2).
    parse_accepted("package Vehicle; //* never closed");
    parse_accepted("package Vehicle;\n//* never closed\npackage Engine;");
    parse_accepted("package Vehicle; //*/");
}

#[test]
fn a_comment_opener_whose_last_two_characters_are_the_terminator_is_still_unterminated() {
    // `/*/` ends in `*/` while being unterminated: those are the opener's own
    // characters. The check has to exclude the opener before looking.
    parse_rejected("package Vehicle; /*/");
}

#[test]
fn text_after_an_unclosed_note_opener_is_still_parsed() {
    // The negative side of the fallback: the note ends at its line, so an invalid
    // line after it is reported rather than swallowed as note text.
    parse_rejected("package Vehicle;\n//* never closed\n}");
}

#[test]
fn a_closed_comment_is_not_reported() {
    // The positive case: a check that fired on every comment would reject the corpus.
    parse_accepted("package Vehicle; /* closed */");
    parse_accepted("package Vehicle; //* closed */");
    parse_accepted("/* before */ package Vehicle; // to end of line");
    // The shortest closed forms, either side of the `/*/` case above.
    parse_accepted("package Vehicle; /**/");
    parse_accepted("package Vehicle; //**/");
}

// -- Import, SysML 8.2.2.5.1 ------------------------------------------------------
//
//   Import              = visibility = VisibilityIndicator 'import'
//                         ( isImportAll ?= 'all' )? ImportDeclaration RelationshipBody
//   ImportDeclaration   = MembershipImport | NamespaceImport
//   MembershipImport    = importedMembership = [QualifiedName] ( '::' isRecursive ?= '**' )?
//   NamespaceImport     = importedNamespace = [QualifiedName] '::' '*'
//                         ( '::' isRecursive ?= '**' )?
//                       | importedNamespace = FilterPackage
//   VisibilityIndicator = 'public' | 'private' | 'protected'
//   QualifiedName       = ( '$' '::' )? ( NAME '::' )* NAME      (KerML 8.2.3.4.1)
//   RelationshipBody    = ';' | '{' ( ownedRelationship += OwnedAnnotation )* '}'

#[test]
fn a_membership_import_names_one_member() {
    // `private import Time::DateTime;` — 4 occurrences in the pinned corpus.
    parse_accepted("private import Time::DateTime;");
}

#[test]
fn a_namespace_import_ends_in_a_star() {
    // `private import ISQ::*;` — 14 occurrences, the most common form in the corpus.
    parse_accepted("private import ISQ::*;");
    parse_accepted("public import Definitions::*;");
}

#[test]
fn a_recursive_import_ends_in_a_double_star() {
    // `public import vehicle_b::**;` — 8 occurrences. MembershipImport's
    // ( '::' isRecursive ?= '**' )?.
    parse_accepted("public import vehicle_b::**;");
    // NamespaceImport carries the same suffix after its '*'.
    parse_accepted("public import vehicle::*::**;");
}

#[test]
fn a_qualified_name_may_be_more_than_two_segments() {
    // `public import VehicleConfigurations::VehicleConfiguration_b::**;` — 6 in corpus.
    parse_accepted("public import VehicleConfigurations::VehicleConfiguration_b::**;");
}

#[test]
fn every_visibility_indicator_is_accepted() {
    // VisibilityIndicator = 'public' | 'private' | 'protected'. All three, because a
    // parser that only ever saw the two common ones would pass a corpus sweep.
    parse_accepted("public import A::*;");
    parse_accepted("private import A::*;");
    parse_accepted("protected import A::*;");
}

#[test]
fn an_import_may_be_marked_all() {
    // ( isImportAll ?= 'all' )?, between 'import' and the declaration.
    parse_accepted("public import all A::*;");
    parse_accepted("public import all A::B;");
}

#[test]
fn a_qualified_name_may_start_at_global_scope() {
    // QualifiedName = ( '$' '::' )? ( NAME '::' )* NAME — the global scope qualifier.
    parse_accepted("public import $::A::*;");
}

#[test]
fn a_relationship_body_may_be_braces_instead_of_a_semicolon() {
    // RelationshipBody = ';' | '{' ( ownedRelationship += OwnedAnnotation )* '}'.
    // OwnedAnnotation is not implemented, so only the empty body is accepted here.
    parse_accepted("public import A::* { }");
}

#[test]
fn an_import_is_a_package_body_element() {
    // PackageBodyElement = PackageMember | ElementFilterMember | AliasMember | Import.
    // Import is the alternative implemented so far, so it must parse inside a body
    // and not only at the root.
    parse_accepted("package Vehicle { private import ISQ::*; }");
}

#[test]
fn an_import_builds_the_nodes_the_grammar_names() {
    // The shape is read off the productions, not off what the parser printed:
    //   Import > VisibilityIndicator, ImportDeclaration > NamespaceImport >
    //   QualifiedName, and RelationshipBody.
    let parsed = parse_accepted("public import A::B::*;");
    let rendered = render(&parsed.syntax());
    for node in [
        "Import",
        "VisibilityIndicator",
        "ImportDeclaration",
        "NamespaceImport",
        "QualifiedName",
        "RelationshipBody",
    ] {
        assert!(rendered.contains(node), "no {node} node:\n{rendered}");
    }
    // A membership import is the other alternative, and must not be built here.
    assert!(!rendered.contains("MembershipImport"), "{rendered}");
}

#[test]
fn a_membership_import_builds_the_other_alternative() {
    // The negative half of the pair above: the same prefix, a different node.
    let parsed = parse_accepted("public import A::B;");
    let rendered = render(&parsed.syntax());
    assert!(rendered.contains("MembershipImport"), "{rendered}");
    assert!(!rendered.contains("NamespaceImport"), "{rendered}");
}

#[test]
fn a_qualified_name_keeps_the_separator_that_is_its_own() {
    // `A::B::*` — the first two `::` belong to the QualifiedName and the third to
    // the NamespaceImport. Getting that split wrong still round-trips, so only the
    // tree shape can catch it.
    let parsed = parse_accepted("public import A::B::*;");
    let rendered = render(&parsed.syntax());
    let qualified = subtree(&rendered, "QualifiedName");
    assert_eq!(
        qualified.matches("ColonColon").count(),
        1,
        "the name owns one `::`, the import owns the other:\n{rendered}"
    );
}

// -- PackageMember, SysML 8.2.2.5.1 -----------------------------------------------
//
//   PackageMember : OwningMembership =
//       MemberPrefix ( ownedRelatedElement += DefinitionElement
//                    | ownedRelatedElement = UsageElement )
//   MemberPrefix : Membership = ( visibility = VisibilityIndicator )?

#[test]
fn a_package_member_may_carry_a_visibility() {
    // MemberPrefix's visibility is optional, unlike Import's, and all three
    // indicators are available to it.
    parse_accepted("public package Vehicle;");
    parse_accepted("private package Vehicle;");
    parse_accepted("protected package Vehicle;");
}

#[test]
fn a_package_member_need_not_carry_one() {
    // ( visibility = VisibilityIndicator )? — the whole prefix is optional, which is
    // why every package written so far has parsed without one.
    parse_accepted("package Vehicle;");
}

#[test]
fn a_package_member_builds_the_prefix_whether_or_not_it_is_filled() {
    // The slot exists in the production either way. A tree that dropped the node
    // when the slot was empty would make every consumer handle two shapes for one
    // construct.
    let with = render(&parse_accepted("public package Vehicle;").syntax());
    let without = render(&parse_accepted("package Vehicle;").syntax());
    assert!(with.contains("MemberPrefix") && with.contains("VisibilityIndicator"));
    assert!(without.contains("MemberPrefix"));
    assert!(!without.contains("VisibilityIndicator"), "{without}");
}

#[test]
fn a_visibility_does_not_by_itself_say_which_element_follows() {
    // Import's visibility is required and MemberPrefix's is optional, so both may
    // open the same way. The keyword after the indicator is what separates them —
    // the one place a single token of lookahead is not enough.
    let import = render(&parse_accepted("public import A::*;").syntax());
    assert!(import.contains("Import") && !import.contains("PackageMember"));

    let member = render(&parse_accepted("public package P;").syntax());
    assert!(member.contains("PackageMember") && !member.contains("Import"));
}

#[test]
fn a_package_member_nests_inside_a_package_body() {
    // PackageBody = '{' PackageBodyElement* '}', and PackageMember is one of them,
    // so the visibility is available at every depth rather than only at the root.
    let parsed = parse_accepted("package Outer { private package Inner; }");
    let rendered = render(&parsed.syntax());
    assert_eq!(rendered.matches("PackageMember").count(), 2, "{rendered}");
    assert_eq!(
        rendered.matches("VisibilityIndicator").count(),
        1,
        "{rendered}"
    );
}

// -- AliasMember, SysML 8.2.2.5.1 -------------------------------------------------
//
//   AliasMember : Membership =
//       MemberPrefix 'alias' ( '<' memberShortName = NAME '>' )?
//       ( memberName = NAME )? 'for' memberElement = [QualifiedName]
//       RelationshipBody
//
// Membership, not OwningMembership (KerML 8.3.2.4.3): the target is a reference to an
// element declared elsewhere, not a nested one.

#[test]
fn an_alias_names_an_element_declared_elsewhere() {
    // `alias Car for Vehicle;` and `alias Torque for ISQ::TorqueValue;` are both in
    // the pinned corpus. The target is a QualifiedName, so it may be qualified.
    parse_accepted("alias Car for Vehicle;");
    parse_accepted("alias Torque for ISQ::TorqueValue;");
    parse_accepted("alias us for w::g;");
}

#[test]
fn an_alias_name_may_be_unrestricted() {
    // `alias 'Sport Sedan' for vehicle1_c1;` — from the corpus. memberName is a NAME,
    // and NAME = BASIC_NAME | UNRESTRICTED_NAME (KerML 8.2.2.3).
    parse_accepted("alias 'Sport Sedan' for vehicle1_c1;");
}

#[test]
fn an_alias_may_carry_a_visibility() {
    // `public alias Car for Automobile;` — one occurrence in the corpus. AliasMember
    // opens with MemberPrefix, so the optional visibility is available to it.
    parse_accepted("public alias Car for Automobile;");
    parse_accepted("private alias Car for Automobile;");
}

#[test]
fn an_alias_may_have_a_short_name() {
    // ( '<' memberShortName = NAME '>' )?, before the optional memberName.
    //
    // NOT exercised by the pinned corpus: the only `alias <` in the 311 files is
    // inside a comment. Constructed from the production rather than found, which is
    // why both orderings are checked explicitly.
    parse_accepted("alias <c> Car for Vehicle;");
    parse_accepted("alias <c> for Vehicle;");
}

#[test]
fn both_alias_name_slots_are_optional() {
    // memberName and memberShortName are both 0..1 on Membership (KerML 8.3.2.4.3),
    // and the production marks both slots `?`. So this parses. Whether an alias with
    // no name means anything is a constraint question, not a grammar one, and
    // ADR-0002 says validity gates writes rather than reads.
    parse_accepted("alias for Vehicle;");
}

#[test]
fn an_alias_body_may_be_braces() {
    // RelationshipBody = ';' | '{' OwnedAnnotation* '}'. The corpus has three braced
    // aliases and every one holds a `doc`, which is an OwnedAnnotation and therefore
    // unimplemented — so only the empty body is accepted here, as for Import.
    parse_accepted("alias Car for Automobile { }");
}

#[test]
fn an_alias_builds_the_nodes_the_grammar_names() {
    let parsed = parse_accepted("public alias <c> Car for A::B;");
    let rendered = render(&parsed.syntax());
    for node in [
        "AliasMember",
        "MemberPrefix",
        "VisibilityIndicator",
        "QualifiedName",
        "RelationshipBody",
    ] {
        assert!(rendered.contains(node), "no {node} node:\n{rendered}");
    }
    // The target is a reference, never a nested element: no PackageMember here.
    assert!(!rendered.contains("PackageMember"), "{rendered}");
}

#[test]
fn an_alias_is_told_apart_from_the_other_prefixed_elements() {
    // alias, package and import can all open with the same visibility indicator, so
    // the deciding keyword is the one after it.
    let alias = render(&parse_accepted("public alias A for B;").syntax());
    assert!(alias.contains("AliasMember") && !alias.contains("Import"));

    let import = render(&parse_accepted("public import A::*;").syntax());
    assert!(import.contains("Import") && !import.contains("AliasMember"));

    let member = render(&parse_accepted("public package P;").syntax());
    assert!(member.contains("PackageMember") && !member.contains("AliasMember"));
}

#[test]
fn an_alias_nests_inside_a_package_body() {
    parse_accepted("package P { alias Car for Vehicle; }");
}
