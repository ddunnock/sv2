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

use sv2_syntax::{DiagnosticCode, Language, Parse, Severity, SyntaxElement, SyntaxNode, parse};

/// Parse text the `SysML` grammar accepts, asserting that nothing was reported.
///
/// Every case in this file is `SysML`: the productions it covers are stated in
/// `SysML` 8.2.2, and several of them — `PartDefinition`, the usages,
/// `ElementFilterMember` at a root — are not reachable from `KerML`'s start symbol at
/// all (ADR-0014). The `KerML` cases live in tests/kerml.rs.
fn parse_accepted(source: &str) -> Parse {
    let parsed = parse(source, Language::SysMl);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    parsed
}

/// Parse text the grammar rejects, asserting that it is reported and that the tree
/// still carries every byte.
fn parse_rejected(source: &str) -> Parse {
    let parsed = parse(source, Language::SysMl);
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

/// The DIRECT children of the first `kind` node in `rendered`, in order.
///
/// `subtree` gives descendants, which answers "contains". A production says what it
/// OWNS, and the two differ by exactly one level: an `EmptyResultMember` belonging to a
/// nested operand is in the subtree of every node above it, and belongs to none of them.
fn child_kinds(rendered: &str, kind: &str) -> Vec<String> {
    let body = subtree(rendered, kind);
    let mut lines = body.lines();
    let Some(head) = lines.next() else {
        return Vec::new();
    };
    let depth = head.len() - head.trim_start().len();
    lines
        .filter(|l| l.len() - l.trim_start().len() == depth + 2)
        .map(|l| l.split_whitespace().next().unwrap_or_default().to_owned())
        .collect()
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
            let parsed = parse(prefix, Language::SysMl);
            assert_eq!(parsed.text(), prefix);
        }
    }
}

// -- annotating elements as members, SysML 8.2.2.6.1 ------------------------------
//
// DefinitionElement's third alternative is AnnotatingElement, so an annotating
// element is a PackageMember and a DefinitionMember without any production of its
// own. AnnotatingMember exists in the grammar but is referenced only by
// EnumerationBody, which is not implemented, so it is not built here.

#[test]
fn an_annotating_element_is_a_definition_element() {
    parse_accepted("doc /* what this file is */");
    parse_accepted("comment C about X /* on X */");
    parse_accepted("rep r language \"alf\" /* f(); */");
    parse_accepted("package P { doc /* on P */ }");
    parse_accepted("part def V { doc /* on V */ }");
}

#[test]
fn an_annotating_member_may_carry_a_visibility() {
    // MemberPrefix is part of the membership, not of the annotating element, so the
    // same prefix that precedes a package precedes a doc.
    parse_accepted("package P { private doc /* internal */ }");
}

#[test]
fn a_metadata_annotating_element_is_not_implemented() {
    // The fourth alternative, MetadataUsage in SysML by the recorded deviation.
    parse_rejected("package P { metadata Safety about Q; }");
}

// -- the definitions, SysML 8.2.2 -------------------------------------------------
//
// Eight productions of one shape, `<prefix> KEYWORD 'def' Definition`, differing in
// the keyword and in which prefix they take.

/// Every definition keyword of the shared spine, and whether it is an occurrence.
const DEFINITION_KEYWORDS: [(&str, bool); 8] = [
    ("attribute", false),
    ("occurrence", true),
    ("item", true),
    ("part", true),
    ("connection", true),
    ("flow", true),
    ("allocation", true),
    ("rendering", true),
];

#[test]
fn every_definition_keyword_reads_the_same_spine() {
    for (keyword, _) in DEFINITION_KEYWORDS {
        parse_accepted(&format!("{keyword} def A;"));
        parse_accepted(&format!("{keyword} def A {{ }}"));
        parse_accepted(&format!("abstract {keyword} def <a> A :> B;"));
    }
}

#[test]
fn only_an_occurrence_definition_may_be_individual() {
    // OccurrenceDefinitionPrefix carries `individual`; DefinitionPrefix does not
    // (SysML 8.2.2.9.1 against 8.2.2.6.1). Held as a file by
    // tests/rejection/attribute-definition-is-not-an-occurrence.sysml.
    for (keyword, is_occurrence) in DEFINITION_KEYWORDS {
        let source = format!("individual {keyword} def A;");
        if is_occurrence {
            parse_accepted(&source);
        } else {
            parse_rejected(&source);
        }
    }
}

#[test]
fn a_definition_is_told_from_the_usage_spelled_the_same_way() {
    // Every definition keyword but three also opens a usage, and only the `def`
    // separates them (SysML 8.2.2.6.1). Both must parse, as different nodes.
    parse_accepted("part def V;");
    parse_accepted("part v;");
    parse_accepted("attribute def A;");
    parse_accepted("attribute a;");
    parse_accepted("part def V { part inner; attribute def A; }");
}

#[test]
fn a_definition_whose_body_is_not_a_definition_body_is_not_in_the_table() {
    // Fourteen of the twenty-two `def` productions end in a specialised body, and of
    // those bodies only RequirementBody is implemented. PortDefinition shares the spine
    // but adds a ConjugatedPortDefinitionMember. Each has a file in tests/rejection/.
    //
    // `requirement def R;` was in this list and is not any more: RequirementBody is
    // implemented, and it left the list because the production is now read, not because
    // the claim was relaxed. The four that remain are still absent.
    for source in ["action def Brake;", "calc def C;", "state def S;"] {
        parse_rejected(source);
    }
    // `requirement def` left this list when RequirementBody landed, and
    // `constraint def` leaves it now that CalculationBody has. Both left because the
    // production is read, not because the claim was relaxed. `calc def` shares
    // ConstraintDefinition's body and is still absent, because nothing dispatches to
    // it — held by tests/rejection/calculation-definition-is-not-implemented.sysml.
    parse_accepted("requirement def R;");
    parse_accepted("constraint def C;");
}

// -- CalculationBody, SysML 8.2.2.19 ----------------------------------------------
//
// CalculationBody        = ';' | '{' CalculationBodyPart '}'
// CalculationBodyPart    = CalculationBodyItem* ResultExpressionMember?
// ResultExpressionMember = MemberPrefix? OwnedExpression
//
// The first body whose last part is an expression rather than a member. Reached through
// ConstraintDefinition (8.2.2.20), which is here as its caller.

#[test]
fn a_calculation_body_ends_in_an_expression_with_no_semicolon() {
    // The corpus form. ResultExpressionMember has no terminator, which is what makes it
    // tell-apart-able from an item only by lookahead.
    parse_accepted("constraint def C { a <= b }");
    parse_accepted("constraint def C { 1 + 2 * 3 }");
    parse_accepted("constraint def C { (a + b) <= c }");
    // A bare name alone is the hardest case: it is also how a DefaultReferenceUsage
    // opens.
    parse_accepted("constraint def C { x }");
}

#[test]
fn a_calculation_body_takes_both_forms_and_may_hold_no_expression() {
    // ';' is one of the two alternatives, and the ResultExpressionMember is optional, so
    // a braced body with only items is a body with no expression at all.
    parse_accepted("constraint def C;");
    parse_accepted("constraint def C { }");
    parse_accepted("constraint def C { attribute x; }");
}

#[test]
fn a_calculation_body_reads_the_items_a_definition_body_reads() {
    // CalculationBodyItem reaches ActionBodyItem reaches NonBehaviorBodyItem, whose
    // Import, AliasMember and DefinitionMember are the three a definition body reads
    // (SysML 8.2.2.17.1). The other alternatives are the action layer and are absent.
    parse_accepted("constraint def C { doc /* why this holds */ a <= b }");
    parse_accepted("constraint def C { private import ISQ::*; a <= b }");
    parse_accepted("constraint def C { alias q for r; a <= b }");
    parse_accepted("constraint def C { attribute x; attribute y; x + y > 0 }");
}

#[test]
fn an_item_with_a_braced_body_is_not_mistaken_for_the_expression() {
    // The case the lookahead exists for. `part def Inner { }` is an item that ends in a
    // brace rather than a semicolon, and `a` after it is the result expression. A rule
    // that only asked "is there a `;` before the `}`" would read the whole run as one
    // expression.
    let rendered = render(&parse_accepted("constraint def C { part def Inner { } a }").syntax());
    assert_eq!(nodes_named(&rendered, "PartDefinition"), 1, "{rendered}");
    assert_eq!(
        nodes_named(&rendered, "ResultExpressionMember"),
        1,
        "{rendered}"
    );
}

#[test]
fn a_bare_name_is_an_item_when_a_usage_completion_follows_it() {
    // Both a DefaultReferenceUsage and an expression open on a bare name, and only what
    // comes after separates them: a usage reaches a `;` or a braced body
    // (SysML 8.2.2.6.2), an expression reaches the enclosing `}`.
    let item = render(&parse_accepted("constraint def C { x; }").syntax());
    assert_eq!(nodes_named(&item, "DefaultReferenceUsage"), 1, "{item}");
    assert_eq!(nodes_named(&item, "ResultExpressionMember"), 0, "{item}");

    let expression = render(&parse_accepted("constraint def C { x }").syntax());
    assert_eq!(
        nodes_named(&expression, "ResultExpressionMember"),
        1,
        "{expression}"
    );
    assert_eq!(
        nodes_named(&expression, "DefaultReferenceUsage"),
        0,
        "{expression}"
    );

    // And both in one body: the first has a completion, the second does not.
    let both = render(&parse_accepted("constraint def C { x; y }").syntax());
    assert_eq!(nodes_named(&both, "DefaultReferenceUsage"), 1, "{both}");
    assert_eq!(nodes_named(&both, "ResultExpressionMember"), 1, "{both}");
}

