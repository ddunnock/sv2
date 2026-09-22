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
///
/// Unconditional trivia is skipped, because a production does not own the author's
/// spacing: `1200 [kg]` and `1200[kg]` are the same production and differ by one
/// `Whitespace` child. Losslessness is asserted by the round-trip property test in
/// tests/roundtrip.rs, which is where it belongs — an assertion here that happened to
/// include a space would be testing the input's formatting, not the grammar.
///
/// `RegularComment` is NOT skipped. It is trivia in most positions and a TOKEN in an
/// annotating one (`doc /* … */`), so a helper that dropped it could hide the
/// difference.
fn child_kinds(rendered: &str, kind: &str) -> Vec<String> {
    const TRIVIA: [&str; 3] = ["Whitespace", "SingleLineNote", "MultilineNote"];
    let body = subtree(rendered, kind);
    let mut lines = body.lines();
    let Some(head) = lines.next() else {
        return Vec::new();
    };
    let depth = head.len() - head.trim_start().len();
    lines
        .filter(|l| l.len() - l.trim_start().len() == depth + 2)
        .map(|l| l.split_whitespace().next().unwrap_or_default().to_owned())
        .filter(|name| !TRIVIA.contains(&name.as_str()))
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
    // `attribute mass : Real;`, `item wheel : Wheel;` and `connection fuelLine connect a
    // to b;`. Each stopped being a rejection when its production landed, and the
    // positive cases hold all five. The property the test protects never changes: text this parser cannot
    // read is reported, not silently accepted. Only the example moves.
    //
    // It is deliberately no longer a usage of the `<prefix> KEYWORD Usage` shape.
    // Those now arrive in batches — seven of them are one table — so any of them
    // would be a placeholder with a short life. A SatisfyRequirementUsage has a shape of
    // its own, a reference and a `by` subject, so it will not land incidentally alongside
    // something else.
    //
    // SysML 8.2.2.21.2 — SatisfyRequirementUsage = OccurrenceUsagePrefix 'assert'? 'not'?
    //                    'satisfy' ( OwnedReferenceSubsetting ... ) ValuePart?
    //                    ( 'by' SatisfactionSubjectMember )? RequirementBody
    let parsed = parse_rejected("satisfy vehicleSpecification by vehicle_design;");
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
    // Fourteen of the twenty-two `def` productions end in a specialised body, and three
    // of those bodies are implemented: RequirementBody, CalculationBody and ActionBody.
    // PortDefinition shares the spine but adds a ConjugatedPortDefinitionMember. Each
    // body still absent has a file in tests/rejection/.
    //
    // The list empties as the specialised bodies land. `state def S;` was the last input
    // held here by absence, and leaves it ahead of the commit that reads StateDefBody
    // (8.2.2.18.1), which asserts it accepted.
    //
    // `requirement def` left this list when RequirementBody landed, `constraint def`
    // when CalculationBody did, and `calc def` leaves it now that something dispatches
    // to the body those two already shared. Each left because the production is read,
    // not because the claim was relaxed.
    parse_accepted("requirement def R;");
    parse_accepted("constraint def C;");
    parse_accepted("calc def C;");
    parse_accepted("action def Brake;");
}

// -- ActionDefinition and ActionBody, SysML 8.2.2.17.1 ----------------------------
//
// ActionDefinition = OccurrenceDefinitionPrefix 'action' 'def'
//                    DefinitionDeclaration ActionBody
// ActionBody       = ';' | '{' ActionBodyItem* '}'
//
// The shell of the action layer. ActionBodyItem's three control-flow alternatives are
// absent, which is deliberate and is most of what the layer is.

#[test]
fn an_action_definition_reads_the_declaration_and_its_own_body() {
    parse_accepted("action def Brake;");
    parse_accepted("action def Brake { }");
    parse_accepted("action def <'a1'> GenerateTorque :> Behavior { }");
    parse_accepted("individual action def Brake;");
    parse_accepted("abstract action def Brake;");
}

#[test]
fn an_action_body_reads_the_items_a_definition_body_reads() {
    // ActionBodyItem's first alternative is NonBehaviorBodyItem, whose Import,
    // AliasMember and DefinitionMember are the three a definition body already read
    // (SysML 8.2.2.17.1). Nothing new was needed for any of these.
    parse_accepted("action def Brake { part p; }");
    parse_accepted("action def Brake { doc /* how it stops */ }");
    parse_accepted("action def Brake { private import Actions::*; }");
    parse_accepted("action def Brake { alias b for brake; }");
    parse_accepted("action def Brake { attribute force; part def Pedal; }");
}

#[test]
fn an_action_body_does_not_admit_the_control_flow_layer() {
    // The alternatives of ActionBodyItem that are NOT NonBehaviorBodyItem, less
    // InitialNodeMember: successions and guards. Rejected by absence — every one is well-formed
    // SysML. Held as a file by tests/rejection/action-body-control-flow-is-not-implemented.sysml.
    //
    // `first start;` was here and is not: it is InitialNodeMember, ActionBodyItem's
    // second alternative (SysML 8.2.2.17.1) — well-formed, and rejected only while that
    // production was absent. Asserting it rejected is asserting the absence, which is
    // the one thing this case must not outlive.
    //
    // `accept Signal;`, `send Sig to target;` and `assign x := 1;` were here and are not:
    // they are AcceptNode, SendNode and AssignmentNode (8.2.2.17.4, 8.2.2.17.5), ActionNodes
    // reached through ActionBehaviorMember -- well-formed, and rejected only while those
    // productions were absent. The next commit implements them.
    parse_rejected("action def B { then stop; }");
}

#[test]
fn an_action_definition_owns_no_definition_node() {
    // Like RequirementDefinition and ConstraintDefinition, it names the declaration and
    // the body separately rather than taking a Definition (SysML 8.2.2.17.1).
    // The direct children are the production, part for part. That alone says the body is
    // an ActionBody and that no Definition node stands between the declaration and it.
    let rendered = render(&parse_accepted("action def Brake { part p; }").syntax());
    assert_eq!(
        child_kinds(&rendered, "ActionDefinition"),
        [
            "OccurrenceDefinitionPrefix",
            "KwAction",
            "KwDef",
            "DefinitionDeclaration",
            "ActionBody"
        ],
        "{rendered}"
    );
    // Counted over the WHOLE tree only where nothing nested can contribute one: the
    // `part p;` above owns a DefinitionBody of its own through its UsageBody, and a
    // whole-tree count would read the neighbour's node as this one's. That is the same
    // contains-versus-owns confusion child_kinds exists to avoid.
    let empty = render(&parse_accepted("action def Brake;").syntax());
    assert_eq!(nodes_named(&empty, "DefinitionBody"), 0, "{empty}");
    assert_eq!(nodes_named(&empty, "Definition"), 0, "{empty}");
    assert_eq!(nodes_named(&empty, "ActionBody"), 1, "{empty}");
}

#[test]
fn an_action_definition_needs_a_body_and_a_def() {
    // ActionBody is not optional. Held as a file by
    // tests/rejection/action-definition-missing-action-body.sysml.
    parse_rejected("action def Brake");
}

// -- PerformActionUsage, SysML 8.2.2.17.2 -----------------------------------------
//
// PerformActionUsage            = OccurrenceUsagePrefix 'perform'
//                                 PerformActionUsageDeclaration ActionBody
// PerformActionUsageDeclaration = ( OwnedReferenceSubsetting FeatureSpecializationPart?
//                                 | 'action' UsageDeclaration ) ValuePart?
//
// It performs an action rather than being one, and the two alternatives are the two ways
// of saying which action.

#[test]
fn a_perform_reads_both_of_its_declarations() {
    // Every one of these is a form the pinned corpus writes.
    parse_accepted("action def B { perform action producerBehavior; }");
    parse_accepted("action def B { perform providePower.generateTorque; }");
    parse_accepted("action def B { perform action 'provide power' : 'Provide Power'; }");
    parse_accepted("action def B { perform 'provide power' :>> VehicleA::'provide power'; }");
    // It ends in an ActionBody, so it may hold one.
    parse_accepted("action def B { perform providePower { part p; } }");
    parse_accepted("package P { perform stop; }");
}

#[test]
fn the_two_perform_declarations_are_told_apart_by_one_keyword() {
    // The second alternative opens on `action` and the first on a QualifiedName, and a
    // keyword is not a name (SysML 8.2.2.1.2) — the same shape RequirementConstraintUsage
    // has one clause along.
    let by_reference = render(&parse_accepted("action def B { perform providePower; }").syntax());
    let declared = render(&parse_accepted("action def B { perform action p; }").syntax());
    assert_eq!(
        nodes_named(&by_reference, "OwnedReferenceSubsetting"),
        1,
        "{by_reference}"
    );
    assert_eq!(
        nodes_named(&by_reference, "UsageDeclaration"),
        0,
        "{by_reference}"
    );
    assert_eq!(
        nodes_named(&declared, "OwnedReferenceSubsetting"),
        0,
        "{declared}"
    );
    assert_eq!(nodes_named(&declared, "UsageDeclaration"), 1, "{declared}");
}

#[test]
fn a_perform_by_reference_may_name_a_chain() {
    // `perform providePower.generateTorque` is the commonest `perform` in the corpus,
    // and its target is an OwnedFeatureChain (SysML 8.2.2.6.5) rather than a plain
    // QualifiedName. This is the form that found the four reference productions
    // claiming a chain alternative none of them read.
    let rendered =
        render(&parse_accepted("action def B { perform providePower.generateTorque; }").syntax());
    assert_eq!(nodes_named(&rendered, "OwnedFeatureChain"), 1, "{rendered}");
    assert_eq!(
        nodes_named(&rendered, "OwnedFeatureChaining"),
        2,
        "{rendered}"
    );
}

#[test]
fn a_perform_must_name_what_it_performs_although_an_action_need_not() {
    // ActionUsageDeclaration is a UsageDeclaration, every part of which is optional, so
    // `action;` is an anonymous action. Neither alternative of
    // PerformActionUsageDeclaration derives the empty string, so `perform;` is not a
    // shorter form — it is not the production. Held as a file by
    // tests/rejection/perform-by-reference-needs-a-target.sysml.
    parse_accepted("package P { action; }");
    parse_rejected("action def B { perform; }");
}

// -- OwnedFeatureChain, SysML 8.2.2.6.5 -------------------------------------------
//
// OwnedFeatureChain    = OwnedFeatureChaining ( '.' OwnedFeatureChaining )+
// OwnedFeatureChaining = chainingFeature = [QualifiedName]
//
// The REFERENCE layer's chain, which is not the expression layer's. All four of
// 8.2.2.6.5's reference productions take it, and all four were marked with it absent.

#[test]
fn a_reference_may_be_a_feature_chain() {
    // Every one of these is a shape the corpus writes; a chained reference appears 60
    // times. Before this they were rejected, while coverage counted the four reference
    // productions as implemented.
    parse_accepted("package P { part p :>> a.b; }");
    parse_accepted("package P { part p :>> localClock.currentTime; }");
    parse_accepted("package P { part p :>> a.b.c; }");
    parse_accepted("package P { part p :> a.b; }");
    parse_accepted("package P { attribute x subsets a.b; }");
    parse_accepted("package P { attribute x redefines a.b; }");
    // The FIFTH of the five, and the one a first sweep missed: OwnedFeatureTyping has
    // the same `[QualifiedName] | OwnedFeatureChain` shape but is a typing rather than a
    // reference, so a search for reference productions did not reach it.
    parse_accepted("package P { attribute x : a.b; }");
    parse_accepted("package P { part p : Vehicle::Engine.torque; }");
    // The plain QualifiedName alternative still works, and `::` is within one link.
    parse_accepted("package P { part p :>> a; }");
    parse_accepted("package P { part p :>> X::y.z; }");
}

#[test]
fn a_reference_chain_is_flat_where_an_expression_chain_folds() {
    // Two productions, two shapes, and the position is what chooses between them — as it
    // is for `[` between a MultiplicityRange and a BracketExpression.
    //
    // SysML 8.2.2.6.5 — OwnedFeatureChain = OwnedFeatureChaining
    //                                       ( '.' OwnedFeatureChaining )+   flat
    // KerML 8.2.5.8.2 — FeatureChainExpression = NonFeatureChainPrimaryArgumentMember
    //                                            '.' FeatureChainMember     folds left
    let reference = render(&parse_accepted("package P { part p :>> a.b.c; }").syntax());
    assert_eq!(
        nodes_named(&reference, "OwnedFeatureChain"),
        1,
        "{reference}"
    );
    // Three links as SIBLINGS under one chain, not three nested chains.
    assert_eq!(
        nodes_named(&reference, "OwnedFeatureChaining"),
        3,
        "{reference}"
    );
    assert_eq!(
        nodes_named(&reference, "FeatureChainExpression"),
        0,
        "{reference}"
    );

    let expression = render(&parse_accepted("constraint def C { a.b.c }").syntax());
    assert_eq!(
        nodes_named(&expression, "FeatureChainExpression"),
        2,
        "{expression}"
    );
    assert_eq!(
        nodes_named(&expression, "OwnedFeatureChain"),
        0,
        "{expression}"
    );
}

#[test]
fn a_reference_of_one_link_is_not_a_chain() {
    // The `+` means a chain is at least two links, so a bare name is the QualifiedName
    // alternative. A chain node over one link would be a level carrying nothing.
    let rendered = render(&parse_accepted("package P { part p :>> a; }").syntax());
    assert_eq!(nodes_named(&rendered, "OwnedFeatureChain"), 0, "{rendered}");
    assert_eq!(nodes_named(&rendered, "OwnedRedefinition"), 1, "{rendered}");
    // Held as files by tests/rejection/feature-chain-needs-a-link-after-every-dot.sysml
    // and owned-feature-chain-needs-two-links.sysml.
    parse_rejected("package P { part p :>> a.; }");
    parse_rejected("package P { part p :>> .b; }");
}

// -- ActionUsage, SysML 8.2.2.17.2 ------------------------------------------------
//
// ActionUsage            = OccurrenceUsagePrefix 'action'
//                          ActionUsageDeclaration ActionBody
// ActionUsageDeclaration = UsageDeclaration ValuePart?
//
// Off the SIMPLE_USAGES spine at the body end, the way ActionDefinition is off the
// definition spine.

#[test]
fn an_action_usage_is_a_declaration_over_an_action_body() {
    parse_accepted("package P { action brake; }");
    parse_accepted("package P { action brake : Braking; }");
    parse_accepted("package P { action brake { part pedal; } }");
    parse_accepted("package P { action a = b; }");
    parse_accepted("package P { individual action brake; }");
    // Inside the bodies that admit a usage, including an action body.
    parse_accepted("part def V { action brake; }");
    parse_accepted("action def B { action inner; }");
}

#[test]
fn an_action_usage_may_name_nothing_at_all() {
    // The same optionality SubjectUsage has: UsageDeclaration is
    // `Identification FeatureSpecializationPart?` and Identification is two optional
    // parts (SysML 8.2.2.6.2, 8.2.3.1), so an anonymous action is grammatical.
    parse_accepted("package P { action; }");
}

#[test]
fn the_def_is_what_separates_an_action_usage_from_an_action_definition() {
    // `at_action_usage` requires that no `def` follow the keyword, exactly as every one
    // of SIMPLE_USAGES does. Both productions are implemented, so this is a rule about
    // which one is reached and not about which one exists.
    let usage = render(&parse_accepted("package P { action brake; }").syntax());
    assert_eq!(nodes_named(&usage, "ActionUsage"), 1, "{usage}");
    assert_eq!(nodes_named(&usage, "ActionDefinition"), 0, "{usage}");

    let definition = render(&parse_accepted("package P { action def Brake; }").syntax());
    assert_eq!(
        nodes_named(&definition, "ActionDefinition"),
        1,
        "{definition}"
    );
    assert_eq!(nodes_named(&definition, "ActionUsage"), 0, "{definition}");

    // Both in one body, told apart by that one word.
    let both = render(&parse_accepted("package P { action def B; action b : B; }").syntax());
    assert_eq!(nodes_named(&both, "ActionDefinition"), 1, "{both}");
    assert_eq!(nodes_named(&both, "ActionUsage"), 1, "{both}");

    // Held as a file by tests/rejection/action-usage-is-not-an-action-definition.sysml.
    parse_rejected("package P { action def def Brake; }");
}

#[test]
fn an_action_usage_takes_an_action_body_not_a_usage_body() {
    // The seven of SIMPLE_USAGES end in `Usage`, which reaches UsageBody and so
    // DefinitionBody. This one ends in an ActionBody (SysML 8.2.2.17.2), which is the
    // whole reason it is not a row in that table.
    let rendered = render(&parse_accepted("package P { action brake { part p; } }").syntax());
    assert_eq!(
        child_kinds(&rendered, "ActionUsage"),
        [
            "OccurrenceUsagePrefix",
            "KwAction",
            "ActionUsageDeclaration",
            "ActionBody"
        ],
        "{rendered}"
    );
    // A part usage beside it still takes the usage spine, so the two shapes coexist.
    let part = render(&parse_accepted("package P { part p { } }").syntax());
    assert_eq!(nodes_named(&part, "UsageBody"), 1, "{part}");
    assert_eq!(nodes_named(&part, "ActionBody"), 0, "{part}");
}

