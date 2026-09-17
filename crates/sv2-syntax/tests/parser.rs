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

/// How many nodes in `rendered` are exactly `kind`.
///
/// Whole lines, not substrings: several node names contain another's, so
/// `FeatureTyping` counted as a substring also finds every `OwnedFeatureTyping`.
/// A node line is its name alone; a token line carries its text after it.
fn nodes_named(rendered: &str, kind: &str) -> usize {
    rendered.lines().filter(|l| l.trim() == kind).count()
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
fn text_no_implemented_production_accepts_becomes_an_error_node() {
    // This case has now been `part def Engine;`, `part engine : Engine;`,
    // `attribute mass : Real;` and `item wheel : Wheel;`. Each stopped being a
    // rejection when its production landed, and the positive cases below hold all
    // four. The property the test protects never changes: text this parser cannot
    // read is reported, not silently accepted. Only the example moves.
    //
    // It is deliberately no longer a usage of the `<prefix> KEYWORD Usage` shape.
    // Those now arrive in batches — seven of them are one table — so any of them
    // would be a placeholder with a short life. A ConnectionUsage has a shape of its
    // own, a BinaryConnectorPart naming two ends, so it will not land incidentally
    // alongside something else.
    //
    // SysML 8.2.2.13 — ConnectionUsage = OccurrenceUsagePrefix 'connection'
    //                    ConnectionUsageDeclaration ...
    let parsed = parse_rejected("connection fuelLine connect a to b;");
    assert!(render(&parsed.syntax()).contains("Error"));
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
    // The `*` admits zero annotations, so the empty braced body is well formed. The
    // bodies that hold annotations are covered under OwnedAnnotation below.
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
    // RelationshipBody = ';' | '{' OwnedAnnotation* '}'. The empty braced body is
    // the zero-annotation case; the corpus's braced aliases, which hold a `doc` or a
    // bare comment, are covered under OwnedAnnotation below.
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

// -- OwnedAnnotation in a RelationshipBody, SysML 8.2.2.2 and 8.2.2.4 --------------
//
//   RelationshipBody      = ';' | '{' ( ownedRelationship += OwnedAnnotation )* '}'
//   OwnedAnnotation       = ownedRelatedElement += AnnotatingElement
//   AnnotatingElement     = Comment | Documentation | TextualRepresentation
//                         | MetadataUsage      (MetadataUsage by deviation; unimplemented)
//   Comment               = ( 'comment' Identification
//                             ( 'about' Annotation ( ',' Annotation )* )? )?
//                           ( 'locale' STRING_VALUE )? REGULAR_COMMENT
//   Documentation         = 'doc' Identification ( 'locale' STRING_VALUE )? REGULAR_COMMENT
//   TextualRepresentation = ( 'rep' Identification )? 'language' STRING_VALUE
//                           REGULAR_COMMENT
//   Annotation            = annotatedElement = [QualifiedName]
//
// REGULAR_COMMENT is a token (KerML 8.2.2.2), the body these productions own.

#[test]
fn a_documented_import_owns_its_annotation() {
    // The rejection-by-absence case this replaces, now accepted: a braced
    // RelationshipBody owning one Documentation through an OwnedAnnotation.
    let parsed = parse_accepted("public import A::* { doc /* an annotation */ }");
    insta::assert_snapshot!(render(&parsed.syntax()));
}

#[test]
fn a_documentation_body_belongs_to_the_documentation_node() {
    // body = REGULAR_COMMENT is part of Documentation, so the comment token must sit
    // inside that node — not before it as trivia, which would round-trip identically
    // and so only the tree shape can catch.
    let parsed = parse_accepted("public import A::* { doc /* body */ }");
    let rendered = render(&parsed.syntax());
    let documentation = subtree(&rendered, "Documentation");
    assert!(
        documentation.contains("RegularComment \"/* body */\""),
        "{rendered}"
    );
    let relationship = subtree(&rendered, "RelationshipBody");
    assert!(relationship.contains("OwnedAnnotation"), "{rendered}");
}

#[test]
fn an_alias_body_may_hold_documentation_as_the_corpus_writes_it() {
    // Training "01. Packages/Documentation Example.sysml", lines 10-12.
    parse_accepted(
        "alias Car for Automobile {\n\t\tdoc /* This is documentation of the alias. */\n\t}",
    );
}

#[test]
fn a_doc_keyword_and_its_body_may_be_on_separate_lines() {
    // Validation "15_10-Primitive Data Types.sysml", lines 8-12: `doc`, a newline,
    // then the body. White space between tokens is not significant (KerML 8.2.2.1).
    parse_accepted(
        "private import ScalarValues::Integer {\n\tdoc\n\t/*\n\t * The unqualified Integer is signed.\n\t */\n\t}",
    );
}

#[test]
fn a_bare_regular_comment_in_a_relationship_body_is_a_comment_element() {
    // Validation "1a-Parts Tree.sysml", lines 27-32:
    // `private import Definitions::* { /* ... */ }`. Everything before Comment's body
    // is optional, so the body alone is a Comment.
    let parsed = parse_accepted(
        "private import Definitions::* {\n\t/*\n\t * A \"private\" private import.\n\t */\n}",
    );
    let rendered = render(&parsed.syntax());
    assert!(
        subtree(&rendered, "Comment").contains("RegularComment"),
        "{rendered}"
    );
    assert!(rendered.contains("OwnedAnnotation"), "{rendered}");
}

#[test]
fn a_comment_may_carry_a_header_and_an_about_list() {
    // `comment Comment1 /* This is a named comment. */` and
    // `comment cmt_cmt about cmt /* Comment about Comment */` are corpus lines; both
    // are placed in an import's body here, where RelationshipBody admits them.
    parse_accepted("public import A::* { comment Comment1 /* This is a named comment. */ }");
    parse_accepted("public import A::* { comment cmt_cmt about cmt /* about */ }");
    // ( ',' Annotation )*, and an Annotation's target is a QualifiedName.
    let parsed = parse_accepted("public import A::* { comment about B, C::D /* both */ }");
    let rendered = render(&parsed.syntax());
    let annotations = rendered
        .lines()
        .filter(|line| line.trim_start() == "Annotation")
        .count();
    assert_eq!(annotations, 2, "{rendered}");
}

#[test]
fn a_comment_or_documentation_may_state_a_locale() {
    // ( 'locale' locale = STRING_VALUE )?, after the header and before the body.
    parse_accepted("public import A::* { locale \"en_US\" /* bare, with a locale */ }");
    parse_accepted("public import A::* { comment c locale \"en_US\" /* named */ }");
    parse_accepted("public import A::* { doc locale \"fr\" /* documentation */ }");
    parse_accepted("public import A::* { doc <d> Intro locale \"fr\" /* identified */ }");
}

#[test]
fn a_textual_representation_names_its_language() {
    // `rep inOCL language "ocl"` (KerML "Simple Tests/TextualRepresentation.kerml"
    // line 7) and `language "Alf" /* ... */` ("Opaque Action Example.sysml" line 9).
    parse_accepted("public import A::* { rep inOCL language \"ocl\" /* self.x > 0 */ }");
    let parsed = parse_accepted("public import A::* { language \"Alf\" /* x = 1; */ }");
    assert!(render(&parsed.syntax()).contains("TextualRepresentation"));
}

#[test]
fn a_relationship_body_may_own_several_annotations() {
    // ( ownedRelationship += OwnedAnnotation )*.
    let parsed = parse_accepted(
        "alias Car for Automobile { doc /* one */ /* two */ language \"x\" /* three */ }",
    );
    let rendered = render(&parsed.syntax());
    assert_eq!(rendered.matches("OwnedAnnotation").count(), 3, "{rendered}");
}

#[test]
fn documentation_without_a_body_is_reported() {
    // Documentation = 'doc' Identification ( 'locale' STRING_VALUE )? REGULAR_COMMENT.
    // The body is not optional; `Intro` is only the Identification.
    parse_rejected("public import A::* { doc Intro }");
    parse_rejected("public import A::* { doc }");
}

#[test]
fn a_comment_header_without_a_body_is_reported() {
    // Comment ends in REGULAR_COMMENT whatever header precedes it.
    parse_rejected("public import A::* { comment c }");
    parse_rejected("public import A::* { comment about X }");
}

#[test]
fn an_about_list_requires_an_annotated_element() {
    // 'about' Annotation, and Annotation = [QualifiedName], which is not empty.
    parse_rejected("public import A::* { comment about /* nothing */ }");
    parse_rejected("public import A::* { comment about X, /* trailing comma */ }");
}

#[test]
fn a_locale_requires_a_string() {
    // 'locale' locale = STRING_VALUE. A regular comment is a token, not skipped
    // trivia, so it cannot stand between `locale` and the string.
    parse_rejected("public import A::* { doc locale /* body */ }");
    parse_rejected("public import A::* { doc locale /* x */ \"en\" /* body */ }");
}

#[test]
fn a_textual_representation_requires_its_language() {
    // 'language' language = STRING_VALUE is required even after 'rep' Identification.
    parse_rejected("public import A::* { rep R /* text */ }");
    parse_rejected("public import A::* { language /* text */ }");
    parse_rejected("public import A::* { language \"ocl\" }");
}

#[test]
fn a_relationship_body_holds_only_annotations() {
    // SysML narrows RelationshipBody to OwnedAnnotation (8.2.2.2); a package member
    // is not one.
    parse_rejected("public import A::* { package P; }");
    parse_rejected("alias Car for Automobile { alias X for Y; }");
}

#[test]
fn metadata_in_a_relationship_body_is_reported_not_accepted() {
    // MetadataUsage is AnnotatingElement's fourth alternative (deviation
    // AnnotatingElement) and valid SysML here, but it is not implemented. It must be
    // reported rather than silently accepted: a rejection by absence, not by rule.
    parse_rejected("public import A::* { @Rationale; }");
    parse_rejected("public import A::* { metadata Rationale; }");
    // Recovery is at the body: an implemented annotation after it still parses.
    let parsed = parse_rejected("public import A::* { @Rationale; doc /* kept */ }");
    assert!(render(&parsed.syntax()).contains("Documentation"));
}

#[test]
fn an_unterminated_annotation_body_is_reported_and_kept() {
    // REGULAR_COMMENT = '/*' COMMENT_TEXT '*/' (KerML 8.2.2.2) requires the
    // terminator, whether the comment is trivia or a body.
    let parsed = parse_rejected("public import A::* { doc /* never closed");
    assert!(
        parsed.errors().iter().any(|e| e.contains("never closed")),
        "{:?}",
        parsed.errors()
    );
}

// -- PartDefinition, SysML 8.2.2.11 -----------------------------------------------
//
//   PartDefinition             = OccurrenceDefinitionPrefix 'part' 'def' Definition
//   OccurrenceDefinitionPrefix = BasicDefinitionPrefix?
//                                ( 'individual' EmptyMultiplicityMember )?
//                                DefinitionExtensionKeyword*         (8.2.2.9.1)
//   BasicDefinitionPrefix      = 'abstract' | 'variation'            (8.2.2.6.1)
//   Definition                 = DefinitionDeclaration DefinitionBody
//   DefinitionDeclaration      = Identification SubclassificationPart?
//   DefinitionBody             = ';' | '{' DefinitionBodyItem* '}'
//   DefinitionMember           = MemberPrefix DefinitionElement
//   SubclassificationPart      = SPECIALIZES OwnedSubclassification
//                                ( ',' OwnedSubclassification )*     (8.2.2.6.5)
//   OwnedSubclassification     = [QualifiedName]
//   SPECIALIZES                = ':>' | 'specializes'                (KerML 8.2.2.7)

#[test]
fn a_part_definition_is_a_package_member() {
    // The rejection-by-absence case this replaces, now accepted.
    let parsed = parse_accepted("public part def Vehicle;");
    insta::assert_snapshot!(render(&parsed.syntax()));
}

#[test]
fn part_definitions_as_the_corpus_writes_them() {
    // Each line is from the pinned corpus.
    parse_accepted("part def Vehicle;");
    parse_accepted("part def 'Fuel Station';");
    parse_accepted("abstract part def VehiclePart;");
    parse_accepted("part def Engine :> VehiclePart;");
    parse_accepted("private part def Automobile;");
    // SimpleVehicleModel.sysml line 1484, with no space around ':>'.
    parse_accepted("variation part def TransmissionChoices:>Transmission { }");
    // Training "28. Individuals/Individuals and Roles-1.sysml" line 11.
    parse_accepted("individual part def Wheel_1 :> Wheel;");
}

#[test]
fn a_part_definition_builds_the_nodes_the_grammar_names() {
    let parsed = parse_accepted("part def Car :> Vehicle, Base::Thing { }");
    let rendered = render(&parsed.syntax());
    for node in [
        "PackageMember",
        "PartDefinition",
        "OccurrenceDefinitionPrefix",
        "KwPart",
        "KwDef",
        "Definition",
        "DefinitionDeclaration",
        "Identification",
        "SubclassificationPart",
        "OwnedSubclassification",
        "QualifiedName",
        "DefinitionBody",
    ] {
        assert!(rendered.contains(node), "no {node} node:\n{rendered}");
    }
    assert_eq!(
        rendered.matches("OwnedSubclassification").count(),
        2,
        "{rendered}"
    );
    // No prefix keyword was written, so none of the prefix's optional parts is built.
    assert!(!rendered.contains("BasicDefinitionPrefix"), "{rendered}");
    assert!(!rendered.contains("EmptyMultiplicityMember"), "{rendered}");
}

#[test]
fn specializes_may_be_spelled_out() {
    // SPECIALIZES = ':>' | 'specializes' (KerML 8.2.2.7).
    parse_accepted("part def Car specializes Vehicle;");
}

#[test]
fn a_part_definition_need_not_declare_a_name() {
    // DefinitionDeclaration = Identification SubclassificationPart?, and
    // Identification is nullable (SysML 8.2.2.2).
    parse_accepted("part def;");
    parse_accepted("part def :> Vehicle;");
}

#[test]
fn an_individual_definition_owns_an_empty_multiplicity() {
    // ( isIndividual ?= 'individual' ownedRelationship += EmptyMultiplicityMember )?,
    // EmptyMultiplicity = { } — an element with no tokens.
    let parsed = parse_accepted("abstract individual part def Vehicle_1 :> Vehicle;");
    let rendered = render(&parsed.syntax());
    for node in [
        "BasicDefinitionPrefix",
        "KwIndividual",
        "EmptyMultiplicityMember",
        "EmptyMultiplicity",
    ] {
        assert!(rendered.contains(node), "no {node} node:\n{rendered}");
    }
}

#[test]
fn a_definition_body_holds_the_items_implemented_so_far() {
    // DefinitionBodyItem = DefinitionMember | ... | AliasMember | Import, and
    // DefinitionElement includes both Package and PartDefinition.
    let parsed = parse_accepted(
        "part def Vehicle {\n  private import ISQ::*;\n  alias Car for Vehicle;\n  public part def Engine;\n  package Notes;\n}",
    );
    let rendered = render(&parsed.syntax());
    assert_eq!(
        rendered.matches("DefinitionMember").count(),
        2,
        "{rendered}"
    );
    assert!(
        rendered.contains("Import") && rendered.contains("AliasMember"),
        "{rendered}"
    );
}

#[test]
fn a_part_definition_nests_inside_a_package() {
    let parsed = parse_accepted("package Vehicles { part def Vehicle { part def Engine; } }");
    let rendered = render(&parsed.syntax());
    assert_eq!(rendered.matches("PartDefinition").count(), 2, "{rendered}");
}

#[test]
fn a_part_definition_without_a_body_is_reported() {
    // DefinitionBody = ';' | '{' DefinitionBodyItem* '}' is not optional.
    parse_rejected("part def Vehicle");
    parse_rejected("part def Vehicle {");
}

#[test]
fn a_specialization_needs_a_superclass() {
    // SPECIALIZES OwnedSubclassification, and OwnedSubclassification = [QualifiedName].
    parse_rejected("part def Vehicle :> ;");
    parse_rejected("part def Car :> Vehicle, ;");
    parse_rejected("part def Car specializes;");
}

#[test]
fn a_basic_definition_prefix_is_one_keyword_at_most() {
    // BasicDefinitionPrefix? — `abstract` or `variation`, not both.
    parse_rejected("abstract variation part def Vehicle;");
}

#[test]
fn individual_follows_the_basic_definition_prefix() {
    // BasicDefinitionPrefix? then ( 'individual' ... )?, in that order.
    parse_rejected("individual abstract part def Vehicle;");
}

#[test]
fn a_reserved_word_cannot_name_a_part_definition() {
    // KerML 8.2.2.6: `part` is reserved, so Identification has no name and the body
    // is missing.
    parse_rejected("part def part;");
}

#[test]
fn unimplemented_definition_body_items_are_reported_at_the_body() {
    // The unimplemented item here moves for the same reason as the case above, and
    // to the same construct: the usages it previously held are all read now, and each
    // is exercised inside a definition body as a positive case below. A
    // ConnectionUsage (SysML 8.2.2.13) is not. It must be reported, and the
    // definition after it must still parse — recovery happens at the enclosing body.
    let parsed =
        parse_rejected("part def Vehicle { connection c connect a to b; part def Wheel; }");
    let rendered = render(&parsed.syntax());
    assert_eq!(nodes_named(&rendered, "PartDefinition"), 2, "{rendered}");
}

#[test]
fn prefix_metadata_on_a_definition_is_reported_not_accepted() {
    // DefinitionExtensionKeyword (`#` PrefixMetadataMember) is valid SysML and not
    // implemented: a rejection by absence. The definition after it still parses.
    let parsed = parse_rejected("#Safety part def Brake;");
    assert!(render(&parsed.syntax()).contains("PartDefinition"));
}

#[test]
fn parsing_annotations_and_definitions_never_panics_on_truncated_input() {
    let source = "public abstract individual part def <V> Vehicle :> A::B, C {\n  alias X for Y { comment c about Z locale \"en\" /* b */ rep r language \"l\" /* t */ }\n}";
    for end in 0..=source.len() {
        if let Some(prefix) = source.get(..end) {
            assert_eq!(parse(prefix).text(), prefix);
        }
    }
}

// -- PartUsage, SysML 8.2.2.11 ----------------------------------------------------
//
//   PartUsage              = OccurrenceUsagePrefix 'part' Usage
//   OccurrenceUsagePrefix  = ( EndUsagePrefix
//                            | BasicUsagePrefix 'individual'? PortionKind? )
//                            UsageExtensionKeyword*                (8.2.2.9.2)
//   BasicUsagePrefix       = RefPrefix 'ref'?                      (8.2.2.6.2)
//   RefPrefix              = FeatureDirection? 'derived'?
//                            ( 'abstract' | 'variation' )? 'constant'?
//   FeatureDirection       = 'in' | 'out' | 'inout'
//   PortionKind            = 'snapshot' | 'timeslice'              (8.2.2.9.2)
//   Usage                  = UsageDeclaration UsageCompletion      (8.2.2.6.2)
//   UsageDeclaration       = Identification FeatureSpecializationPart?
//   UsageCompletion        = ValuePart? UsageBody
//   UsageBody              = DefinitionBody
//   FeatureSpecializationPart = FeatureSpecialization+ MultiplicityPart?
//                               FeatureSpecialization*
//                             | MultiplicityPart FeatureSpecialization*  (KerML 8.2.4.3.1)
//   Typings                = TypedBy ( ',' FeatureTyping )*        (8.2.2.6.5)
//   TypedBy                = ( ':' | 'defined' 'by' ) FeatureTyping
//   FeatureTyping          = OwnedFeatureTyping | ConjugatedPortTyping
//   OwnedFeatureTyping     = QualifiedName | OwnedFeatureChain

#[test]
fn a_part_usage_is_a_package_member() {
    // The rejection-by-absence case this replaces, now accepted.
    let parsed = parse_accepted("part engine : Engine;");
    insta::assert_snapshot!(render(&parsed.syntax()));
}

#[test]
fn part_usages_as_the_corpus_writes_them() {
    // Each line is from the pinned corpus.
    parse_accepted("part vehicle : Vehicle { }");
    parse_accepted("part eng : Engine;");
    parse_accepted("part interior { }");
    // "Interaction Sequencing Examples/ServerSequenceModel.sysml" line 8.
    parse_accepted("ref part subscriber;");
    // SimpleVehicleModel.sysml line 1491.
    parse_accepted("abstract part vehicleFamily { }");
    // "10-Analysis and Trades/10b-Trade-off Among Alternative Configurations" line 40.
    parse_accepted("variation part engineChoice;");
    // The same file, line 80.
    parse_accepted("in part anEngine;");
}

#[test]
fn a_part_usage_builds_the_nodes_the_grammar_names() {
    let parsed = parse_accepted("part engine : Engine, Base::Motor { }");
    let rendered = render(&parsed.syntax());
    for node in [
        "PackageMember",
        "PartUsage",
        "OccurrenceUsagePrefix",
        "KwPart",
        "Usage",
        "UsageDeclaration",
        "Identification",
        "FeatureSpecializationPart",
        "Typings",
        "TypedBy",
        "FeatureTyping",
        "OwnedFeatureTyping",
        "QualifiedName",
        "UsageCompletion",
        "UsageBody",
        "DefinitionBody",
    ] {
        assert!(rendered.contains(node), "no {node} node:\n{rendered}");
    }
    // Typings = TypedBy ( ',' FeatureTyping )* — two types, one TypedBy.
    // Counted as whole lines: `FeatureTyping` is a substring of `OwnedFeatureTyping`,
    // so a substring count would report four.
    assert_eq!(nodes_named(&rendered, "FeatureTyping"), 2, "{rendered}");
    assert_eq!(
        nodes_named(&rendered, "OwnedFeatureTyping"),
        2,
        "{rendered}"
    );
    assert_eq!(nodes_named(&rendered, "TypedBy"), 1, "{rendered}");
    // No prefix keyword was written, so none of the prefix's parts is built.
    assert!(!rendered.contains("BasicUsagePrefix"), "{rendered}");
    assert!(!rendered.contains("RefPrefix"), "{rendered}");
}

#[test]
fn a_part_usage_need_not_be_typed() {
    // UsageDeclaration = Identification FeatureSpecializationPart?, both nullable
    // (SysML 8.2.2.6.2, and Identification at 8.2.2.2).
    parse_accepted("part engine;");
    parse_accepted("part;");
    let parsed = parse_accepted("part engine;");
    assert!(
        !render(&parsed.syntax()).contains("FeatureSpecializationPart"),
        "an untyped usage builds no specialization part"
    );
}

#[test]
fn typed_by_may_be_spelled_out() {
    // TypedBy = ( ':' | 'defined' 'by' ) FeatureTyping (SysML 8.2.2.6.5). The corpus
    // writes only ':' — every 'defined by' in it is prose inside a comment — so this
    // expectation comes from the clause.
    parse_accepted("part engine defined by Engine;");
}

#[test]
fn the_usage_prefix_takes_its_keywords_in_the_order_the_clause_gives() {
    // RefPrefix = FeatureDirection? 'derived'? ( 'abstract' | 'variation' )?
    // 'constant'?, then BasicUsagePrefix adds 'ref'?, then OccurrenceUsagePrefix
    // adds 'individual'? and PortionKind? (SysML 8.2.2.6.2, 8.2.2.9.2).
    let parsed = parse_accepted("in derived abstract constant ref individual snapshot part p;");
    let rendered = render(&parsed.syntax());
    for node in [
        "OccurrenceUsagePrefix",
        "BasicUsagePrefix",
        "RefPrefix",
        "FeatureDirection",
        "PortionKind",
    ] {
        assert!(rendered.contains(node), "no {node} node:\n{rendered}");
    }
}

#[test]
fn each_feature_direction_and_portion_kind_is_accepted() {
    for direction in ["in", "out", "inout"] {
        parse_accepted(&format!("{direction} part p;"));
    }
    for portion in ["snapshot", "timeslice"] {
        parse_accepted(&format!("{portion} part p;"));
    }
}

#[test]
fn a_part_usage_is_not_a_part_definition() {
    // The two differ only by 'def' (SysML 8.2.2.11). Each must build its own node.
    let usage = render(&parse_accepted("part Engine;").syntax());
    assert!(usage.contains("PartUsage"), "{usage}");
    assert!(!usage.contains("PartDefinition"), "{usage}");

    let definition = render(&parse_accepted("part def Engine;").syntax());
    assert!(definition.contains("PartDefinition"), "{definition}");
    assert!(!definition.contains("PartUsage"), "{definition}");
}

#[test]
fn a_part_usage_holds_members_in_its_body() {
    // UsageBody = DefinitionBody = ';' | '{' DefinitionBodyItem* '}' (SysML 8.2.2.6.1).
    let parsed = parse_accepted("part vehicle : Vehicle { part engine : Engine; }");
    let rendered = render(&parsed.syntax());
    assert_eq!(rendered.matches("PartUsage").count(), 2, "{rendered}");
}

#[test]
fn a_multiplicity_on_a_usage_is_reported_not_accepted() {
    // `part frontSeat[2];` is valid SysML and appears in the corpus, but
    // MultiplicityPart (KerML 8.2.4.3.1) needs an expression parser and is not
    // implemented: a rejection by absence. Replace this when MultiplicityPart lands.
    parse_rejected("part frontSeat[2];");
}

#[test]
fn a_value_on_a_usage_is_reported_not_accepted() {
    // ValuePart = FeatureValue = ( '=' | ':=' | 'default' ... ) OwnedExpression
    // (SysML 8.2.2.6.2). FeatureValue needs an expression parser and is not
    // implemented: a rejection by absence. Replace this when ValuePart lands.
    parse_rejected("part engine : Engine = x;");
}

#[test]
fn an_end_usage_prefix_is_reported_not_accepted() {
    // OccurrenceUsagePrefix's other alternative, EndUsagePrefix = 'end'
    // OwnedCrossFeatureMember? (SysML 8.2.2.9.2, 8.2.2.6.2), is not implemented:
    // a rejection by absence. The corpus writes no `end part`, but the clause admits
    // it. Replace this when EndUsagePrefix lands.
    parse_rejected("end part p;");
}

#[test]
fn parsing_a_part_usage_never_panics_on_truncated_input() {
    let source = "in derived abstract constant ref individual snapshot part <e> engine : Engine, Base::Motor { part inner; }";
    for end in 0..=source.len() {
        if let Some(prefix) = source.get(..end) {
            assert_eq!(parse(prefix).text(), prefix);
        }
    }
}

// -- FeatureSpecialization's other alternatives, SysML 8.2.2.6.5 ------------------
//
//   FeatureSpecialization = Typings | Subsettings | References | Crosses
//                         | Redefinitions
//   Subsettings           = Subsets ( ',' OwnedSubsetting )*
//   Subsets               = SUBSETS OwnedSubsetting
//   Redefinitions         = Redefines ( ',' OwnedRedefinition )*
//   Redefines             = REDEFINES OwnedRedefinition
//   References            = REFERENCES OwnedReferenceSubsetting
//   Crosses               = CROSSES OwnedCrossSubsetting
//   SUBSETS   = ':>'  | 'subsets'                                (SysML 8.2.2.1.2)
//   REDEFINES = ':>>' | 'redefines'
//   REFERENCES = '::>' | 'references'
//   CROSSES   = '=>'  | 'crosses'

#[test]
fn a_usage_may_subset_another() {
    // SimpleVehicleModel.sysml line 1241.
    let parsed = parse_accepted("part vehicle_UnitUnderTest :> vehicle_b;");
    let rendered = render(&parsed.syntax());
    for node in [
        "FeatureSpecializationPart",
        "Subsettings",
        "Subsets",
        "OwnedSubsetting",
    ] {
        assert!(rendered.contains(node), "no {node} node:\n{rendered}");
    }
    // "29. Expressions/Car Mass Rollup Example 2.sysml" line 14.
    parse_accepted("part engine :> carParts { }");
}

#[test]
fn a_usage_may_redefine_another() {
    // "27. Occurrences/Message Payload Example.sysml" line 21.
    let parsed = parse_accepted("ref part vehicle :>> vehicle1;");
    let rendered = render(&parsed.syntax());
    for node in ["Redefinitions", "Redefines", "OwnedRedefinition"] {
        assert!(rendered.contains(node), "no {node} node:\n{rendered}");
    }
}

#[test]
fn a_usage_may_reference_or_cross_another() {
    // Neither spelling appears on a usage in the pinned corpus — the corpus's only
    // '::>' is an InterfaceEnd's `NAME REFERENCES` (SysML 8.2.2.14.2), which is a
    // different production, and it writes no '=>' at all. Both expectations come
    // from the clause.
    let referenced = render(&parse_accepted("part p ::> q;").syntax());
    for node in ["References", "OwnedReferenceSubsetting"] {
        assert!(referenced.contains(node), "no {node} node:\n{referenced}");
    }
    let crossed = render(&parse_accepted("part p => q;").syntax());
    for node in ["Crosses", "OwnedCrossSubsetting"] {
        assert!(crossed.contains(node), "no {node} node:\n{crossed}");
    }
}

#[test]
fn every_specialization_operator_may_be_spelled_out() {
    // SysML 8.2.2.1.2 gives each a word form as well as a symbol.
    parse_accepted("part p subsets q;");
    parse_accepted("part p redefines q;");
    parse_accepted("part p references q;");
    parse_accepted("part p crosses q;");
    // And ':' | 'defined' 'by' for Typings, already covered above.
}

#[test]
fn subsettings_and_redefinitions_take_a_comma_separated_list() {
    // Subsettings = Subsets ( ',' OwnedSubsetting )*, and Redefinitions likewise.
    let subsets = render(&parse_accepted("part p :> a, b, c;").syntax());
    assert_eq!(nodes_named(&subsets, "OwnedSubsetting"), 3, "{subsets}");
    let redefines = render(&parse_accepted("part p :>> a, b;").syntax());
    assert_eq!(
        nodes_named(&redefines, "OwnedRedefinition"),
        2,
        "{redefines}"
    );
}

#[test]
fn references_and_crosses_take_exactly_one_target() {
    // Unlike Subsettings and Redefinitions, neither carries a repetition
    // (SysML 8.2.2.6.5), so a comma after the target is not part of them.
    parse_rejected("part p ::> a, b;");
    parse_rejected("part p => a, b;");
}

#[test]
fn specializations_may_be_written_together() {
    // FeatureSpecializationPart = FeatureSpecialization+ ... (KerML 8.2.4.3.1), so
    // more than one may appear, in any order.
    let parsed = parse_accepted("part p : Engine :> carParts :>> old;");
    let rendered = render(&parsed.syntax());
    for node in ["Typings", "Subsettings", "Redefinitions"] {
        assert!(rendered.contains(node), "no {node} node:\n{rendered}");
    }
    assert_eq!(
        nodes_named(&rendered, "FeatureSpecializationPart"),
        1,
        "{rendered}"
    );
}

// -- AttributeUsage, SysML 8.2.2.7 ------------------------------------------------
//
//   AttributeUsage         = UsagePrefix 'attribute' Usage
//   UsagePrefix            = UnextendedUsagePrefix UsageExtensionKeyword*  (8.2.2.6.2)
//   UnextendedUsagePrefix  = EndUsagePrefix | BasicUsagePrefix
//
// An attribute's prefix is UsagePrefix, not OccurrenceUsagePrefix: an attribute is
// not an occurrence, so it carries no 'individual' and no PortionKind.

#[test]
fn an_attribute_usage_is_a_package_member() {
    // The rejection-by-absence case this replaces, now accepted.
    let parsed = parse_accepted("attribute mass : MassValue;");
    insta::assert_snapshot!(render(&parsed.syntax()));
}

#[test]
fn attribute_usages_as_the_corpus_writes_them() {
    // Each line is from the pinned corpus.
    parse_accepted("attribute mass : MassValue;");
    parse_accepted("attribute mass :> ISQ::mass;");
    parse_accepted("attribute m : MassValue;");
    parse_accepted("attribute isMandatory : Boolean;");
}

#[test]
fn an_attribute_usage_builds_the_nodes_the_grammar_names() {
    let parsed = parse_accepted("ref attribute mass :> ISQ::mass;");
    let rendered = render(&parsed.syntax());
    for node in [
        "AttributeUsage",
        "UsagePrefix",
        "BasicUsagePrefix",
        "KwAttribute",
        "Usage",
        "UsageDeclaration",
        "Subsettings",
        "UsageBody",
    ] {
        assert!(rendered.contains(node), "no {node} node:\n{rendered}");
    }
    // An attribute is not an occurrence, so its prefix is UsagePrefix and it builds
    // no OccurrenceUsagePrefix (SysML 8.2.2.7 against 8.2.2.11).
    assert!(!rendered.contains("OccurrenceUsagePrefix"), "{rendered}");
}

#[test]
fn an_attribute_is_not_an_occurrence_and_takes_no_portion_kind() {
    // UsagePrefix has no 'individual' and no PortionKind; those belong to
    // OccurrenceUsagePrefix (SysML 8.2.2.9.2), which an attribute does not use.
    parse_rejected("snapshot attribute mass : MassValue;");
    parse_rejected("individual attribute mass : MassValue;");
}

#[test]
fn an_attribute_usage_is_not_an_attribute_definition() {
    // The two differ by 'def', as part usage and part definition do. AttributeDefinition
    // is not implemented, so it is still reported: a rejection by absence.
    let usage = render(&parse_accepted("attribute Mass;").syntax());
    assert!(usage.contains("AttributeUsage"), "{usage}");
    parse_rejected("attribute def Mass;");
}

#[test]
fn an_attribute_may_be_a_member_of_a_part() {
    let parsed = parse_accepted("part engine : Engine { attribute mass : MassValue; }");
    let rendered = render(&parsed.syntax());
    assert!(rendered.contains("PartUsage"), "{rendered}");
    assert!(rendered.contains("AttributeUsage"), "{rendered}");
}

// -- the other usages of the same shape -------------------------------------------
//
//   ItemUsage        = OccurrenceUsagePrefix 'item'      Usage   (SysML 8.2.2.10)
//   OccurrenceUsage  = OccurrenceUsagePrefix 'occurrence' Usage  (SysML 8.2.2.9.2)
//   PortUsage        = OccurrenceUsagePrefix 'port'      Usage   (SysML 8.2.2.12)
//   RenderingUsage   = OccurrenceUsagePrefix 'rendering' Usage   (SysML 8.2.2.26.3)
//   EnumerationUsage = UsagePrefix           'enum'      Usage   (SysML 8.2.2.8)
//
// Each is a prefix, one keyword and the Usage spine. The occurrence four take an
// OccurrenceUsagePrefix and so may carry 'individual' and a PortionKind; an
// enumeration takes a UsagePrefix and may not, as an attribute may not.

#[test]
fn the_remaining_usages_build_their_own_nodes() {
    for (source, node) in [
        ("item wheel : Wheel;", "ItemUsage"),
        ("occurrence o : Thing;", "OccurrenceUsage"),
        ("port fuelPort : FuelPort;", "PortUsage"),
        ("rendering r : AsTable;", "RenderingUsage"),
        ("enum green : Color;", "EnumerationUsage"),
    ] {
        let rendered = render(&parse_accepted(source).syntax());
        assert!(
            rendered.contains(node),
            "{source} built no {node}:\n{rendered}"
        );
        assert!(rendered.contains("UsageDeclaration"), "{rendered}");
        assert!(rendered.contains("UsageBody"), "{rendered}");
    }
}

#[test]
fn the_remaining_usages_as_the_corpus_writes_them() {
    // Each line is from the pinned corpus.
    // "17. Control/Camera.sysml" lines 9, 13 and 14.
    parse_accepted("ref item scene : Scene;");
    parse_accepted("in ref item scene : Scene;");
    parse_accepted("out ref item realImage : Image;");
    // "12. Binding Connectors/Binding Connectors Example-1.sysml" line 10 — the
    // redefinition spelling this batch depends on.
    parse_accepted("port redefines fuelTankPort { }");
    // "27. Occurrences/Interaction Realization-2.sysml" line 5.
    parse_accepted("port setSpeedPort { }");
    // SimpleVehicleModel.sysml line 785.
    parse_accepted("occurrence CruiseControl1 { }");
    // "06. Enumeration Definitions/Enumeration Definitions-1.sysml" lines 5 to 7.
    parse_accepted("enum green;");
    // "42. Views/Views Example.sysml" line 16, and ViewTest.sysml line 38.
    parse_accepted("rendering r2;");
}

#[test]
fn an_enumeration_is_not_an_occurrence() {
    // EnumerationUsage takes a UsagePrefix (SysML 8.2.2.8), which carries no
    // 'individual' and no PortionKind — the same distinction AttributeUsage draws.
    parse_rejected("snapshot enum green;");
    // The occurrence usages do carry them.
    parse_accepted("snapshot item wheel;");
    parse_accepted("individual occurrence o;");
    parse_accepted("timeslice port p;");
}

#[test]
fn each_remaining_usage_is_not_its_definition() {
    // The `def` separates usage from definition throughout (SysML 8.2.2.6.1). None of
    // these definitions is implemented, so each is still reported: rejection by
    // absence, which the usage landing here does not change.
    for source in [
        "item def Wheel;",
        "occurrence def Thing;",
        "port def FuelPort;",
        "rendering def AsTable;",
        "enum def Color;",
    ] {
        parse_rejected(source);
    }
}

#[test]
fn usages_nest_in_one_another() {
    let parsed = parse_accepted(
        "part vehicle : Vehicle { port fuelPort : FuelPort; item fuel : Fuel; attribute mass : MassValue; }",
    );
    let rendered = render(&parsed.syntax());
    for node in ["PartUsage", "PortUsage", "ItemUsage", "AttributeUsage"] {
        assert!(rendered.contains(node), "no {node} node:\n{rendered}");
    }
}