#[test]
fn a_constraint_definition_owns_no_definition_node() {
    // Like RequirementDefinition, it names the declaration and the body separately
    // rather than taking a Definition (SysML 8.2.2.20).
    let rendered = render(&parse_accepted("constraint def C { a <= b }").syntax());
    assert_eq!(
        nodes_named(&rendered, "ConstraintDefinition"),
        1,
        "{rendered}"
    );
    assert_eq!(nodes_named(&rendered, "CalculationBody"), 1, "{rendered}");
    assert_eq!(
        nodes_named(&rendered, "CalculationBodyPart"),
        1,
        "{rendered}"
    );
    assert_eq!(nodes_named(&rendered, "DefinitionBody"), 0, "{rendered}");
    assert_eq!(nodes_named(&rendered, "Definition"), 0, "{rendered}");
}

#[test]
fn a_calculation_body_does_not_admit_a_return_or_an_action() {
    // CalculationBodyItem = ActionBodyItem | ReturnParameterMember, and
    // ReturnParameterMember plus three of ActionBodyItem's four alternatives are the
    // part that is absent. Held as a file by
    // tests/rejection/calculation-body-return-parameter-member-is-not-implemented.sysml.
    parse_rejected("constraint def C { return x; }");
    parse_rejected("constraint def C { first a then b; }");
}

#[test]
fn a_constraint_definition_needs_a_body_and_a_def() {
    // CalculationBody is not optional. Held as a file by
    // tests/rejection/constraint-definition-missing-calculation-body.sysml.
    parse_rejected("constraint def C");
    // Without `def` it is a ConstraintUsage, unimplemented, and it is what
    // `require constraint { ... }` will need. Held as a file by
    // tests/rejection/constraint-usage-is-not-a-constraint-definition.sysml.
    parse_rejected("constraint c { a <= b }");
    // An unclosed body is still an error, and the expression inside it is still read.
    parse_rejected("constraint def C { a <= b");
}

// -- FeatureChainExpression, KerML 8.2.5.8.2 --------------------------------------
//
// FeatureChainExpression = NonFeatureChainPrimaryArgumentMember '.' FeatureChainMember
//
// PrimaryExpression's other alternative, written postfix, and the only expression in
// this parser that folds to the LEFT.

#[test]
fn a_feature_chain_reads_the_postfix_dot() {
    // The corpus form, from vendor/corpus/sysml/src/validation/08-Requirements:
    // `vehicle.fuelMass == vehicle.fuelFullMass`.
    parse_accepted("constraint def C { a.b }");
    parse_accepted("constraint def C { a.b.c.d }");
    parse_accepted("constraint def C { vehicle.fuelMass == vehicle.fuelFullMass }");
    // A chain link is a QualifiedName, so `::` may appear within a link.
    parse_accepted("constraint def C { x::y.z }");
}