#[test]
fn an_action_definition_nests_where_a_definition_element_may_go() {
    parse_accepted("package P { action def Brake; }");
    parse_accepted("part def Vehicle { action def Brake; }");
    // And the line the training corpus file needed, beside the port it sits next to.
    parse_accepted("package P { port def ClutchPort; action def GenerateTorque; }");
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
fn a_calculation_body_admits_every_item_an_action_body_does() {
    // CalculationBodyItem = ActionBodyItem | ReturnParameterMember (8.2.2.19), so every
    // usage an action body reads is an item here too, and the result expression, if
    // written, still follows them. Each of these opens on a keyword, so none can be the
    // expression: SuccessionAsUsage and BindingConnectorAsUsage are
    // NonOccurrenceUsageElements, AssertConstraintUsage and CalculationUsage
    // BehaviorUsageElements (8.2.2.6.4).
    for (item, node) in [
        ("first a then b;", "SuccessionAsUsage"),
        ("bind a = b;", "BindingConnectorAsUsage"),
        ("assert constraint c { y }", "AssertConstraintUsage"),
        ("calc d { y }", "CalculationUsage"),
    ] {
        for owner in [
            "constraint def C",
            "calc def C",
            "calc c",
            "assert constraint",
        ] {
            let source = format!("{owner} {{ {item} x }}");
            let tree = render(&parse_accepted(&source).syntax());
            assert!(nodes_named(&tree, node) >= 1, "{source}: {tree}");
            assert_eq!(
                nodes_named(&tree, "ResultExpressionMember"),
                1 + usize::from(item.contains("{ y }")),
                "{source}: {tree}"
            );
        }
    }
    // With no result expression after it, as the removed absence test wrote it.
    parse_accepted("constraint def C { first a then b; }");
}

#[test]
fn a_constraint_definition_needs_a_body_and_a_def() {
    // CalculationBody is not optional. Held as a file by
    // tests/rejection/constraint-definition-missing-calculation-body.sysml.
    parse_rejected("constraint def C");
    // An unclosed body is still an error, and the expression inside it is still read.
    parse_rejected("constraint def C { a <= b");
}

// -- CalculationDefinition, SysML 8.2.2.19 ----------------------------------------
//
// CalculationDefinition = OccurrenceDefinitionPrefix 'calc' 'def'
//                         DefinitionDeclaration CalculationBody
//
// ConstraintDefinition's shape (8.2.2.20) differing in one keyword, over the body the
// two share. The metaclass is SysML::CalculationDefinition (8.3.19.2), an
// ActionDefinition that is also a Function.

#[test]
fn a_calculation_definition_reads_the_body_a_constraint_definition_reads() {
    // One keyword apart from `constraint def`, over the same CalculationBody, so every
    // form that body takes is a form this definition takes.
    parse_accepted("calc def C;");
    parse_accepted("calc def C { }");
    parse_accepted("calc def C { a + b }");
    parse_accepted("calc def C { attribute x; attribute y; x + y }");
    // The declaration is a DefinitionDeclaration, so it specializes like any other.
    parse_accepted("calc def C :> Base;");
}

#[test]
fn a_calculation_definition_takes_the_occurrence_definition_prefix() {
    // OccurrenceDefinitionPrefix = DefinitionPrefix ( 'individual' ... )?
    // (SysML 8.2.2.9.1), the same prefix ConstraintDefinition and ActionDefinition take.
    parse_accepted("abstract calc def C;");
    parse_accepted("variation calc def C;");
}

#[test]
fn a_calculation_definition_owns_no_definition_node() {
    // Like ConstraintDefinition, it names the declaration and the body separately
    // rather than taking a Definition (SysML 8.2.2.19), so no Definition node is built.
    let rendered = render(&parse_accepted("calc def C { a + b }").syntax());
    assert_eq!(
        nodes_named(&rendered, "CalculationDefinition"),
        1,
        "{rendered}"
    );
    assert_eq!(nodes_named(&rendered, "CalculationBody"), 1, "{rendered}");
    assert_eq!(
        nodes_named(&rendered, "CalculationBodyPart"),
        1,
        "{rendered}"
    );
    assert_eq!(
        nodes_named(&rendered, "ResultExpressionMember"),
        1,
        "{rendered}"
    );
    assert_eq!(nodes_named(&rendered, "DefinitionBody"), 0, "{rendered}");
    assert_eq!(nodes_named(&rendered, "Definition"), 0, "{rendered}");
    // And it is NOT the sibling it shares a body with.
    assert_eq!(
        nodes_named(&rendered, "ConstraintDefinition"),
        0,
        "{rendered}"
    );
}

#[test]
fn a_calculation_definition_nests_where_a_definition_element_may_stand() {
    // It is dispatched from `membership`, so it stands wherever a DefinitionElement may.
    parse_accepted("package P { calc def C { a + b } }");
    parse_accepted("part def V { calc def C; }");
}

#[test]
fn a_calculation_definition_needs_a_body_and_a_def() {
    // CalculationBody is not optional, and it is what the `calc` keyword alone does not
    // supply. Held as a file by
    // tests/rejection/calculation-definition-missing-calculation-body.sysml.
    parse_rejected("calc def C");
    // An unclosed body is still an error, as it is for the sibling.
    parse_rejected("calc def C { a + b");
}

// -- ReturnParameterMember, SysML 8.2.2.19 ----------------------------------------
//
// CalculationBodyItem   = ActionBodyItem | ReturnParameterMember
// ReturnParameterMember = MemberPrefix? 'return'
//                         ownedRelatedElement += UsageElement
//
// The second alternative of CalculationBodyItem, and what every one of the thirteen
// corpus files that writes `calc def` also writes. The metaclass is KerML's
// ReturnParameterMembership (8.3.4.7.8), a ParameterMembership.

#[test]
fn a_return_parameter_member_reads_the_keywordless_usage_forms() {
    // The corpus writes these, from vendor/corpus/sysml/src/examples/Simple Tests/
    // CalculationTest.sysml and the Analysis Examples: a bare name, a name with a
    // typing, and a name with a typing and a value.
    parse_accepted("calc def C { return x; }");
    parse_accepted("calc def C { return v : SpeedValue; }");
    parse_accepted("calc def C { return v : SpeedValue = v0 + a * dt; }");
    // NOT here: `return totalMass : MassValue = sum(partMasses);`, the CalculationTest
    // line. The member reads it; the VALUE does not, because `sum(...)` is an
    // InvocationExpression (KerML 8.2.5.8.3) and that is unimplemented. It is the next
    // blocker behind this member, not a defect in it — held by
    // tests/rejection/invocation-expression-is-not-implemented.sysml.
    parse_accepted("calc def C { return totalMass : MassValue = partMasses; }");
    // A name with a subsetting, and one with a subsetting and a value —
    // `return distance :> length;` and `return dpv :> distancePerVolume = 1/f;`.
    parse_accepted("calc def C { return distance :> length; }");
    parse_accepted("calc def C { return dpv :> distancePerVolume = 1/f; }");
    // A value with no typing at all: `return p = rho * R_bar * T;`.
    parse_accepted("calc def C { return p = rho * R_bar * T; }");
}

#[test]
fn a_return_parameter_member_reads_the_anonymous_forms() {
    // DefaultReferenceUsage's second form is a bare FeatureSpecializationPart with no
    // Identification (SysML 8.2.2.6.2), and the corpus returns through it more often
    // than not: `return : Real;`, `return : AccelerationValue = p / (m * v);` and
    // `return :>> result : Real = a;` are all written.
    parse_accepted("calc def C { return : Real; }");
    parse_accepted("calc def C { return : AccelerationValue = p / (m * v); }");
    parse_accepted("calc def C { return :>> result : Real = a; }");
    parse_accepted("calc def C { return :>> verdict = evaluatePassFail.verdict; }");
}

#[test]
fn a_return_parameter_member_reads_a_keyword_usage_too() {
    // UsageElement is an alternation, not a synonym for the keywordless usage, so the
    // corpus's `return attribute eval : Real = ...;` and `return part : Engine;` are
    // the same member over a different alternative.
    parse_accepted("calc def C { return attribute eval : Real; }");
    parse_accepted("calc def C { return part : Engine; }");
    parse_accepted("calc def C { return part :>> selectedAlternative : Engine; }");
}

#[test]
fn a_return_parameter_member_takes_a_visibility() {
    // MemberPrefix? = VisibilityIndicator? (SysML 8.2.2.5.1). The corpus does not
    // write it here, but the production states it and the parser must not require the
    // corpus's habits.
    parse_accepted("calc def C { private return x; }");
    parse_accepted("calc def C { public return : Real; }");
}

#[test]
fn a_return_parameter_member_is_a_member_and_not_the_result_expression() {
    // CalculationBodyPart = CalculationBodyItem* ResultExpressionMember?, so a `return`
    // is in the item run and the trailing expression is still readable after it. The
    // two are different nodes and the body may hold both.
    let both = render(&parse_accepted("calc def C { return x : Real; x + 1 }").syntax());
    assert_eq!(nodes_named(&both, "ReturnParameterMember"), 1, "{both}");
    assert_eq!(nodes_named(&both, "ResultExpressionMember"), 1, "{both}");

    // And a `return` alone is a member with no trailing expression at all.
    let member = render(&parse_accepted("calc def C { return x; }").syntax());
    assert_eq!(nodes_named(&member, "ReturnParameterMember"), 1, "{member}");
    assert_eq!(
        nodes_named(&member, "ResultExpressionMember"),
        0,
        "{member}"
    );
    // It owns its element through its OWN membership, so no DefinitionMember is built
    // around it — the same reason SubjectMember does not go through `membership`.
    assert_eq!(nodes_named(&member, "DefinitionMember"), 0, "{member}");
}

#[test]
fn a_return_parameter_member_is_admitted_by_the_body_and_not_the_definition() {
    // CalculationBody is shared by ConstraintDefinition (SysML 8.2.2.20), so a
    // `constraint def` body admits a `return` for the same reason a `calc def` body
    // does: the alternative belongs to CalculationBodyItem, not to either definition.
    parse_accepted("constraint def C { return x; }");
    // But a DefinitionBodyItem has no such alternative (SysML 8.2.2.6.1), and neither
    // does an ActionBodyItem (8.2.2.17.1) or a PackageBodyElement (8.2.2.5.1). Held as
    // files by tests/rejection/return-parameter-member-is-not-a-definition-body-item.sysml
    // and return-parameter-member-is-not-an-action-body-item.sysml.
    parse_rejected("part def V { return x; }");
    parse_rejected("action def A { return x; }");
    parse_rejected("package P { return x; }");
    parse_rejected("return x;");
}

#[test]
fn a_return_parameter_member_needs_a_usage_element() {
    // UsageElement is not optional: `return` alone is the keyword with nothing to own.
    // Held as a file by tests/rejection/return-parameter-member-needs-a-usage.sysml.
    parse_rejected("calc def C { return; }");
}

// -- Usage memberships, SysML 8.2.2.6.1 / 8.2.2.6.4 / 8.2.2.17.1 --------------------
//
// DefinitionMember : OwningMembership = MemberPrefix DefinitionElement      (8.2.2.6.1)
//
// owns DEFINITIONS only. A usage in a body is owned through the membership the body's
// item production names for its kind (8.2.2.6.4 sorts the usages):
//
// DefinitionBodyItem = DefinitionMember | VariantUsageMember
//                    | NonOccurrenceUsageMember
//                    | SourceSuccessionMember? OccurrenceUsageMember | ...  (8.2.2.6.1)
// NonBehaviorBodyItem = ... | DefinitionMember | VariantUsageMember
//                     | NonOccurrenceUsageMember
//                     | SourceSuccessionMember? StructureUsageMember        (8.2.2.17.1)
// ActionBodyItem = NonBehaviorBodyItem | ...
//                | SourceSuccessionMember? ActionBehaviorMember ...         (8.2.2.17.1)
// ActionBehaviorMember = BehaviorUsageMember | ActionNodeMember             (8.2.2.17.1)
//
// NonOccurrenceUsageElement: ReferenceUsage, DefaultReferenceUsage, AttributeUsage,
//     EnumerationUsage, ...
// StructureUsageElement: OccurrenceUsage, ItemUsage, PartUsage, PortUsage,
//     RenderingUsage, ...
// BehaviorUsageElement: ActionUsage, PerformActionUsage, ...                (8.2.2.6.4)

/// The kind of the one membership in `action def A { <item> }` or
/// `part def P { <item> }`, whichever `body` names.
fn member_of(body: &str, item: &str) -> Vec<String> {
    let source = format!("{body} {{ {item} }}");
    let tree = render(&parse_accepted(&source).syntax());
    child_kinds(&tree, "DefinitionBody")
        .into_iter()
        .chain(child_kinds(&tree, "ActionBody"))
        .filter(|kind| kind.ends_with("Member"))
        .collect()
}

#[test]
fn a_definition_body_owns_a_usage_through_its_kind_of_usage_membership() {
    for item in ["attribute a;", "enum e;", "ref r;", "x;"] {
        assert_eq!(
            member_of("part def P", item),
            ["NonOccurrenceUsageMember"],
            "{item}"
        );
    }
    // OccurrenceUsageElement = StructureUsageElement | BehaviorUsageElement: a
    // definition body does not tell the two apart.
    for item in [
        "occurrence o;",
        "item i;",
        "part p;",
        "port q;",
        "action a;",
    ] {
        assert_eq!(
            member_of("part def P", item),
            ["OccurrenceUsageMember"],
            "{item}"
        );
    }
    // And a definition is still a DefinitionMember.
    assert_eq!(member_of("part def P", "part def Q;"), ["DefinitionMember"]);
}

#[test]
fn an_action_body_tells_structure_from_behaviour() {
    for item in ["attribute a;", "ref r;", "x;"] {
        assert_eq!(
            member_of("action def A", item),
            ["NonOccurrenceUsageMember"],
            "{item}"
        );
    }
    for item in ["occurrence o;", "item i;", "part p;"] {
        assert_eq!(
            member_of("action def A", item),
            ["StructureUsageMember"],
            "{item}"
        );
    }
    // Behaviour usages are ActionBodyItem's third alternative, through
    // ActionBehaviorMember, which has no node of its own: it is an alternation.
    for item in ["action a;", "perform p;"] {
        assert_eq!(
            member_of("action def A", item),
            ["BehaviorUsageMember"],
            "{item}"
        );
    }
    assert_eq!(
        member_of("action def A", "part def Q;"),
        ["DefinitionMember"]
    );
}

#[test]
fn a_usage_membership_owns_its_prefix_and_its_usage() {
    // MemberPrefix ownedRelatedElement += <kind>UsageElement — all four usage
    // memberships are stated so in 8.2.2.6.1. The visibility is the member's.
    let tree = render(&parse_accepted("action def A { private action a; }").syntax());
    assert_eq!(
        child_kinds(&tree, "BehaviorUsageMember"),
        ["MemberPrefix", "ActionUsage"],
        "{tree}"
    );
    assert_eq!(nodes_named(&tree, "DefinitionMember"), 0, "{tree}");
    // A package still owns a usage through PackageMember, whose UsageElement
    // alternative is exactly that (8.2.2.5.1).
    // The package body's one member is a PackageMember (the root owns `P` through
    // another, which is why this asks the body and does not count the tree).
    let package = render(&parse_accepted("package P { part p; }").syntax());
    assert_eq!(
        child_kinds(&package, "PackageBody"),
        ["LBrace", "PackageMember", "RBrace"],
        "{package}"
    );
}

// -- FlowUsage, SysML 8.2.2.16 ----------------------------------------------------
//
// FlowUsage = OccurrenceUsagePrefix 'flow' FlowDeclaration DefinitionBody
// FlowDeclaration : FlowUsage =
//       UsageDeclaration ValuePart?
//       ( 'of'  FlowPayloadFeatureMember )?
//       ( 'from' FlowEndMember 'to' FlowEndMember )?
//     | FlowEndMember 'to' FlowEndMember
// FlowEndMember : EndFeatureMembership = FlowEnd
// FlowEnd = FlowEndSubsetting? FlowFeatureMember
// FlowEndSubsetting : ReferenceSubsetting = [QualifiedName] '.' | FeatureChainPrefix
//     — the '.' is deviation FlowEndSubsetting (follow_xtext): the clause drops it,
//       and nothing else in the clause could consume it.
// FeatureChainPrefix : Feature =
//     ( OwnedFeatureChaining '.' )+ OwnedFeatureChaining '.'
// FlowFeatureMember : FeatureMembership = FlowFeature
// FlowFeature : ReferenceUsage = FlowFeatureRedefinition
// FlowFeatureRedefinition : Redefinition = [QualifiedName]
// FlowPayloadFeatureMember : FeatureMembership = FlowPayloadFeature
// FlowPayloadFeature : PayloadFeature = PayloadFeature
// PayloadFeature : Feature =
//       Identification? PayloadFeatureSpecializationPart ValuePart?
//     | OwnedFeatureTyping OwnedMultiplicity?
//     | OwnedMultiplicity OwnedFeatureTyping
//
// FlowUsage is a StructureUsageElement (8.2.2.6.4), so it is owned as the other
// structure usages are: a StructureUsageMember in an action body, an
// OccurrenceUsageMember in a definition body, a PackageMember in a package.

#[test]
fn a_flow_usage_reads_the_corpus_forms() {
    // vendor/corpus/sysml/src/training/15. Actions/Action Decomposition.sysml:19
    let from =
        render(&parse_accepted("action def A { flow from focus.image to shoot.image; }").syntax());
    assert_eq!(nodes_named(&from, "FlowUsage"), 1, "{from}");
    assert_eq!(
        member_of("action def A", "flow from a.b to c.d;"),
        ["StructureUsageMember"]
    );
    // vendor/corpus/sysml/src/training/17. Control/Camera.sysml:17 — the second
    // FlowDeclaration alternative, in a part definition.
    assert_eq!(
        member_of(
            "part def P",
            "flow autoFocus.realImage to imager.focusedImage;"
        ),
        ["OccurrenceUsageMember"]
    );
    // vendor/corpus/sysml/src/training/13. Flows/Flow Usage Example.sysml:10-12 — a
    // payload, three-segment ends, and the statement across three lines.
    parse_accepted(
        "part def P {\n\tflow of Fuel\n\t  from tankAssy.fuelTankPort.fuelSupply\n\t\tto eng.engineFuelPort.fuelSupply;\n}",
    );
    // vendor/corpus/sysml/src/examples/Camera Example/PictureTaking.sysml:9
    parse_accepted("action def A { flow of Exposure from focus.xrsl to shoot.xsf; }");
    // And at package level, through PackageMember's UsageElement.
    parse_accepted("package P { flow a.b to c.d; }");
}

#[test]
fn a_flow_usage_owns_what_its_productions_write() {
    let tree = render(&parse_accepted("part def P { flow from a.b to c.d; }").syntax());
    assert_eq!(
        child_kinds(&tree, "FlowUsage"),
        [
            "OccurrenceUsagePrefix",
            "KwFlow",
            "FlowDeclaration",
            "DefinitionBody"
        ],
        "{tree}"
    );
    // The first alternative: its UsageDeclaration is there even when it declares
    // nothing, as Identification may be empty.
    assert_eq!(
        child_kinds(&tree, "FlowDeclaration"),
        [
            "UsageDeclaration",
            "KwFrom",
            "FlowEndMember",
            "KwTo",
            "FlowEndMember"
        ],
        "{tree}"
    );
    assert_eq!(child_kinds(&tree, "FlowEndMember"), ["FlowEnd"], "{tree}");
    assert_eq!(
        child_kinds(&tree, "FlowEnd"),
        ["FlowEndSubsetting", "FlowFeatureMember"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "FlowEndSubsetting"),
        ["QualifiedName", "Dot"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "FlowFeatureMember"),
        ["FlowFeature"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "FlowFeature"),
        ["FlowFeatureRedefinition"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "FlowFeatureRedefinition"),
        ["QualifiedName"],
        "{tree}"
    );

    // The second alternative has no UsageDeclaration at all.
    let bare = render(&parse_accepted("part def P { flow a.b to c.d; }").syntax());
    assert_eq!(
        child_kinds(&bare, "FlowDeclaration"),
        ["FlowEndMember", "KwTo", "FlowEndMember"],
        "{bare}"
    );
}

#[test]
fn a_flow_end_splits_its_segments_by_the_productions() {
    // One segment: no FlowEndSubsetting, the feature alone.
    let one = render(&parse_accepted("part def P { flow a to b; }").syntax());
    assert_eq!(child_kinds(&one, "FlowEnd"), ["FlowFeatureMember"], "{one}");
    // Three: FeatureChainPrefix takes all but the last, each chaining with its '.'.
    let three = render(&parse_accepted("part def P { flow a.b.c to d.e; }").syntax());
    assert_eq!(
        child_kinds(&three, "FlowEndSubsetting"),
        ["FeatureChainPrefix"],
        "{three}"
    );
    assert_eq!(
        child_kinds(&three, "FeatureChainPrefix"),
        ["OwnedFeatureChaining", "Dot", "OwnedFeatureChaining", "Dot"],
        "{three}"
    );
    // A segment is a QualifiedName, so `::` stays inside one.
    parse_accepted("part def P { flow A::a.b to C::c.d; }");
}

#[test]
fn a_flow_declaration_takes_a_name_a_type_a_value_and_a_payload() {
    // UsageDeclaration: `flow : FuelFlow of Fuel` is the corpus's (training/13. Flows/
    // Flow Definition Example.sysml); a named one is the same production.
    parse_accepted("part def P { flow : FuelFlow of Fuel from a.b to c.d; }");
    parse_accepted("part def P { flow f : FuelFlow from a.b to c.d; }");
    // Every part of the first alternative is optional; a braced body too.
    parse_accepted("part def P { flow f; }");
    parse_accepted("part def P { flow f { } }");
}

#[test]
fn a_payload_feature_reads_all_three_alternatives() {
    fn payload(item: &str) -> Vec<String> {
        let tree = render(&parse_accepted(&format!("part def P {{ {item} }}")).syntax());
        child_kinds(&tree, "PayloadFeature")
    }
    // OwnedFeatureTyping OwnedMultiplicity? — the corpus's `of Fuel`.
    assert_eq!(
        payload("flow of Fuel from a.b to c.d;"),
        ["OwnedFeatureTyping"]
    );
    assert_eq!(
        payload("flow of Fuel[1] from a.b to c.d;"),
        ["OwnedFeatureTyping", "OwnedMultiplicity"]
    );
    // OwnedMultiplicity OwnedFeatureTyping.
    assert_eq!(
        payload("flow of [1] Fuel from a.b to c.d;"),
        ["OwnedMultiplicity", "OwnedFeatureTyping"]
    );
    // Identification? PayloadFeatureSpecializationPart ValuePart?
    assert_eq!(
        payload("flow of fuel : Fuel from a.b to c.d;"),
        ["Identification", "PayloadFeatureSpecializationPart"]
    );
    assert_eq!(
        payload("flow of fuel [1] : Fuel = f from a.b to c.d;"),
        [
            "Identification",
            "PayloadFeatureSpecializationPart",
            "ValuePart"
        ]
    );
    // The member and its wrapper are one node each, as their productions are.
    let tree = render(&parse_accepted("part def P { flow of Fuel from a.b to c.d; }").syntax());
    assert_eq!(
        child_kinds(&tree, "FlowPayloadFeatureMember"),
        ["FlowPayloadFeature"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "FlowPayloadFeature"),
        ["PayloadFeature"],
        "{tree}"
    );
}

#[test]
fn a_flow_usage_keeps_every_byte() {
    let source = "part def P {\n\tflow /* c */ of Fuel // n\n\t  from a . b\n\t\tto c.d.e ;\n}\n";
    assert_eq!(parse_accepted(source).text(), source);
}

#[test]
fn a_flow_usage_is_not_a_flow_definition() {
    // `flow def` is FlowDefinition (8.2.2.16), already implemented: the `def` after the
    // keyword is what separates the two, as it does for every usage and its definition.
    let tree = render(&parse_accepted("part def P { flow def F; }").syntax());
    assert_eq!(nodes_named(&tree, "FlowUsage"), 0, "{tree}");
    assert_eq!(nodes_named(&tree, "FlowDefinition"), 1, "{tree}");
    assert_eq!(member_of("part def P", "flow def F;"), ["DefinitionMember"]);
}

#[test]
fn a_flow_usage_rejects_what_its_productions_do_not_state() {
    // Held as files by tests/rejection/flow-*.sysml.
    parse_rejected("part def P { flow from a.b; }");
    parse_rejected("part def P { flow a. to c.d; }");
    parse_rejected("part def P { flow of [1] from a.b to c.d; }");
    // A named payload with a multiplicity keyword and no FeatureSpecialization: the
    // one this list lacked until a mutation of payload_feature_specialization_part
    // went uncaught.
    parse_rejected("part def P { flow of x ordered from a.b to c.d; }");
}

// -- InitialNodeMember, SysML 8.2.2.17.1 ------------------------------------------
//
// ActionBodyItem    = NonBehaviorBodyItem
//                   | InitialNodeMember ActionTargetSuccessionMember*
//                   | …
// InitialNodeMember : FeatureMembership =
//     MemberPrefix 'first' memberFeature = [QualifiedName] RelationshipBody
//
// `first X;` names the source of a succession apart from its target, which the next
// `then` supplies (SysML 7.17.4). ActionBodyItem is reached by ActionBody,
// CalculationBodyItem (8.2.2.19), CaseBodyItem (8.2.2.22), ActionBodyParameter
// (8.2.2.17.7) and the four Transition*ActionUsage bodies (8.2.2.18.3) — never by
// DefinitionBodyItem or RequirementBodyItem.

#[test]
fn an_initial_node_member_reads_the_corpus_form() {
    // `first start;` is 15 of the corpus's 16 `first X;` lines — e.g. vendor/corpus/
    // sysml/src/training/17. Control/Merge Example.sysml. `start` is the snapshot every
    // action inherits from Actions::Action (7.17.4). ActionBody belongs to both the
    // definition (8.2.2.17.1) and the usage (8.2.2.17.2).
    let def = render(&parse_accepted("action def A { first start; }").syntax());
    assert_eq!(nodes_named(&def, "InitialNodeMember"), 1, "{def}");
    let usage = render(&parse_accepted("action a { first start; }").syntax());
    assert_eq!(nodes_named(&usage, "InitialNodeMember"), 1, "{usage}");
}

#[test]
fn an_initial_node_member_owns_exactly_what_the_production_writes() {
    // MemberPrefix 'first' [QualifiedName] RelationshipBody, in that order. The
    // memberFeature is a REFERENCE, so the name is a QualifiedName and no usage node is
    // built for it — and no DefinitionMember is built around the member, because it is
    // a FeatureMembership of its own, dispatched as ReturnParameterMember is.
    let tree = render(&parse_accepted("action def A { first start; }").syntax());
    assert_eq!(
        child_kinds(&tree, "InitialNodeMember"),
        [
            "MemberPrefix",
            "KwFirst",
            "QualifiedName",
            "RelationshipBody"
        ],
        "{tree}"
    );
    assert_eq!(nodes_named(&tree, "DefinitionMember"), 0, "{tree}");
}

#[test]
fn an_initial_node_member_takes_a_visibility_a_qualified_name_and_a_braced_body() {
    // `private first A3;` is the 16th, in vendor/corpus/sysml/src/examples/Simple Tests/
    // DecisionTest.sysml.
    parse_accepted("action def A { private first A3; }");
    // [QualifiedName], not a simple name (8.2.2.17.1); the corpus does not qualify one.
    parse_accepted("action def A { first Actions::Action::start; }");
    // RelationshipBody's braced form (8.2.2.2). The corpus writes none; the production
    // states it, so the parser must not require the corpus's habits.
    parse_accepted("action def A { first start { /* the initial node */ } }");
}

#[test]
fn an_initial_node_member_is_admitted_by_a_calculation_body_too() {
    // CalculationBodyItem = ActionBodyItem | ReturnParameterMember (8.2.2.19), so the
    // containment runs from calculation to action and `first` comes with it. A
    // constraint def shares CalculationBody (8.2.2.20), so it does as well.
    parse_accepted("calc def C { first start; }");
    parse_accepted("constraint def C { first start; }");
    // And it is an item, so the trailing ResultExpressionMember still follows it.
    let tree = render(&parse_accepted("calc def C { first start; x + 1 }").syntax());
    assert_eq!(nodes_named(&tree, "InitialNodeMember"), 1, "{tree}");
    assert_eq!(nodes_named(&tree, "ResultExpressionMember"), 1, "{tree}");
}

#[test]
fn an_initial_node_member_is_not_a_definition_or_requirement_body_item() {
    // DefinitionBodyItem has no ActionBodyItem alternative (8.2.2.6.1), and
    // RequirementBodyItem reaches only DefinitionBodyItem (8.2.2.21.1) — so a requirement
    // body does NOT admit `first`, however much it looks like a behaviour. Held as files
    // by tests/rejection/initial-node-member-is-not-a-definition-body-item.sysml and
    // initial-node-member-is-not-a-requirement-body-item.sysml.
    parse_rejected("part def V { first start; }");
    parse_rejected("requirement def R { first start; }");
    parse_rejected("package P { first start; }");
    parse_rejected("first start;");
}

#[test]
fn an_initial_node_member_needs_its_name_and_its_body() {
    // memberFeature = [QualifiedName] is not optional, and neither is RelationshipBody.
    parse_rejected("action def A { first; }");
    parse_rejected("action def A { first start }");
    // A feature chain is not a QualifiedName (KerML 8.2.3.4.1 against 8.2.5.8.2); the
    // successions that take one write `then` after it. Held as a file by
    // tests/rejection/initial-node-member-names-a-qualified-name-not-a-feature-chain.sysml.
    parse_rejected("action def A { first a.b; }");
}

#[test]
fn a_succession_that_opens_on_first_is_not_an_initial_node_member() {
    // `first a then b;` is SuccessionAsUsage (8.2.2.13.3), and GuardedSuccession
    // (8.2.2.17.8) opens on `first` too. Whether the text is accepted is that
    // production's question, not this test's: what is asserted here is that it never
    // builds an InitialNodeMember, because `first a` followed by `then` is not one — the
    // production ends in RelationshipBody, which is `;` or `{`. A rejection file cannot
    // show this; the tree can.
    let tree = render(&parse("action def A { first a then b; }", Language::SysMl).syntax());
    assert_eq!(nodes_named(&tree, "InitialNodeMember"), 0, "{tree}");
}

// -- ActionTargetSuccessionMember, SysML 8.2.2.17.1 / 8.2.2.17.8 -------------------
//
// ActionTargetSuccessionMember : FeatureMembership =
//     MemberPrefix ownedRelatedElement += ActionTargetSuccession        (8.2.2.17.1)
// ActionTargetSuccession = ( TargetSuccession | GuardedTargetSuccession
//                          | DefaultTargetSuccession ) UsageBody        (8.2.2.17.8)
// TargetSuccession : SuccessionAsUsage =
//     SourceEndMember 'then' ConnectorEndMember                         (8.2.2.17.8)
// SourceEnd = OwnedMultiplicity?                                        (8.2.2.9.3)
// ConnectorEnd = OwnedCrossMultiplicityMember? ( NAME REFERENCES )?
//                OwnedReferenceSubsetting                               (8.2.2.13.1)
//
// Only the TargetSuccession form: GuardedTargetSuccession (`then` behind `if`) and
// DefaultTargetSuccession (`else`) are unimplemented, as is OwnedCrossMultiplicityMember.
// And only after InitialNodeMember — the other predecessor ActionBodyItem gives it,
// ActionBehaviorMember, is unimplemented.

#[test]
fn a_target_succession_reads_the_corpus_form() {
    // SysML 7.17.4's own example, `first action1; then action2;` (wiki receipt 339ef468):
    // "the target of a succession may be specified separately from the source by using
    // the keyword then followed by a qualified name or feature chain for the target
    // action usage". NO corpus file writes this exact shape — a bare-name `then Y;`
    // directly after `first X;` — measured over all ten files with a `first X;`: every
    // `then` after one opens on a keyword (`then merge m;`, `then fork;`), which is
    // SourceSuccessionMember ActionBehaviorMember, not this.
    let spec = render(&parse_accepted("action def A { first action1; then action2; }").syntax());
    assert_eq!(
        nodes_named(&spec, "ActionTargetSuccessionMember"),
        1,
        "{spec}"
    );
    let def = render(&parse_accepted("action def A { first start; then a; }").syntax());
    assert_eq!(
        nodes_named(&def, "ActionTargetSuccessionMember"),
        1,
        "{def}"
    );
    let usage = render(&parse_accepted("action a { first start; then b; }").syntax());
    assert_eq!(
        nodes_named(&usage, "ActionTargetSuccessionMember"),
        1,
        "{usage}"
    );
}

#[test]
fn a_target_succession_owns_exactly_what_the_productions_write() {
    let tree = render(&parse_accepted("action def A { first start; then a; }").syntax());
    assert_eq!(
        child_kinds(&tree, "ActionTargetSuccessionMember"),
        ["MemberPrefix", "ActionTargetSuccession"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "ActionTargetSuccession"),
        ["TargetSuccession", "UsageBody"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "TargetSuccession"),
        ["SourceEndMember", "KwThen", "ConnectorEndMember"],
        "{tree}"
    );
    // The source end is WRITTEN EMPTY: SourceEnd is `OwnedMultiplicity?` and nothing
    // stands before the `then`. The node is there with nothing in it, the same honest
    // shape as an empty MemberPrefix.
    assert_eq!(
        child_kinds(&tree, "SourceEndMember"),
        ["SourceEnd"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "SourceEnd"),
        Vec::<String>::new(),
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "ConnectorEndMember"),
        ["ConnectorEnd"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "ConnectorEnd"),
        ["OwnedReferenceSubsetting"],
        "{tree}"
    );
    // The member is its own FeatureMembership, a sibling of the InitialNodeMember and
    // not inside it: ActionBodyItem writes them one after the other.
    assert_eq!(
        child_kinds(&tree, "InitialNodeMember"),
        [
            "MemberPrefix",
            "KwFirst",
            "QualifiedName",
            "RelationshipBody"
        ],
        "{tree}"
    );
}

#[test]
fn target_successions_repeat_after_one_initial_node_member() {
    // `ActionTargetSuccessionMember*` (8.2.2.17.1).
    let tree =
        render(&parse_accepted("action def A { first start; then a; then b; then c; }").syntax());
    assert_eq!(nodes_named(&tree, "InitialNodeMember"), 1, "{tree}");
    assert_eq!(
        nodes_named(&tree, "ActionTargetSuccessionMember"),
        3,
        "{tree}"
    );
}

#[test]
fn a_target_succession_takes_every_form_its_productions_state() {
    // A feature chain: OwnedReferenceSubsetting's second alternative (8.2.2.6.5), and
    // 7.17.4 says `then` takes "a qualified name or feature chain". The corpus's
    // `then returnack.done;` is NOT evidence here: it continues a `succession first …`,
    // a different production.
    let chain = render(&parse_accepted("action def A { first start; then a.done; }").syntax());
    assert_eq!(nodes_named(&chain, "OwnedFeatureChain"), 1, "{chain}");
    // A qualified name, a visibility (MemberPrefix), and UsageBody's braced form.
    parse_accepted("action def A { first start; then P::a; }");
    parse_accepted("action def A { first start; private then a; }");
    parse_accepted("action def A { first start; then a { } }");
    // ConnectorEnd's `( NAME REFERENCES )?`, both spellings of REFERENCES (8.2.2.1.2).
    // The corpus writes neither; the production states them.
    let named = render(&parse_accepted("action def A { first start; then e ::> a; }").syntax());
    assert_eq!(
        child_kinds(&named, "ConnectorEnd"),
        ["BasicName", "ColonColonGt", "OwnedReferenceSubsetting"],
        "{named}"
    );
    parse_accepted("action def A { first start; then e references a; }");
    // TargetSuccession = SourceEndMember 'then' ConnectorEndMember: the source end's
    // OwnedMultiplicity (8.2.2.9.3) stands BEFORE the `then`.
    let multiplied = render(&parse_accepted("action def A { first start; [1] then a; }").syntax());
    assert_eq!(
        child_kinds(&multiplied, "SourceEnd"),
        ["OwnedMultiplicity"],
        "{multiplied}"
    );
    assert_eq!(
        nodes_named(&multiplied, "ActionTargetSuccessionMember"),
        1,
        "{multiplied}"
    );
    // CalculationBodyItem reaches ActionBodyItem (8.2.2.19), and the trailing
    // result expression still follows the item run.
    let calc = render(&parse_accepted("calc def C { first start; then a; x + 1 }").syntax());
    assert_eq!(
        nodes_named(&calc, "ActionTargetSuccessionMember"),
        1,
        "{calc}"
    );
    assert_eq!(nodes_named(&calc, "ResultExpressionMember"), 1, "{calc}");
}

#[test]
fn a_target_succession_keeps_every_byte() {
    // Invariant 1: the empty SourceEnd is built from no tokens, and the trivia around
    // it must still reach the tree in order.
    let source = "action def A {\n\tfirst start; // s\n\n\tthen /* t */ a . done ;\n}\n";
    assert_eq!(parse_accepted(source).text(), source);
}

#[test]
fn a_target_succession_follows_an_initial_node_member_and_nothing_else() {
    // The BNF admits ActionTargetSuccessionMember only directly after an
    // InitialNodeMember or an ActionBehaviorMember (8.2.2.17.1). 7.17.4's prose — the
    // source is "the nearest occurrence lexically previous to the then" — decides which
    // element is the source, not what parses; that is resolution, not syntax.
    // Held as files by tests/rejection/target-succession-member-*.sysml.
    parse_rejected("action def A { then a; }");
    parse_rejected("action def A { first start; attribute x; then a; }");
    // And not outside an action body at all.
    parse_rejected("part def V { first start; then a; }");
}

#[test]
fn a_target_succession_needs_its_target_and_its_body() {
    parse_rejected("action def A { first start; then; }");
    parse_rejected("action def A { first start; then a }");
}

#[test]
fn a_then_that_opens_another_production_is_not_a_target_succession() {
    // Productions that put `then` before something that is not a ConnectorEnd followed
    // by UsageBody, and are not implemented, so each is rejected — but WITHOUT an
    // ActionTargetSuccessionMember in the tree, because the text is not one.
    //
    // `then s send x;` is REJECTED: a declared send writes `action` (deviation SendNode,
    // follow_xtext, 8.2.2.17.4), so `s send x` is no SendNode: `then s` is reported, and
    // `send x;` after it is a SendNode of its own. The literal clause line's
    // ActionUsageDeclaration would read the whole as one;
    // tests/rejection/send-node-declaration-writes-action.sysml holds that reading out.
    //
    // `then action a;` was here too, and is not: it is the same alternative over an
    // ActionUsage, well-formed SysML, now implemented and asserted accepted with no
    // ActionTargetSuccessionMember. `then fork;` was here, and is not, for the same
    // reason over a ForkNode (8.2.2.17.3); the next commit implements it.
    let source = "action def A { first start; then s send x; }";
    let tree = render(&parse_rejected(source).syntax());
    assert_eq!(
        nodes_named(&tree, "ActionTargetSuccessionMember"),
        0,
        "{source}\n{tree}"
    );
    assert_eq!(
        nodes_named(&tree, "InitialNodeMember"),
        1,
        "{source}\n{tree}"
    );
}

// -- SourceSuccessionMember, SysML 8.2.2.9.3 / 8.2.2.6.1 / 8.2.2.17.1 --------------
//
// SourceSuccessionMember : FeatureMembership = 'then' SourceSuccession      (8.2.2.9.3)
// SourceSuccession : SuccessionAsUsage = SourceEndMember                    (8.2.2.9.3)
// SourceEnd = OwnedMultiplicity?                                            (8.2.2.9.3)
//
// A prefix, never an item alone. It stands before the occurrence usage it makes the
// TARGET of a succession, as a sibling member:
//
// DefinitionBodyItem  = ... | SourceSuccessionMember? OccurrenceUsageMember  (8.2.2.6.1)
// NonBehaviorBodyItem = ... | SourceSuccessionMember? StructureUsageMember   (8.2.2.17.1)
// ActionBodyItem      = ... | SourceSuccessionMember? ActionBehaviorMember
//                             ActionTargetSuccessionMember*                (8.2.2.17.1)
//
// The source is not written: 7.17.4 makes it the nearest occurrence lexically before
// the `then`, which is resolution's to find.

#[test]
fn then_before_an_action_reads_the_corpus_form() {
    // vendor/corpus/sysml/src/training/14. Action Definitions/Action Shorthand
    // Example.sysml:20 — after a flow, which is a structure usage; the grammar does not
    // ask what came before.
    let tree = render(
        &parse_accepted(
            "action def A { flow from focus.image to shoot.image; then action shoot : Shoot { } }",
        )
        .syntax(),
    );
    assert_eq!(nodes_named(&tree, "SourceSuccessionMember"), 1, "{tree}");
    // 7.17.4's own example: `first start; then action a;` — and with no
    // ActionTargetSuccessionMember, since `action a` is not a ConnectorEnd.
    let spec = render(&parse_accepted("action def A { first start; then action a; }").syntax());
    assert_eq!(nodes_named(&spec, "SourceSuccessionMember"), 1, "{spec}");
    assert_eq!(
        nodes_named(&spec, "ActionTargetSuccessionMember"),
        0,
        "{spec}"
    );
    parse_accepted("action def A { then perform p; }");
}

#[test]
fn a_source_succession_member_owns_what_its_productions_write() {
    let tree = render(&parse_accepted("action def A { then action a; }").syntax());
    // The prefix and the member it precedes are siblings in the body.
    assert_eq!(
        child_kinds(&tree, "ActionBody"),
        [
            "LBrace",
            "SourceSuccessionMember",
            "BehaviorUsageMember",
            "RBrace"
        ],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "SourceSuccessionMember"),
        ["KwThen", "SourceSuccession"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "SourceSuccession"),
        ["SourceEndMember"],
        "{tree}"
    );
    // Written empty, as TargetSuccession's is...
    assert_eq!(
        child_kinds(&tree, "SourceEnd"),
        Vec::<String>::new(),
        "{tree}"
    );
    // ...or with its OwnedMultiplicity, which here comes AFTER the `then`.
    let multiplied = render(&parse_accepted("action def A { then [1] action a; }").syntax());
    assert_eq!(
        child_kinds(&multiplied, "SourceEnd"),
        ["OwnedMultiplicity"],
        "{multiplied}"
    );
}

#[test]
fn then_prefixes_occurrence_and_structure_usages_too() {
    // DefinitionBodyItem: before an OccurrenceUsageMember — `then part p;` in a part def.
    assert_eq!(
        member_of("part def P", "then part p;"),
        ["SourceSuccessionMember", "OccurrenceUsageMember"]
    );
    assert_eq!(
        member_of("part def P", "then action a;"),
        ["SourceSuccessionMember", "OccurrenceUsageMember"]
    );
    // NonBehaviorBodyItem: before a StructureUsageMember in an action body.
    assert_eq!(
        member_of("action def A", "then part p;"),
        ["SourceSuccessionMember", "StructureUsageMember"]
    );
    // And a requirement body, which reaches DefinitionBodyItem (8.2.2.21.1).
    parse_accepted("requirement def R { then part p; }");
}

#[test]
fn target_successions_follow_a_behaviour_usage() {
    // `SourceSuccessionMember? ActionBehaviorMember ActionTargetSuccessionMember*`: the
    // `then X;` loop runs after a behaviour usage, with or without the `then` before it.
    let tree = render(&parse_accepted("action def A { action a; then b; then c; }").syntax());
    assert_eq!(
        nodes_named(&tree, "ActionTargetSuccessionMember"),
        2,
        "{tree}"
    );
    let prefixed = render(&parse_accepted("action def A { then action a; then b; }").syntax());
    assert_eq!(
        nodes_named(&prefixed, "ActionTargetSuccessionMember"),
        1,
        "{prefixed}"
    );
    // A calculation body reaches ActionBodyItem (8.2.2.19), and its item run does not
    // end at a behaviour usage — it did, until at_result_expression learned them.
    let calc = render(&parse_accepted("calc def C { action a; then b; x + 1 }").syntax());
    assert_eq!(nodes_named(&calc, "BehaviorUsageMember"), 1, "{calc}");
    assert_eq!(nodes_named(&calc, "ResultExpressionMember"), 1, "{calc}");
    parse_accepted("calc def C { perform p; flow a.b to c.d; then action q; x }");
}

#[test]
fn a_source_succession_keeps_every_byte() {
    let source = "action def A {\n\tthen /* s */ [ 1 ] // n\n\t\taction a;\n}\n";
    assert_eq!(parse_accepted(source).text(), source);
}

#[test]
fn a_source_succession_member_needs_an_occurrence_usage_after_it() {
    // Held as files by tests/rejection/source-succession-member-*.sysml.
    // Alone: the prefix is never an item.
    parse_rejected("action def A { then; }");
    // Before a non-occurrence usage: no item production pairs the two.
    parse_rejected("part def P { then attribute x; }");
    parse_rejected("action def A { then attribute x; }");
    // Before a definition.
    parse_rejected("part def P { then part def Q; }");
    // In a package body, which has no SourceSuccessionMember (8.2.2.5.1).
    parse_rejected("package P { then part p; }");
    // After a structure usage in an action body there is no target loop: only
    // ActionBehaviorMember takes the ActionTargetSuccessionMember* suffix.
    parse_rejected("action def A { part p; then b; }");
}

// -- ControlNode, SysML 8.2.2.17.3 ----------------------------------------------------
//
// ActionBehaviorMember = BehaviorUsageMember | ActionNodeMember              (8.2.2.17.1)
// ActionNodeMember : FeatureMembership = MemberPrefix ActionNode            (8.2.2.17.1)
// ControlNode = MergeNode | DecisionNode | JoinNode | ForkNode              (8.2.2.17.3)
// ControlNodePrefix : OccurrenceUsage =
//     RefPrefix 'individual'? PortionKind? UsageExtensionKeyword*           (8.2.2.17.3)
// MergeNode = ControlNodePrefix 'merge' UsageDeclaration ActionBody         (8.2.2.17.3)
//
// and DecisionNode, JoinNode and ForkNode the same over `decide`, `join` and `fork`.
// ControlNode is an ActionNode, so it reaches ActionBodyItem's third alternative
// through ActionNodeMember, and takes the same `then` prefix and the same
// ActionTargetSuccessionMember* after it as a behaviour usage does.

#[test]
fn a_control_node_reads_the_corpus_forms() {
    // vendor/corpus/sysml/src/training/17. Control/Merge Example.sysml:14 — the merge
    // is a succession's target, and the `then action` after it is NOT a target
    // succession of the merge but the next item's source-succession prefix.
    let merge = render(
        &parse_accepted(
            "action def A { first start; then merge continue; then action trigger { } }",
        )
        .syntax(),
    );
    assert_eq!(nodes_named(&merge, "MergeNode"), 1, "{merge}");
    assert_eq!(nodes_named(&merge, "SourceSuccessionMember"), 2, "{merge}");
    assert_eq!(
        nodes_named(&merge, "ActionTargetSuccessionMember"),
        0,
        "{merge}"
    );
    // training/17. Control/Fork Join Example.sysml:14-17, 38-39 — an anonymous fork
    // followed by its target successions, and a named join followed by one.
    let fork = render(
        &parse_accepted(
            "action def A { first start; then fork; then a; then b; join j; then done; }",
        )
        .syntax(),
    );
    assert_eq!(nodes_named(&fork, "ForkNode"), 1, "{fork}");
    assert_eq!(nodes_named(&fork, "JoinNode"), 1, "{fork}");
    assert_eq!(
        nodes_named(&fork, "ActionTargetSuccessionMember"),
        3,
        "{fork}"
    );
    // examples/Simple Tests/ControlNodeTest.sysml:13-19 — a node with a braced
    // ActionBody holding directed parameters, then its targets.
    let braced = render(
        &parse_accepted("action def A { join J; then fork F { in a; out b1; } then B1; then B2; }")
            .syntax(),
    );
    assert_eq!(
        nodes_named(&braced, "ActionTargetSuccessionMember"),
        2,
        "{braced}"
    );
    // training/17. Control/Decision Example.sysml writes `then decide;`.
    parse_accepted("action def A { first start; then decide; }");
}

#[test]
fn every_control_node_keyword_builds_its_own_node() {
    for (keyword, node) in [
        ("merge", "MergeNode"),
        ("decide", "DecisionNode"),
        ("join", "JoinNode"),
        ("fork", "ForkNode"),
    ] {
        let source = format!("action def A {{ {keyword} n; }}");
        let tree = render(&parse_accepted(&source).syntax());
        assert_eq!(
            member_of("action def A", &format!("{keyword} n;")),
            ["ActionNodeMember"],
            "{source}"
        );
        assert_eq!(
            child_kinds(&tree, "ActionNodeMember"),
            ["MemberPrefix", node],
            "{tree}"
        );
        // `isComposite ?= 'merge'` sets a property; it adds no token beyond the keyword.
        assert_eq!(
            child_kinds(&tree, node),
            [
                "ControlNodePrefix",
                &format!("Kw{}", capitalise(keyword)),
                "UsageDeclaration",
                "ActionBody"
            ],
            "{tree}"
        );
    }
}