#[test]
fn a_feature_chain_folds_to_the_left() {
    // `a.b.c` is `(a.b).c`, NOT `a.(b.c)`. The member is called
    // NonFeatureChainPrimaryArgumentMember but its body is PrimaryArgument, which
    // reaches PrimaryExpression and so admits a chain on the left — both the clause and
    // Tier B' state it that way, and the Pilot builds the same association with a
    // repetition that folds (KerMLExpressions.xtext:301, :319).
    let rendered = render(&parse_accepted("constraint def C { a.b.c }").syntax());
    assert_eq!(
        nodes_named(&rendered, "FeatureChainExpression"),
        2,
        "{rendered}"
    );
    // The OUTERMOST chain's left operand holds `a` and `b`; `c` is its member. Under a
    // right fold the left operand would hold only `a`.
    let left = subtree(&rendered, "NonFeatureChainPrimaryArgumentMember");
    assert!(left.contains(r#""a""#), "{left}");
    assert!(left.contains(r#""b""#), "{left}");
    assert!(!left.contains(r#""c""#), "{left}");
}

#[test]
fn a_feature_chain_owns_no_result_member() {
    // The metaclass IS an OperatorExpression (KerML 8.3.4.8.4) and every operator in the
    // infix table owns an EmptyResultMember. This production does not, because the BNF
    // writes EmptyResultMember explicitly where there is one — BinaryOperatorExpression
    // and FeatureReferenceExpression both name it (8.2.5.8.1, 8.2.5.8.3) — and
    // 8.2.5.8.2 does not. Adding one by analogy would put an element in the tree that
    // the grammar does not state.
    // Asked of the DIRECT children, not the subtree: the subtree does hold one, and it
    // belongs to the FeatureReferenceExpression on the left, which has one by
    // 8.2.5.8.3. Confusing "contains" with "owns" is how this test first passed the
    // wrong claim.
    let rendered = render(&parse_accepted("constraint def C { a.b }").syntax());
    assert_eq!(
        child_kinds(&rendered, "FeatureChainExpression"),
        [
            "NonFeatureChainPrimaryArgumentMember",
            "Dot",
            "FeatureReferenceMember"
        ],
        "{rendered}"
    );
    // The one in the tree is the left operand's, one level further down.
    assert_eq!(nodes_named(&rendered, "EmptyResultMember"), 1, "{rendered}");
    assert_eq!(
        child_kinds(&rendered, "FeatureReferenceExpression"),
        ["FeatureReferenceMember", "EmptyResultMember"],
        "{rendered}"
    );
    // And the contrast that makes the point: a binary operator DOES own one (8.2.5.8.1).
    let sum = render(&parse_accepted("constraint def C { a + b }").syntax());
    assert!(
        child_kinds(&sum, "BinaryOperatorExpression").contains(&"EmptyResultMember".to_owned()),
        "{sum}"
    );
}

#[test]
fn a_dot_after_a_primary_is_not_always_a_chain() {
    // Three other productions put a `.` after a primary, and none is a chain
    // (KerML 8.2.5.8.2, 8.2.5.8.3). All three are unimplemented, and must stay that way
    // rather than be quietly accepted as chains. Held as files by
    // tests/rejection/metadata-access-expression-is-not-implemented.sysml and
    // tests/rejection/collect-expression-is-not-implemented.sysml.
    parse_rejected("constraint def C { E.metadata }");
    parse_rejected("constraint def C { x.{ a } }");
    parse_rejected("constraint def C { x.?{ a } }");
    // A chain needs a name after the dot; `a.` alone is neither.
    parse_rejected("constraint def C { a. }");
}

#[test]
fn a_feature_chain_binds_tighter_than_any_infix_operator() {
    // It is a PrimaryExpression, so it is below every tier of table 6. `a.b + c.d` is
    // `(a.b) + (c.d)`: one addition over two chains, not a chain over an addition.
    let rendered = render(&parse_accepted("constraint def C { a.b + c.d }").syntax());
    assert_eq!(
        nodes_named(&rendered, "FeatureChainExpression"),
        2,
        "{rendered}"
    );
    assert_eq!(
        nodes_named(&rendered, "BinaryOperatorExpression"),
        1,
        "{rendered}"
    );
    // The addition is the outer node, so the chains are inside it.
    let sum = subtree(&rendered, "BinaryOperatorExpression");
    assert_eq!(nodes_named(&sum, "FeatureChainExpression"), 2, "{sum}");
}

#[test]
fn a_feature_chain_is_bounded_by_the_depth_limit() {
    // Each link WRAPS the last, so the tree is as deep as the chain is long even though
    // the fold is a loop and uses no stack. The first draft of this test asserted the
    // opposite — that chain length was free — and a 50000-link chain aborted the test
    // thread, which is what invariant 3 forbids. Chains are counted against MAX_DEPTH
    // like any other nesting.
    //
    // Losslessness at that length is asserted in tests/roundtrip.rs beside the three
    // other constructs the same guard covers.
    let source = format!("constraint def C {{ a{} }}", ".b".repeat(50_000));
    let parsed = parse(&source, Language::SysMl);
    assert!(
        parsed
            .errors()
            .iter()
            .any(|d| d.code() == DiagnosticCode::TooDeeplyNested),
        "expected a depth diagnostic, got {:?}",
        parsed.errors()
    );
    // A chain shorter than the limit is read whole and reports nothing.
    parse_accepted(&format!("constraint def C {{ a{} }}", ".b".repeat(100)));
}

#[test]
fn the_result_expression_is_only_read_where_a_body_ends_in_one() {
    // ends_in_result_expression is asked of the body, not of the token. A definition
    // body has no ResultExpressionMember alternative (SysML 8.2.2.6.1), so a bare
    // expression in one is recovered over rather than read.
    parse_rejected("part def V { a <= b }");
    parse_rejected("requirement def R { a <= b }");
    parse_accepted("constraint def C { a <= b }");
}

// -- RequirementDefinition, SysML 8.2.2.21.1 --------------------------------------
//
// RequirementDefinition = OccurrenceDefinitionPrefix 'requirement' 'def'
//                         DefinitionDeclaration RequirementBody
//
// Off the shared spine at the BODY end: it names the declaration and the body
// separately where the eight take a Definition.

#[test]
fn a_requirement_definition_reads_the_declaration_and_its_own_body() {
    parse_accepted("requirement def R;");
    parse_accepted("requirement def R { }");
    // Both forms taken from vendor/corpus/sysml/src/training/32. Requirements/
    // Requirement Definitions.sysml, which writes the reqId as a short name.
    parse_accepted("requirement def MassLimitationRequirement { }");
    parse_accepted(
        "requirement def <'1'> VehicleMassLimitationRequirement :> MassLimitationRequirement { }",
    );
}

#[test]
fn a_requirement_definition_owns_no_definition_node() {
    // The eight on the spine take `Definition = DefinitionDeclaration DefinitionBody`,
    // so their trees carry a Definition node. This production names the two parts
    // itself (SysML 8.2.2.21.1), so there is no such node and the body is a
    // RequirementBody rather than a DefinitionBody. Reading it with the shared spine
    // would invent one node and mislabel the other.
    // Counted by whole line rather than by substring: `Definition` is a prefix of
    // `DefinitionDeclaration` and of `RequirementDefinition`, both of which ARE in this
    // tree, so a substring test for the absent node would find them and pass for the
    // wrong reason.
    let rendered = render(&parse_accepted("requirement def R;").syntax());
    assert_eq!(
        nodes_named(&rendered, "RequirementDefinition"),
        1,
        "{rendered}"
    );
    assert_eq!(nodes_named(&rendered, "RequirementBody"), 1, "{rendered}");
    assert_eq!(
        nodes_named(&rendered, "DefinitionDeclaration"),
        1,
        "{rendered}"
    );
    assert_eq!(nodes_named(&rendered, "DefinitionBody"), 0, "{rendered}");
    assert_eq!(nodes_named(&rendered, "Definition"), 0, "{rendered}");
}

#[test]
fn a_requirement_definition_is_an_occurrence() {
    // OccurrenceDefinitionPrefix, not DefinitionPrefix (SysML 8.2.2.21.1 against
    // 8.2.2.9.1), so `individual` is part of the prefix here as it is for a part.
    parse_accepted("individual requirement def R;");
    parse_accepted("abstract requirement def R;");
    parse_accepted("variation requirement def R;");
}

#[test]
fn a_requirement_body_admits_what_a_definition_body_admits() {
    // RequirementBodyItem = DefinitionBodyItem | six more (SysML 8.2.2.21.1). It is a
    // SUPERSET, so everything a definition body reads today a requirement body reads
    // too — which is the whole reason this production costs one method.
    parse_accepted("requirement def R { attribute massActual; }");
    parse_accepted("requirement def R { doc /* the mass shall be bounded */ }");
    parse_accepted("requirement def R { private import ISQ::*; }");
    parse_accepted("requirement def R { alias m for massActual; }");
    parse_accepted("requirement def R { part def Inner; }");
}

#[test]
fn a_requirement_body_does_not_admit_the_five_members_it_has_not_got() {
    // The part of RequirementBodyItem that is NOT DefinitionBodyItem, less SubjectMember
    // which is now implemented. Rejected by absence, not by rule — each is well-formed
    // SysML. Held as a file by
    // tests/rejection/requirement-body-constraint-member-is-not-implemented.sysml.
    parse_rejected("requirement def R { require constraint { a <= b } }");
    parse_rejected("requirement def R { assume constraint { a > 0 } }");
    parse_rejected("requirement def R { frame concern c; }");
    parse_rejected("requirement def R { actor operator; }");
    parse_rejected("requirement def R { stakeholder owner; }");
}

#[test]
fn a_requirement_definition_needs_a_body_and_a_def() {
    // RequirementBody is not optional. Held as a file by
    // tests/rejection/requirement-definition-missing-requirement-body.sysml.
    parse_rejected("requirement def R");
    // Without `def` it is a RequirementUsage, which is a different production and
    // unimplemented. Held as a file by
    // tests/rejection/requirement-usage-is-not-a-requirement-definition.sysml.
    parse_rejected("requirement r;");
}

#[test]
fn a_requirement_definition_nests_where_a_definition_element_may_go() {
    // DefinitionBodyItem reaches DefinitionMember reaches DefinitionElement, and a
    // RequirementDefinition is one of its thirty alternatives (SysML 8.2.2.6.1).
    parse_accepted("part def V { requirement def R; }");
    parse_accepted("package P { requirement def R { part def Inner; } }");
    parse_accepted("requirement def Outer { requirement def Inner; }");
}

// -- SubjectMember, SysML 8.2.2.21.1 ----------------------------------------------
//
// SubjectMember = MemberPrefix SubjectUsage
// SubjectUsage  = 'subject' UsageExtensionKeyword* Usage
//
// The first of RequirementBodyItem's six extra members, and the only one of them whose
// parts are all implemented: SubjectUsage ends in a Usage, which already exists.

#[test]
fn a_subject_member_is_a_usage_behind_a_keyword() {
    // The corpus form, from vendor/corpus/omg/SimpleVehicleModel.sysml:
    // `subject generateTorque:ActionDefinitions::GenerateTorque;`
    parse_accepted("requirement def R { subject vehicle : Vehicle; }");
    parse_accepted("requirement def R { subject s; }");
    parse_accepted("requirement def R { subject generateTorque : Actions::GenerateTorque; }");
    // MemberPrefix is a VisibilityIndicator?, so a visibility is admitted before it.
    parse_accepted("requirement def R { private subject vehicle : Vehicle; }");
}

#[test]
fn a_subject_member_owns_its_usage_through_its_own_membership() {
    // SubjectMember : SubjectMembership, not the DefinitionMember the body's ordinary
    // items are owned through (SysML 8.2.2.21.1, metaclass 8.3.21.11). Dispatched beside
    // NamespaceFeatureMember for that reason, and the tree has to show it.
    let rendered = render(&parse_accepted("requirement def R { subject v : Vehicle; }").syntax());
    // Read from the SubjectMember down, not from the root: the enclosing PackageMember
    // has a MemberPrefix of its own, and counting over the whole tree would read the
    // neighbour's node as this one's.
    let member = subtree(&rendered, "SubjectMember");
    assert_eq!(nodes_named(&member, "MemberPrefix"), 1, "{member}");
    assert_eq!(nodes_named(&member, "SubjectUsage"), 1, "{member}");
    // It ends in a Usage, which is what makes the typing work with no new code.
    assert_eq!(nodes_named(&member, "Usage"), 1, "{member}");
    // The subject is NOT owned through the body's ordinary member node.
    assert_eq!(nodes_named(&rendered, "DefinitionMember"), 0, "{rendered}");
    assert_eq!(nodes_named(&rendered, "SubjectMember"), 1, "{rendered}");
}

#[test]
fn a_subject_is_only_a_subject_member_where_the_grammar_reaches_one() {
    // RequirementBodyItem has a SubjectMember alternative and DefinitionBodyItem has
    // none (SysML 8.2.2.21.1 against 8.2.2.6.1), so the same eight characters are a
    // member in one body and not a construct at all in the other. This is the whole
    // reason Body::Requirement exists. Held as a file by
    // tests/rejection/subject-member-is-not-a-definition-body-item.sysml.
    parse_rejected("part def V { subject vehicle : Vehicle; }");
    parse_rejected("package P { subject vehicle : Vehicle; }");
    parse_rejected("attribute def A { subject s; }");
    // And it is not a root element either.
    parse_rejected("subject vehicle : Vehicle;");
    // The requirement body does reach it.
    parse_accepted("requirement def R { subject vehicle : Vehicle; }");
}

#[test]
fn a_subject_member_carries_no_prefix_metadata() {
    // SubjectUsage = 'subject' UsageExtensionKeyword* Usage, and UsageExtensionKeyword
    // is a PrefixMetadataMember (SysML 8.2.2.6.2), unimplemented everywhere in this
    // parser. Zero of them is the common case, which is why SubjectUsage is useful
    // without it — and why SubjectUsage is NOT marked for coverage while SubjectMember
    // is. Held as a file by
    // tests/rejection/subject-usage-carries-no-prefix-metadata.sysml.
    parse_rejected("requirement def R { subject #approved v : Vehicle; }");
}

#[test]
fn a_subject_may_name_nothing_at_all() {
    // `subject;` PARSES, and the first draft of this test asserted it did not. Every
    // part of a Usage below the body is optional:
    //   SysML 8.2.2.21.1 — SubjectUsage     = 'subject' UsageExtensionKeyword* Usage
    //   SysML 8.2.2.6.2  — Usage            = UsageDeclaration UsageCompletion
    //                      UsageDeclaration = Identification FeatureSpecializationPart?
    //   SysML 8.2.3.1    — Identification   = ( '<' NAME '>' )? ( NAME )?
    // so an anonymous subject is grammatical, exactly as `feature;` is in KerML. The
    // multiplicity 1 on SubjectMembership::ownedSubjectParameter (SysML 8.3.21.11) says
    // the parameter must EXIST, not that it must be named, and an anonymous usage is
    // still a usage. Whether it is useful is a constraint question and not this layer's
    // (ADR-0002).
    parse_accepted("requirement def R { subject; }");
    let member = subtree(
        &render(&parse_accepted("requirement def R { subject; }").syntax()),
        "SubjectMember",
    );
    assert_eq!(nodes_named(&member, "Usage"), 1, "{member}");
    assert_eq!(nodes_named(&member, "Identification"), 1, "{member}");
}

// -- PortDefinition, SysML 8.2.2.12 -----------------------------------------------
//
// PortDefinition = DefinitionPrefix 'port' 'def' Definition
//                  ConjugatedPortDefinitionMember
//
// Off the shared spine, because of a trailing part that consumes no tokens at all.

#[test]
fn a_port_definition_reads_the_definition_spine() {
    parse_accepted("port def P;");
    parse_accepted("port def P { }");
    parse_accepted("abstract port def <p> P :> Q;");
}

#[test]
fn a_port_definition_always_declares_its_conjugate() {
    // ConjugatedPortDefinitionMember consumes nothing, and is built anyway: `port def P;`
    // gives you `~P`, and the abstract syntax says three elements are there. Omitting
    // the nodes would leave a consumer to know to synthesise them.
    let rendered = render(&parse_accepted("port def P;").syntax());
    for node in [
        "ConjugatedPortDefinitionMember",
        "ConjugatedPortDefinition",
        "PortConjugation",
    ] {
        assert!(rendered.contains(node), "{node} missing from {rendered}");
    }
}

#[test]
fn a_port_definition_is_not_an_occurrence() {
    // DefinitionPrefix, not OccurrenceDefinitionPrefix (SysML 8.2.2.12 against
    // 8.2.2.9.1). Held as a file by
    // tests/rejection/port-definition-is-not-an-occurrence.sysml.
    parse_rejected("individual port def P;");
    // The occurrence definitions do carry it.
    parse_accepted("individual part def V;");
}

#[test]
fn a_port_usage_is_not_a_port_definition() {
    let usage = render(&parse_accepted("port p;").syntax());
    assert!(usage.contains("PortUsage"), "{usage}");
    assert!(!usage.contains("PortDefinition"), "{usage}");
}

// -- the diagnostics themselves ---------------------------------------------------

#[test]
fn only_errors_are_raised_today() {
    // Severity has three variants and this crate produces one of them. Asserted rather
    // than assumed, so that the first Warning or Info raised has to come here and say
    // what it is: the doc comment on Severity claims this, and a claim nothing checks
    // is how a doc comment stops being true.
    for source in [
        "package",
        "class Wrong;",
        "package P { filter",
        "/* never closed",
        "part def",
        "}}}",
    ] {
        for diagnostic in parse(source, Language::SysMl).errors() {
            assert_eq!(
                diagnostic.severity(),
                Severity::Error,
                "{source:?} raised {diagnostic:?}"
            );
        }
    }
}

#[test]
fn a_diagnostic_points_at_the_token_it_is_about() {
    // The whole point of the range: a caller underlines it. `class` is KerML's, so in
    // a SysML file it is unexpected, and the range must cover exactly those five bytes.
    let source = "package P { class Wrong; }";
    let parsed = parse(source, Language::SysMl);
    let first = parsed
        .errors()
        .iter()
        .find(|d| d.code() == DiagnosticCode::Unexpected)
        .expect("`class` is not a SysML element");
    let start = usize::from(first.range().start());
    let end = usize::from(first.range().end());
    // `get`, not a slice: the workspace forbids indexing a str, because a range landing
    // inside a multi-byte character panics — which is the same reason the range is
    // required to be on character boundaries in the first place.
    assert_eq!(source.get(start..end), Some("class"));
}

#[test]
fn a_diagnostic_about_the_end_of_the_file_is_an_empty_range_there() {
    // "expected X, found end of file" is about a position, not about any bytes.
    let source = "package P {";
    let parsed = parse(source, Language::SysMl);
    let last = parsed
        .errors()
        .last()
        .expect("an unclosed body is reported");
    assert!(last.range().is_empty(), "{last:?}");
    assert_eq!(usize::from(last.range().start()), source.len());
}

// -- the usages written without a keyword, SysML 8.2.2.6.2 ------------------------
//
// ReferenceUsage        = ( EndUsagePrefix | RefPrefix ) 'ref' Usage
// DefaultReferenceUsage = 'end'? RefPrefix
//                         ( Identification FeatureSpecializationPart?
//                         | FeatureSpecializationPart ) UsageCompletion

#[test]
fn a_reference_usage_carries_its_own_ref_keyword() {
    parse_accepted("package P { ref y; }");
    parse_accepted("package P { ref z : ScalarValues::Integer; }");
    parse_accepted("package P { in ref y : A; }");
}

#[test]
fn ref_before_a_usage_keyword_is_the_prefixs_ref_and_not_a_reference_usage() {
    // `ref attribute y;` is an AttributeUsage whose BasicUsagePrefix carries `ref`.
    // Only the absence of a usage keyword makes `ref y;` a ReferenceUsage, which is
    // why the keyword usages are asked first.
    let attribute =
        render(&parse_accepted("package P { derived constant ref attribute y :> x; }").syntax());
    assert!(attribute.contains("AttributeUsage"), "{attribute}");
    assert!(!attribute.contains("ReferenceUsage"), "{attribute}");

    let reference = render(&parse_accepted("package P { ref y; }").syntax());
    assert!(reference.contains("ReferenceUsage"), "{reference}");
    assert!(!reference.contains("AttributeUsage"), "{reference}");
}

#[test]
fn a_usage_may_be_written_with_no_keyword_at_all() {
    // DefaultReferenceUsage: the declaration alone carries it, as KerML's keywordless
    // Feature does. Both forms of its declaration are read.
    parse_accepted("package P { y : A; }");
    parse_accepted("package P { <y> yy : A; }");
    // The bare-specialization form, which names nothing: this is what the corpus
    // writes inside a definition body.
    parse_accepted("part def V { :>> length = 4800; }");
    parse_accepted("part def V { :>> self : Timeslice; }");
}

#[test]
fn a_keyword_is_not_read_as_a_keywordless_usage() {
    // A reserved keyword is not a name (KerML 8.2.2.6). Without that, every
    // `package P;` would be a usage called `package`.
    let package = render(&parse_accepted("package P;").syntax());
    assert!(package.contains("Package"), "{package}");
    assert!(!package.contains("DefaultReferenceUsage"), "{package}");
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
        parsed
            .errors()
            .iter()
            .any(|e| e.code() == DiagnosticCode::UnterminatedComment),
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
            assert_eq!(parse(prefix, Language::SysMl).text(), prefix);
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

// The two rejections that stood here — `part frontSeat[2];` for MultiplicityPart and
// `part engine : Engine = x;` for ValuePart — were rejections BY ABSENCE, each asking
// to be replaced when its production landed. Both have. The same two inputs are now
// asserted positively, by `a_multiplicity_bounds_a_usage_as_the_corpus_writes_it` and
// `a_usage_may_carry_a_value` in the expression section below.

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
            assert_eq!(parse(prefix, Language::SysMl).text(), prefix);
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
    // The two differ by 'def', as part usage and part definition do. Both are
    // implemented now, so the claim is no longer "the definition is reported" but the
    // stronger one: the same word reads as a different node on either side of `def`.
    let usage = render(&parse_accepted("attribute Mass;").syntax());
    assert!(usage.contains("AttributeUsage"), "{usage}");
    assert!(!usage.contains("AttributeDefinition"), "{usage}");

    let definition = render(&parse_accepted("attribute def Mass;").syntax());
    assert!(definition.contains("AttributeDefinition"), "{definition}");
    assert!(!definition.contains("AttributeUsage"), "{definition}");
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
    // The `def` separates usage from definition throughout (SysML 8.2.2.6.1). Three of
    // these definitions are implemented now and read as their own node; the usage
    // spelled the same way still reads as the usage.
    for (source, node) in [
        ("item def Wheel;", "ItemDefinition"),
        ("occurrence def Thing;", "OccurrenceDefinition"),
        ("rendering def AsTable;", "RenderingDefinition"),
    ] {
        let rendered = render(&parse_accepted(source).syntax());
        assert!(rendered.contains(node), "{rendered}");
        assert!(!rendered.contains("Usage"), "{rendered}");
    }
    // PortDefinition is implemented now, off the shared spine and with its own method,
    // so it reads as its own node.
    let port = render(&parse_accepted("port def FuelPort;").syntax());
    assert!(port.contains("PortDefinition"), "{port}");
    // Still rejection by absence: EnumerationDefinition takes an EnumerationBody rather
    // than a DefinitionBody, and tests/rejection/ names the clause.
    parse_rejected("enum def Color;");
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

// -- the expression layer, KerML 8.2.5.8 -------------------------------------------
//
// PRECEDENCE IS NOT IN THE GRAMMAR. KerML 8.2.5.8.1 note 2 states that the grouping
// of nested OperatorExpressions is not expressed in the productions and is given by
// that clause's table 6, so these cases are the parser's only check that it reads the
// table rather than the rule order. Each names the tiers it separates.

/// The `ArgumentValue` bodies directly under the first `kind` node, in source order.
///
/// An operand of an operator expression is always an `ArgumentValue` (or, for a
/// short-circuiting operator, an `ArgumentExpressionValue`), so this is what says
/// which operand a subexpression landed in — which is the whole of a precedence
/// claim. Depth is measured from the node, so only its own operands are returned and
/// not a nested expression's.
fn operands(rendered: &str, kind: &str) -> Vec<String> {
    let subtree = subtree(rendered, kind);
    let mut out = Vec::new();
    let mut lines = subtree.lines().peekable();
    while let Some(line) = lines.next() {
        // An operand is an `ArgumentValue`, or an `ArgumentExpressionValue` where the
        // operator short-circuits. Every other line is a membership on the way to one.
        if !matches!(line.trim(), "ArgumentValue" | "ArgumentExpressionValue") {
            continue;
        }
        let depth = line.len() - line.trim_start().len();
        // Take the whole operand and name it by its first line. CONSUMING its
        // descendants is what keeps a nested operand — the `b * c` of `a + b * c` —
        // from being counted as one of this operator's own.
        let mut body: Option<String> = None;
        while lines
            .peek()
            .is_some_and(|next| next.len() - next.trim_start().len() > depth)
        {
            let next = lines.next().unwrap_or_default();
            body.get_or_insert_with(|| next.trim().to_owned());
        }
        out.push(body.unwrap_or_default());
    }
    out
}

/// One tier as `docs/operator-precedence.toml` records it.
struct RecordedTier {
    n: u8,
    arity: String,
    assoc: String,
    operators: Vec<String>,
}

/// The `[[tier]]` blocks of `docs/operator-precedence.toml`.
///
/// The file is read at compile time rather than parsed with a TOML crate: the shape
/// wanted here is four keys, and hand-reading them needs no dependency.
fn recorded_tiers() -> Vec<RecordedTier> {
    let toml = include_str!("../../../docs/operator-precedence.toml");
    let mut tiers = Vec::new();
    for block in toml.split("[[tier]]").skip(1) {
        let mut tier = RecordedTier {
            n: 0,
            arity: String::new(),
            assoc: String::new(),
            operators: Vec::new(),
        };
        for line in block.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("n = ") {
                tier.n = rest.trim().parse().unwrap_or(0);
            } else if let Some(rest) = line.strip_prefix("arity = ") {
                rest.trim().trim_matches('"').clone_into(&mut tier.arity);
            } else if let Some(rest) = line.strip_prefix("assoc = ") {
                rest.trim().trim_matches('"').clone_into(&mut tier.assoc);
            } else if let Some(rest) = line.strip_prefix("operators = ") {
                tier.operators = rest
                    .trim()
                    .trim_start_matches('[')
                    .trim_end_matches(']')
                    .split(',')
                    .map(|o| o.trim().trim_matches('"').to_owned())
                    .filter(|o| !o.is_empty())
                    .collect();
            }
        }
        assert!(tier.n > 0, "a [[tier]] block with no `n`:\n{block}");
        tiers.push(tier);
    }
    tiers
}

/// The binary tiers of the recorded table, flattened to `(tier, operator, assoc)`.
fn recorded_infix() -> Vec<(u8, String, String)> {
    recorded_tiers()
        .iter()
        .filter(|tier| tier.arity == "binary")
        .flat_map(|tier| {
            tier.operators
                .iter()
                .map(|o| (tier.n, o.clone(), tier.assoc.clone()))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The two tier-8 operators the parser decides BEFORE the name rather than folding
/// around an expression.
///
/// `@@` and `meta` are tier 8 in the recorded table, with the four classification
/// operators, and they are deliberately not in the parser's infix table: their left
/// operand is a `MetadataArgumentMember`, which reaches a `QualifiedName` and not an
/// `OwnedExpression` (`KerML` 8.2.5.8.1), so they cannot be folded around an
/// expression the way an infix operator is.
///
/// Named here rather than filtered silently, so that an operator going missing for
/// any OTHER reason still fails.
const DECIDED_BEFORE_THE_NAME: [&str; 2] = ["@@", "meta"];

#[test]
fn a_multiplicative_operator_binds_tighter_than_an_additive_one() {
    // Table 6: `*` `/` `%` are tier 4 and `+` `-` are tier 5, and a lower tier groups
    // more tightly. So `a + b * c` is `a + (b * c)`: the outermost operator is the
    // `+`, and the `*` is inside its RIGHT operand.
    let rendered = render(&parse_accepted("attribute x = a + b * c;").syntax());
    assert_eq!(
        operands(&rendered, "BinaryOperatorExpression"),
        vec![
            "FeatureReferenceExpression".to_owned(),
            "BinaryOperatorExpression".to_owned()
        ],
        "`a + b * c` did not group as `a + (b * c)`:\n{rendered}"
    );
    // And the other way round, `a * b + c` is `(a * b) + c` — the nesting moves to
    // the left operand without the operators changing tier.
    let rendered = render(&parse_accepted("attribute x = a * b + c;").syntax());
    assert_eq!(
        operands(&rendered, "BinaryOperatorExpression"),
        vec![
            "BinaryOperatorExpression".to_owned(),
            "FeatureReferenceExpression".to_owned()
        ],
        "`a * b + c` did not group as `(a * b) + c`:\n{rendered}"
    );
}

#[test]
fn every_binary_operator_but_exponentiation_groups_to_the_left() {
    // Note 2: "all BinaryOperators other than exponentiation are left-associative".
    // `a - b - c` is `(a - b) - c`, so the nested expression is in the LEFT operand.
    // Subtraction is the case that shows it: `(a - b) - c` and `a - (b - c)` differ
    // in value, so a parser that gets this wrong is wrong and not merely differently
    // shaped.
    for source in [
        "attribute x = a - b - c;",
        "attribute x = a / b / c;",
        "attribute x = a + b + c;",
        "attribute x = a == b == c;",
        "attribute x = a & b & c;",
        "attribute x = a | b | c;",
        "attribute x = a xor b xor c;",
    ] {
        let rendered = render(&parse_accepted(source).syntax());
        assert_eq!(
            operands(&rendered, "BinaryOperatorExpression")
                .first()
                .map(String::as_str),
            Some("BinaryOperatorExpression"),
            "{source} did not group to the left:\n{rendered}"
        );
    }
}

#[test]
fn exponentiation_groups_to_the_right() {
    // Note 2: "the exponentiation operators (^ and **) are right-associative". This
    // is the one tier in table 6 that does, so `a ** b ** c` is `a ** (b ** c)` and
    // the nested expression is in the RIGHT operand — the mirror of the case above.
    for source in ["attribute x = a ** b ** c;", "attribute x = a ^ b ^ c;"] {
        let rendered = render(&parse_accepted(source).syntax());
        assert_eq!(
            operands(&rendered, "BinaryOperatorExpression"),
            vec![
                "FeatureReferenceExpression".to_owned(),
                "BinaryOperatorExpression".to_owned()
            ],
            "{source} did not group to the right:\n{rendered}"
        );
    }
}

#[test]
fn the_range_operator_chains_because_the_specification_says_it_groups_left() {
    // Tier 6 is `..`, and note 2 exempts only exponentiation from left-associativity,
    // so `a..b..c` is `(a..b)..c` and the operator repeats like any other.
    //
    // THE PILOT REJECTS THIS. Its RangeExpression rule takes `( ... )?` and so at most
    // one `..` (KerMLExpressions.xtext). That is a property of its parser generator,
    // not of the language, and deviations.json records the decision under
    // RangeExpression as xtext_only/follow_spec — this parser is deliberately the more
    // permissive of the two. The claim is here rather than only in a comment, because
    // a deviation nobody tests is a deviation nobody can tell has been reverted.
    let rendered = render(&parse_accepted("attribute x = a..b..c;").syntax());
    assert_eq!(
        nodes_named(&rendered, "BinaryOperatorExpression"),
        2,
        "`a..b..c` did not chain:\n{rendered}"
    );
    assert_eq!(
        operands(&rendered, "BinaryOperatorExpression")
            .first()
            .map(String::as_str),
        Some("BinaryOperatorExpression"),
        "`a..b..c` did not group as `(a..b)..c`:\n{rendered}"
    );
}

#[test]
fn a_unary_operator_binds_tighter_than_exponentiation() {
    // Table 6 puts the unary operators at tier 2 and exponentiation at tier 3, so
    // `-2 ** 2` is `(-2) ** 2` — the UnaryOperatorExpression is the LEFT operand of
    // the `**`, not the whole expression wrapping it.
    //
    // This is the opposite of the C and Python convention, where unary minus binds
    // LOOSER than exponentiation and `-2 ** 2` is `-(2 ** 2)`. The two readings
    // differ in sign, so the reflex is the failure mode: the citation, not the habit,
    // is what decides.
    let rendered = render(&parse_accepted("attribute x = -2 ** 2;").syntax());
    assert_eq!(
        operands(&rendered, "BinaryOperatorExpression"),
        vec![
            "UnaryOperatorExpression".to_owned(),
            "LiteralInteger".to_owned()
        ],
        "`-2 ** 2` did not group as `(-2) ** 2`:\n{rendered}"
    );
    // `- -a` nests, because ArgumentMember reaches OwnedExpression (KerML 8.2.5.8.1).
    // The Pilot's UnaryExpression rule does not recurse and rejects this; that is a
    // property of its parser generator, not of the language.
    let rendered = render(&parse_accepted("attribute x = - -a;").syntax());
    assert_eq!(
        nodes_named(&rendered, "UnaryOperatorExpression"),
        2,
        "`- -a` did not nest:\n{rendered}"
    );
}

#[test]
fn the_boolean_tiers_nest_in_the_order_the_table_gives() {
    // Table 6, tightest first: `&`/`and` is tier 10, `xor` 11, `|`/`or` 12,
    // `implies` 13, `??` 14. So in `a & b xor c | d implies e` the outermost
    // operator is `implies` and each tier below it sits in the left operand of the
    // one above. Five tiers in one expression, which is what makes this the case
    // that a rule-ordering mistake cannot survive.
    let rendered = render(&parse_accepted("attribute x = a & b xor c | d implies e;").syntax());
    // `implies` is a ConditionalBinaryOperator, so the outermost node is the
    // conditional kind and there is exactly one of it.
    assert_eq!(
        nodes_named(&rendered, "ConditionalBinaryOperatorExpression"),
        1,
        "`implies` is not the one outermost operator:\n{rendered}"
    );
    // The three BinaryOperators below it — `&`, `xor` and `|` — each build their own.
    assert_eq!(
        nodes_named(&rendered, "BinaryOperatorExpression"),
        3,
        "the three binary tiers did not each build a node:\n{rendered}"
    );
    // And the nesting runs through the LEFT operand at every tier, all four being
    // left-associative.
    assert_eq!(
        operands(&rendered, "ConditionalBinaryOperatorExpression")
            .first()
            .map(String::as_str),
        Some("BinaryOperatorExpression"),
        "the tighter tiers are not in `implies`'s left operand:\n{rendered}"
    );
}

#[test]
fn a_short_circuiting_operator_references_its_right_operand() {
    // ConditionalBinaryOperator = '??' | 'or' | 'and' | 'implies', and
    // ConditionalBinaryOperatorExpression's right operand is an
    // ArgumentExpressionMember where BinaryOperatorExpression's is an ArgumentMember
    // (KerML 8.2.5.8.1). That is not cosmetic: the expression is REFERENCED rather
    // than evaluated, which is how the abstract syntax records the short circuit.
    //
    // `&` and `and` are the same tier 10 and mean the same thing, and differ only in
    // this, so the pair is the case that proves the parser reads the distinction
    // rather than the tier.
    let rendered = render(&parse_accepted("attribute x = a and b;").syntax());
    assert_eq!(
        nodes_named(&rendered, "ConditionalBinaryOperatorExpression"),
        1,
        "`and` built no ConditionalBinaryOperatorExpression:\n{rendered}"
    );
    assert_eq!(
        nodes_named(&rendered, "ArgumentExpressionMember"),
        1,
        "`and`'s right operand is not referenced:\n{rendered}"
    );
    // The five memberships the clause names over a referenced operand.
    for node in [
        "ArgumentExpression",
        "ArgumentExpressionValue",
        "OwnedExpressionReference",
        "OwnedExpressionMember",
    ] {
        assert!(rendered.contains(node), "no {node} node:\n{rendered}");
    }

    let rendered = render(&parse_accepted("attribute x = a & b;").syntax());
    assert_eq!(
        nodes_named(&rendered, "BinaryOperatorExpression"),
        1,
        "`&` built no BinaryOperatorExpression:\n{rendered}"
    );
    assert_eq!(
        nodes_named(&rendered, "ArgumentExpressionMember"),
        0,
        "`&` does not short-circuit and must not reference its operand:\n{rendered}"
    );
}

#[test]
fn a_classification_expression_takes_a_type_and_not_an_expression() {
    // ClassificationExpression = ArgumentMember? ( ClassificationTestOperator
    // TypeReferenceMember | CastOperator TypeResultMember ) EmptyResultMember
    // (KerML 8.2.5.8.1). The right operand is a TypeReference over a QualifiedName,
    // so `x istype T` has an expression on the left and a type on the right.
    for (source, member) in [
        ("attribute x = y istype T;", "TypeReferenceMember"),
        ("attribute x = y hastype T;", "TypeReferenceMember"),
        ("attribute x = y @ T;", "TypeReferenceMember"),
        // A cast names its type through a TypeResultMember, because a cast's type IS
        // its result — a distinction in the abstract syntax and not in the text.
        ("attribute x = y as T;", "TypeResultMember"),
    ] {
        let rendered = render(&parse_accepted(source).syntax());
        assert_eq!(
            nodes_named(&rendered, "ClassificationExpression"),
            1,
            "{source} built no ClassificationExpression:\n{rendered}"
        );
        assert!(
            rendered.contains(member),
            "{source} did not own a {member}:\n{rendered}"
        );
        for node in ["TypeReference", "ReferenceTyping"] {
            assert!(
                rendered.contains(node),
                "no {node} in {source}:\n{rendered}"
            );
        }
    }
}

#[test]
fn a_classification_expression_may_have_no_left_operand() {
    // Its ArgumentMember is the one optional operand in the clause, which is what
    // makes a bare classification operator an expression. `filter @Safety;` is the
    // corpus idiom, and it is why ElementFilterMember needed this layer and nothing
    // more.
    let rendered = render(&parse_accepted("package P { filter @Safety; }").syntax());
    assert_eq!(
        nodes_named(&rendered, "ElementFilterMember"),
        1,
        "no ElementFilterMember:\n{rendered}"
    );
    assert_eq!(
        nodes_named(&rendered, "ClassificationExpression"),
        1,
        "`@Safety` built no ClassificationExpression:\n{rendered}"
    );
    // No left operand means no ArgumentMember at all.
    assert_eq!(
        nodes_named(&rendered, "ArgumentMember"),
        0,
        "a bare `@` must own no ArgumentMember:\n{rendered}"
    );
}

#[test]
fn an_extent_expression_owns_no_result_parameter() {
    // ExtentExpression = 'all' TypeReferenceMember (KerML 8.2.5.8.1) — tier 1, the
    // tightest, and the one OperatorExpression in the clause that owns no
    // EmptyResultMember. Its result comes from the type, so none is written.
    let rendered = render(&parse_accepted("attribute x = all T;").syntax());
    assert_eq!(
        nodes_named(&rendered, "ExtentExpression"),
        1,
        "no ExtentExpression:\n{rendered}"
    );
    let extent = subtree(&rendered, "ExtentExpression");
    assert!(
        !extent.contains("EmptyResultMember"),
        "ExtentExpression owns a result parameter the clause does not give it:\n{extent}"
    );
}

#[test]
fn a_metaclassification_takes_a_reference_and_not_an_expression() {
    // MetaclassificationExpression's left operand is a MetadataArgumentMember, and
    // that reaches a QualifiedName — MetadataArgument -> MetadataValue ->
    // MetadataReference -> ElementReferenceMember = [QualifiedName] (KerML
    // 8.2.5.8.1, 8.2.5.8.3) — and NOT an OwnedExpression. So the left of `meta` is a
    // reference, where the left of `istype` at the same tier 8 is an expression.
    //
    // From the pinned corpus: "Simple Tests/Classifications.kerml" line 7 writes
    // `b = x meta KerML::Feature;`, and SimpleVehicleModel.sysml line 460 the same
    // shape.
    let rendered = render(&parse_accepted("attribute b = x meta KerML::Feature;").syntax());
    assert_eq!(
        nodes_named(&rendered, "MetaclassificationExpression"),
        1,
        "no MetaclassificationExpression:\n{rendered}"
    );
    for node in [
        "MetadataArgumentMember",
        "MetadataArgument",
        "MetadataValue",
        "MetadataReference",
        "ElementReferenceMember",
    ] {
        assert!(rendered.contains(node), "no {node} node:\n{rendered}");
    }
    // The left operand is a reference, so no ArgumentMember and no
    // FeatureReferenceExpression wraps it.
    assert_eq!(
        nodes_named(&rendered, "ArgumentMember"),
        0,
        "`meta`'s left operand must not be an argument:\n{rendered}"
    );
    // `meta` casts, so its type is a TypeResultMember; `@@` tests, so its type is a
    // TypeReferenceMember.
    assert!(rendered.contains("TypeResultMember"), "{rendered}");
    let rendered = render(&parse_accepted("attribute b = x @@ KerML::Feature;").syntax());
    assert!(rendered.contains("TypeReferenceMember"), "{rendered}");
}

#[test]
fn a_conditional_expression_references_both_of_its_branches() {
    // ConditionalExpression = 'if' ArgumentMember '?' ArgumentExpressionMember
    // 'else' ArgumentExpressionMember EmptyResultMember (KerML 8.2.5.8.1). Tier 15,
    // the loosest. The condition is an argument and both branches are references,
    // which is the short circuit again: only one branch is evaluated.
    let rendered = render(&parse_accepted("attribute x = if a? b else c;").syntax());
    assert_eq!(
        nodes_named(&rendered, "ConditionalExpression"),
        1,
        "no ConditionalExpression:\n{rendered}"
    );
    assert_eq!(
        nodes_named(&rendered, "ArgumentMember"),
        1,
        "the condition is the one eager operand:\n{rendered}"
    );
    assert_eq!(
        nodes_named(&rendered, "ArgumentExpressionMember"),
        2,
        "both branches must be referenced:\n{rendered}"
    );
}

#[test]
fn a_conditional_expression_nests_in_its_else_branch() {
    // The `else` branch is read at tier 15 and the condition one tier tighter, so
    // `if a? b else if c? d else e` nests to the right with no parentheses while a
    // bare `if` in the condition does not parse. That asymmetry is the clause's:
    // ArgumentExpressionMember for the branches, ArgumentMember for the condition.
    let rendered = render(&parse_accepted("attribute x = if a? b else if c? d else e;").syntax());
    assert_eq!(
        nodes_named(&rendered, "ConditionalExpression"),
        2,
        "the trailing `if` did not nest in the `else`:\n{rendered}"
    );
}

#[test]
fn the_five_literals_each_build_their_own_node() {
    // LiteralExpression = LiteralBoolean | LiteralString | LiteralInteger
    // | LiteralReal | LiteralInfinity (KerML 8.2.5.8.4).
    for (source, node) in [
        ("attribute x = true;", "LiteralBoolean"),
        ("attribute x = false;", "LiteralBoolean"),
        ("attribute x = \"a string\";", "LiteralString"),
        ("attribute x = 42;", "LiteralInteger"),
        ("attribute x = 1.5;", "LiteralReal"),
        ("attribute x = *;", "LiteralInfinity"),
    ] {
        let rendered = render(&parse_accepted(source).syntax());
        assert!(
            rendered.contains(node),
            "{source} built no {node}:\n{rendered}"
        );
    }
}

#[test]
fn a_real_is_told_from_an_integer_by_its_decimal_point() {
    // RealValue = DECIMAL_VALUE? '.' ( DECIMAL_VALUE | EXPONENTIAL_VALUE )
    // | EXPONENTIAL_VALUE (KerML 8.2.5.8.4). RealValue is a production, not a
    // terminal, and the lexer does not take a '.' into a number — so `1.5` arrives
    // as three tokens and the node is what holds them together.
    for source in [
        "attribute x = 1.5;",   // DECIMAL_VALUE '.' DECIMAL_VALUE
        "attribute x = .5;",    // the leading part is optional
        "attribute x = 1e5;",   // EXPONENTIAL_VALUE alone
        "attribute x = 1.5e3;", // DECIMAL_VALUE '.' EXPONENTIAL_VALUE
    ] {
        let rendered = render(&parse_accepted(source).syntax());
        assert!(
            rendered.contains("LiteralReal"),
            "{source} is a real:\n{rendered}"
        );
        assert_eq!(
            nodes_named(&rendered, "LiteralInteger"),
            0,
            "{source} is not an integer:\n{rendered}"
        );
    }
    // And the range operator is never a decimal point: '..' is one token by maximal
    // munch, so `1..5` is two integers around a tier-6 operator and not two reals.
    let rendered = render(&parse_accepted("attribute x = 1..5;").syntax());
    assert_eq!(
        nodes_named(&rendered, "LiteralInteger"),
        2,
        "`1..5` is two integers:\n{rendered}"
    );
    assert_eq!(
        nodes_named(&rendered, "LiteralReal"),
        0,
        "`1..5` holds no real:\n{rendered}"
    );
}

#[test]
fn infinity_and_multiplication_are_the_same_token_in_two_positions() {
    // LiteralInfinity = '*' (KerML 8.2.5.8.4) and `*` is also the tier-4
    // multiplication operator. Position separates them with no lookahead: an operand
    // position reads the literal and an operator position reads the operator.
    let rendered = render(&parse_accepted("part p[0..*];").syntax());
    assert!(
        rendered.contains("LiteralInfinity"),
        "the `*` of an unbounded multiplicity is the literal:\n{rendered}"
    );
    let rendered = render(&parse_accepted("attribute x = a * b;").syntax());
    assert_eq!(
        nodes_named(&rendered, "LiteralInfinity"),
        0,
        "the `*` between two operands is the operator:\n{rendered}"
    );
}

#[test]
fn a_parenthesised_expression_is_a_sequence_expression() {
    // SequenceExpression = '(' SequenceExpressionList ')' and
    // SequenceExpressionList = OwnedExpression ','? | SequenceOperatorExpression
    // (KerML 8.2.5.8.2). One expression in parentheses is the first alternative.
    let rendered = render(&parse_accepted("attribute x = (a);").syntax());
    assert_eq!(
        nodes_named(&rendered, "SequenceExpression"),
        1,
        "{rendered}"
    );
    assert_eq!(
        nodes_named(&rendered, "SequenceOperatorExpression"),
        0,
        "one expression is not a sequence operator expression:\n{rendered}"
    );
    // A comma makes it the second alternative, which is the recursive one.
    let rendered = render(&parse_accepted("attribute x = (a, b, c);").syntax());
    assert_eq!(
        nodes_named(&rendered, "SequenceOperatorExpression"),
        2,
        "`(a, b, c)` is two comma operators:\n{rendered}"
    );
    // And a TRAILING comma is the first alternative's `','?`, not an operator with a
    // missing operand — the two are told apart by what follows the comma.
    let rendered = render(&parse_accepted("attribute x = (a,);").syntax());
    assert_eq!(
        nodes_named(&rendered, "SequenceOperatorExpression"),
        0,
        "a trailing comma is not a sequence operator:\n{rendered}"
    );
    // Parentheses regroup, which is the only way to reach a looser tier from a
    // tighter position: `(a + b) * c` puts the `+` inside the `*`'s left operand.
    let rendered = render(&parse_accepted("attribute x = (a + b) * c;").syntax());
    assert_eq!(
        operands(&rendered, "BinaryOperatorExpression")
            .first()
            .map(String::as_str),
        Some("SequenceExpression"),
        "parentheses did not regroup:\n{rendered}"
    );
}

#[test]
fn null_is_written_two_ways() {
    // NullExpression = 'null' | '(' ')' (KerML 8.2.5.8.3). The empty pair must be
    // decided before SequenceExpression, which would otherwise take the '(' and find
    // no expression after it.
    for source in ["attribute x = null;", "attribute x = ();"] {
        let rendered = render(&parse_accepted(source).syntax());
        assert_eq!(
            nodes_named(&rendered, "NullExpression"),
            1,
            "{source} built no NullExpression:\n{rendered}"
        );
        assert_eq!(
            nodes_named(&rendered, "SequenceExpression"),
            0,
            "{source} is not a sequence expression:\n{rendered}"
        );
    }
}

// -- multiplicity, SysML 8.2.2.6.6 -------------------------------------------------

#[test]
fn a_multiplicity_bounds_a_usage_as_the_corpus_writes_it() {
    // `part frontSeat[2];` is well-formed SysML and the pinned corpus writes it nine
    // times, among them SimpleVehicleModel.sysml line 691 and
    // "40. Filtering/Filtering Example-1.sysml" line 12. It is the case this whole
    // layer was blocking.
    let rendered = render(&parse_accepted("part frontSeat[2];").syntax());
    for node in [
        "MultiplicityPart",
        "OwnedMultiplicity",
        "MultiplicityRange",
        "MultiplicityExpressionMember",
    ] {
        assert!(rendered.contains(node), "no {node} node:\n{rendered}");
    }
    // A single bound is the UPPER one: the clause makes the lower bound optional.
    assert_eq!(
        nodes_named(&rendered, "MultiplicityExpressionMember"),
        1,
        "`[2]` is one bound:\n{rendered}"
    );
}

#[test]
fn a_multiplicity_range_takes_two_bounds_around_a_range_operator() {
    // MultiplicityRange = '[' ( MultiplicityExpressionMember '..' )?
    // MultiplicityExpressionMember ']' (SysML 8.2.2.6.6).
    for source in ["part p[0..*];", "part p[1..5];", "part p[0..n];"] {
        let rendered = render(&parse_accepted(source).syntax());
        assert_eq!(
            nodes_named(&rendered, "MultiplicityExpressionMember"),
            2,
            "{source} has two bounds:\n{rendered}"
        );
    }
}

#[test]
fn a_multiplicity_bound_is_a_literal_or_a_name_and_nothing_else() {
    // MultiplicityExpressionMember = LiteralExpression | FeatureReferenceExpression
    // (SysML 8.2.2.6.6). It does NOT reach OwnedExpression, so a bound is not an
    // operator expression — which is what kept this production cheap.
    let rendered = render(&parse_accepted("part p[n];").syntax());
    assert!(
        rendered.contains("FeatureReferenceExpression"),
        "a name is a bound:\n{rendered}"
    );
    // `[1+1]` is therefore not a multiplicity. The `[` is read, the `1` is a bound,
    // and the `+` has nowhere to go — a rejection by rule, not by absence.
    parse_rejected("part p[1+1];");
}

#[test]
fn a_multiplicity_part_may_carry_the_two_keywords_in_either_order() {
    // MultiplicityPart = OwnedMultiplicity | OwnedMultiplicity?
    // ( 'ordered' 'nonunique'? | 'nonunique' 'ordered'? ) (SysML 8.2.2.6.6). The
    // second alternative's OwnedMultiplicity is optional, so the keywords stand
    // alone; both orderings set the same two flags, which is why the clause writes
    // each keyword twice.
    for source in [
        "part p[1] ordered;",
        "part p[1] nonunique;",
        "part p[1] ordered nonunique;",
        "part p[1] nonunique ordered;",
        "part p ordered;",
        "part p nonunique ordered;",
    ] {
        let rendered = render(&parse_accepted(source).syntax());
        assert!(
            rendered.contains("MultiplicityPart"),
            "{source} built no MultiplicityPart:\n{rendered}"
        );
    }
}

#[test]
fn a_multiplicity_part_sits_among_the_feature_specializations() {
    // FeatureSpecializationPart = FeatureSpecialization+ MultiplicityPart?
    // FeatureSpecialization* | MultiplicityPart FeatureSpecialization*
    // (KerML 8.2.4.3.1). The two alternatives together admit the multiplicity before
    // or after a specialization, and at most one of it.
    for source in [
        "part p[2] : Seat;",
        "part p : Seat[2];",
        "part p : Seat[2] :> base;",
    ] {
        let rendered = render(&parse_accepted(source).syntax());
        assert_eq!(
            nodes_named(&rendered, "MultiplicityPart"),
            1,
            "{source} built no MultiplicityPart:\n{rendered}"
        );
        assert!(rendered.contains("FeatureSpecializationPart"), "{rendered}");
    }
    // At most one, because neither alternative can produce a second.
    parse_rejected("part p[1][2];");
}

// -- the value a usage carries, SysML 8.2.2.6.2 ------------------------------------

#[test]
fn a_usage_may_carry_a_value() {
    // UsageCompletion = ValuePart? UsageBody, ValuePart = FeatureValue, and
    // FeatureValue = ( '=' | ':=' | 'default' ( '=' | ':=' )? ) OwnedExpression
    // (SysML 8.2.2.6.2). The three prefixes are flags on one relationship: `=` binds,
    // `:=` initialises, and `default` marks either as a default.
    for source in [
        "attribute m = 5;",
        "attribute m := 5;",
        "attribute m default = 5;",
        "attribute m default := 5;",
        // After `default` the `=` is optional, so this is the same FeatureValue with
        // isDefault set and nothing else.
        "attribute m default 5;",
        "part engine : Engine = x;",
    ] {
        let rendered = render(&parse_accepted(source).syntax());
        for node in ["ValuePart", "FeatureValue"] {
            assert!(
                rendered.contains(node),
                "{source} built no {node}:\n{rendered}"
            );
        }
    }
}

#[test]
fn a_value_part_comes_before_the_body_and_not_after() {
    // UsageCompletion = ValuePart? UsageBody (SysML 8.2.2.6.2) — in that order, so
    // the value precedes the `;` or the braces that end the usage.
    let rendered = render(&parse_accepted("attribute m = 5;").syntax());
    let completion = subtree(&rendered, "UsageCompletion");
    let value = completion.find("ValuePart");
    let body = completion.find("UsageBody");
    assert!(
        value < body,
        "the ValuePart must precede the UsageBody:\n{completion}"
    );
    // A value after the body is not a UsageCompletion.
    parse_rejected("attribute m; = 5");
}

#[test]
fn a_value_expression_reaches_the_whole_expression_layer() {
    // FeatureValue's operand is an OwnedExpression, so everything the layer reads is
    // reachable from a value. These are the corpus shapes, each from a pinned file.
    //
    // "Analysis Examples/Vehicle Analysis Demo.sysml" line 106.
    parse_accepted("attribute v : SpeedValue = v0 + a * dt;");
    // "Analysis Examples/Dynamics.sysml" line 9.
    parse_accepted("attribute tp : PowerValue = whlpwr - Cd * v - Cf * tm * v;");
    // "v1 Spec Examples/8.4.1 Wheel Hub Assembly/Wheel Package.sysml" line 9 — the
    // case that needs `^` to bind tighter than `/`.
    let rendered = render(&parse_accepted("attribute pressure = force / length^2;").syntax());
    assert_eq!(
        operands(&rendered, "BinaryOperatorExpression")
            .get(1)
            .map(String::as_str),
        Some("BinaryOperatorExpression"),
        "`force / length^2` must group as `force / (length^2)`:\n{rendered}"
    );
}

// -- the fourth PackageBodyElement, SysML 8.2.2.5.1 --------------------------------

#[test]
fn an_element_filter_member_is_a_package_body_element() {
    // ElementFilterMember = MemberPrefix 'filter' OwnedExpression ';'
    // (SysML 8.2.2.5.1). The corpus writes a bare classification operator:
    // "40. Filtering/Filtering Example-1.sysml" line 12 and
    // "Simple Tests/Filtering.kerml" line 44.
    for source in [
        "package P { filter @Safety; }",
        "package P { private filter @Safety; }",
        "package P { filter @Safety and @Security; }",
        "package P { filter not @Safety; }",
    ] {
        let rendered = render(&parse_accepted(source).syntax());
        assert_eq!(
            nodes_named(&rendered, "ElementFilterMember"),
            1,
            "{source} built no ElementFilterMember:\n{rendered}"
        );
    }
}

#[test]
fn an_element_filter_member_is_not_a_definition_body_item() {
    // PackageBodyElement lists ElementFilterMember (SysML 8.2.2.5.1);
    // DefinitionBodyItem does not (8.2.2.6.1). So a `filter` in a definition body is
    // reported, and this is a rejection BY RULE — the production is implemented, and
    // the same text in a package body parses.
    parse_rejected("part def V { filter @Safety; }");
    parse_accepted("package P { filter @Safety; }");
}

// -- the table the parser reads ----------------------------------------------------

#[test]
fn the_recorded_table_is_the_fifteen_tiers_of_table_6() {
    // docs/operator-precedence.toml records table 6 of KerML 8.2.5.8.1 as data, cited
    // to the clause, because no production can carry it — note 2 says so. This holds
    // the FILE to the shape the clause describes; the test below holds the parser to
    // the file.
    let tiers = recorded_tiers();
    assert_eq!(tiers.len(), 15, "table 6 has fifteen tiers");
    for (i, tier) in tiers.iter().enumerate() {
        assert_eq!(
            usize::from(tier.n),
            i + 1,
            "the tiers must be numbered 1 to 15 in order"
        );
    }

    // Note 2: "all BinaryOperators other than exponentiation are left-associative
    // ... while the exponentiation operators (^ and **) are right-associative".
    for tier in tiers.iter().filter(|t| t.arity == "binary") {
        let expected = if tier.operators.contains(&"**".to_owned()) {
            "right"
        } else {
            "left"
        };
        assert_eq!(tier.assoc, expected, "tier {} groups the wrong way", tier.n);
    }

    // The three prefix tiers, and the one ordering a reader is most likely to get
    // wrong: tier 2 binds TIGHTER than tier 3, so `-2 ** 2` is `(-2) ** 2`.
    assert_eq!(tiers[0].operators, ["all"], "tier 1 is `all`");
    assert_eq!(
        tiers[1].operators,
        ["+", "-", "~", "not"],
        "tier 2 is the four unary operators"
    );
    assert_eq!(
        tiers[2].operators,
        ["^", "**"],
        "tier 3 is exponentiation, LOOSER than the unary tier above it"
    );
    assert_eq!(tiers[14].operators, ["if"], "tier 15 is `if`");
}

#[test]
fn the_infix_table_is_the_recorded_precedence_table() {
    // The parser's INFIX table is docs/operator-precedence.toml in the form it reads.
    // Neither is derived from the other, so this is what stops one being edited alone
    // — which would leave the parser grouping by a table that no longer cites
    // anything, and the citation is the only reason to believe the grouping.
    let (deferred, recorded): (Vec<_>, Vec<_>) = recorded_infix()
        .into_iter()
        .partition(|(_, o, _)| DECIDED_BEFORE_THE_NAME.contains(&o.as_str()));
    assert_eq!(
        deferred.len(),
        DECIDED_BEFORE_THE_NAME.len(),
        "the file no longer records both metaclassification operators at a binary tier"
    );
    for (n, ..) in &deferred {
        assert_eq!(*n, 8, "a metaclassification operator left tier 8");
    }

    // Every operator the file records, the parser reads at that tier and that
    // associativity — and every operator the parser reads, the file records. Both
    // directions, so neither table may hold one the other does not.
    let read = sv2_syntax::infix_table_for_test();
    for entry in &recorded {
        assert!(
            read.contains(entry),
            "the file records {entry:?} and the parser does not read it:\n{read:?}"
        );
    }
    for entry in &read {
        assert!(
            recorded.contains(entry),
            "the parser reads {entry:?} and the file does not record it:\n{recorded:?}"
        );
    }
    assert_eq!(
        recorded.len(),
        read.len(),
        "the two tables differ in length"
    );
}

#[test]
fn the_metaclassification_operators_are_read_at_their_tier() {
    // The two operators left out of the infix table are still read — at tier 8, as
    // the file records them, and through the metaclassification path because their
    // left operand is a reference. `a_metaclassification_takes_a_reference_and_not_
    // _an_expression` holds their shape; this holds that they parse at all, so
    // leaving them out of INFIX cannot quietly become leaving them out.
    parse_accepted("attribute x = y meta T;");
    parse_accepted("attribute x = y @@ T;");
}