fn capitalise(word: &str) -> String {
    let mut chars = word.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

#[test]
fn a_control_node_takes_a_ref_prefix_but_not_ref() {
    // ControlNodePrefix = RefPrefix 'individual'? PortionKind? — every part optional,
    // and the node is built even when empty, as OccurrenceUsagePrefix's is.
    let tree = render(&parse_accepted("action def A { merge m; }").syntax());
    assert_eq!(
        child_kinds(&tree, "ControlNodePrefix"),
        ["RefPrefix"],
        "{tree}"
    );
    let full =
        render(&parse_accepted("action def A { abstract individual snapshot fork f; }").syntax());
    assert_eq!(
        child_kinds(&full, "ControlNodePrefix"),
        ["RefPrefix", "KwIndividual", "PortionKind"],
        "{full}"
    );
    // RefPrefix, NOT BasicUsagePrefix: there is no `ref` slot (8.2.2.6.2 against
    // 8.2.2.17.3). Held as a file by tests/rejection/control-node-prefix-has-no-ref.sysml.
    parse_rejected("action def A { ref merge m; }");
}

#[test]
fn a_control_node_declares_a_usage() {
    // UsageDeclaration = Identification FeatureSpecializationPart?, all optional.
    parse_accepted("action def A { merge m : M [1]; }");
    parse_accepted("action def A { private decide <d> 'the decision'; }");
    parse_accepted("action def A { then [1] join; }");
}

#[test]
fn a_control_node_is_an_action_body_item_only() {
    // A calculation body reaches ActionBodyItem (8.2.2.19), and its item run does not
    // end at a control node.
    let calc = render(&parse_accepted("calc def C { merge m; then b; x }").syntax());
    assert_eq!(nodes_named(&calc, "ActionNodeMember"), 1, "{calc}");
    assert_eq!(nodes_named(&calc, "ResultExpressionMember"), 1, "{calc}");
    // An action usage's body is an ActionBody too.
    parse_accepted("part def P { action a { fork f; } }");
    // DefinitionBodyItem (8.2.2.6.1), RequirementBodyItem (8.2.2.21.1) and
    // PackageBodyElement (8.2.2.5.1) have no ActionNodeMember, which is also what
    // validateControlNodeOwningType states of the metaclass (8.3.17.6, receipt 695df335).
    // Held as files by tests/rejection/control-node-is-not-*.sysml.
    parse_rejected("part def P { merge m; }");
    parse_rejected("part def P { then merge m; }");
    parse_rejected("requirement def R { decide d; }");
    parse_rejected("package P { fork f; }");
    parse_rejected("join j;");
}

#[test]
fn a_control_node_needs_its_action_body() {
    // ActionBody = ';' | '{' ActionBodyItem* '}' — not optional in the production.
    // Held as a file by tests/rejection/control-node-needs-an-action-body.sysml.
    parse_rejected("action def A { merge m }");
    // And the keyword is not a name: `merge def M;` is no definition.
    parse_rejected("action def A { merge def M; }");
}

#[test]
fn a_control_node_keeps_every_byte() {
    let source = "action def A {\n\tthen /* s */ merge // n\n\t\tm { }\n\tthen b;\n}\n";
    assert_eq!(parse_accepted(source).text(), source);
}

// -- GuardedTargetSuccession, SysML 8.2.2.17.8 ----------------------------------------
//
// ActionTargetSuccession : Usage =
//     ( TargetSuccession | GuardedTargetSuccession | DefaultTargetSuccession )
//     UsageBody                                                        (8.2.2.17.8)
// GuardedTargetSuccession : TransitionUsage =
//     GuardExpressionMember 'then' TransitionSuccessionMember           (8.2.2.17.8)
// GuardExpressionMember : TransitionFeatureMembership =
//     'if' { kind = 'guard' } OwnedExpression                           (8.2.2.18.3)
// TransitionSuccessionMember : OwningMembership = TransitionSuccession  (8.2.2.18.3)
// TransitionSuccession : Succession = EmptyEndMember ConnectorEndMember (8.2.2.18.3)
// EmptyEndMember : EndFeatureMembership = EmptyFeature                  (8.2.2.18.3)
//
// ActionTargetSuccession's second alternative, so it stands where `then X;` stands: in
// the ActionTargetSuccessionMember* loop after an InitialNodeMember or an
// ActionBehaviorMember. Where TargetSuccession writes its source end before the `then`,
// this writes a guard expression there, and its metaclass is a TransitionUsage rather
// than a SuccessionAsUsage (8.3.18.9, receipt a6f32577).

#[test]
fn a_guarded_target_succession_reads_the_corpus_forms() {
    // vendor/corpus/sysml/src/training/17. Control/Decision Example.sysml:22-24 — two
    // guarded successions after a decision node.
    let decision = render(
        &parse_accepted(
            "action def A { first start; then decide; \
             if monitor.batteryCharge < 100 then addCharge; \
             if monitor.batteryCharge >= 100 then endCharging; }",
        )
        .syntax(),
    );
    assert_eq!(
        nodes_named(&decision, "GuardedTargetSuccession"),
        2,
        "{decision}"
    );
    assert_eq!(
        nodes_named(&decision, "ActionTargetSuccessionMember"),
        2,
        "{decision}"
    );
    // training/16. Conditional Succession/Conditional Succession Example-2.sysml:20 —
    // after a behaviour usage, with no initial node in front of it.
    let conditional = render(
        &parse_accepted(
            "action def A { action focus : Focus { } if focus.image.isWellFocused then shoot; }",
        )
        .syntax(),
    );
    assert_eq!(
        nodes_named(&conditional, "GuardedTargetSuccession"),
        1,
        "{conditional}"
    );
}

#[test]
fn a_guarded_target_succession_owns_what_its_productions_write() {
    let tree = render(&parse_accepted("action def A { first start; if x then b; }").syntax());
    assert_eq!(
        child_kinds(&tree, "ActionTargetSuccessionMember"),
        ["MemberPrefix", "ActionTargetSuccession"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "ActionTargetSuccession"),
        ["GuardedTargetSuccession", "UsageBody"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "GuardedTargetSuccession"),
        [
            "GuardExpressionMember",
            "KwThen",
            "TransitionSuccessionMember"
        ],
        "{tree}"
    );
    // `'if' { kind = 'guard' } OwnedExpression`: the assignment contributes no token.
    assert_eq!(
        child_kinds(&tree, "GuardExpressionMember"),
        ["KwIf", "FeatureReferenceExpression"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "TransitionSuccessionMember"),
        ["TransitionSuccession"],
        "{tree}"
    );
    // A Succession, not a SuccessionAsUsage: its source end is EMPTY where
    // TargetSuccession's is a SourceEnd carrying an optional multiplicity.
    assert_eq!(
        child_kinds(&tree, "TransitionSuccession"),
        ["EmptyEndMember", "ConnectorEndMember"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "EmptyEndMember"),
        ["EmptyFeature"],
        "{tree}"
    );
    assert_eq!(nodes_named(&tree, "TargetSuccession"), 0, "{tree}");
    assert_eq!(nodes_named(&tree, "SourceEnd"), 0, "{tree}");
}

#[test]
fn a_guard_reads_a_whole_expression() {
    // OwnedExpression, not a name: every tier is admitted before the `then`.
    parse_accepted("action def A { first start; if a.b.c == 1 and d then x; }");
    parse_accepted("action def A { first start; if (x + 1) > 2 then x; }");
    parse_accepted("action def A { first start; if not x then y; }");
    // The target is a ConnectorEnd, so it takes a feature chain and the `references` form.
    parse_accepted("action def A { first start; if x then a.b.c; }");
    parse_accepted("action def A { first start; if x then e references a; }");
    // UsageBody may be braced rather than `;`.
    let braced = render(&parse_accepted("action def A { first start; if x then b { } }").syntax());
    assert_eq!(
        nodes_named(&braced, "GuardedTargetSuccession"),
        1,
        "{braced}"
    );
}

#[test]
fn an_if_that_is_not_a_guarded_succession_is_left_alone() {
    // IfNode = ActionNodePrefix 'if' ExpressionParameterMember ActionBodyParameterMember
    // ( 'else' … )? (8.2.2.17.7) — an ActionNode, unimplemented, and it opens on `if`
    // too. What separates the two is the `then` before the body, so an `if` with none is
    // declined at recognition and reported. Held as a file by
    // tests/rejection/if-node-is-not-implemented.sysml.
    let if_node = render(&parse_rejected("action def A { action a; if i < 0 { } }").syntax());
    assert_eq!(
        nodes_named(&if_node, "GuardedTargetSuccession"),
        0,
        "{if_node}"
    );
    // And a calculation body's result expression may be a ConditionalExpression, which
    // opens on `if` as well (KerML 8.2.5.8.1). It has no `then`, so the item loop
    // declines it and the body reads it as the expression it is.
    let conditional = render(&parse_accepted("calc def C { action a; if x ? 1 else 2 }").syntax());
    assert_eq!(
        nodes_named(&conditional, "ConditionalExpression"),
        1,
        "{conditional}"
    );
    assert_eq!(
        nodes_named(&conditional, "ResultExpressionMember"),
        1,
        "{conditional}"
    );
    assert_eq!(
        nodes_named(&conditional, "GuardedTargetSuccession"),
        0,
        "{conditional}"
    );
}

#[test]
fn a_guarded_target_succession_is_a_suffix_and_nothing_else() {
    // The same four rules TargetSuccession's own cases hold, over the guarded form.
    // Held as files by tests/rejection/guarded-target-succession-*.sysml.
    parse_rejected("action def A { if x then b; }");
    parse_rejected("action def A { part p; if x then b; }");
    parse_rejected("part def P { action a; if x then b; }");
    parse_rejected("action def A { first start; if x then b }");
}

// -- one diagnostic per start offset ---------------------------------------------------
//
// Not a grammar rule: a reporting rule, and the reason it belongs beside them is that a
// reader cannot act on four reports about one token. The expectation comes from what the
// text contains, not from what the parser printed: `{ 1 }` is ONE construct this parser
// does not read (a BodyExpression, KerML 8.2.5.8.3), so it is one defect at one position,
// however many nested productions each raise their own failure there. The input wrote
// `new T()` until ConstructorExpression landed; the rule is unchanged, only the construct
// standing in for "not read".

#[test]
fn one_token_is_reported_once_however_many_productions_fail_on_it() {
    // The expression, the parenthesis around it and the enclosing definition each fail at
    // the `{`. Before this rule they reported four times at one offset.
    let parsed = parse_rejected("part def P {\n\t:>> x = ({ 1 });\n}\n");
    let starts: Vec<usize> = parsed
        .errors()
        .iter()
        .map(|d| usize::from(d.range().start()))
        .collect();
    let mut unique = starts.clone();
    unique.dedup();
    assert_eq!(
        starts,
        unique,
        "one report per start offset: {:?}",
        parsed.errors()
    );
    // The FIRST raiser at an offset is the one kept, and it is the innermost: the
    // expression that could not be read, not the definition body that gave up later.
    let first = parsed.errors().first().expect("a diagnostic");
    assert!(
        first.message().contains("expression"),
        "the innermost expectation is the one kept: {first:?}"
    );
}

#[test]
fn a_diagnostic_at_a_new_offset_is_never_dropped() {
    // The rule drops a repeat at ONE offset; it must not swallow the next position, or a
    // file with two defects would report one. Two value parts with no expression, a valid
    // item between them: `attribute ;` alone is NOT one of them — an anonymous
    // AttributeUsage whose Identification is empty is well formed (SysML 8.2.2.6.2).
    let parsed = parse_rejected("part def P { attribute x = ; part q; attribute y = ; }");
    let starts: Vec<usize> = parsed
        .errors()
        .iter()
        .map(|d| usize::from(d.range().start()))
        .collect();
    assert!(
        starts.len() >= 2 && starts[0] != starts[1],
        "two defects, two offsets: {starts:?}"
    );
}

// -- GuardedSuccession, SysML 8.2.2.17.8 ----------------------------------------------
//
// ActionBodyItem = ... | ownedRelationship += GuardedSuccessionMember      (8.2.2.17.1)
// GuardedSuccessionMember : FeatureMembership =
//     MemberPrefix ownedRelatedElement += GuardedSuccession               (8.2.2.17.1)
// GuardedSuccession : TransitionUsage =
//     ( 'succession' UsageDeclaration )?
//     'first' FeatureChainMember GuardExpressionMember
//     'then' TransitionSuccessionMember UsageBody                         (8.2.2.17.8)
// FeatureChainMember : Membership =
//     memberElement = [QualifiedName] | OwnedFeatureChainMember           (8.2.2.17.5)
// OwnedFeatureChainMember : OwningMembership = OwnedFeatureChain          (8.2.2.17.5)
//
// ActionBodyItem's FOURTH and last alternative, and an item in its own right rather than
// a suffix: it writes its own source after `first`, where a target succession leaves the
// source to the item before it. It takes NO ActionTargetSuccessionMember* after it, where
// the second and third alternatives both do (the first, NonBehaviorBodyItem, takes none
// either — what is particular here is a succession with no suffix).

#[test]
fn a_guarded_succession_reads_the_corpus_forms() {
    // vendor/corpus/sysml/src/training/16. Conditional Succession/Conditional Succession
    // Example-1.sysml:21-22 — `first focus` and the guard on the next line.
    let example = render(
        &parse_accepted(
            "action def A { action focus : Focus { } \
             first focus if focus.image.isWellFocused then shoot; }",
        )
        .syntax(),
    );
    assert_eq!(nodes_named(&example, "GuardedSuccession"), 1, "{example}");
    assert_eq!(nodes_named(&example, "InitialNodeMember"), 0, "{example}");
    // vendor/corpus/sysml/src/examples/Simple Tests/DecisionTest.sysml:17-18 — with the
    // optional `succession UsageDeclaration` and a visibility on the member.
    let declared = render(
        &parse_accepted("action def A { public succession S first A1 if x == 0 then A2; }")
            .syntax(),
    );
    assert_eq!(nodes_named(&declared, "GuardedSuccession"), 1, "{declared}");
    assert_eq!(nodes_named(&declared, "UsageDeclaration"), 1, "{declared}");
}

#[test]
fn a_guarded_succession_owns_what_its_productions_write() {
    let tree = render(&parse_accepted("action def A { first a if x then b; }").syntax());
    assert_eq!(
        child_kinds(&tree, "GuardedSuccessionMember"),
        ["MemberPrefix", "GuardedSuccession"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "GuardedSuccession"),
        [
            "KwFirst",
            "FeatureChainMember",
            "GuardExpressionMember",
            "KwThen",
            "TransitionSuccessionMember",
            "UsageBody"
        ],
        "{tree}"
    );
    // The same guard and the same succession the target forms own.
    assert_eq!(
        child_kinds(&tree, "TransitionSuccession"),
        ["EmptyEndMember", "ConnectorEndMember"],
        "{tree}"
    );
    // With the optional declaration, two tokens and a UsageDeclaration come first.
    let declared =
        render(&parse_accepted("action def A { succession S first a if x then b; }").syntax());
    assert_eq!(
        child_kinds(&declared, "GuardedSuccession"),
        [
            "KwSuccession",
            "UsageDeclaration",
            "KwFirst",
            "FeatureChainMember",
            "GuardExpressionMember",
            "KwThen",
            "TransitionSuccessionMember",
            "UsageBody"
        ],
        "{declared}"
    );
}

#[test]
fn a_guarded_successions_source_takes_either_alternative() {
    // FeatureChainMember = memberElement = [QualifiedName] | OwnedFeatureChainMember, and
    // OwnedFeatureChain's `+` means a chain has at least two links — so one name is the
    // reference alternative and never a chain of one.
    let one = render(&parse_accepted("action def A { first a if x then b; }").syntax());
    assert_eq!(
        child_kinds(&one, "FeatureChainMember"),
        ["QualifiedName"],
        "{one}"
    );
    assert_eq!(nodes_named(&one, "OwnedFeatureChainMember"), 0, "{one}");
    // Two or more links take the owned alternative. The corpus does not write one —
    // OwnedFeatureChainMember is a spec_only deviation (follow_spec) — so this case is
    // constructed from the production, which is what that deviation asks for.
    let chained = render(&parse_accepted("action def A { first a.b.c if x then d; }").syntax());
    assert_eq!(
        child_kinds(&chained, "FeatureChainMember"),
        ["OwnedFeatureChainMember"],
        "{chained}"
    );
    assert_eq!(
        child_kinds(&chained, "OwnedFeatureChainMember"),
        ["OwnedFeatureChain"],
        "{chained}"
    );
    // A qualified name is not a chain: `::` stays inside the one member element.
    let qualified = render(&parse_accepted("action def A { first p::a if x then b; }").syntax());
    assert_eq!(
        child_kinds(&qualified, "FeatureChainMember"),
        ["QualifiedName"],
        "{qualified}"
    );
}

#[test]
fn first_tells_the_initial_node_from_the_guarded_succession() {
    // InitialNodeMember = MemberPrefix 'first' [QualifiedName] RelationshipBody
    // (8.2.2.17.1) against GuardedSuccession's `'first' FeatureChainMember
    // GuardExpressionMember` — decided by what follows the name: `;` or `{` is the
    // initial node, `if` is the guarded succession.
    let initial = render(&parse_accepted("action def A { first start; }").syntax());
    assert_eq!(nodes_named(&initial, "InitialNodeMember"), 1, "{initial}");
    assert_eq!(nodes_named(&initial, "GuardedSuccession"), 0, "{initial}");
    // DecisionTest.sysml:20-21 — an initial node, then a guarded TARGET succession on the
    // next line. Both forms in one body, and neither becomes the other.
    let both = render(
        &parse_accepted("action def A { private first A3; if x > 0 then 'test x'; }").syntax(),
    );
    assert_eq!(nodes_named(&both, "InitialNodeMember"), 1, "{both}");
    assert_eq!(nodes_named(&both, "GuardedTargetSuccession"), 1, "{both}");
    assert_eq!(nodes_named(&both, "GuardedSuccession"), 0, "{both}");
    // `first a.b;` is still no production at all: the initial node takes a QualifiedName,
    // and the two that take a chain both go on to a guard or a `then`. Held by
    // tests/rejection/initial-node-member-names-a-qualified-name-not-a-feature-chain.sysml.
    parse_rejected("action def A { first a.b; }");
}

#[test]
fn a_guarded_succession_is_an_item_with_no_suffix() {
    // An item in its own right: no predecessor is needed, unlike every target succession.
    parse_accepted("action def A { first a if x then b; }");
    // And a calculation body reaches ActionBodyItem (8.2.2.19).
    let calc = render(&parse_accepted("calc def C { first a if x then b; y }").syntax());
    assert_eq!(nodes_named(&calc, "GuardedSuccession"), 1, "{calc}");
    assert_eq!(nodes_named(&calc, "ResultExpressionMember"), 1, "{calc}");
    // The fourth alternative carries no ActionTargetSuccessionMember* where the second
    // and third do, so a `then c;` following it is no item. Held as a file by
    // tests/rejection/guarded-succession-takes-no-target-succession.sysml.
    parse_rejected("action def A { first a if x then b; then c; }");
    // Not an item of a definition or package body either.
    parse_rejected("part def P { first a if x then b; }");
    parse_rejected("package P { first a if x then b; }");
    // UsageBody is not optional.
    parse_rejected("action def A { first a if x then b }");
}

#[test]
fn a_guarded_succession_keeps_every_byte() {
    let source =
        "action def A {\n\tsuccession /* d */ S // n\n\t\tfirst a.b\n\t\tif x == 1 then c { }\n}\n";
    assert_eq!(parse_accepted(source).text(), source);
}

// -- SuccessionAsUsage, SysML 8.2.2.13.3 ----------------------------------------------
//
// SuccessionAsUsage =
//     UsagePrefix ( 'succession' UsageDeclaration )?
//     'first' ownedRelationship += ConnectorEndMember
//     'then' ownedRelationship += ConnectorEndMember
//     UsageBody                                                           (8.2.2.13.3)
// NonOccurrenceUsageElement = ... | SuccessionAsUsage | ...               (8.2.2.6.4)
//
// A NON-occurrence usage (7.13.5), so it is owned wherever an attribute is: a
// NonOccurrenceUsageMember in a definition body, a PackageMember in a package, and in an
// action body the NonBehaviorBodyItem alternative, which takes no target-succession
// suffix. It is the third production reachable from an action body that opens on
// `first`, and the only one whose source end is followed directly by `then`.

#[test]
fn a_succession_as_usage_reads_the_corpus_forms() {
    // vendor/corpus/sysml/src/training/28. Individuals/Individuals and Snapshots
    // Example.sysml:22, in the individual part def that holds the two snapshots.
    let in_part = render(
        &parse_accepted(
            "individual part def V :> Vehicle { snapshot part t0; snapshot part t1; \
             first t0 then t1; }",
        )
        .syntax(),
    );
    assert_eq!(nodes_named(&in_part, "SuccessionAsUsage"), 1, "{in_part}");
    // training/14. Action Definitions/Action Succession Example-1.sysml:19, in the
    // action def TakePicture — `first` in an action body is not an initial node here.
    let in_action = render(
        &parse_accepted(
            "action def TakePicture { action focus; action shoot; first focus then shoot; }",
        )
        .syntax(),
    );
    assert_eq!(
        nodes_named(&in_action, "SuccessionAsUsage"),
        1,
        "{in_action}"
    );
    assert_eq!(
        nodes_named(&in_action, "InitialNodeMember"),
        0,
        "{in_action}"
    );
    // vendor/corpus/omg/SimpleVehicleModel.sysml:780 — feature chains at both ends.
    let chained = render(
        &parse_accepted("part def P { first vehicle.doorClosed then driver.driverReady; }")
            .syntax(),
    );
    assert_eq!(nodes_named(&chained, "SuccessionAsUsage"), 1, "{chained}");
    assert_eq!(nodes_named(&chained, "OwnedFeatureChain"), 2, "{chained}");
    // examples/Arrowhead Framework Example/AHFSequences.sysml:100-101 — the `succession`
    // keyword with an EMPTY UsageDeclaration, and the `then` on the next line.
    let keyword = render(
        &parse_accepted(
            "part def P {\n\tsuccession first call_getItems.start\n\tthen returnack.done;\n}",
        )
        .syntax(),
    );
    assert_eq!(nodes_named(&keyword, "SuccessionAsUsage"), 1, "{keyword}");
    assert_eq!(nodes_named(&keyword, "UsageDeclaration"), 1, "{keyword}");
}

#[test]
fn a_succession_as_usage_owns_what_its_production_writes() {
    let tree = render(&parse_accepted("part def P { first a then b; }").syntax());
    assert_eq!(
        child_kinds(&tree, "NonOccurrenceUsageMember"),
        ["MemberPrefix", "SuccessionAsUsage"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "SuccessionAsUsage"),
        [
            "UsagePrefix",
            "KwFirst",
            "ConnectorEndMember",
            "KwThen",
            "ConnectorEndMember",
            "UsageBody"
        ],
        "{tree}"
    );
    // With the declaration, and a UsagePrefix that is not empty.
    let declared = render(
        &parse_accepted("part def P { abstract succession s : HappensBefore first a then b { } }")
            .syntax(),
    );
    assert_eq!(
        child_kinds(&declared, "SuccessionAsUsage"),
        [
            "UsagePrefix",
            "KwSuccession",
            "UsageDeclaration",
            "KwFirst",
            "ConnectorEndMember",
            "KwThen",
            "ConnectorEndMember",
            "UsageBody"
        ],
        "{declared}"
    );
    // A UsageElement, so a package owns one through PackageMember (8.2.2.5.1). Written
    // at the root, which reaches PackageMember the same way, so that the member asked
    // about is the only one in the tree.
    let package = render(&parse_accepted("first a then b;").syntax());
    assert_eq!(
        child_kinds(&package, "PackageMember"),
        ["MemberPrefix", "SuccessionAsUsage"],
        "{package}"
    );
    // ConnectorEnd's `NAME REFERENCES` form names the end (8.2.2.13.1).
    let named =
        render(&parse_accepted("part def P { first e ::> a then f references b; }").syntax());
    assert_eq!(nodes_named(&named, "ConnectorEnd"), 2, "{named}");
}

#[test]
fn first_is_three_productions_told_apart_after_the_source() {
    // InitialNodeMember ends its name in `;` or `{`, GuardedSuccession writes `if`, and
    // SuccessionAsUsage writes `then` (8.2.2.17.1, 8.2.2.17.8, 8.2.2.13.3). All three in
    // one action body, and none becomes another.
    let tree = render(
        &parse_accepted("action def A { first start; first a if x then b; first c then d; }")
            .syntax(),
    );
    assert_eq!(nodes_named(&tree, "InitialNodeMember"), 1, "{tree}");
    assert_eq!(nodes_named(&tree, "GuardedSuccession"), 1, "{tree}");
    assert_eq!(nodes_named(&tree, "SuccessionAsUsage"), 1, "{tree}");
}

#[test]
fn a_succession_as_usage_is_bounded_by_its_rules() {
    // UsageBody is not optional.
    parse_rejected("part def P { first a then b }");
    // Both ends are required, and so is `first`: the declaration alone is no succession.
    parse_rejected("part def P { first a then; }");
    parse_rejected("part def P { succession s; }");
    // UsagePrefix, not OccurrenceUsagePrefix: "the notations for time slices, snapshots
    // and individuals ... do not apply to it" (7.13.5, receipt 2abd302c).
    parse_rejected("part def P { snapshot first a then b; }");
    // NonBehaviorBodyItem, so no ActionTargetSuccessionMember* after it in an action body.
    parse_rejected("action def A { first a then b; then c; }");
    // No SourceSuccessionMember before it either: that prefixes occurrence usages only
    // (8.2.2.6.1, 8.2.2.17.1).
    parse_rejected("part def P { part p; then first a then b; }");
    // OwnedCrossMultiplicityMember, ConnectorEnd's first part, is unimplemented, so a
    // multiplicity on an end is rejected BY ABSENCE — 7.13.5's own example writes it.
    parse_rejected("part def P { first [1] a then b; }");
}

#[test]
fn a_succession_as_usage_keeps_every_byte() {
    let source =
        "part def P {\n\tsuccession /* d */ s // n\n\t\tfirst a.b\n\t\tthen x ::> c { }\n}\n";
    assert_eq!(parse_accepted(source).text(), source);
}

// -- BindingConnectorAsUsage, SysML 8.2.2.13.2 ----------------------------------------
//
// BindingConnectorAsUsage =
//     UsagePrefix ( 'binding' UsageDeclaration )?
//     'bind' ownedRelationship += ConnectorEndMember
//     '=' ownedRelationship += ConnectorEndMember
//     UsageBody                                                           (8.2.2.13.2)
// NonOccurrenceUsageElement = ... | BindingConnectorAsUsage | ...         (8.2.2.6.4)
//
// SuccessionAsUsage's sibling in 8.2.2.13, and owned the same way: "a binding is not a
// kind of occurrence usage" (7.13.3, receipt 6db87b41). Unlike `first`, `bind` and
// `binding` are reserved (8.2.2.1.2) and open no other production, so the keyword alone
// decides and a malformed binding is read and reported rather than skipped.

#[test]
fn a_binding_connector_as_usage_reads_the_corpus_forms() {
    // training/12. Binding Connectors/Binding Connectors Example-1.sysml:15-16, in a part
    // usage's body, both ends feature chains.
    let in_part = render(
        &parse_accepted(
            "part tank : FuelTankAssembly { bind fuelTankPort.fuelSupply = pump.pumpOut; \
             bind fuelTankPort.fuelReturn = tank.fuelIn; }",
        )
        .syntax(),
    );
    assert_eq!(
        nodes_named(&in_part, "BindingConnectorAsUsage"),
        2,
        "{in_part}"
    );
    // training/14. Action Definitions/Action Definition Example.sysml:10, in an action
    // def's body, one end a bare name.
    let in_action =
        render(&parse_accepted("action def TakePicture { bind focus.scene = scene; }").syntax());
    assert_eq!(
        nodes_named(&in_action, "BindingConnectorAsUsage"),
        1,
        "{in_action}"
    );
    // validation/03-Function-based Behavior/3e-Function-based Behavior-item.sysml:49 —
    // quoted names, three links.
    let quoted = render(
        &parse_accepted(
            "part def P { bind 'assemble vehicle'.'assemble engine into vehicle'.assembledVehicle \
             = vehicle; }",
        )
        .syntax(),
    );
    assert_eq!(
        nodes_named(&quoted, "BindingConnectorAsUsage"),
        1,
        "{quoted}"
    );
    // validation/01-Parts Tree/1d-Parts Tree with Reference.sysml:27-32 — a body holding
    // only a comment.
    let bodied = render(
        &parse_accepted(
            "part def P { bind vehicle1_c1.hitchBall = trailerHitch.hitchBall {\n\
             /* a binding connector */\n} }",
        )
        .syntax(),
    );
    assert_eq!(
        nodes_named(&bodied, "BindingConnectorAsUsage"),
        1,
        "{bodied}"
    );
    // 7.13.3's own example (receipt 6db87b41): the `binding` keyword with a name, and
    // examples/Simple Tests/ConnectionTest.sysml:24 with a typed declaration.
    let named = render(
        &parse_accepted(
            "part def Vehicle { binding fuelFlowBinding \
             bind fuelTank.fuelFlowOut = engine.fuelFlowIn; binding ab1 : AB bind a = b; }",
        )
        .syntax(),
    );
    assert_eq!(nodes_named(&named, "BindingConnectorAsUsage"), 2, "{named}");
    assert_eq!(nodes_named(&named, "UsageDeclaration"), 2, "{named}");
}

#[test]
fn a_binding_connector_as_usage_owns_what_its_production_writes() {
    let tree = render(&parse_accepted("part def P { bind a = b; }").syntax());
    assert_eq!(
        child_kinds(&tree, "NonOccurrenceUsageMember"),
        ["MemberPrefix", "BindingConnectorAsUsage"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "BindingConnectorAsUsage"),
        [
            "UsagePrefix",
            "KwBind",
            "ConnectorEndMember",
            "Eq",
            "ConnectorEndMember",
            "UsageBody"
        ],
        "{tree}"
    );
    // With the declaration, and a UsagePrefix that is not empty.
    let declared =
        render(&parse_accepted("part def P { derived binding b : B bind a = c { } }").syntax());
    assert_eq!(
        child_kinds(&declared, "BindingConnectorAsUsage"),
        [
            "UsagePrefix",
            "KwBinding",
            "UsageDeclaration",
            "KwBind",
            "ConnectorEndMember",
            "Eq",
            "ConnectorEndMember",
            "UsageBody"
        ],
        "{declared}"
    );
    // A UsageElement, so a package owns one through PackageMember (8.2.2.5.1).
    let package = render(&parse_accepted("bind a = b;").syntax());
    assert_eq!(
        child_kinds(&package, "PackageMember"),
        ["MemberPrefix", "BindingConnectorAsUsage"],
        "{package}"
    );
    // ConnectorEnd's `NAME REFERENCES` form names the end (8.2.2.13.1).
    let ends = render(&parse_accepted("part def P { bind e ::> a = f references b; }").syntax());
    assert_eq!(nodes_named(&ends, "ConnectorEnd"), 2, "{ends}");
}

#[test]
fn a_binding_connector_as_usage_is_bounded_by_its_rules() {
    // UsageBody is not optional.
    parse_rejected("part def P { bind a = b }");
    // Both ends and the `=` are required.
    parse_rejected("part def P { bind a; }");
    parse_rejected("part def P { bind a = ; }");
    // `bind` is required: the declaration alone is no binding.
    parse_rejected("part def P { binding b; }");
    // UsagePrefix, not OccurrenceUsagePrefix: "the notations for time slices, snapshots
    // and individuals ... do not apply to it" (7.13.3, receipt 6db87b41).
    parse_rejected("part def P { snapshot bind a = b; }");
    parse_rejected("part def P { individual bind a = b; }");
    // NonBehaviorBodyItem, so no ActionTargetSuccessionMember* after it in an action body
    // (8.2.2.17.1).
    parse_rejected("action def A { bind a = b; then c; }");
    // OwnedCrossMultiplicityMember, ConnectorEnd's first part, is unimplemented, so a
    // multiplicity on an end is rejected BY ABSENCE.
    parse_rejected("part def P { bind [1] a = b; }");
}

#[test]
fn a_binding_connector_as_usage_keeps_every_byte() {
    let source = "part def P {\n\tbinding /* d */ b // n\n\t\tbind a.b\n\t\t= x ::> c { }\n}\n";
    assert_eq!(parse_accepted(source).text(), source);
}

// -- AssertConstraintUsage, SysML 8.2.2.20 --------------------------------------------
//
// AssertConstraintUsage =
//     OccurrenceUsagePrefix 'assert' ( isNegated ?= 'not' )?
//     ( ownedRelationship += OwnedReferenceSubsetting FeatureSpecializationPart?
//     | 'constraint' ConstraintUsageDeclaration )
//     CalculationBody                                                     (8.2.2.20)
// BehaviorUsageElement = ... | AssertConstraintUsage | ...                (8.2.2.6.4)
//
// An OCCURRENCE usage, unlike the two bindings and the succession: it takes
// OccurrenceUsagePrefix, and it is owned as an action is — an OccurrenceUsageMember in a
// definition body, a BehaviorUsageMember in an action body, where a `then` may come before
// it and target successions after it (8.2.2.6.1, 8.2.2.17.1).

#[test]
fn an_assert_constraint_usage_reads_the_corpus_forms() {
    // examples/Simple Tests/TextualRepresentationTest.sysml:5-9 — a named assertion whose
    // body is a textual representation, in an item def.
    let named = render(
        &parse_accepted(
            "item def C { attribute x: Real; assert constraint x_constraint {\n\
             rep inOCL language \"ocl\"\n/* self.x > 0.0 */\n} }",
        )
        .syntax(),
    );
    assert_eq!(nodes_named(&named, "AssertConstraintUsage"), 1, "{named}");
    // training/31. Constraints/Constraint Assertions-1.sysml:17,19 — typed, with an `in`
    // parameter bound in the body, in a part def.
    parse_accepted(
        "part def Vehicle { assert constraint massConstraint : MassConstraint { \
         in massLimit = 2500[kg]; } }",
    );
    // training/31. Constraints/Derivation Constraints.sysml:7 — anonymous, its body the
    // result expression alone, in a part USAGE's body.
    parse_accepted(
        "part vehicle1 : Vehicle { attribute totalMass : MassValue; \
         assert constraint {totalMass == chassisMass + engine.mass + transmission.mass} }",
    );
    // validation/15-Properties-Values-Expressions/15_01-Constants.sysml:22-24 — in an
    // attribute usage's body.
    parse_accepted(
        "attribute e: Real { assert constraint { round(e * 1E20) == 271828182845904523536.0 } }",
    );
    // 15_04-Logical Expressions.sysml:18-21 — a conditional expression as the result.
    parse_accepted(
        "part def Vehicle { assert constraint {\n\
         if isHighPerformance? engine istype '6CylEngine'\n\
         else engine istype '4CylEngine'\n} }",
    );
    // 7.20.3's own examples (receipt 44d633db): negated, and by reference with no
    // `constraint` keyword, whose body binds a parameter by redefinition.
    let negated = render(
        &parse_accepted(
            "part testObject { attribute computedMass : MassValue; \
             assert constraint { computedMass >= 0[kg] } \
             assert not constraint { computedMass < 0[kg] } }",
        )
        .syntax(),
    );
    assert_eq!(
        nodes_named(&negated, "AssertConstraintUsage"),
        2,
        "{negated}"
    );
    let referenced = render(
        &parse_accepted(
            "part testObject { attribute computedMass : MassValue; \
             assert not negativeMass { :>> mass = computedMass; } }",
        )
        .syntax(),
    );
    assert_eq!(
        nodes_named(&referenced, "OwnedReferenceSubsetting"),
        1,
        "{referenced}"
    );
}

#[test]
fn an_assert_constraint_usage_owns_what_its_production_writes() {
    let tree = render(&parse_accepted("part def P { assert constraint c { x > 0 } }").syntax());
    assert_eq!(
        child_kinds(&tree, "OccurrenceUsageMember"),
        ["MemberPrefix", "AssertConstraintUsage"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "AssertConstraintUsage"),
        [
            "OccurrenceUsagePrefix",
            "KwAssert",
            "KwConstraint",
            "ConstraintUsageDeclaration",
            "CalculationBody"
        ],
        "{tree}"
    );
    // The first alternative: a reference and its specialization part, no `constraint`.
    let referenced = render(&parse_accepted("part def P { assert not c : C; }").syntax());
    assert_eq!(
        child_kinds(&referenced, "AssertConstraintUsage"),
        [
            "OccurrenceUsagePrefix",
            "KwAssert",
            "KwNot",
            "OwnedReferenceSubsetting",
            "FeatureSpecializationPart",
            "CalculationBody"
        ],
        "{referenced}"
    );
    // OccurrenceUsagePrefix, so the occurrence keywords apply to it — the binding and the
    // succession reject exactly these (7.13.3, 7.13.5).
    parse_accepted("part def P { individual assert constraint c; snapshot assert c; }");
    // A BehaviorUsageElement, so in an action body it is a BehaviorUsageMember, a `then`
    // may precede it and target successions may follow it (8.2.2.17.1).
    let action = render(
        &parse_accepted("action def A { action a; then assert constraint c { x > 0 } then b; }")
            .syntax(),
    );
    assert_eq!(nodes_named(&action, "BehaviorUsageMember"), 2, "{action}");
    assert_eq!(
        nodes_named(&action, "SourceSuccessionMember"),
        1,
        "{action}"
    );
    assert_eq!(
        nodes_named(&action, "ActionTargetSuccessionMember"),
        1,
        "{action}"
    );
}

#[test]
fn an_assert_constraint_usage_is_bounded_by_its_rules() {
    // CalculationBody is not optional.
    parse_rejected("part def P { assert constraint c }");
    // One of the two alternatives is required: a reference, or `constraint`.
    parse_rejected("part def P { assert; }");
    parse_rejected("part def P { assert not; }");
    // The reference alternative takes no declaration: the name after the reference is
    // no FeatureSpecializationPart and no body.
    parse_rejected("part def P { assert c d; }");
    // `assert satisfy` is SatisfyRequirementUsage (8.2.2.21.2), unimplemented, and the
    // recogniser declines it, so it is rejected BY ABSENCE rather than read as an
    // assertion referencing `satisfy` — which is reserved and cannot be a name.
    parse_rejected("part def P { assert satisfy r; }");
    parse_rejected("part def P { assert not satisfy r; }");
}

#[test]
fn an_assert_constraint_usage_keeps_every_byte() {
    let source =
        "part def P {\n\tassert /* n */ not constraint c // d\n\t\t: C {\n\t\tx > 0\n\t}\n}\n";
    assert_eq!(parse_accepted(source).text(), source);
}

// -- CalculationUsage, SysML 8.2.2.19 -------------------------------------------------
//
// CalculationUsage =
//     OccurrenceUsagePrefix 'calc' ActionUsageDeclaration CalculationBody  (8.2.2.19)
// BehaviorUsageElement = ... | CalculationUsage | ...                       (8.2.2.6.4)
//
// "declared as an action definition or usage ... but using the keyword calc instead of
// action", with a CalculationBody where the action has an ActionBody (7.19.2, receipt
// 14c3c04e). Told from CalculationDefinition by the `def` after `calc`, exactly as
// ActionUsage is told from ActionDefinition.

#[test]
fn a_calculation_usage_reads_the_corpus_forms() {
    // examples/Simple Tests/CalculationTest.sysml:22-25 — in a package body, an `in`
    // parameter bound to a sequence and a `return` naming the result.
    let in_package = render(
        &parse_accepted(
            "package CalculationExample { calc ms: MassSum {\n\
             in partMasses = (vehicle.eng.m, vehicle.trans.m);\n\
             return totalMass;\n} }",
        )
        .syntax(),
    );
    assert_eq!(
        nodes_named(&in_package, "CalculationUsage"),
        1,
        "{in_package}"
    );
    // training/30. Calculations/Calculation Usages-1.sysml:19-24 — in an action usage's
    // body, an invocation as an argument.
    let in_action = render(
        &parse_accepted(
            "action straightLineDynamics { calc acc : Acceleration {\n\
             in tp = Power(wheelPower, C_d, C_f, mass, v_in);\n\
             in tm = mass;\n\
             in v = v_in;\n\
             return a;\n} }",
        )
        .syntax(),
    );
    assert_eq!(
        nodes_named(&in_action, "CalculationUsage"),
        1,
        "{in_action}"
    );
    // training/30. Calculations/Calculation Usages-2.sysml:17-19 — untyped, in a part def,
    // with typed parameters.
    parse_accepted(
        "part def VehicleDynamics { calc updateState {\n\
         in delta_t : TimeValue;\n\
         in currState : DynamicState;\n} }",
    );
    // 7.19.2's CalculationDefinition example (receipt 14c3c04e), written as a usage —
    // "declared as an action definition or usage ... but using the keyword calc".
    parse_accepted(
        "calc v {\n\
         in v_i : VelocityValue;\n\
         in a : AccelerationValue;\n\
         in dt : TimeValue;\n\
         return v_f : VelocityValue;\n}",
    );
}

#[test]
fn a_calculation_usage_owns_what_its_production_writes() {
    let tree = render(&parse_accepted("part def P { calc c : C { a + b } }").syntax());
    assert_eq!(
        child_kinds(&tree, "OccurrenceUsageMember"),
        ["MemberPrefix", "CalculationUsage"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "CalculationUsage"),
        [
            "OccurrenceUsagePrefix",
            "KwCalc",
            "ActionUsageDeclaration",
            "CalculationBody"
        ],
        "{tree}"
    );
    // The input the absence rejection held until the commit before this one: a usage,
    // and NOT read as the definition it shares every token with before the `def`.
    let usage = render(&parse_accepted("calc c { a + b }").syntax());
    assert_eq!(nodes_named(&usage, "CalculationUsage"), 1, "{usage}");
    assert_eq!(nodes_named(&usage, "CalculationDefinition"), 0, "{usage}");
    assert_eq!(nodes_named(&usage, "ResultExpressionMember"), 1, "{usage}");
    let definition = render(&parse_accepted("calc def C { a + b }").syntax());
    assert_eq!(
        nodes_named(&definition, "CalculationDefinition"),
        1,
        "{definition}"
    );
    assert_eq!(
        nodes_named(&definition, "CalculationUsage"),
        0,
        "{definition}"
    );
    // OccurrenceUsagePrefix, so the occurrence keywords apply.
    parse_accepted("part def P { individual calc c; snapshot calc d { x } }");
    // A BehaviorUsageElement: a BehaviorUsageMember in an action body, after a `then`
    // and before target successions (8.2.2.17.1).
    let action =
        render(&parse_accepted("action def A { action a; then calc c { x } then b; }").syntax());
    assert_eq!(nodes_named(&action, "BehaviorUsageMember"), 2, "{action}");
    assert_eq!(
        nodes_named(&action, "SourceSuccessionMember"),
        1,
        "{action}"
    );
    assert_eq!(
        nodes_named(&action, "ActionTargetSuccessionMember"),
        1,
        "{action}"
    );
}

#[test]
fn a_calculation_usage_is_bounded_by_its_rules() {
    // CalculationBody is not optional.
    parse_rejected("part def P { calc c }");
    // Its body is a CalculationBody: `CalculationBodyItem* ResultExpressionMember?`, so the
    // result expression is last and no item follows it (8.2.2.19). Parenthesised, so that
    // it can only be the expression.
    parse_rejected("part def P { calc c { (a + b) attribute x; } }");
    // Unclosed.
    parse_rejected("part def P { calc c { a + b }");
}

#[test]
fn a_calculation_usage_keeps_every_byte() {
    let source = "part def P {\n\tcalc /* n */ c // d\n\t\t: C {\n\t\tin x;\n\t\tx + 1\n\t}\n}\n";
    assert_eq!(parse_accepted(source).text(), source);
}

// -- ConstraintUsage, SysML 8.2.2.20 --------------------------------------------------
//
// ConstraintUsage =
//     OccurrenceUsagePrefix 'constraint' ConstraintUsageDeclaration CalculationBody
//                                                                         (8.2.2.20)
// BehaviorUsageElement = ... | ConstraintUsage | ...                      (8.2.2.6.4)
//
// "declared as a kind of occurrence definition or usage ... using the kind keyword
// constraint", with a body "like the body of a calculation definition or usage" (7.20.2,
// receipt 0014441c). AssertConstraintUsage's `constraint` alternative less the `assert`,
// over the same declaration and body; told from ConstraintDefinition by the `def`.

#[test]
fn a_constraint_usage_reads_the_corpus_forms() {
    // training/31. Constraints/Constraints Example-1.sysml:17-20 — typed, in a part def,
    // its parameters bound in the body.
    let in_part = render(
        &parse_accepted(
            "part def Vehicle { constraint massConstraint : MassConstraint {\n\
             in partMasses = (chassisMass, engine.mass, transmission.mass);\n\
             in massLimit = 2500[kg];\n} }",
        )
        .syntax(),
    );
    assert_eq!(nodes_named(&in_part, "ConstraintUsage"), 1, "{in_part}");
    // validation/15-Properties-Values-Expressions/15_03-Value Expression.sysml:27 — the
    // body the result expression alone, with a quantity.
    parse_accepted("part def Tire { constraint hasLegalProfileDepth {profileDepth >= 3.5 [mm]} }");
    // examples/Simple Tests/ConstraintTest.sysml:88-89 — keywordless parameters then the
    // result expression, and the assertion that references it by name.
    let asserted = render(
        &parse_accepted(
            "package ConstraintTest { constraint massLimitation { mass : MassValue; \
             massLimit : MassValue; mass < massLimit } \
             assert not massLimitation { :>> mass = vehicle3.mass; \
             :>> massLimit = vehicle4.mass; } }",
        )
        .syntax(),
    );
    assert_eq!(nodes_named(&asserted, "ConstraintUsage"), 1, "{asserted}");
    assert_eq!(
        nodes_named(&asserted, "AssertConstraintUsage"),
        1,
        "{asserted}"
    );
    // 7.20.2's own example (receipt 0014441c).
    parse_accepted(
        "part def Vehicle { part fuelTank : FuelTank; \
         constraint isFull : IsFull { in tank = fuelTank; } }",
    );
}

#[test]
fn a_constraint_usage_owns_what_its_production_writes() {
    let tree = render(&parse_accepted("part def P { constraint c : C { a <= b } }").syntax());
    assert_eq!(
        child_kinds(&tree, "OccurrenceUsageMember"),
        ["MemberPrefix", "ConstraintUsage"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "ConstraintUsage"),
        [
            "OccurrenceUsagePrefix",
            "KwConstraint",
            "ConstraintUsageDeclaration",
            "CalculationBody"
        ],
        "{tree}"
    );
    // The two inputs the absence rejections held until the commit before this one: usages,
    // and NOT read as the definition they share every token with before the `def`.
    for source in [
        "constraint c { a <= b }",
        "part def P { constraint c { x > 0 } }",
    ] {
        let usage = render(&parse_accepted(source).syntax());
        assert_eq!(nodes_named(&usage, "ConstraintUsage"), 1, "{usage}");
        assert_eq!(nodes_named(&usage, "ConstraintDefinition"), 0, "{usage}");
    }
    let definition = render(&parse_accepted("constraint def C { a <= b }").syntax());
    assert_eq!(
        nodes_named(&definition, "ConstraintUsage"),
        0,
        "{definition}"
    );
    // Nor as the assertion: `assert constraint` is the other production.
    let assertion = render(&parse_accepted("part def P { assert constraint c; }").syntax());
    assert_eq!(nodes_named(&assertion, "ConstraintUsage"), 0, "{assertion}");
    // OccurrenceUsagePrefix, and a BehaviorUsageElement in an action body (8.2.2.17.1).
    parse_accepted("part def P { individual constraint c; snapshot constraint d { x } }");
    let action = render(
        &parse_accepted("action def A { action a; then constraint c { x } then b; }").syntax(),
    );
    assert_eq!(
        nodes_named(&action, "SourceSuccessionMember"),
        1,
        "{action}"
    );
    assert_eq!(
        nodes_named(&action, "ActionTargetSuccessionMember"),
        1,
        "{action}"
    );
    // And an item of a calculation body, through the one list at_result_expression asks.
    let nested = render(&parse_accepted("constraint def C { constraint d { y } x }").syntax());
    assert_eq!(nodes_named(&nested, "ConstraintUsage"), 1, "{nested}");
}

#[test]
fn a_constraint_usage_is_bounded_by_its_rules() {
    // CalculationBody is not optional.
    parse_rejected("part def P { constraint c }");
    // The result expression is last (8.2.2.19).
    parse_rejected("part def P { constraint c { (a <= b) attribute x; } }");
    // Unclosed.
    parse_rejected("part def P { constraint c { a <= b }");
}

#[test]
fn a_constraint_usage_keeps_every_byte() {
    let source =
        "part def P {\n\tconstraint /* n */ c // d\n\t\t: C {\n\t\tin x;\n\t\tx > 0\n\t}\n}\n";
    assert_eq!(parse_accepted(source).text(), source);
}

// -- RequirementUsage, SysML 8.2.2.21.2 -----------------------------------------------
//
// RequirementUsage =
//     OccurrenceUsagePrefix 'requirement' ConstraintUsageDeclaration RequirementBody
//                                                                       (8.2.2.21.2)
// BehaviorUsageElement = ... | RequirementUsage | ...                     (8.2.2.6.4)
//
// "declared as a kind of constraint definition or usage ... using the kind keyword
// requirement" (7.21.2, receipt 021b9219), so it takes ConstraintUsageDeclaration, and the
// RequirementBody RequirementDefinition reads. Told from RequirementDefinition by the `def`.

#[test]
fn a_requirement_usage_reads_the_corpus_forms() {
    // training/32. Requirements/Requirement Usages.sysml:5-13 — a short name, which "is
    // also considered to be its requirement ID" (7.21.2), a subject, a redefined attribute
    // and an assumed constraint.
    let usage = render(
        &parse_accepted(
            "package 'Requirement Usages' {\n\
             requirement <'1.1'> fullVehicleMassLimit : VehicleMassLimitationRequirement {\n\
             subject vehicle : Vehicle;\n\
             attribute :>> massReqd = 2000[kg];\n\
             assume constraint {\n\
             doc /* Full tank is full. */\n\
             vehicle.fuelMass == vehicle.fuelFullMass\n\
             }\n} }",
        )
        .syntax(),
    );
    assert_eq!(nodes_named(&usage, "RequirementUsage"), 1, "{usage}");
    assert_eq!(nodes_named(&usage, "SubjectMember"), 1, "{usage}");
    // examples/Requirements Examples/HSUVRequirements.sysml:4-9 — composite
    // sub-requirements, requirement usages nested in a requirement usage's body.
    let nested = render(
        &parse_accepted(
            "requirement <'UR1.1'> Load: FunctionalRequirementCheck {\n\
             // The following requirements are composite sub-requirements.\n\
             requirement Passengers;\n\
             requirement FuelCapacity;\n\
             requirement Cargo;\n}",
        )
        .syntax(),
    );
    assert_eq!(nodes_named(&nested, "RequirementUsage"), 4, "{nested}");
    // training/32. Requirements/Requirement Groups.sysml:19-31 — a group whose subject
    // each nested requirement binds by value.
    let group = render(
        &parse_accepted(
            "requirement engineSpecification {\n\
             doc /* Engine power requirements group */\n\
             subject engine : Engine;\n\
             requirement drivePowerInterface : DrivePowerInterface {\n\
             subject = engine.clutchPort;\n\
             }\n\
             requirement torqueGeneration : TorqueGeneration {\n\
             subject = engine.generateTorque;\n\
             }\n}",
        )
        .syntax(),
    );
    assert_eq!(nodes_named(&group, "RequirementUsage"), 3, "{group}");
    assert_eq!(nodes_named(&group, "SubjectMember"), 3, "{group}");
}

#[test]
fn a_requirement_usage_owns_what_its_production_writes() {
    let tree = render(
        &parse_accepted("part def P { requirement r : R { require constraint { x } } }").syntax(),
    );
    assert_eq!(
        child_kinds(&tree, "OccurrenceUsageMember"),
        ["MemberPrefix", "RequirementUsage"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "RequirementUsage"),
        [
            "OccurrenceUsagePrefix",
            "KwRequirement",
            "ConstraintUsageDeclaration",
            "RequirementBody"
        ],
        "{tree}"
    );
    // The input the absence rejection held until the commit before this one: a usage, and
    // NOT read as the definition it shares every token with before the `def`.
    let usage = render(&parse_accepted("requirement r;").syntax());
    assert_eq!(nodes_named(&usage, "RequirementUsage"), 1, "{usage}");
    assert_eq!(nodes_named(&usage, "RequirementDefinition"), 0, "{usage}");
    let definition = render(&parse_accepted("requirement def R;").syntax());
    assert_eq!(
        nodes_named(&definition, "RequirementUsage"),
        0,
        "{definition}"
    );
    // Nor as a `require`d constraint in a requirement body, which is its own member.
    let required = render(&parse_accepted("requirement def R { require c; }").syntax());
    assert_eq!(nodes_named(&required, "RequirementUsage"), 0, "{required}");
    // OccurrenceUsagePrefix, and a BehaviorUsageElement in an action body (8.2.2.17.1)
    // and a calculation body, through the one list.
    parse_accepted("part def P { individual requirement r; snapshot requirement s; }");
    let action =
        render(&parse_accepted("action def A { action a; then requirement r; then b; }").syntax());
    assert_eq!(
        nodes_named(&action, "SourceSuccessionMember"),
        1,
        "{action}"
    );
    let nested = render(&parse_accepted("constraint def C { requirement r; x }").syntax());
    assert_eq!(nodes_named(&nested, "RequirementUsage"), 1, "{nested}");
}

#[test]
fn a_requirement_usage_is_bounded_by_its_rules() {
    // RequirementBody is not optional.
    parse_rejected("part def P { requirement r }");
    // A RequirementBody is not a CalculationBody: it ends in no result expression
    // (8.2.2.21.1), which is the difference `requirement` makes over `constraint`.
    parse_rejected("part def P { requirement r { a <= b } }");
    // Unclosed.
    parse_rejected("part def P { requirement r { subject s;");
}

#[test]
fn a_requirement_usage_keeps_every_byte() {
    let source =
        "part def P {\n\trequirement /* n */ <'1'> r // d\n\t\t: R {\n\t\tsubject s;\n\t}\n}\n";
    assert_eq!(parse_accepted(source).text(), source);
}

// -- EnumerationDefinition, SysML 8.2.2.8 ---------------------------------------------
//
// EnumerationDefinition  = DefinitionExtensionKeyword* 'enum' 'def'
//                          DefinitionDeclaration EnumerationBody
// EnumerationBody        = ';' | '{' ( AnnotatingMember | EnumerationUsageMember )* '}'
// EnumerationUsageMember = MemberPrefix EnumeratedValue
// EnumeratedValue        = 'enum'? Usage                                  (8.2.2.8)
// AnnotatingMember       = MemberPrefix AnnotatingElement                 (8.2.2.4.1)
//
// "Any owned members declared in the body of an enumeration definition must be
// enumeration usages ... the declaration of an enumerated value may omit the enum
// keyword" (7.8.2, receipt a2406cd5).

#[test]
fn an_enumeration_definition_reads_the_corpus_forms() {
    // training/06. Enumeration Definitions/Enumeration Definitions-1.sysml:4-8.
    let keyworded = render(
        &parse_accepted(
            "enum def TrafficLightColor {\n\tenum green;\n\tenum yellow;\n\tenum red;\n}",
        )
        .syntax(),
    );
    assert_eq!(nodes_named(&keyworded, "EnumeratedValue"), 3, "{keyworded}");
    // examples/Simple Tests/EnumerationTest.sysml:30-36 — keywordless values, then a doc
    // comment, which is an AnnotatingMember.
    let bare = render(
        &parse_accepted(
            "enum def E1 { a; b; c;\n\tdoc\n\t/*\n\t * The \"enum\" keyword is optional.\n\t */\n}",
        )
        .syntax(),
    );
    assert_eq!(nodes_named(&bare, "EnumeratedValue"), 3, "{bare}");
    assert_eq!(nodes_named(&bare, "AnnotatingMember"), 1, "{bare}");
    // examples/Simple Tests/EnumerationTest.sysml:38 — the empty body.
    parse_accepted("enum def E2;");
    // training/06. Enumeration Definitions/Enumeration Definitions-2.sysml:10-14 and
    // 25-31 — a value with a body of redefinitions, and values bound to numbers, each
    // definition specializing an attribute definition.
    parse_accepted(
        "enum def ClassificationKind specializes ClassificationLevel {\n\
         unclassified {\n\
         :>> code = \"uncl\";\n\
         :>> color = TrafficLightColor::green;\n\
         }\n}",
    );
    parse_accepted("enum def GradePoints :> Real { A = 4.0; B = 3.0; C = 2.0; D = 1.0; F = 0.0; }");
    // validation/15-Properties-Values-Expressions/15_10-Primitive Data Types.sysml:82-86 —
    // values bound to quantities.
    parse_accepted(
        "enum def DiameterChoice :> Diameter { small = 60 [SI::mm]; medium = 70 [SI::mm]; }",
    );
    // examples/Simple Tests/EnumerationTest.sysml:47-51 — values with NO declaration,
    // only a ValuePart: Usage's declaration is optional in full (8.2.2.6.2).
    let valued = render(
        &parse_accepted("enum def SizeChoice :> Size {\n\t= 60.0;\n\t= 70.0;\n\t= 80.0;\n}")
            .syntax(),
    );
    assert_eq!(nodes_named(&valued, "EnumeratedValue"), 3, "{valued}");
    // And with no completion but the UsageBody, which the grammar admits as well.
    parse_accepted("enum def E { ; { } }");
    // 7.8.2's own example (receipt a2406cd5).
    parse_accepted(
        "enum def ConditionColor { red; green; yellow; } \
         enum def RiskLevel :> ConditionLevel { enum low { :>> color = ConditionColor::green; } }",
    );
}

#[test]
fn an_enumeration_definition_owns_what_its_production_writes() {
    let tree = render(&parse_accepted("enum def E :> A { doc /* d */ enum a = 1; }").syntax());
    assert_eq!(
        child_kinds(&tree, "EnumerationDefinition"),
        [
            "KwEnum",
            "KwDef",
            "DefinitionDeclaration",
            "EnumerationBody"
        ],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "EnumerationUsageMember"),
        ["MemberPrefix", "EnumeratedValue"],
        "{tree}"
    );
    assert_eq!(nodes_named(&tree, "AnnotatingMember"), 1, "{tree}");
    // The input the absence rejection held until the commit before this one.
    let empty = render(&parse_accepted("enum def Color;").syntax());
    assert_eq!(nodes_named(&empty, "EnumerationDefinition"), 1, "{empty}");
    // A DefinitionElement, so it nests where one may, and is an item of a calculation
    // body through the one list.
    parse_accepted("part def P { enum def E { a; } }");
    parse_accepted("constraint def C { enum def E { a; } x }");
    // An EnumerationUsage outside an enumeration body is still the SIMPLE_USAGE it was.
    let usage = render(&parse_accepted("part def P { enum e : E; }").syntax());
    assert_eq!(nodes_named(&usage, "EnumerationUsage"), 1, "{usage}");
    assert_eq!(nodes_named(&usage, "EnumeratedValue"), 0, "{usage}");
}

#[test]
fn an_enumeration_definition_is_bounded_by_its_rules() {
    // EnumerationBody is not optional.
    parse_rejected("enum def E");
    // No DefinitionPrefix: "The keywords abstract and variation may not be used with an
    // enumeration definition" (7.8.2).
    parse_rejected("abstract enum def E;");
    parse_rejected("variation enum def E;");
    // EnumeratedValue is `'enum'? Usage`, with no UsagePrefix: an enumerated value "may
    // not include ... any direction keywords, abstract, derived, etc." (7.8.2).
    parse_rejected("enum def E { in a; }");
    parse_rejected("enum def E { derived a; }");
    // "Any owned members declared in the body of an enumeration definition must be
    // enumeration usages" (7.8.2): no other member is an item of EnumerationBody.
    parse_rejected("enum def E { part p; }");
    parse_rejected("enum def E { attribute a; }");
    parse_rejected("enum def E { enum def F; }");
    // Unclosed.
    parse_rejected("enum def E { a;");
}

#[test]
fn an_enumeration_definition_keeps_every_byte() {
    let source = "enum /* n */ def E // d\n\t:> A {\n\tdoc /* v */\n\tenum a = 1;\n\tb { }\n}\n";
    assert_eq!(parse_accepted(source).text(), source);
}

// -- VariantUsageMember, SysML 8.2.2.6.1 ----------------------------------------------
//
// VariantUsageMember : VariantMembership =
//     MemberPrefix 'variant' ownedVariantUsage = VariantUsageElement      (8.2.2.6.1)
// VariantUsageElement = VariantReference | ReferenceUsage | AttributeUsage
//                     | BindingConnectorAsUsage | SuccessionAsUsage | OccurrenceUsage
//                     | ... | PartUsage | PortUsage | FlowUsage | BehaviorUsageElement
//                                                                         (8.2.2.6.4)
// VariantReference : ReferenceUsage =
//     ownedRelationship += OwnedReferenceSubsetting
//     FeatureSpecialization* UsageBody                                    (8.2.2.6.3)
//
// An alternative of DefinitionBodyItem (8.2.2.6.1) and NonBehaviorBodyItem (8.2.2.17.1),
// and so of every definition, usage, requirement, action and calculation body; NOT of
// PackageBodyElement (8.2.2.5.1). "Variant usages may only be declared within a
// variation" (7.6.7, receipt 5a7843af) is validateVariantMembershipOwningNamespace
// (8.3.6.5, receipt 49805baa), a constraint on the membership's owner and not grammar,
// so a `variant` in a non-variation body parses (ADR-0002).

#[test]
fn a_variant_usage_member_reads_the_clause_examples() {
    // 7.6.7 (receipt 5a7843af): variant usages with a kind keyword in a variation
    // definition, and bare references to separately declared usages in a variation usage.
    let tree = render(
        &parse_accepted(
            "variation part def TransmissionChoices :> Transmission {\n\
             \tvariant part manual : ManualTransmission;\n\
             \tvariant part automatic : AutomaticTransmission;\n\
             }\n\
             part smallEngine : FourCylinderEngine;\n\
             part bigEngine : SixCylinderEngine;\n\
             part def Vehicle {\n\
             \tvariation part engine : Engine {\n\
             \t\tvariant smallEngine;\n\
             \t\tvariant bigEngine;\n\
             \t}\n\
             }",
        )
        .syntax(),
    );
    assert_eq!(nodes_named(&tree, "VariantUsageMember"), 4, "{tree}");
    assert_eq!(nodes_named(&tree, "VariantReference"), 2, "{tree}");
}

#[test]
fn a_variant_usage_member_reads_the_corpus_forms() {
    // training/36. Variability/Variation Definitions.sysml:25-33 — attribute variants
    // bound to values, and quoted-name references.
    let defs = render(
        &parse_accepted(
            "variation attribute def DiameterChoices :> Diameter {\n\
             \tvariant attribute diameterSmall = 70[mm];\n\
             \tvariant attribute diameterLarge = 100[mm];\n\
             }\n\
             variation part def EngineChoices :> Engine {\n\
             \tvariant '4cylEngine';\n\
             \tvariant '6cylEngine';\n\
             }",
        )
        .syntax(),
    );
    assert_eq!(nodes_named(&defs, "VariantUsageMember"), 4, "{defs}");
    assert_eq!(nodes_named(&defs, "AttributeUsage"), 2, "{defs}");
    assert_eq!(nodes_named(&defs, "VariantReference"), 2, "{defs}");
    // examples/Variability Examples/VehicleVariabilityModel.sysml:78-82 — a reference
    // with a UsageBody, and variants nested in it; :114-115, variants with a multiplicity.
    parse_accepted(
        "variation part def EngineChoices :> Engine {\n\
         \tvariant '6cylEngine' {\n\
         \t\tvariation port :>> autoPort {\n\
         \t\t\tvariant port autoPort1;\n\
         \t\t\tvariant port autoPort2;\n\
         \t\t}\n\
         \t}\n\
         }\n\
         part def V { variation part :>> sunroof {\n\
         \tvariant part withSunroof[1];\n\
         \tvariant part withoutSunroof[0];\n\
         } }",
    );
    // VehicleVariabilityModel.sysml:125-129 — in an action body, which reaches the
    // member through NonBehaviorBodyItem (8.2.2.17.1).
    parse_accepted(
        "action providePowerFamily : ProvidePower {\n\
         \tvariation action generateTorque : GenerateTorque {\n\
         \t\tvariant generateTorque4Cyl;\n\
         \t\tvariant generateTorque6Cyl;\n\
         \t}\n\
         }",
    );
    // validation/07-Variant Configuration/7a1-Variant Configuration - General Concept-a
    // .sysml:14-17 and examples/Simple Tests/VariabilityTest.sysml:23-26 — behaviour
    // usages as variants.
    parse_accepted(
        "part part5 { variation perform action doXorY { variant perform doX; variant perform doY; } }",
    );
    parse_accepted("variation action def A { variant action a1; variant action a2; }");
}

#[test]
fn a_variant_usage_member_owns_what_its_production_writes() {
    let tree = render(&parse_accepted("part def P { private variant part p : Q; }").syntax());
    assert_eq!(
        child_kinds(&tree, "VariantUsageMember"),
        ["MemberPrefix", "KwVariant", "PartUsage"],
        "{tree}"
    );
    let reference = render(&parse_accepted("part def P { variant a::b :> c { } }").syntax());
    assert_eq!(
        child_kinds(&reference, "VariantUsageMember"),
        ["MemberPrefix", "KwVariant", "VariantReference"],
        "{reference}"
    );
    assert_eq!(
        child_kinds(&reference, "VariantReference"),
        ["OwnedReferenceSubsetting", "Subsettings", "UsageBody"],
        "{reference}"
    );
    // A VariantMembership is not a FeatureMembership (8.4.2.3, receipt 4ad35baf): the
    // variant is owned through this node alone, not through the body's usage member.
    assert_eq!(nodes_named(&tree, "OccurrenceUsageMember"), 0, "{tree}");
    // Every body family that reaches DefinitionBodyItem or NonBehaviorBodyItem: a
    // requirement body (8.2.2.21.1) and a calculation body, where the member must be
    // read as an item and not as the start of the result expression (8.2.2.19).
    parse_accepted("requirement def R { variant r1; }");
    let calc = render(&parse_accepted("constraint def C { variant a; a }").syntax());
    assert_eq!(nodes_named(&calc, "VariantUsageMember"), 1, "{calc}");
    assert_eq!(nodes_named(&calc, "ResultExpressionMember"), 1, "{calc}");
}

#[test]
fn a_variant_usage_member_is_bounded_by_its_rules() {
    // Not a PackageBodyElement (8.2.2.5.1): not at the root, not in a package body.
    parse_rejected("variant part p;");
    parse_rejected("package P { variant part p; }");
    // EnumerationUsage is not a VariantUsageElement (8.2.2.6.4), nor is
    // DefaultReferenceUsage: `variant x;` is a VariantReference, which has no ValuePart,
    // no Identification and no bare specialization (8.2.2.6.3).
    parse_rejected("variation attribute def A { variant enum e; }");
    parse_rejected("part def P { variant x = 1; }");
    parse_rejected("part def P { variant :>> x; }");
    parse_rejected("part def P { variant <s> x; }");
    // ExtendedUsage is not a VariantUsageElement either. Rejected today because it is
    // unimplemented; this holds the rule for the day `usage_element_of_class` reads it.
    parse_rejected("part def P { variant #M x; }");
    // VariantReference writes no MultiplicityPart, though 7.6.7 (receipt 5a7843af) says a
    // variant reference "may also optionally further constrain the variant usage by
    // including a multiplicity". The grammar is followed; the prose conflict is recorded
    // at `variant_reference`.
    parse_rejected("part def P { variant x[1]; }");
    // The element is not optional, and there is one `variant`.
    parse_rejected("part def P { variant; }");
    parse_rejected("part def P { variant variant part p; }");
    // SourceSuccessionMember prefixes an occurrence usage member, never this one
    // (8.2.2.6.1).
    parse_rejected("part def P { then variant part p; }");
    // An enumerated value "may not include the keyword variant" (7.8.2).
    parse_rejected("enum def E { variant a; }");
    // Unclosed.
    parse_rejected("part def P { variant x {");
}

#[test]
fn a_variant_usage_member_keeps_every_byte() {
    let source =
        "part def P {\n\tprivate /* v */ variant // d\n\t\tpart p : Q;\n\tvariant a . b { }\n}\n";
    assert_eq!(parse_accepted(source).text(), source);
}

// -- Dependency, SysML 8.2.2.3 --------------------------------------------------------
//
// Dependency = PrefixMetadataAnnotation* 'dependency' DependencyDeclaration
//              RelationshipBody
// DependencyDeclaration = ( Identification 'from' )?
//     client += [QualifiedName] ( ',' client += [QualifiedName] )* 'to'
//     supplier += [QualifiedName] ( ',' supplier += [QualifiedName] )*       (8.2.2.3)
// RelationshipBody = ';' | '{' OwnedAnnotation* '}'                          (8.2.2.2)
//
// A DefinitionElement (8.2.2.5.2), so a package member and a definition member alike.
// "If no short name or name is given for the dependency, then the keyword from may be
// omitted" (7.3.2, receipt 65989bd2): without `from`, the first name is a client.

#[test]
fn a_dependency_reads_the_corpus_forms() {
    // examples/Simple Tests/DependencyTest.sysml:11-18 — named with `from`, unnamed with
    // `from`, and with no `from`, where `z` is the client and not the name.
    let tree = render(
        &parse_accepted(
            "package P {\n\
             \tdependency Use from 'Application Layer' to 'Service Layer';\n\
             \tdependency from 'Service Layer' to 'Data Layer';\n\
             \tdependency z to x, y;\n\
             }",
        )
        .syntax(),
    );
    assert_eq!(nodes_named(&tree, "Dependency"), 3, "{tree}");
    assert_eq!(nodes_named(&tree, "DependencyDeclaration"), 3, "{tree}");
    // Two of the three take the `( Identification 'from' )?` group.
    let froms = tree
        .lines()
        .filter(|l| l.trim_start().starts_with("KwFrom "))
        .count();
    assert_eq!(froms, 2, "{tree}");
    // training/37. Dependencies/Dependency Example.sysml:22-26 — qualified clients and
    // suppliers, and a declaration over three lines.
    parse_accepted(
        "dependency from 'System Assembly'::'Computer Subsystem' to 'Software Design';\n\
         dependency Schemata \n\
         \tfrom 'System Assembly'::'Storage Subsystem' \n\
         \tto 'Software Design'::MessageSchema, 'Software Design'::DataSchema;",
    );
    // 7.3.2's own examples (receipt 65989bd2): a body of annotating elements.
    parse_accepted(
        "dependency 'Service Layer'\n\
         to 'Data Layer', 'External Interface Layer' {\n\
         /* 'Service Layer' is the client of this dependency,\n\
         * not its name. */\n\
         }",
    );
    // A DefinitionElement, so a member of a definition body (8.2.2.6.1) and, through
    // NonBehaviorBodyItem's DefinitionMember, of an action body (8.2.2.17.1).
    parse_accepted("part def D { dependency a to b; }");
    parse_accepted("action def A { dependency a to b; }");
}

#[test]
fn a_dependency_owns_what_its_production_writes() {
    let tree = render(&parse_accepted("dependency <u> Use from a, b to c::d;").syntax());
    assert_eq!(
        child_kinds(&tree, "Dependency"),
        ["KwDependency", "DependencyDeclaration", "RelationshipBody"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "DependencyDeclaration"),
        [
            "Identification",
            "KwFrom",
            "QualifiedName",
            "Comma",
            "QualifiedName",
            "KwTo",
            "QualifiedName"
        ],
        "{tree}"
    );
    // Without `from` there is no Identification: the `( Identification 'from' )?` group
    // was not taken.
    let bare = render(&parse_accepted("dependency a to b;").syntax());
    assert_eq!(
        child_kinds(&bare, "DependencyDeclaration"),
        ["QualifiedName", "KwTo", "QualifiedName"],
        "{bare}"
    );
    let member = render(&parse_accepted("part def D { dependency a to b; }").syntax());
    assert_eq!(nodes_named(&member, "DefinitionMember"), 1, "{member}");
}

#[test]
fn a_dependency_is_bounded_by_its_rules() {
    // A name needs its `from` (7.3.2): here `a` could only be a second name.
    parse_rejected("dependency Use a to b;");
    parse_rejected("dependency <u> a to b;");
    // client and supplier are each 1..* (KerML 8.3.2.2.2, receipt ec1e3424), and `to` is
    // not optional.
    parse_rejected("dependency from to b;");
    parse_rejected("dependency to b;");
    parse_rejected("dependency a to;");
    parse_rejected("dependency a;");
    parse_rejected("dependency a to b, ;");
    // [QualifiedName], not a feature chain.
    parse_rejected("dependency a to b.c;");
    // SysML's RelationshipBody owns annotations only (8.2.2.2).
    parse_rejected("dependency a to b { part p; }");
    // RelationshipBody is not optional.
    parse_rejected("dependency a to b");
}

#[test]
fn a_dependency_keeps_every_byte() {
    let source = "dependency /* n */ Use // d\n\tfrom a ,b\n\tto c::d {\n\t/* why */\n}\n";
    assert_eq!(parse_accepted(source).text(), source);
}

// -- ConjugatedPortTyping, SysML 8.2.2.12 ---------------------------------------------
//
// FeatureTyping        = OwnedFeatureTyping | ConjugatedPortTyping          (8.2.2.6.5)
// ConjugatedPortTyping = '~' originalPortDefinition = ~[QualifiedName]      (8.2.2.12)
//
// "port p : ~P; is equivalent to port p : P::'~P';" (7.12.3, receipt f0b805cf). The `~`
// is not part of the name, so it sits outside the quotes of an unrestricted one. Which
// element `~[QualifiedName]` resolves to is sv2-resolve's (8.2.2.12, Note 2).

#[test]
fn a_conjugated_port_typing_reads_the_corpus_forms() {
    // training/10. Ports/Port Conjugation Example.sysml:18.
    let tree = render(&parse_accepted("part def E { port engineFuelPort : ~FuelPort; }").syntax());
    assert_eq!(nodes_named(&tree, "ConjugatedPortTyping"), 1, "{tree}");
    assert_eq!(nodes_named(&tree, "OwnedFeatureTyping"), 0, "{tree}");
    // examples/Room Model/RoomModel.sysml:16 — no space after the colon.
    parse_accepted("part def H { port hallExit_to_Classroom: ~EntryWay_to_Classroom; }");
    // 7.12.3's own examples (receipt f0b805cf): the shorthand, the actual name it stands
    // for, and an unrestricted name with the `~` outside its quotes.
    parse_accepted("port p : ~P;");
    parse_accepted("port p : P::'~P';");
    parse_accepted("port p1 : ~'P-1';");
    // A FeatureTyping, so after a comma too (Typings, 8.2.2.6.5), and after `defined by`.
    parse_accepted("port p : ~P, Q;");
    parse_accepted("port p defined by ~A::B::C;");
}

#[test]
fn a_conjugated_port_typing_owns_what_its_production_writes() {
    let tree = render(&parse_accepted("port p : ~A::B;").syntax());
    assert_eq!(
        child_kinds(&tree, "FeatureTyping"),
        ["ConjugatedPortTyping"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "ConjugatedPortTyping"),
        ["Tilde", "QualifiedName"],
        "{tree}"
    );
    // The grammar writes it in FeatureTyping, which every usage typed through Typings or
    // TypedBy reaches; that the type be a ConjugatedPortDefinition is the metaclass's
    // (8.3.12.3), not the parser's.
    parse_accepted("part x : ~P;");
}

#[test]
fn a_conjugated_port_typing_is_bounded_by_its_rules() {
    // ~[QualifiedName]: a name, and no feature chain.
    parse_rejected("port p : ~a.b;");
    parse_rejected("port p : ~;");
    parse_rejected("port p : ~~P;");
    // Only a typing: no subsetting, redefinition or subclassification is conjugated.
    parse_rejected("port p :> ~q;");
    parse_rejected("port p :>> ~q;");
    parse_rejected("port def Q :> ~P;");
}

#[test]
fn a_conjugated_port_typing_keeps_every_byte() {
    let source = "port p : ~ /* c */ 'P-1' , ~ Q;\n";
    assert_eq!(parse_accepted(source).text(), source);
}

// -- States and transitions, SysML 8.2.2.18 -------------------------------------------
//
// StateDefinition  = OccurrenceDefinitionPrefix 'state' 'def'
//                    DefinitionDeclaration StateDefBody                     (8.2.2.18.1)
// StateDefBody     = ';' | 'parallel'? '{' StateBodyItem* '}'
// StateBodyItem    = NonBehaviorBodyItem
//                  | SourceSuccessionMember? BehaviorUsageMember
//                    TargetTransitionUsageMember*
//                  | TransitionUsageMember
//                  | EntryActionMember EntryTransitionMember*
//                  | DoActionMember | ExitActionMember
// StateUsage       = OccurrenceUsagePrefix 'state' ActionUsageDeclaration
//                    StateUsageBody                                         (8.2.2.18.2)
// TransitionUsage  = 'transition' ( UsageDeclaration 'first' )?
//                    FeatureChainMember EmptyParameterMember
//                    ( EmptyParameterMember TriggerActionMember )?
//                    GuardExpressionMember? EffectBehaviorMember?
//                    'then' TransitionSuccessionMember ActionBody           (8.2.2.18.3)
// TargetTransitionUsage = EmptyParameterMember ( … trigger, guard, effect … )?
//                    'then' TransitionSuccessionMember ActionBody
// TriggerActionMember = 'accept' TriggerAction; TriggerAction = AcceptParameterPart
// AcceptParameterPart = PayloadParameterMember ( 'via' NodeParameterMember )? (8.2.2.17.4)
//
// Entry, do and exit actions (EntryActionMember, DoActionMember, ExitActionMember,
// EntryTransitionMember), a transition's `do` effect, and the time and change triggers
// (`at`, `after`, `when`) are NOT implemented, and are reported.

#[test]
fn a_state_definition_reads_the_corpus_forms() {
    // training/23. State Definitions/State Definition Example-1.sysml:7-30 — transition
    // usages with a declaration, a source and an accepter.
    let one = render(
        &parse_accepted(
            "state def VehicleStates {\n\
             \tfirst start then off;\n\
             \tstate off;\n\
             \ttransition off_to_starting\n\
             \t\tfirst off\n\
             \t\taccept VehicleStartSignal\n\
             \t\tthen starting;\n\
             \tstate starting;\n\
             \ttransition starting_to_on\n\
             \t\tfirst starting\n\
             \t\taccept VehicleOnSignal\n\
             \t\tthen on;\n\
             \tstate on;\n\
             \ttransition on_to_off\n\
             \t\tfirst on\n\
             \t\taccept VehicleOffSignal\n\
             \t\tthen off;\n\
             }",
        )
        .syntax(),
    );
    assert_eq!(nodes_named(&one, "StateUsage"), 3, "{one}");
    assert_eq!(nodes_named(&one, "TransitionUsageMember"), 3, "{one}");
    assert_eq!(nodes_named(&one, "TargetTransitionUsageMember"), 0, "{one}");
    // training/23. State Definitions/State Definition Example-2.sysml:7-21 — the same
    // transitions as target transitions, each after the state that is its source.
    let two = render(
        &parse_accepted(
            "state def VehicleStates {\n\
             \tfirst start then off;\n\
             \tstate off;\n\
             \taccept VehicleStartSignal\n\
             \t\tthen starting;\n\
             \tstate starting;\n\
             \taccept VehicleOnSignal\n\
             \t\tthen on;\n\
             \tstate on;\n\
             \taccept VehicleOffSignal\n\
             \t\tthen off;\n\
             }",
        )
        .syntax(),
    );
    assert_eq!(nodes_named(&two, "TargetTransitionUsageMember"), 3, "{two}");
    assert_eq!(nodes_named(&two, "TransitionUsageMember"), 0, "{two}");
}

#[test]
fn a_state_definition_reads_the_clause_examples() {
    // 7.18.2 (receipt 42b13f63): a parallel state and its substates.
    parse_accepted(
        "state def VehicleStates parallel {\n\
         \tstate OperationalStates;\n\
         \tstate HealthStates;\n\
         }",
    );
    // 7.18.3 (receipt 6e6e9493), OnOff1-OnOff3, less their entry actions: a transition
    // with only a source and a target, an accepter with a receiver, and a guard after
    // the accepter.
    parse_accepted(
        "state def OnOff {\n\
         \tport commPort;\n\
         \tstate off;\n\
         \tstate on;\n\
         \ttransition off_on first off then on;\n\
         \ttransition on_off\n\
         \t\tfirst on\n\
         \t\taccept TurnOn via commPort\n\
         \t\tif isEnabled\n\
         \t\tthen off;\n\
         }",
    );
    // Adapted from OnOff5 and OnOff6: their entry actions, effects, time triggers
    // (`accept after 5[min]`) and `terminate` are unimplemented and dropped, and `then
    // done;` stands in for OnOff6's timed transition to `done`. What is left is target
    // transitions with an accepter, a receiver and a guard.
    parse_accepted(
        "state def OnOff {\n\
         \tstate off;\n\
         \taccept TurnOn via commPort\n\
         \t\tif isEnabled\n\
         \t\tthen on;\n\
         \taccept Abort via commPort then stop;\n\
         \tstate on;\n\
         \tthen done;\n\
         \taction stop;\n\
         }",
    );
    // A state usage is a BehaviorUsageElement (8.2.2.6.4), so an action body holds one,
    // and so does a part; and it nests, with its own body.
    parse_accepted("action def A { state s; }");
    parse_accepted("part def P { state s : S { state inner; } }");
    parse_accepted("state s parallel { state a; state b; }");
}

#[test]
fn a_state_definition_owns_what_its_production_writes() {
    let tree = render(
        &parse_accepted(
            "state def D { state a; transition t first a accept S via p if g then b; }",
        )
        .syntax(),
    );
    assert_eq!(
        child_kinds(&tree, "StateDefinition"),
        [
            "OccurrenceDefinitionPrefix",
            "KwState",
            "KwDef",
            "DefinitionDeclaration",
            "StateDefBody"
        ],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "TransitionUsage"),
        [
            "KwTransition",
            "UsageDeclaration",
            "KwFirst",
            "FeatureChainMember",
            "EmptyParameterMember",
            "EmptyParameterMember",
            "TriggerActionMember",
            "GuardExpressionMember",
            "KwThen",
            "TransitionSuccessionMember",
            "ActionBody"
        ],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "TriggerActionMember"),
        ["KwAccept", "TriggerAction"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "AcceptParameterPart"),
        ["PayloadParameterMember", "KwVia", "NodeParameterMember"],
        "{tree}"
    );
    // The input the absence assertion held until the commit before this one.
    let empty = render(&parse_accepted("state def S;").syntax());
    assert_eq!(
        child_kinds(&empty, "StateDefBody"),
        ["Semicolon"],
        "{empty}"
    );
    // A transition with no declaration names its source directly.
    let plain = render(&parse_accepted("state def D { transition a then b; }").syntax());
    assert_eq!(nodes_named(&plain, "UsageDeclaration"), 0, "{plain}");
}

#[test]
fn a_target_transition_owns_what_its_production_writes() {
    // A state in a state body is a BehaviorUsageMember (StateBodyItem's second
    // alternative), and a target transition after it is its own member.
    let target = render(&parse_accepted("state def D { state a; accept S then b; }").syntax());
    assert_eq!(nodes_named(&target, "BehaviorUsageMember"), 1, "{target}");
    assert_eq!(
        child_kinds(&target, "TargetTransitionUsage"),
        [
            "EmptyParameterMember",
            "EmptyParameterMember",
            "TriggerActionMember",
            "KwThen",
            "TransitionSuccessionMember",
            "ActionBody"
        ],
        "{target}"
    );
    // A `then` before a usage keyword is the NEXT item's SourceSuccessionMember, not a
    // target transition of the state before it (8.2.2.18.1).
    let source = render(&parse_accepted("state def D { state a; then state s; }").syntax());
    assert_eq!(
        nodes_named(&source, "TargetTransitionUsageMember"),
        0,
        "{source}"
    );
    assert_eq!(
        nodes_named(&source, "SourceSuccessionMember"),
        1,
        "{source}"
    );
    assert_eq!(nodes_named(&source, "BehaviorUsageMember"), 2, "{source}");
    let bare = render(&parse_accepted("state def D { state a; then b; }").syntax());
    assert_eq!(
        child_kinds(&bare, "TargetTransitionUsage"),
        [
            "EmptyParameterMember",
            "KwThen",
            "TransitionSuccessionMember",
            "ActionBody"
        ],
        "{bare}"
    );
}

#[test]
fn a_state_definition_is_bounded_by_its_rules() {
    // StateDefBody is not optional, and `parallel` stands only before braces.
    parse_rejected("state def D");
    parse_rejected("state def D parallel;");
    // TransitionUsageMember is a StateBodyItem and no ActionBodyItem or
    // DefinitionBodyItem (8.2.2.17.1, 8.2.2.6.1).
    parse_rejected("action def A { transition first a then b; }");
    parse_rejected("part def P { transition first a then b; }");
    // A target transition follows a behaviour usage member, and nothing else.
    parse_rejected("state def D { accept S then b; }");
    parse_rejected("state def D { attribute x; accept S then b; }");
    // StateBodyItem has no InitialNodeMember and no ControlNode (8.2.2.18.1).
    parse_rejected("state def D { first start; }");
    parse_rejected("state def D { merge m; }");
    // The trigger comes before the guard in TransitionUsage (8.2.2.18.3), although
    // 7.18.3's OnOff4 writes `if isEnabled accept TurnOn …`; the grammar is followed.
    parse_rejected("state def D { transition t first a if g accept S then b; }");
    // A transition names its target.
    parse_rejected("state def D { transition t first a then; }");
    // Unclosed.
    parse_rejected("state def D { state s;");
}

#[test]
fn a_state_definition_keeps_every_byte() {
    let source = "state /* s */ def D parallel {\n\tstate a; // x\n\taccept S via p\n\t\tif g then b;\n\ttransition t first a then b { }\n}\n";
    assert_eq!(parse_accepted(source).text(), source);
}

// -- State actions, effects, triggers and exhibited states, SysML 8.2.2.17-18 ---------
//
// EntryActionMember     = MemberPrefix 'entry' StateActionUsage             (8.2.2.18.1)
// DoActionMember        = MemberPrefix 'do' StateActionUsage
// ExitActionMember      = MemberPrefix 'exit' StateActionUsage
// EntryTransitionMember = MemberPrefix ( GuardedTargetSuccession
//                                      | 'then' TransitionSuccession ) ';'
//                         (deviation EntryTransitionMember, follow_xtext)
// StateActionUsage      = EmptyActionUsage ';' | StatePerformActionUsage
//                       | StateAcceptActionUsage | StateSendActionUsage
//                       | StateAssignmentActionUsage
// EffectBehaviorMember  = 'do' EffectBehaviorUsage                          (8.2.2.18.3)
// EffectBehaviorUsage   = EmptyActionUsage | TransitionPerformActionUsage
//                       | TransitionAcceptActionUsage | TransitionSendActionUsage
//                       | TransitionAssignmentActionUsage
// PayloadParameter      = PayloadFeature
//                       | Identification PayloadFeatureSpecializationPart?
//                         TriggerValuePart                                  (8.2.2.17.4)
// TriggerExpression     = ( 'at' | 'after' ) ArgumentMember
//                       | 'when' ArgumentExpressionMember
// ExhibitStateUsage     = OccurrenceUsagePrefix 'exhibit'
//                         ( OwnedReferenceSubsetting FeatureSpecializationPart?
//                         | 'state' UsageDeclaration ) ValuePart? StateUsageBody (8.2.2.18.2)
//
// The send and assignment forms read the action nodes' declarations (8.2.2.17.4,
// 8.2.2.17.5); they are tested with those nodes, below.

#[test]
fn a_state_action_reads_the_corpus_forms() {
    // training/24. States/State Actions.sysml:26-30 — entry, do and exit, by reference
    // with a body, and by declaration.
    let tree = render(
        &parse_accepted(
            "state def VehicleStates {\n\
             \tstate on {\n\
             \t\tentry performSelfTest{ in vehicle = operatingVehicle; }\n\
             \t\tdo action providePower { /* ... */ }\n\
             \t\texit action applyParkingBrake { /* ... */ }\n\
             \t}\n\
             }",
        )
        .syntax(),
    );
    assert_eq!(nodes_named(&tree, "EntryActionMember"), 1, "{tree}");
    assert_eq!(nodes_named(&tree, "DoActionMember"), 1, "{tree}");
    assert_eq!(nodes_named(&tree, "ExitActionMember"), 1, "{tree}");
    assert_eq!(nodes_named(&tree, "StatePerformActionUsage"), 3, "{tree}");
    // training/31. Constraints/Time Constraints.sysml:21-25 and examples/Simple
    // Tests/StateTest.sysml:13 — the empty entry action, an entry transition after it,
    // and a time trigger.
    let entry = render(
        &parse_accepted(
            "state def S {\n\
             \tentry; then normal;\n\
             \tstate normal;\n\
             \taccept at vehicle.maintenanceTime\n\
             \t\tthen maintenance;\n\
             \tstate maintenance;\n\
             }",
        )
        .syntax(),
    );
    assert_eq!(nodes_named(&entry, "EmptyActionUsage"), 1, "{entry}");
    assert_eq!(nodes_named(&entry, "EntryTransitionMember"), 1, "{entry}");
    assert_eq!(nodes_named(&entry, "TriggerExpression"), 1, "{entry}");
}

#[test]
fn a_trigger_and_an_effect_read_the_corpus_forms() {
    // training/25. Transitions/Local Clock Example.sysml:17-29, less its `new`
    // instantiation (tested with ConstructorExpression): a receiver, a named payload, and a
    // relative time.
    parse_accepted(
        "state def S {\n\
         \tstate off;\n\
         \taccept Start via requestPort\n\
         \t\tthen waiting;\n\
         \tstate waiting;\n\
         \taccept request : Request via requestPort\n\
         \t\tthen responding;\n\
         \tstate responding;\n\
         \taccept after 5 [SI::min]\n\
         \t\tthen waiting;\n\
         }",
    );
    // training/25. Transitions/Change and Time Triggers.sysml:24, 29, 36 — a do action by
    // reference, a change trigger, and a relative time in hours.
    parse_accepted(
        "state def S {\n\
         \tdo senseTemperature;\n\
         \tstate normal;\n\
         \taccept when senseTemperature.temp > vehicle.maxTemperature\n\
         \t\tthen degraded;\n\
         \tstate degraded;\n\
         \taccept after 48 [h]\n\
         \t\tthen normal;\n\
         }",
    );
    // examples/Simple Tests/StateTest.sysml:16-18, 39, 45 — an effect, an exit action by
    // reference, and a transition out of a nested state.
    parse_accepted(
        "state def S {\n\
         \tstate S1;\n\
         \taccept s : Sig\n\
         \t\tdo action D\n\
         \t\tthen S2;\n\
         \tstate S2;\n\
         \texit act;\n\
         \ttransition first S3.S3a then S1;\n\
         }",
    );
}

#[test]
fn a_state_action_reads_the_clause_examples() {
    // 7.18.2 (receipt 42b13f63): entry, do and exit by declaration, a do action with a
    // body of successions, the empty entry action, and `then` from an entry action.
    parse_accepted(
        "state def Exercising {\n\
         \tentry action warmup : WarmUp;\n\
         \tdo action exercise : Exercise {\n\
         \t\taction strengthTraining;\n\
         \t\tthen action cardioTraining;\n\
         \t}\n\
         \texit action cooldown : Cooldown;\n\
         }\n\
         state def TurnedOn {\n\
         \tentry;\n\
         \tdo monitorTemperature;\n\
         }\n\
         state def OperationalStates {\n\
         \tentry action initial;\n\
         \tthen off;\n\
         \tstate off;\n\
         }",
    );
    // 7.18.2's conditional entry transitions (GuardedTargetSuccession after the entry).
    parse_accepted(
        "state def OperationalStates {\n\
         \tentry action initial { out attribute isStarting : Boolean; }\n\
         \tif not initial.isStarting then off;\n\
         \tif initial.isStarting then starting;\n\
         \tstate off;\n\
         \tstate starting;\n\
         }",
    );
    // 7.18.3 (receipt 6e6e9493), OnOff4 and OnOff5 as the grammar writes them: the
    // accepter before the guard, where OnOff4 writes `if isEnabled accept …`, and no `;`
    // after the effect, where both write `do action powerUp : PowerUp;` (SYSML21-450).
    // deviations.json records both, TransitionUsage, follow_spec.
    parse_accepted(
        "state def OnOff {\n\
         \tentry action init;\n\
         \ttransition first init if isInitOff then off;\n\
         \tstate off;\n\
         \tstate on;\n\
         \ttransition off_on\n\
         \t\tfirst off\n\
         \t\taccept TurnOn via commPort\n\
         \t\tif isEnabled\n\
         \t\tdo action powerUp : PowerUp\n\
         \t\tthen on;\n\
         }",
    );
    // An accept action as a state action (StateAcceptActionUsage), with and without the
    // `action` declaration 7.17.8 (receipt bb0d6dc7) says may be omitted.
    parse_accepted("state def S { do accept Sig via p; entry action a accept Sig; }");
    // 7.18.4 (receipt dbedb2cc): the two exhibit forms.
    parse_accepted(
        "part def Vehicle {\n\
         \texhibit state operatingState references VehicleStates::operating;\n\
         \tabstract exhibit state monitoringState;\n\
         }\n\
         part vehicle : Vehicle {\n\
         \texhibit VehicleStates::monitoring :> Vehicle::monitoringState;\n\
         }",
    );
}

#[test]
fn a_state_action_owns_what_its_production_writes() {
    let tree = render(&parse_accepted("state def D { entry; then a; do b; state a; }").syntax());
    assert_eq!(
        child_kinds(&tree, "EntryActionMember"),
        ["MemberPrefix", "KwEntry", "EmptyActionUsage", "Semicolon"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "EntryTransitionMember"),
        [
            "MemberPrefix",
            "KwThen",
            "TransitionSuccession",
            "Semicolon"
        ],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "StatePerformActionUsage"),
        ["PerformActionUsageDeclaration", "ActionBody"],
        "{tree}"
    );
    let effect = render(
        &parse_accepted("state def D { state a; accept at t do action e then b; }").syntax(),
    );
    assert_eq!(
        child_kinds(&effect, "PayloadParameter"),
        ["Identification", "TriggerValuePart"],
        "{effect}"
    );
    assert_eq!(
        child_kinds(&effect, "TriggerExpression"),
        ["KwAt", "ArgumentMember"],
        "{effect}"
    );
    assert_eq!(
        child_kinds(&effect, "EffectBehaviorMember"),
        ["KwDo", "TransitionPerformActionUsage"],
        "{effect}"
    );
    // A `when` trigger's expression is referenced, not evaluated (8.2.2.17.4).
    let when = render(&parse_accepted("state def D { state a; accept when c then b; }").syntax());
    assert_eq!(
        child_kinds(&when, "TriggerExpression"),
        ["KwWhen", "ArgumentExpressionMember"],
        "{when}"
    );
    let exhibit = render(&parse_accepted("part def P { exhibit state s; }").syntax());
    assert_eq!(nodes_named(&exhibit, "ExhibitStateUsage"), 1, "{exhibit}");
    assert_eq!(nodes_named(&exhibit, "StateUsageBody"), 1, "{exhibit}");
}

#[test]
fn a_state_action_is_bounded_by_its_rules() {
    // Entry, do and exit actions are StateBodyItems, not ActionBodyItems (8.2.2.18.1).
    parse_rejected("action def A { entry; }");
    parse_rejected("part def P { exit a; }");
    // An entry transition follows an entry action and nothing else.
    parse_rejected("state def D { do a; then b; }");
    parse_rejected("state def D { state s; exit x; if g then b; }");
    // A transition's perform effect takes a braced body or none: never `;`.
    parse_rejected("state def D { transition t first a do action x; then b; }");
    // The effect comes after the guard (8.2.2.18.3).
    parse_rejected("state def D { transition t first a do action x if g then b; }");
    // A trigger kind takes its expression.
    parse_rejected("state def D { state a; accept after then b; }");
    // The empty state action writes its `;` (StateActionUsage's `EmptyActionUsage ';'`).
    parse_rejected("state def D { entry }");
    // `exhibit` takes `state` or a reference, and nothing else.
    parse_rejected("part def P { exhibit part p; }");
}

#[test]
fn a_state_action_keeps_every_byte() {
    let source = "state def D {\n\tentry /* e */ ; then a;\n\tdo action d { }\n\tstate a;\n\taccept after 5 [s] do e then a;\n}\n";
    assert_eq!(parse_accepted(source).text(), source);
}

// -- SendNode, AcceptNode and AssignmentNode, SysML 8.2.2.17.4-5 -----------------------
//
// SendNode       = OccurrenceUsagePrefix ActionNodeUsageDeclaration? 'send'
//                  ( NodeParameterMember SenderReceiverPart?
//                  | EmptyParameterMember SenderReceiverPart )? ActionBody  (8.2.2.17.4,
//                  with deviation SendNode, follow_xtext)
// SendNodeDeclaration = ActionNodeUsageDeclaration? 'send'
//                  NodeParameterMember SenderReceiverPart?
// SenderReceiverPart = 'via' NodeParameterMember ( 'to' NodeParameterMember )?
//                    | EmptyParameterMember 'to' NodeParameterMember
// AcceptNode     = OccurrenceUsagePrefix AcceptNodeDeclaration ActionBody
// AssignmentNode = OccurrenceUsagePrefix AssignmentNodeDeclaration ActionBody (8.2.2.17.5)
// AssignmentNodeDeclaration = ActionNodeUsageDeclaration? 'assign'
//                  AssignmentTargetMember FeatureChainMember ':=' NodeParameterMember
// AssignmentTargetMember    = AssignmentTargetParameter
// AssignmentTargetParameter = ( AssignmentTargetBinding '.' )?
// AssignmentTargetBinding   = NonFeatureChainPrimaryExpression
//
// All three are ActionNodes, reached through ActionNodeMember as ControlNode is
// (8.2.2.17.1), and the state and transition forms (8.2.2.18.1, 8.2.2.18.3) read the
// declarations with their own bodies.

#[test]
fn a_send_node_reads_the_corpus_forms() {
    // examples/Simple Tests/ActionTest.sysml:21 — a send after `then`, its payload a
    // constructor, its receiver after `to`.
    let then_send =
        render(&parse_accepted("action def A { first start; then send new S() to b; }").syntax());
    assert_eq!(nodes_named(&then_send, "SendNode"), 1, "{then_send}");
    assert_eq!(
        nodes_named(&then_send, "SourceSuccessionMember"),
        1,
        "{then_send}"
    );
    assert_eq!(
        nodes_named(&then_send, "ConstructorExpression"),
        1,
        "{then_send}"
    );
    assert_eq!(
        child_kinds(&then_send, "SendNode"),
        [
            "OccurrenceUsagePrefix",
            "KwSend",
            "NodeParameterMember",
            "SenderReceiverPart",
            "ActionBody"
        ],
        "{then_send}"
    );
    // `x to y`: the receiver alternative, whose sender is the EmptyParameterMember the
    // text never writes -- validateSendActionParameters wants three input parameters
    // (8.3.17.15, receipt 320cf1d4), payload, sender and receiver in that order.
    assert_eq!(
        child_kinds(&then_send, "SenderReceiverPart"),
        ["EmptyParameterMember", "KwTo", "NodeParameterMember"],
        "{then_send}"
    );
    // ActionTest.sysml:34-36 — a declared send with no parameters in its declaration and
    // its payload bound in the body instead (7.17.7, receipt db730711).
    let bare = render(
        &parse_accepted("action def A { action snd send {\n\t\tin :>> payload = s;\n\t} }")
            .syntax(),
    );
    assert_eq!(
        child_kinds(&bare, "SendNode"),
        [
            "OccurrenceUsagePrefix",
            "ActionNodeUsageDeclaration",
            "KwSend",
            "ActionBody"
        ],
        "{bare}"
    );
}

#[test]
fn a_send_node_reads_the_sender_forms() {
    // examples/Simple Tests/ActionTest.sysml:37 — no payload, a sender and a receiver:
    // the second alternative.
    let via_to = render(
        &parse_accepted("action def A { action snd2 send via this to aa.target; }").syntax(),
    );
    assert_eq!(
        child_kinds(&via_to, "SendNode"),
        [
            "OccurrenceUsagePrefix",
            "ActionNodeUsageDeclaration",
            "KwSend",
            "EmptyParameterMember",
            "SenderReceiverPart",
            "ActionBody"
        ],
        "{via_to}"
    );
    assert_eq!(
        child_kinds(&via_to, "SenderReceiverPart"),
        [
            "KwVia",
            "NodeParameterMember",
            "KwTo",
            "NodeParameterMember"
        ],
        "{via_to}"
    );
    // examples/Interaction Sequencing Examples/ServerSequenceRealization-2.sysml:19 —
    // the line the SendNode deviation turns on: `action NAME send`.
    parse_accepted(
        "action def A { action publish send new Publish(someTopic, somePublication) via publicationPort; }",
    );
    // The clause's own example, 7.17.7 (receipt db730711).
    parse_accepted(
        "part monitor { action sendReadingTo { in part destination;\n\
         \tperform getReading { out reading : SensorReading; }\n\
         \taction sendReading\n\
         \t\tsend getReading.reading via monitor to destination;\n\
         \tsend getReading.reading via monitor to destination;\n\
         \tsend getReading.reading to destination;\n\
         } }",
    );
}

#[test]
fn an_action_node_is_an_action_node_member_and_takes_target_successions() {
    for item in ["send s to b;", "accept S;", "assign x := 1;"] {
        assert_eq!(
            member_of("action def A", item),
            ["ActionNodeMember"],
            "{item}"
        );
    }
    // ActionBodyItem's third alternative: `SourceSuccessionMember? ActionBehaviorMember
    // ActionTargetSuccessionMember*` (8.2.2.17.1), and an ActionNodeMember is an
    // ActionBehaviorMember, so the `then` members after one are its own.
    let tree = render(&parse_accepted("action def A { send s to b; then c; then d; }").syntax());
    assert_eq!(
        nodes_named(&tree, "ActionTargetSuccessionMember"),
        2,
        "{tree}"
    );
}

#[test]
fn an_accept_node_reads_the_corpus_forms() {
    // examples/Simple Tests/ActionTest.sysml:17-19 — a type, a time trigger, and an
    // absolute time whose value is a constructor.
    let tree = render(
        &parse_accepted(
            "action def A {\n\
             \tfirst start;\n\
             \tthen accept S;\n\
             \tthen accept sig after 10[SI::s];\n\
             \tthen accept at new Time::Iso8601DateTime(\"2022-01-30T01:00:00Z\");\n\
             }",
        )
        .syntax(),
    );
    assert_eq!(nodes_named(&tree, "AcceptNode"), 3, "{tree}");
    assert_eq!(
        child_kinds(&tree, "AcceptNode"),
        [
            "OccurrenceUsagePrefix",
            "AcceptNodeDeclaration",
            "ActionBody"
        ],
        "{tree}"
    );
}

#[test]
fn an_assignment_node_reads_the_corpus_forms() {
    // examples/Simple Tests/AssignmentTest.sysml:7 — no target: the target parameter is
    // empty and the referent is one name (7.17.9, receipt 7d690ecc: "If the target
    // expression ... is omitted, then the target is implicitly the occurrence owning the
    // assignment action usage").
    let bare = render(&parse_accepted("action incr { assign count := count + 1; }").syntax());
    assert_eq!(
        child_kinds(&bare, "AssignmentNode"),
        [
            "OccurrenceUsagePrefix",
            "AssignmentNodeDeclaration",
            "ActionBody"
        ],
        "{bare}"
    );
    assert_eq!(
        child_kinds(&bare, "AssignmentNodeDeclaration"),
        [
            "KwAssign",
            "AssignmentTargetMember",
            "FeatureChainMember",
            "ColonEq",
            "NodeParameterMember"
        ],
        "{bare}"
    );
    assert_eq!(
        child_kinds(&bare, "AssignmentTargetParameter"),
        Vec::<String>::new(),
        "{bare}"
    );
    // AssignmentTest.sysml:49 — the target is the first primary and the referent the
    // chain after its `.`: 7.17.9's own example, `assign sim.vehicle.position := ...`,
    // says "The target of the assignment below is "sim". The referent feature chain is
    // "vehicle.position"".
    let chained = render(
        &parse_accepted(
            "action a { assign counting.counter.count := counting.counter.count + 1; }",
        )
        .syntax(),
    );
    assert_eq!(
        child_kinds(&chained, "AssignmentTargetParameter"),
        ["AssignmentTargetBinding", "Dot"],
        "{chained}"
    );
    assert_eq!(
        child_kinds(&chained, "FeatureChainMember"),
        ["OwnedFeatureChainMember"],
        "{chained}"
    );
    // validation/03-Function-based Behavior/3c-Function-based Behavior-structure
    // mod-1.sysml:38 — a quoted name as the target, a constructor as the value.
    parse_accepted(
        "action a { assign 'vehicle-trailer system'.trailerHitch := new TrailerHitch(); }",
    );
    // AssignmentTest.sysml:50 — an invocation, then a chain, as the value.
    parse_accepted(
        "action a { assign counting.counter.count := Increment(counting.counter).count; }",
    );
    // A declared assignment: `action NAME assign`.
    parse_accepted("action def A { action reset assign counter.count := 0; }");
}

#[test]
fn a_state_and_a_transition_read_assignment_actions() {
    // examples/Simple Tests/AssignmentTest.sysml:18-37 — an entry assignment and two do
    // assignments.
    let state = render(
        &parse_accepted(
            "state def Counting {\n\
             \tpart counter : Counter;\n\
             \tentry assign counter.count := 0;\n\
             \tthen state wait;\n\
             \tstate increment {\n\
             \t\tdo assign counter.count := counter.count + 1;\n\
             \t}\n\
             \tthen wait;\n\
             }",
        )
        .syntax(),
    );
    assert_eq!(
        nodes_named(&state, "StateAssignmentActionUsage"),
        2,
        "{state}"
    );
    assert_eq!(
        child_kinds(&state, "StateAssignmentActionUsage"),
        ["AssignmentNodeDeclaration", "ActionBody"],
        "{state}"
    );
    // training/31. Constraints/Time Constraints.sysml:30 — an entry assignment whose
    // value adds two chains.
    parse_accepted(
        "state def S { state maintenance {\n\
         \tentry assign vehicle.maintenanceTime := vehicle.maintenanceTime + vehicle.maintenanceInterval;\n\
         } }",
    );
    // A transition's assignment effect, braced.
    let effect = render(
        &parse_accepted("state def S { transition t first a do assign x := 1 { } then b; }")
            .syntax(),
    );
    assert_eq!(
        child_kinds(&effect, "TransitionAssignmentActionUsage"),
        ["AssignmentNodeDeclaration", "LBrace", "RBrace"],
        "{effect}"
    );
}

#[test]
fn a_state_and_a_transition_read_send_actions() {
    // examples/Simple Tests/StateTest.sysml:20-23 and 32-37 — a do send in a state, and
    // a send as a transition's effect, ended by the `then` rather than a `;`.
    let sends = render(
        &parse_accepted(
            "state def S {\n\
             \tstate S2 {\n\
             \t\tdo send new Sig(T.s.x) to p;\n\
             \t\tstate S3;\n\
             \t}\n\
             \ttransition T\n\
             \t\tfirst S2.S3\n\
             \t\taccept s : Sig via p\n\
             \t\tif true\n\
             \t\tdo send s to p\n\
             \t\tthen S1;\n\
             }",
        )
        .syntax(),
    );
    assert_eq!(nodes_named(&sends, "StateSendActionUsage"), 1, "{sends}");
    assert_eq!(
        child_kinds(&sends, "StateSendActionUsage"),
        ["SendNodeDeclaration", "ActionBody"],
        "{sends}"
    );
    assert_eq!(
        child_kinds(&sends, "TransitionSendActionUsage"),
        ["SendNodeDeclaration"],
        "{sends}"
    );
    // training/25. Transitions/Transition Actions.sysml:31 — a target transition's
    // effect, a send of a constructed signal.
    parse_accepted(
        "state def S {\n\
         \tstate off;\n\
         \taccept VehicleStartSignal\n\
         \t\tdo send new ControllerStartSignal() to controller\n\
         \t\tthen on;\n\
         \tstate on;\n\
         }",
    );
}

#[test]
fn send_assign_and_accept_nodes_are_bounded_by_their_rules() {
    // `via` before `to`: SenderReceiverPart's first alternative is `'via' ... ( 'to'
    // ... )?`, and its second has no `via` at all. Held as a file by
    // tests/rejection/sender-receiver-part-writes-via-before-to.sysml.
    parse_rejected("action def A { send s to b via p; }");
    // A state or transition send names its payload: SendNodeDeclaration's
    // NodeParameterMember is not optional, unlike SendNode's (8.2.2.17.4). Held as a
    // file by tests/rejection/state-send-action-names-its-payload.sysml.
    parse_rejected("state def D { entry send via p; }");
    parse_rejected("state def D { entry send; }");
    // A declared send writes `action` (deviation SendNode, follow_xtext): `snd send x;`
    // is the literal clause line's reading, which the corpus contradicts. Held as a file
    // by tests/rejection/send-node-declaration-writes-action.sysml.
    parse_rejected("action def A { snd send x; }");
    // An assignment writes `:=`, not `=` (8.2.2.17.5). Held as a file by
    // tests/rejection/assignment-writes-colon-equals.sysml.
    parse_rejected("action def A { assign x = 1; }");
    // The referent is not optional: AssignmentTargetParameter may be empty, the
    // FeatureChainMember may not. Held as a file by
    // tests/rejection/assignment-names-its-referent.sysml.
    parse_rejected("action def A { assign := 1; }");
    // A transition's send effect takes a braced body or none, never `;`
    // (TransitionSendActionUsage, 8.2.2.18.3). Held as a file by
    // tests/rejection/transition-send-effect-takes-no-semicolon.sysml.
    parse_rejected("state def D { transition t first a do send s to p; then b; }");
    // An ActionNode is an item of the action-body family only (8.2.2.17.1): a part
    // definition's body has no ActionNodeMember. Held as a file by
    // tests/rejection/action-node-is-not-a-definition-body-item.sysml.
    parse_rejected("part def P { assign x := 1; }");
    parse_rejected("part def P { send s to b; }");
}

#[test]
fn send_and_assignment_nodes_keep_every_byte() {
    let source = "action def A {\n\tthen /* s */ send new S ( 1 ) via p /* v */ to q ;\n\tassign a . b . c := 1 { }\n\taction x send { }\n}\n";
    assert_eq!(parse_accepted(source).text(), source);
}

// -- DefaultTargetSuccession, SysML 8.2.2.17.8 ----------------------------------------
//
// DefaultTargetSuccession : TransitionUsage =
//     'else' ownedRelationship += TransitionSuccessionMember            (8.2.2.17.8)
//
// ActionTargetSuccession's third and last alternative: the branch taken when no guard
// before it was true. A TransitionUsage like the guarded form, over the same
// TransitionSuccessionMember, and it writes no guard at all — the `else` IS the whole
// condition.

#[test]
fn a_default_target_succession_reads_the_corpus_form() {
    // vendor/corpus/sysml/src/examples/Simple Tests/DecisionTest.sysml:4-7 — a decision
    // node, two guarded successions, and the default after them.
    let tree = render(
        &parse_accepted(
            "action def A { attribute x = 1; decide 'test x'; \
             if x == 1 then A1; if x > 1 then A2; else A3; }",
        )
        .syntax(),
    );
    assert_eq!(nodes_named(&tree, "DefaultTargetSuccession"), 1, "{tree}");
    assert_eq!(nodes_named(&tree, "GuardedTargetSuccession"), 2, "{tree}");
    assert_eq!(
        nodes_named(&tree, "ActionTargetSuccessionMember"),
        3,
        "{tree}"
    );
}

#[test]
fn a_default_target_succession_owns_what_its_production_writes() {
    let tree = render(&parse_accepted("action def A { first start; else b; }").syntax());
    assert_eq!(
        child_kinds(&tree, "ActionTargetSuccession"),
        ["DefaultTargetSuccession", "UsageBody"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "DefaultTargetSuccession"),
        ["KwElse", "TransitionSuccessionMember"],
        "{tree}"
    );
    // The same TransitionSuccession the guarded form owns: an empty source end and the
    // target's ConnectorEnd.
    assert_eq!(
        child_kinds(&tree, "TransitionSuccession"),
        ["EmptyEndMember", "ConnectorEndMember"],
        "{tree}"
    );
    // No guard is written, so there is no GuardExpressionMember to own.
    assert_eq!(nodes_named(&tree, "GuardExpressionMember"), 0, "{tree}");
    // The target is a ConnectorEnd, so a chain and the `references` form are admitted,
    // and UsageBody may be braced.
    parse_accepted("action def A { first start; else a.b.c; }");
    parse_accepted("action def A { first start; else e references a; }");
    parse_accepted("action def A { first start; else b { } }");
    parse_accepted("action def A { first start; private else b; }");
}

#[test]
fn an_else_that_belongs_to_an_expression_is_left_alone() {
    // KerML's ConditionalExpression is `'if' Expression '?' Expression 'else' Expression`
    // (8.2.5.8.1), so its `else` is INSIDE the expression and never at an item position.
    // A calculation body writing one as its result keeps it.
    let tree = render(&parse_accepted("calc def C { action a; if x ? 1 else 2 }").syntax());
    assert_eq!(nodes_named(&tree, "ConditionalExpression"), 1, "{tree}");
    assert_eq!(nodes_named(&tree, "DefaultTargetSuccession"), 0, "{tree}");
}

#[test]
fn a_default_target_succession_is_a_suffix_and_nothing_else() {
    // The same rules the other two alternatives obey. Held as files by
    // tests/rejection/default-target-succession-*.sysml.
    parse_rejected("action def A { else b; }");
    parse_rejected("action def A { part p; else b; }");
    parse_rejected("part def P { action a; else b; }");
    parse_rejected("action def A { first start; else b }");
}

#[test]
fn a_guarded_target_succession_keeps_every_byte() {
    let source = "action def A {\n\tfirst start;\n\tif /* g */ x == 1 // n\n\t\tthen b;\n}\n";
    assert_eq!(parse_accepted(source).text(), source);
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

// -- BracketExpression, KerML 8.2.5.8.2 -------------------------------------------
//
// BracketExpression = PrimaryArgumentMember '[' SequenceExpressionListMember ']'
//
// The quantity form. The other half of the `[` question: a bracket in an expression is
// this, and a bracket in a declaration is a MultiplicityRange.

#[test]
fn a_bracket_expression_reads_the_quantity_form() {
    // Every one of these is a line the pinned corpus writes.
    parse_accepted("attribute x = 1200 [kg];");
    parse_accepted("attribute x = 4.82 [m];");
    parse_accepted("attribute x = 7.2973525693E-3[one];");
    parse_accepted("attribute x = 299792458[m/s];");
    parse_accepted("attribute x = 9.80665['m/s²'];");
    parse_accepted("attribute x = 5 [N*m];");
    parse_accepted("attribute x = 3 [SI::kg];");
    // The operand may be a parenthesised sequence, not just a literal.
    parse_accepted("attribute x = (0, 0, 0) [spatialCF];");
}

#[test]
fn the_two_roles_of_a_bracket_are_told_apart_by_position() {
    // THE [bracket-role] QUESTION, and the line that answers it. The corpus writes
    //     attribute mass : MassValue[1] = 1200 [kg];
    // where `[1]` is a MultiplicityRange in the declaration (SysML 8.2.2.6.6) and
    // `[kg]` is a BracketExpression over the value (KerML 8.2.5.8.2). The two are told
    // apart by POSITION and nothing else: multiplicity_part is reachable only from
    // feature_specialization_part, which is inside UsageDeclaration, and a UsageDeclaration
    // is finished before the ValuePart's `=` is read. So the roles cannot meet.
    let rendered = render(&parse_accepted("attribute mass : MassValue[1] = 1200 [kg];").syntax());
    assert_eq!(nodes_named(&rendered, "MultiplicityRange"), 1, "{rendered}");
    assert_eq!(nodes_named(&rendered, "BracketExpression"), 1, "{rendered}");
    // The multiplicity is inside the declaration and the bracket is inside the value.
    let declaration = subtree(&rendered, "UsageDeclaration");
    assert_eq!(
        nodes_named(&declaration, "MultiplicityRange"),
        1,
        "{declaration}"
    );
    assert_eq!(
        nodes_named(&declaration, "BracketExpression"),
        0,
        "{declaration}"
    );
    let value = subtree(&rendered, "ValuePart");
    assert_eq!(nodes_named(&value, "BracketExpression"), 1, "{value}");
    assert_eq!(nodes_named(&value, "MultiplicityRange"), 0, "{value}");
    // And a ranged multiplicity beside a qualified unit, which the corpus also writes.
    parse_accepted("attribute q : Real[1..*] = 3 [SI::kg];");
}

#[test]
fn a_bracket_expression_needs_an_operand_and_a_sequence() {
    // The operand is what a MultiplicityRange has not got, so it is the whole of the
    // distinction. Held as files by
    // tests/rejection/bracket-expression-needs-an-operand.sysml,
    // bracket-expression-unclosed.sysml and bracket-expression-needs-a-sequence.sysml.
    parse_rejected("attribute x = [kg];");
    parse_rejected("attribute x = 1200 [kg;");
    parse_rejected("attribute x = 1200 [];");
}

#[test]
fn the_postfix_forms_nest_in_the_order_they_are_written() {
    // FeatureChainExpression and BracketExpression share one left-folding loop, so they
    // nest by what is written rather than by any rule between them — which is how the
    // Pilot arranges them too (KerMLExpressions.xtext:299-322).
    let bracket_over_chain = render(&parse_accepted("attribute x = a.b [kg];").syntax());
    assert_eq!(
        child_kinds(&bracket_over_chain, "BracketExpression")
            .first()
            .map(String::as_str),
        Some("PrimaryArgumentMember"),
        "{bracket_over_chain}"
    );
    // The chain is inside the bracket's operand, so the bracket is the outer node.
    let operand = subtree(&bracket_over_chain, "PrimaryArgumentMember");
    assert_eq!(
        nodes_named(&operand, "FeatureChainExpression"),
        1,
        "{operand}"
    );

    // Written the other way round, the chain is outside.
    let chain_over_bracket = render(&parse_accepted("attribute x = a[1].b;").syntax());
    let chain_operand = subtree(&chain_over_bracket, "NonFeatureChainPrimaryArgumentMember");
    assert_eq!(
        nodes_named(&chain_operand, "BracketExpression"),
        1,
        "{chain_operand}"
    );
}

#[test]
fn a_bracket_expression_owns_no_result_member() {
    // For the reason FeatureChainExpression owns none: the BNF names EmptyResultMember
    // where a production has one (8.2.5.8.1), and 8.2.5.8.2 does not.
    let rendered = render(&parse_accepted("attribute x = 1200 [kg];").syntax());
    assert_eq!(
        child_kinds(&rendered, "BracketExpression"),
        [
            "PrimaryArgumentMember",
            "LBracket",
            "SequenceExpressionListMember",
            "RBracket"
        ],
        "{rendered}"
    );
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
fn a_requirement_body_does_not_admit_the_four_members_it_has_not_got() {
    // The part of RequirementBodyItem that is NOT DefinitionBodyItem, less the two now
    // implemented: SubjectMember and RequirementConstraintMember. Rejected by absence,
    // not by rule — each is well-formed SysML.
    //
    // The count in this test's name is the honest running total of what is left of
    // 8.2.2.21.1, and it has gone six, five, four as the members landed.
    parse_rejected("requirement def R { frame concern c; }");
    parse_rejected("requirement def R { verify requirement r; }");
    parse_rejected("requirement def R { actor operator; }");
    parse_rejected("requirement def R { stakeholder owner; }");
    // The two that left the list, here so it cannot quietly grow back.
    parse_accepted("requirement def R { subject vehicle : Vehicle; }");
    parse_accepted("requirement def R { require constraint { a <= b } }");
}

#[test]
fn a_requirement_definition_needs_a_body_and_a_def() {
    // RequirementBody is not optional. Held as a file by
    // tests/rejection/requirement-definition-missing-requirement-body.sysml.
    parse_rejected("requirement def R");
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

// -- RequirementConstraintMember, SysML 8.2.2.21.1 --------------------------------
//
// RequirementConstraintMember = MemberPrefix? RequirementKind RequirementConstraintUsage
// RequirementKind             = 'assume' | 'require'
// RequirementConstraintUsage  = OwnedReferenceSubsetting FeatureSpecializationPart?
//                               RequirementBody
//                             | ( UsageExtensionKeyword* 'constraint'
//                               | UsageExtensionKeyword+ )
//                               ConstraintUsageDeclaration CalculationBody

#[test]
fn a_requirement_constraint_reads_both_keywords_and_both_alternatives() {
    // The constructed alternative, which is what the corpus writes.
    parse_accepted("requirement def R { require constraint { massActual <= massReqd } }");
    parse_accepted("requirement def R { assume constraint { fuelMass > 0 } }");
    // The by-reference alternative, with each of RequirementBody's two forms.
    parse_accepted("requirement def R { require massLimit; }");
    parse_accepted("requirement def R { assume massLimit; }");
    parse_accepted("requirement def R { require rangeRequirement { :>> actualRange = sim; } }");
    // A visibility may precede it, because MemberPrefix is a VisibilityIndicator?.
    parse_accepted("requirement def R { private require constraint { a <= b } }");
    // A named constructed constraint, since ConstraintUsageDeclaration is a
    // UsageDeclaration (SysML 8.2.2.20).
    parse_accepted("requirement def R { require constraint massCheck { a <= b } }");
}

#[test]
fn the_two_keywords_are_one_production_that_records_which() {
    // RequirementKind sets the membership's `kind` and has no other spelling in the
    // text (metaclass 8.3.21.7), so the keyword is the only record of it — which is why
    // it gets a node, as PortionKind and VisibilityIndicator do.
    for (source, keyword) in [
        (
            "requirement def R { require constraint { a <= b } }",
            "KwRequire",
        ),
        (
            "requirement def R { assume constraint { a <= b } }",
            "KwAssume",
        ),
    ] {
        let rendered = render(&parse_accepted(source).syntax());
        assert_eq!(
            child_kinds(&rendered, "RequirementKind"),
            [keyword],
            "{rendered}"
        );
    }
}

#[test]
fn the_by_reference_alternative_takes_a_requirement_body_not_a_calculation_body() {
    // THE ADJUDICATED CONFLICT. The clause gives the by-reference alternative a
    // RequirementBody; SysML.xtext gives it a CalculationBody. deviations.json records
    // follow_spec, adjudicated 2026-09-17, and this is what that costs and buys.
    //
    // Only a CalculationBody ends in a ResultExpressionMember (SysML 8.2.2.19), so a
    // trailing expression is grammatical after `require constraint` and NOT after
    // `require <name>`. Held as a file by
    // tests/rejection/requirement-constraint-by-reference-takes-a-requirement-body.sysml.
    parse_rejected("requirement def R { require someRef { a <= b } }");
    parse_accepted("requirement def R { require constraint { a <= b } }");

    // And the bodies really are different nodes, not one node under two names.
    let by_reference =
        render(&parse_accepted("requirement def R { require someRef { attribute a; } }").syntax());
    assert_eq!(
        nodes_named(&by_reference, "RequirementBody"),
        2,
        "{by_reference}"
    );
    assert_eq!(
        nodes_named(&by_reference, "CalculationBody"),
        0,
        "{by_reference}"
    );

    let constructed =
        render(&parse_accepted("requirement def R { require constraint { a <= b } }").syntax());
    assert_eq!(
        nodes_named(&constructed, "CalculationBody"),
        1,
        "{constructed}"
    );
    // One RequirementBody, the enclosing definition's — the member does not add another.
    assert_eq!(
        nodes_named(&constructed, "RequirementBody"),
        1,
        "{constructed}"
    );
}

#[test]
fn a_requirement_constraint_owns_its_usage_through_its_own_membership() {
    // RequirementConstraintMember : RequirementConstraintMembership (SysML 8.3.21.7), not
    // the DefinitionMember the body's ordinary items use — so it is dispatched beside
    // SubjectMember rather than inside `membership`.
    let rendered =
        render(&parse_accepted("requirement def R { require constraint { a <= b } }").syntax());
    assert_eq!(
        child_kinds(&rendered, "RequirementConstraintMember"),
        [
            "MemberPrefix",
            "RequirementKind",
            "RequirementConstraintUsage"
        ],
        "{rendered}"
    );
    assert_eq!(nodes_named(&rendered, "DefinitionMember"), 0, "{rendered}");
}

#[test]
fn a_requirement_constraint_is_only_a_member_where_the_grammar_reaches_one() {
    // RequirementBodyItem has the alternative and DefinitionBodyItem has none. A
    // calculation body does not admit one either, although it is the body the
    // constructed alternative ends in. Held as a file by
    // tests/rejection/requirement-constraint-member-is-not-a-definition-body-item.sysml.
    parse_rejected("part def V { require constraint { a <= b } }");
    parse_rejected("constraint def C { require constraint { a <= b } }");
    parse_rejected("package P { require constraint { a <= b } }");
    parse_rejected("require constraint { a <= b }");
    // A `require` with nothing after it is neither alternative.
    parse_rejected("requirement def R { require; }");
    // Prefix metadata standing in for the keyword is the unimplemented half. Held by
    // tests/rejection/requirement-constraint-usage-prefix-metadata-is-not-implemented.sysml.
    parse_rejected("requirement def R { require #approved { a <= b } }");
}

#[test]
fn the_whole_requirement_shape_the_corpus_writes_now_parses() {
    // Taken from vendor/corpus/sysml/src/training/32. Requirements/
    // Requirement Definitions.sysml, which needed five productions this session added:
    // RequirementDefinition, RequirementBody, SubjectMember, CalculationBody and
    // RequirementConstraintMember, plus feature chains for the dotted names.
    parse_accepted(
        "requirement def <'1'> VehicleMassLimitationRequirement :> MassLimitationRequirement {\n\
         \tdoc /* The total mass shall be less than or equal to the required mass. */\n\
         \tsubject vehicle : Vehicle;\n\
         \tattribute massActual;\n\
         \trequire constraint { massActual <= vehicle.massReqd }\n\
         \tassume constraint { vehicle.fuelMass > 0 }\n\
         }",
    );
}

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
fn a_diagnostic_points_at_the_text_it_is_about() {
    // The whole point of the range: a caller underlines it. `class` is KerML's, so in a
    // SysML file it is unexpected, and recovery skips to the end of the statement — so the
    // range covers the RUN it gave up on, from the offending token through the `;`.
    //
    // It covered exactly `class` while recovery took one token at a time and reported each
    // one. That expectation is not relaxed here, it is replaced: the claim is still that
    // the range is exactly the text the diagnostic is about, and the text it is about is
    // now the statement. The sibling case below keeps a witness for the one-token form.
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
    assert_eq!(source.get(start..end), Some("class Wrong;"));
    // One diagnostic for the statement, where there were three: `class`, `Wrong` and `;`.
    assert_eq!(
        parsed
            .errors()
            .iter()
            .filter(|d| d.code() == DiagnosticCode::Unexpected)
            .count(),
        1,
        "{:?}",
        parsed.errors()
    );
}

#[test]
fn a_stray_token_with_no_statement_to_end_is_its_own_range() {
    // A `}` at a root that has no open body is a statement of one token: recovery takes it,
    // stops before the next one rather than swallowing the file, and the range is that one
    // byte. This is the witness that the range stays exact when the run is a single token.
    let source = "}}}";
    let parsed = parse(source, Language::SysMl);
    let unexpected: Vec<_> = parsed
        .errors()
        .iter()
        .filter(|d| d.code() == DiagnosticCode::Unexpected)
        .collect();
    assert_eq!(unexpected.len(), 3, "{:?}", parsed.errors());
    for (n, diagnostic) in unexpected.iter().enumerate() {
        let start = usize::from(diagnostic.range().start());
        let end = usize::from(diagnostic.range().end());
        assert_eq!(source.get(start..end), Some("}"), "diagnostic {n}");
    }
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
    // SatisfyRequirementUsage (SysML 8.2.2.21.2) is not; it replaced a ConnectionUsage
    // when that landed. It must be reported, and the definition after it must still
    // parse — recovery happens at the enclosing body.
    let parsed = parse_rejected("part def Vehicle { satisfy r by v; part def Wheel; }");
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

// -- InvocationExpression and ArgumentList, KerML 8.2.5.8.3 ------------------------
//
// InvocationExpression   = InstantiatedTypeMember ArgumentList EmptyResultMember
// InstantiatedTypeMember = memberElement = InstantiatedTypeReference
//                        | OwnedFeatureChainMember
// ArgumentList           = '(' ( PositionalArgumentList | NamedArgumentList )? ')'
// PositionalArgumentList = ArgumentMember ( ',' ArgumentMember )*
// NamedArgumentList      = NamedArgumentMember ( ',' NamedArgumentMember )*
// NamedArgument          = ParameterRedefinition '=' ArgumentValue
//
// BaseExpression's alternative that a name followed by `(` selects. ArgumentMember,
// Argument, ArgumentValue and EmptyResultMember were already implemented by the
// operator core; this is the caller that reaches them through a written `(`.

#[test]
fn an_invocation_reads_the_positional_form_the_corpus_writes() {
    // `sum(partMasses)` is the line from vendor/corpus/sysml/src/examples/
    // Simple Tests/CalculationTest.sysml that this production exists for.
    parse_accepted("calc def C { sum(partMasses) }");
    parse_accepted("calc def C { return totalMass : MassValue = sum(partMasses); }");
    // Several arguments, as the trade-study files write `EngineEvaluation(a, b, c)`.
    parse_accepted("calc def C { EngineEvaluation(a, b, c) }");
    // An argument is an OwnedExpression, not just a name (ArgumentValue, 8.2.5.8.1).
    parse_accepted("calc def C { f(a + b, c * 2) }");
    // And ParameterTest's `F(a, 2)`.
    parse_accepted("calc def C { F(a, 2) }");
}

#[test]
fn an_invocation_takes_no_arguments_at_all() {
    // The `?` in `'(' ( PositionalArgumentList | NamedArgumentList )? ')'`. The corpus
    // writes `size()`.
    parse_accepted("calc def C { size() }");
}

#[test]
fn an_invocation_names_its_type_with_a_qualified_name() {
    // InstantiatedTypeReference = [QualifiedName] (KerML 8.2.5.8.3), so the `::` form
    // the corpus imports through is a target too.
    parse_accepted("calc def C { ISQ::sum(x) }");
    parse_accepted("calc def C { NumericalFunctions::round(x) }");
}

#[test]
fn an_invocation_reads_the_named_form() {
    // NamedArgument = ParameterRedefinition '=' ArgumentValue (KerML 8.2.5.8.3), which
    // ParameterTest writes as `F(q = 1, p = a)`.
    parse_accepted("calc def C { F(q = 1, p = a) }");
    parse_accepted("calc def C { F(q = 1) }");
}

#[test]
fn an_invocation_nests_in_itself_and_in_other_expressions() {
    // An argument is a full OwnedExpression, so an invocation is an argument.
    parse_accepted("calc def C { sum(f(x), g(y)) }");
    // And an invocation is an operand like any other primary expression.
    parse_accepted("calc def C { sum(x) + 1 }");
    parse_accepted("calc def C { a <= sum(x) }");
}

#[test]
fn an_invocation_is_told_from_the_expressions_that_share_its_tokens() {
    // A `(` with no name before it is a SequenceExpression or a NullExpression
    // (KerML 8.2.5.8.2, 8.2.5.8.3), not an invocation. `and` is a reserved keyword and
    // a keyword is not a name (SysML 8.2.2.1.2), so `x and (y)` is an operator over a
    // parenthesised operand — the case that makes the recogniser ask for a NAME rather
    // than for any token before the `(`.
    let grouped = render(&parse_accepted("calc def C { (a + b) * c }").syntax());
    assert_eq!(
        nodes_named(&grouped, "InvocationExpression"),
        0,
        "{grouped}"
    );
    let conjunction = render(&parse_accepted("calc def C { x and (y) }").syntax());
    assert_eq!(
        nodes_named(&conjunction, "InvocationExpression"),
        0,
        "{conjunction}"
    );
    // A bare name with no `(` after it stays a FeatureReferenceExpression (8.2.5.8.3).
    let bare = render(&parse_accepted("calc def C { x }").syntax());
    assert_eq!(nodes_named(&bare, "InvocationExpression"), 0, "{bare}");
    assert_eq!(
        nodes_named(&bare, "FeatureReferenceExpression"),
        1,
        "{bare}"
    );
}

#[test]
fn an_invocation_builds_the_members_the_clause_names() {
    let rendered = render(&parse_accepted("calc def C { f(a, b) }").syntax());
    assert_eq!(
        nodes_named(&rendered, "InvocationExpression"),
        1,
        "{rendered}"
    );
    assert_eq!(nodes_named(&rendered, "ArgumentList"), 1, "{rendered}");
    assert_eq!(
        nodes_named(&rendered, "PositionalArgumentList"),
        1,
        "{rendered}"
    );
    // One ArgumentMember per argument, and each owns an Argument and an ArgumentValue.
    assert_eq!(nodes_named(&rendered, "ArgumentMember"), 2, "{rendered}");
    assert_eq!(nodes_named(&rendered, "ArgumentValue"), 2, "{rendered}");
    assert_eq!(
        nodes_named(&rendered, "InstantiatedTypeReference"),
        1,
        "{rendered}"
    );
    // THREE EmptyResultMembers, not one. KerML 8.2.5.8.3 names an EmptyResultMember in
    // FeatureReferenceExpression as well as in InvocationExpression, and `a` and `b` are
    // each a FeatureReferenceExpression: one result parameter per argument, plus the
    // invocation's own. The empty form below isolates the invocation's.
    assert_eq!(nodes_named(&rendered, "EmptyResultMember"), 3, "{rendered}");
    assert_eq!(nodes_named(&rendered, "NamedArgumentList"), 0, "{rendered}");
}

#[test]
fn an_empty_argument_list_builds_the_invocations_own_result_parameter() {
    // With no arguments there is no other expression to own one, so the only
    // EmptyResultMember left is the result parameter the invocation itself owns —
    // which is what isolates it from the three the two-argument form has.
    let empty = render(&parse_accepted("calc def C { f() }").syntax());
    assert_eq!(nodes_named(&empty, "EmptyResultMember"), 1, "{empty}");
    assert_eq!(nodes_named(&empty, "ArgumentList"), 1, "{empty}");
    // The `?` was not taken, so neither list node is built.
    assert_eq!(nodes_named(&empty, "PositionalArgumentList"), 0, "{empty}");
    assert_eq!(nodes_named(&empty, "NamedArgumentList"), 0, "{empty}");
}

#[test]
fn a_named_argument_builds_the_other_alternative() {
    // NamedArgument = ParameterRedefinition '=' ArgumentValue (KerML 8.2.5.8.3): the
    // value hangs off the NamedArgument directly, with no Argument between them, which
    // is the shape difference from a positional ArgumentMember.
    let named = render(&parse_accepted("calc def C { f(q = 1) }").syntax());
    assert_eq!(nodes_named(&named, "NamedArgumentList"), 1, "{named}");
    assert_eq!(nodes_named(&named, "NamedArgumentMember"), 1, "{named}");
    assert_eq!(nodes_named(&named, "NamedArgument"), 1, "{named}");
    assert_eq!(nodes_named(&named, "ParameterRedefinition"), 1, "{named}");
    assert_eq!(nodes_named(&named, "ArgumentValue"), 1, "{named}");
    assert_eq!(nodes_named(&named, "PositionalArgumentList"), 0, "{named}");
    assert_eq!(nodes_named(&named, "ArgumentMember"), 0, "{named}");
    assert_eq!(nodes_named(&named, "Argument"), 0, "{named}");
}

#[test]
fn an_argument_list_is_one_alternative_or_the_other_and_never_both() {
    // ArgumentList = '(' ( PositionalArgumentList | NamedArgumentList )? ')' — the two
    // lists are alternatives, so a mixed list is not this grammar. `=` is not an infix
    // operator in KerML 8.2.5.8.1 table 6, so `q = 1` is not an OwnedExpression either
    // and cannot be read as a positional argument. Held as files by
    // tests/rejection/argument-list-does-not-mix-positional-and-named.sysml and
    // argument-list-does-not-mix-named-and-positional.sysml.
    parse_rejected("calc def C { f(a, q = 1) }");
    parse_rejected("calc def C { f(q = 1, a) }");
    // No trailing comma: the list is `ArgumentMember ( ',' ArgumentMember )*`, unlike a
    // SequenceExpressionList, which states a trailing `','?` and does admit `( a , )`.
    // Held as a file by tests/rejection/argument-list-takes-no-trailing-comma.sysml.
    parse_rejected("calc def C { f(a,) }");
    // An unclosed list is still an error.
    parse_rejected("calc def C { f(a }");
}

// -- ConstructorExpression, KerML 8.2.5.8.3 ----------------------------------------
//
// ConstructorExpression   = 'new' InstantiatedTypeMember ConstructorResultMember
// ConstructorResultMember = ConstructorResult
// ConstructorResult       = ArgumentList
//
// BaseExpression's other alternative ending in an ArgumentList: "the keyword new followed
// by the qualified name of a type to be instantiated ... followed by a parenthesized list
// of argument expressions, similarly to an invocation expression" (KerML 7.4.9.4, receipt
// f77ceb64).

#[test]
fn a_constructor_expression_reads_the_corpus_forms() {
    // examples/Simple Tests/ParameterTest.sysml writes `new A(y=a, x="")`: named.
    let named = render(&parse_accepted("calc def C { new A(y = a, x = \"\") }").syntax());
    // NO EmptyResultMember among its own children, unlike InvocationExpression: 8.2.5.8.3
    // names one in the invocation and not here, where the ConstructorResultMember is the
    // result. (The argument `a` owns one, as every FeatureReferenceExpression does.)
    assert_eq!(
        child_kinds(&named, "ConstructorExpression"),
        ["KwNew", "InstantiatedTypeMember", "ConstructorResultMember"],
        "{named}"
    );
    assert_eq!(
        child_kinds(&named, "ConstructorResultMember"),
        ["ConstructorResult"],
        "{named}"
    );
    assert_eq!(
        child_kinds(&named, "ConstructorResult"),
        ["ArgumentList"],
        "{named}"
    );
    // training/25. Transitions/Local Clock Example.sysml:8 — a qualified type, no
    // arguments, as a feature value.
    parse_accepted("part def P { part :>> localClock = new Time::Clock(); }");
    // examples/Vehicle Example/SysML v2 Spec Annex A SimpleVehicleModel.sysml:1328 — a
    // space before the list, and a named argument whose value is qualified.
    parse_accepted("calc def C { new IgnitionCmd (ignitionOnOff=IgnitionOnOff::on) }");
    // validation/09-Verification/9-Verification-simplified.sysml:72 — nested in an
    // invocation's named argument.
    parse_accepted(
        "calc def C { PassIf(vehicleMassRequirement(vehicle = new testVehicle(mass = massProcessed))) }",
    );
}

#[test]
fn a_constructor_expression_is_bounded_by_its_rules() {
    // The ArgumentList is not optional: ConstructorResult = ArgumentList. Held as a file
    // by tests/rejection/constructor-expression-needs-an-argument-list.sysml.
    parse_rejected("calc def C { new A }");
    // `new` takes a type, not an expression.
    parse_rejected("calc def C { new 1() }");
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
