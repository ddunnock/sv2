// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! The `KerML` start symbol, checked against `KerML` 8.2.3.4.1.
//!
//! ```text
//! RootNamespace        = NamespaceBodyElement*
//! NamespaceBodyElement = NamespaceMember | AliasMember | Import
//! NamespaceMember      = NonFeatureMember | NamespaceFeatureMember
//! NonFeatureMember     = MemberPrefix MemberElement
//! MemberElement        = AnnotatingElement | NonFeatureElement
//! PackageBody          = ';' | '{' ( NamespaceBodyElement | ElementFilterMember )* '}'
//! ```
//!
//! `KerML` and `SysML` are two grammars with two start symbols, not one grammar that
//! extends the other (ADR-0014). What this file holds down is the difference: the same
//! text is read against a different grammar, and constructs one language has are not
//! silently borrowed by the other.
//!
//! Of `NonFeatureElement`'s alternatives, `Package`, `Dependency` and the eight
//! classifiers of `KerML` 8.2.4.2 are implemented. `Package` is a shared unit — the same
//! production in both grammars — `Dependency` is stated in each, and the classifiers are
//! `KerML`'s alone. Of `FeatureElement`'s ten alternatives, `Feature`, `Step`,
//! `Connector`, `BindingConnector` and `Succession` are implemented; the other five are
//! not, nor are `Type`,
//! `Function` and `Predicate`, and the cases below say so rather than pretending they
//! parse.

use std::fmt::Write as _;

use sv2_syntax::{Language, Parse, SyntaxElement, SyntaxNode, parse};

/// Parse text the `KerML` grammar accepts, asserting that nothing was reported.
fn kerml_accepted(source: &str) -> Parse {
    let parsed = parse(source, Language::KerMl);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(parsed.text(), source, "the tree is still lossless");
    parsed
}

/// Parse text the `KerML` grammar rejects, asserting it is reported losslessly.
fn kerml_rejected(source: &str) -> Parse {
    let parsed = parse(source, Language::KerMl);
    assert!(
        !parsed.errors().is_empty(),
        "{source:?} must be reported as an error"
    );
    assert_eq!(parsed.text(), source, "the tree is still lossless");
    parsed
}

// -- what a KerML root admits -----------------------------------------------------

#[test]
fn a_package_is_a_non_feature_element_in_both_grammars() {
    // Package is a shared unit: one production, both grammars. It is the only
    // NonFeatureElement this parser implements, so it is the whole of what a KerML
    // root currently accepts by way of a member.
    kerml_accepted("package Classes;");
    kerml_accepted("package <C> Classes { package Inner; }");
}

#[test]
fn alias_and_import_are_namespace_body_elements() {
    // NamespaceBodyElement's other two alternatives. Both are stated the same way in
    // the two grammars, which is why they are read without asking which body this is.
    kerml_accepted("public import A::B;");
    kerml_accepted("alias X for Y;");
}

// -- what a KerML root does not admit ---------------------------------------------

#[test]
fn a_part_definition_is_not_reachable_from_the_kerml_start_symbol() {
    // The case ADR-0014 exists for. `part def` is a SysML DefinitionElement; neither
    // MemberElement nor FeatureElement reaches it. Held as a file by
    // tests/rejection/part-definition-is-not-a-kerml-element.kerml.
    kerml_rejected("part def Vehicle;");
    // And the same text under the grammar that does state it.
    let sysml = parse("part def Vehicle;", Language::SysMl);
    assert!(
        sysml.errors().is_empty(),
        "the same text is SysML: {:?}",
        sysml.errors()
    );
}

#[test]
fn a_usage_is_not_reachable_from_the_kerml_start_symbol() {
    // UsageElement is reached from PackageMember and from no KerML production.
    // SysML 8.2.2.6.1; KerML has no usages at all.
    for source in ["part engine;", "attribute mass;", "item widget;"] {
        kerml_rejected(source);
        let sysml = parse(source, Language::SysMl);
        assert!(
            sysml.errors().is_empty(),
            "{source:?} is SysML: {:?}",
            sysml.errors()
        );
    }
}

#[test]
fn a_variant_is_not_reachable_from_a_kerml_body() {
    // VariantUsageMember is an item of SysML's definition and action bodies (SysML
    // 8.2.2.6.1, 8.2.2.17.1), and TypeBodyElement has no such alternative (KerML 8.2.4.1.1).
    kerml_rejected("classifier C { variant x; }");
    kerml_rejected("classifier C { variant feature f; }");
    let sysml = parse("part def C { variant x; }", Language::SysMl);
    assert!(sysml.errors().is_empty(), "{:?}", sysml.errors());
}

#[test]
fn a_conjugated_port_typing_is_not_kerml() {
    // KerML's TypedBy takes an OwnedFeatureTyping alone (KerML 8.2.4.3.1); the `~` form
    // is SysML's ConjugatedPortTyping (SysML 8.2.2.12), and KerML has no ports.
    kerml_rejected("feature f : ~T;");
    let sysml = parse("port p : ~T;", Language::SysMl);
    assert!(sysml.errors().is_empty(), "{:?}", sysml.errors());
}

// -- Dependency, KerML 8.2.3.2 ----------------------------------------------------
//
// Dependency = PrefixMetadataAnnotation* 'dependency' ( Identification? 'from' )?
//     client += [QualifiedName] ( ',' client += [QualifiedName] )* 'to'
//     supplier += [QualifiedName] ( ',' supplier += [QualifiedName] )*
//     RelationshipBody                                                     (8.2.3.2)
//
// A NonFeatureElement (8.2.3.4.3). The declaration is written inline, with no
// DependencyDeclaration of its own: that production is SysML's (8.2.2.3).

#[test]
fn a_kerml_dependency_reads_the_corpus_forms() {
    // examples/Simple Tests/Dependencies.kerml:11-12.
    let tree = render(
        &kerml_accepted(
            "package D {\n\
             \tdependency Use from 'Application Layer' to 'Service Layer';\n\
             \tdependency from 'Service Layer' to 'Data Layer';\n\
             \tdependency z to x, y;\n\
             }",
        )
        .syntax(),
    );
    assert_eq!(
        tree.lines().filter(|l| l.trim() == "Dependency").count(),
        3,
        "{tree}"
    );
    assert!(!tree.contains("DependencyDeclaration"), "{tree}");
    kerml_accepted("classifier C { dependency a to b; }");
    kerml_rejected("dependency Use a to b;");
    kerml_rejected("dependency a to b.c;");
}

// -- RelationshipBody, KerML 8.2.3.1 ----------------------------------------------
//
// RelationshipBody         = ';' | '{' RelationshipOwnedElement* '}'
// RelationshipOwnedElement = ownedRelatedElement += OwnedRelatedElement
//                          | ownedRelationship += OwnedAnnotation
// OwnedRelatedElement      = NonFeatureElement | FeatureElement            (8.2.3.1)
//
// NOT SysML's RelationshipBody, which owns annotations only (SysML 8.2.2.2). An owned
// related element is owned by the relationship directly, with no Membership, so no
// MemberPrefix is written before it.

#[test]
fn a_kerml_relationship_body_owns_related_elements() {
    // examples/Simple Tests/Dependencies.kerml:18-20.
    let tree = render(&kerml_accepted("dependency z to x, y {\n\tfeature e;\n}").syntax());
    assert_eq!(
        child_kinds(&tree, "RelationshipBody"),
        ["LBrace", "Feature", "RBrace"],
        "{tree}"
    );
    // The other two implemented relationships that end in the body, an import
    // (8.2.3.4.2) and an alias (8.2.3.4.1), holding NonFeatureElements, FeatureElements
    // and an annotation.
    kerml_accepted("public import A::* { class C; succession s first a then b; }");
    kerml_accepted("alias X for Y { doc /* d */ classifier K; package P; }");
    // A comment is an annotation in the body, and trivia again inside a nested one.
    let nested =
        render(&kerml_accepted("dependency a to b { /* c */ class C { /* n */ } }").syntax());
    assert_eq!(
        child_kinds(&nested, "RelationshipBody"),
        ["LBrace", "OwnedAnnotation", "Class", "RBrace"],
        "{nested}"
    );
}

#[test]
fn a_kerml_relationship_body_is_bounded_by_its_rules() {
    // No Membership, so no MemberPrefix.
    kerml_rejected("dependency a to b { private feature e; }");
    // AliasMember and Import are NamespaceBodyElements, not OwnedRelatedElements.
    kerml_rejected("dependency a to b { alias X for Y; }");
    kerml_rejected("dependency a to b { public import A; }");
    // SysML elements are not KerML's.
    kerml_rejected("dependency a to b { part p; }");
    // Unclosed.
    kerml_rejected("dependency a to b { feature e;");
    // SysML's body is unchanged: annotations only (SysML 8.2.2.2).
    let sysml = parse("dependency a to b { attribute e; }", Language::SysMl);
    assert!(!sysml.errors().is_empty());
}

#[test]
fn a_filter_is_admitted_in_a_kerml_package_body_and_not_at_a_kerml_root() {
    // The asymmetry that keeps "which member" and "admits a filter" separate
    // questions. PackageBody@kerml adds ElementFilterMember; NamespaceBodyElement
    // does not have it, and RootNamespace is NamespaceBodyElement*.
    kerml_accepted("package P { filter @Safety; }");
    kerml_rejected("filter @Safety;");
    // SysML admits it in both places, because there it IS a PackageBodyElement.
    for source in ["package P { filter @Safety; }", "filter @Safety;"] {
        let sysml = parse(source, Language::SysMl);
        assert!(
            sysml.errors().is_empty(),
            "{source:?} is SysML: {:?}",
            sysml.errors()
        );
    }
}

#[test]
fn the_other_feature_elements_are_unimplemented_rather_than_accepted() {
    // NamespaceFeatureMember reaches FeatureElement's ten alternatives. Feature,
    // Succession, BindingConnector and Connector are implemented, and Step is next
    // (`step s;` was here); the other five are not, and reporting them is the honest
    // state. `succession flow` is SuccessionFlow, one of the five.
    for source in ["succession flow f from a to b;", "inv { true }"] {
        kerml_rejected(source);
    }
}

// -- classifiers, KerML 8.2.4.2 ---------------------------------------------------
//
// Classifier  = TypePrefix 'classifier'  ClassifierDeclaration TypeBody
// Class       = TypePrefix 'class'       ClassifierDeclaration TypeBody
// Structure   = TypePrefix 'struct'      ClassifierDeclaration TypeBody
// DataType    = TypePrefix 'datatype'    ClassifierDeclaration TypeBody
// Metaclass   = TypePrefix 'metaclass'   ClassifierDeclaration TypeBody
// Association = TypePrefix 'assoc'       ClassifierDeclaration TypeBody
// Behavior    = TypePrefix 'behavior'    ClassifierDeclaration TypeBody
// Interaction = TypePrefix 'interaction' ClassifierDeclaration TypeBody

/// Every keyword of the shared spine, taken from the derived units rather than from
/// the clause prose, because the units are what the parser's table transcribes.
const CLASSIFIER_KEYWORDS: [&str; 8] = [
    "classifier",
    "class",
    "struct",
    "datatype",
    "metaclass",
    "assoc",
    "behavior",
    "interaction",
];

#[test]
fn every_classifier_keyword_reads_the_same_spine() {
    for keyword in CLASSIFIER_KEYWORDS {
        kerml_accepted(&format!("{keyword} A;"));
        kerml_accepted(&format!("{keyword} A {{ }}"));
        kerml_accepted(&format!("abstract {keyword} <a> A;"));
        kerml_accepted(&format!("{keyword} all A;"));
    }
}

#[test]
fn a_classifier_may_specialize_in_either_spelling() {
    // SuperclassingPart = SPECIALIZES OwnedSubclassification
    //                     ( ',' OwnedSubclassification )*
    // SPECIALIZES = ':>' | 'specializes'
    kerml_accepted("class B :> A;");
    kerml_accepted("class B specializes A;");
    kerml_accepted("class B :> A::C, D;");
}

#[test]
fn a_type_body_holds_the_members_a_namespace_body_holds() {
    // TypeBodyElement = NonFeatureMember | FeatureMember | AliasMember | Import.
    // FeatureMember is unimplemented; the other three are the same productions a
    // NamespaceBodyElement owns, so they are read here too.
    kerml_accepted("class A { class B; }");
    kerml_accepted("class A { public import X::*; }");
    kerml_accepted("class A { alias Y for Z; }");
    kerml_accepted("package P { class A { struct S; } }");
}

#[test]
fn a_type_body_does_not_admit_a_filter() {
    // TypeBodyElement has no ElementFilterMember alternative, and unlike PackageBody
    // nothing adds one.
    kerml_rejected("class A { filter @Safety; }");
}

#[test]
fn a_classifier_is_not_reachable_from_the_sysml_start_symbol() {
    // The mirror of the part-def case: every classifier unit is scoped `kerml`.
    // Held as a file by tests/rejection/a-classifier-is-not-a-sysml-element.sysml.
    for keyword in CLASSIFIER_KEYWORDS {
        let source = format!("{keyword} A;");
        let sysml = parse(&source, Language::SysMl);
        assert!(
            !sysml.errors().is_empty(),
            "{source:?} is not SysML and must be reported"
        );
        assert_eq!(sysml.text(), source, "the tree is still lossless");
    }
}

#[test]
fn a_function_is_not_in_the_classifier_table() {
    // Function and Predicate share the keyword-and-declaration shape but take a
    // FunctionBody, so they are not the same spine and are not implemented. Reading
    // them with this table would accept a body the language does not put there.
    kerml_rejected("function f;");
    kerml_rejected("predicate p;");
    // Type takes a TypeDeclaration rather than a ClassifierDeclaration.
    kerml_rejected("type T;");
}

// -- annotating elements as members, KerML 8.2.3.4.1 ------------------------------

#[test]
fn an_annotating_element_is_a_member_element() {
    // MemberElement = AnnotatingElement | NonFeatureElement. The three implemented
    // annotating elements are reachable at every KerML member position, which is the
    // root, a package body and a type body alike.
    kerml_accepted("doc /* what this file is */");
    kerml_accepted("comment C about X /* on X */");
    kerml_accepted("rep r language \"alf\" /* f(); */");
    kerml_accepted("package P { doc /* on P */ }");
    kerml_accepted("class A { doc /* on A */ }");
}

#[test]
fn a_metadata_annotating_element_is_not_implemented() {
    // AnnotatingElement's fourth alternative, the one the two grammars spell
    // differently. KerML says MetadataFeature, which is unimplemented: a rejection by
    // absence. SysML's MetadataUsage is read, and only in a .sysml file.
    kerml_rejected("metadata M about X;");
}

// -- Feature, KerML 8.2.4.3.1 -----------------------------------------------------
//
// Feature = ( FeaturePrefix ( 'feature' | PrefixMetadataMember ) FeatureDeclaration?
//           | ( EndFeaturePrefix | BasicFeaturePrefix ) FeatureDeclaration
//           ) ValuePart? TypeBody
//
// The keyword form is read; the keywordless one is not.

#[test]
fn a_feature_reads_its_declaration_and_body() {
    kerml_accepted("feature f;");
    kerml_accepted("feature f : A;");
    kerml_accepted("feature f typed by A;");
    kerml_accepted("feature <f> vitesse : Speed;");
    kerml_accepted("feature f { feature g : B; }");
    kerml_accepted("feature all f : A;");
}

#[test]
fn a_feature_declaration_is_optional_after_the_keyword() {
    // FeatureDeclaration? — the `?` is on the keyword alternative and nowhere else.
    kerml_accepted("feature;");
    kerml_accepted("feature : A;");
}

#[test]
fn every_basic_feature_prefix_keyword_is_read() {
    // BasicFeaturePrefix = FeatureDirection? 'derived'? 'abstract'?
    //                      ( 'composite' | 'portion' )? ( 'var' | 'const' )?
    for prefix in [
        "in",
        "out",
        "inout",
        "derived",
        "abstract",
        "composite",
        "portion",
        "var",
        "const",
    ] {
        kerml_accepted(&format!("{prefix} feature f : A;"));
    }
    kerml_accepted("in derived abstract composite var feature f : A;");
}

#[test]
fn the_two_prefix_alternations_foreclose() {
    // `composite | portion` and `var | const` are alternations, so taking one rules
    // out the other: the second word is left for the caller to report.
    kerml_rejected("composite portion feature f;");
    kerml_rejected("var const feature f;");
}

#[test]
fn an_end_feature_prefix_is_told_from_a_const_basic_prefix() {
    // EndFeaturePrefix = 'const'? 'end', and BasicFeaturePrefix's last slot is also
    // `const`. Only the `end` tells them apart.
    kerml_accepted("end feature f : A;");
    kerml_accepted("const end feature f : A;");
    kerml_accepted("const feature f : A;");
}

#[test]
fn typed_by_is_spelled_differently_in_the_two_grammars() {
    // TYPED_BY = ':' | 'typed' 'by'     (KerML 8.2.4.3.1)
    // DEFINED_BY = ':' | 'defined' 'by' (SysML 8.2.2.1.2)
    // The `:` form is shared; the word form is not, and each grammar rejects the
    // other's.
    kerml_accepted("feature f typed by A;");
    kerml_rejected("feature f defined by A;");

    let sysml = parse("part p defined by A;", Language::SysMl);
    assert!(sysml.errors().is_empty(), "{:?}", sysml.errors());
    let wrong = parse("part p typed by A;", Language::SysMl);
    assert!(
        !wrong.errors().is_empty(),
        "`typed by` is not SysML's spelling"
    );
}

#[test]
fn a_feature_is_owned_through_a_namespace_feature_member() {
    // NamespaceMember = NonFeatureMember | NamespaceFeatureMember (KerML 8.2.3.4.1).
    // A feature takes the second; a package or a classifier takes the first.
    let feature = render(&kerml_accepted("feature f : A;").syntax());
    assert!(has_node(&feature, "NamespaceFeatureMember"), "{feature}");
    assert!(!has_node(&feature, "NonFeatureMember"), "{feature}");

    let class = render(&kerml_accepted("class A;").syntax());
    assert!(has_node(&class, "NonFeatureMember"), "{class}");
    assert!(!has_node(&class, "NamespaceFeatureMember"), "{class}");
}

#[test]
fn a_feature_may_be_written_with_no_keyword_at_all() {
    // Feature's second alternative: the declaration alone carries it. This is what the
    // corpus writes after a prefix — `composite vitesse : Speed;` — and it was the top
    // KerML blocker once the keyword form landed.
    kerml_accepted("vitesse : Speed;");
    kerml_accepted("f;");
    kerml_accepted("composite vitesse : Speed;");
    kerml_accepted("portion p : Q;");
    kerml_accepted("<v> vitesse : Speed;");
}

#[test]
fn the_keywordless_form_requires_a_declaration() {
    // The asymmetry between the two alternatives: FeatureDeclaration is optional after
    // the keyword and required without it. `end;` is EndFeaturePrefix and nothing else,
    // which is what tests/rejection/end-feature-requires-a-declaration.kerml holds.
    kerml_rejected("end;");
    kerml_accepted("end f;");
    kerml_accepted("feature;");
}

#[test]
fn a_keyword_is_not_mistaken_for_a_feature_name() {
    // KerML 8.2.2.6: a reserved keyword has the shape of a basic name and cannot be
    // used as one. Without that, every `package P;` would read as a keywordless feature
    // called `package`.
    let package = render(&kerml_accepted("package P;").syntax());
    assert!(has_node(&package, "Package"), "{package}");
    assert!(!has_node(&package, "Feature"), "{package}");

    let class = render(&kerml_accepted("class A;").syntax());
    assert!(has_node(&class, "Class"), "{class}");
    assert!(!has_node(&class, "Feature"), "{class}");
}

/// Whether `rendered` contains a node of exactly this kind.
///
/// A substring test is not enough: `NonFeatureMember` contains `Feature`, and
/// `Classifier` contains `Class`. Every rendered line is a kind and its depth, so the
/// kind is the trimmed line up to the first space.
fn has_node(rendered: &str, kind: &str) -> bool {
    rendered
        .lines()
        .any(|line| line.trim().split(' ').next() == Some(kind))
}

// -- Succession, KerML 8.2.5.5.3 -------------------------------------------------
//
// Succession = FeaturePrefix 'succession' SuccessionDeclaration TypeBody
//
// SuccessionDeclaration =
//     FeatureDeclaration ( 'first' ConnectorEndMember 'then' ConnectorEndMember )?
//   | 'all'? ( 'first'? ConnectorEndMember 'then' ConnectorEndMember )?

#[test]
fn a_succession_reads_the_corpus_forms() {
    // "Behavior Examples/Camera.kerml" line 7: the second alternative, no `first`.
    kerml_accepted("class Camera { succession focusedState then shotState; }");
    // "Simple Tests/Connectors.kerml" lines 22-28: all four shapes the file writes —
    // ends alone, a name and ends, the EMPTY declaration with a body, and a typed name.
    kerml_accepted("succession a then b;");
    kerml_accepted("succession s first a then b;");
    kerml_accepted("succession {\n\tend feature references a;\n\tend feature references b;\n}");
    kerml_accepted("succession s1 : AS first a then b;");
    // "KerML Spec Annex A Examples/A-3-7-DecisionsAndMerges.kerml" line 112, less the
    // cross multiplicity on the ends that line does not write: a declaration that is a
    // bare FeatureSpecializationPart with a multiplicity.
    kerml_accepted("succession redefines a_before_i : Link [1] first admit then inspect;");
}

#[test]
fn a_succession_declaration_takes_either_alternative_by_what_follows_all() {
    // The second alternative: `first` there, or an end and then `then`, or nothing.
    for source in [
        "succession first a then b;",
        "succession a::b.c then d;",
        "succession x references a then b;",
        "succession all first a then b;",
        "succession all a then b;",
        "succession all;",
        "succession;",
    ] {
        let tree = render(&kerml_accepted(source).syntax());
        assert!(has_node(&tree, "SuccessionDeclaration"), "{tree}");
        assert!(!has_node(&tree, "FeatureDeclaration"), "{source}: {tree}");
    }
    // The first: anything else after `all` is a FeatureDeclaration.
    for source in [
        "succession s;",
        "succession s first a then b;",
        "succession all s first a then b;",
        "succession : T;",
        "succession <s> first a then b;",
    ] {
        let tree = render(&kerml_accepted(source).syntax());
        assert!(has_node(&tree, "FeatureDeclaration"), "{source}: {tree}");
    }
}

#[test]
fn a_succession_owns_what_its_production_writes() {
    let tree = render(&kerml_accepted("abstract succession s first a then b { }").syntax());
    assert!(has_node(&tree, "NamespaceFeatureMember"), "{tree}");
    assert!(has_node(&tree, "Succession"), "{tree}");
    assert!(has_node(&tree, "FeaturePrefix"), "{tree}");
    assert!(
        !has_node(&tree, "Feature"),
        "a succession is not read as a Feature: {tree}"
    );
    assert_eq!(
        tree.lines()
            .filter(|line| line.trim().split(' ').next() == Some("ConnectorEndMember"))
            .count(),
        2,
        "{tree}"
    );
}

#[test]
fn a_succession_is_bounded_by_its_rules() {
    // The ends are a pair: no source without `then` and a target.
    kerml_rejected("succession first a;");
    kerml_rejected("succession a then;");
    kerml_rejected("succession s first a;");
    // `first` belongs to the ends, not to the declaration: no second one.
    kerml_rejected("succession s first first a then b;");
    // TypeBody is not optional.
    kerml_rejected("succession a then b");
    // No keywordless form in KerML; that is SysML's SuccessionAsUsage (ADR-0014).
    kerml_rejected("first a then b;");
    // `succession first [1] a then b;` was here, rejected by absence while KerML's
    // OwnedMultiplicity was unimplemented; the next commit reads it.
}

#[test]
fn a_kerml_succession_is_not_reachable_from_the_sysml_start_symbol() {
    // The SysML succession needs `first`, so the second alternative's `succession a then
    // b;` is KerML's alone.
    let sysml = parse("part def P { succession a then b; }", Language::SysMl);
    assert!(
        !sysml.errors().is_empty(),
        "SysML states no such succession"
    );
}

#[test]
fn parsing_a_succession_never_hangs_or_loses_bytes_on_truncated_input() {
    let source = "class C { abstract succession s : T first a.b then x references c { } \
                  succession all d then e; succession; }";
    for end in 0..=source.len() {
        if let Some(prefix) = source.get(..end) {
            assert_eq!(parse(prefix, Language::KerMl).text(), prefix);
        }
    }
}

// -- BindingConnector, KerML 8.2.5.5.2 -------------------------------------------
//
// BindingConnector = FeaturePrefix 'binding' BindingConnectorDeclaration TypeBody
//
// BindingConnectorDeclaration =
//     FeatureDeclaration ( 'of' ConnectorEndMember '=' ConnectorEndMember )?
//   | 'all'? ( 'of'? ConnectorEndMember '=' ConnectorEndMember )?
//
// Succession's shape with `binding` for `succession`, `of` for `first` and `=` for
// `then`. "If a binding connector declaration includes only the related features part,
// then the keyword of can be omitted" (7.4.6.3, receipt cda2047f).

#[test]
fn a_binding_connector_reads_the_corpus_forms() {
    // "Simple Tests/Connectors.kerml" lines 14-20: all four shapes the file writes —
    // ends alone, a name and ends, the EMPTY declaration with its ends declared in the
    // body, and a typed name.
    kerml_accepted("binding a = b;");
    kerml_accepted("binding ab of a = b;");
    kerml_accepted("binding {\n\tend feature references a;\n\tend feature references b;\n}");
    kerml_accepted("binding ab1 : AS of a = b;");
    // "Variable Feature Examples/Enhancements/ExtendedOccurrences.kerml" line 21 — a
    // feature chain as the source end.
    kerml_accepted("binding result.portionOf = that;");
    // 7.4.6.3's own example (receipt cda2047f), in a classifier body.
    kerml_accepted(
        "struct Vehicle { binding fuelFlowBinding of fuelTank.fuelFlowOut = engine.fuelFlowIn; \
         binding fuelTank.fuelFlowOut = engine.fuelFlowIn; }",
    );
}

#[test]
fn a_binding_connector_declaration_takes_either_alternative_by_what_follows_all() {
    // The second alternative: `of` there, or an end and then `=`, or nothing.
    for source in [
        "binding of a = b;",
        "binding a::b.c = d;",
        "binding x references a = b;",
        "binding all of a = b;",
        "binding all a = b;",
        "binding all;",
        "binding;",
    ] {
        let tree = render(&kerml_accepted(source).syntax());
        assert!(has_node(&tree, "BindingConnectorDeclaration"), "{tree}");
        assert!(!has_node(&tree, "FeatureDeclaration"), "{source}: {tree}");
    }
    // The first: anything else after `all` is a FeatureDeclaration — a named binding
    // with no ends is one, since its ends may be declared in its body.
    for source in [
        "binding b;",
        "binding b of a = c;",
        "binding all b of a = c;",
        "binding : T;",
        "binding <b> of a = c;",
    ] {
        let tree = render(&kerml_accepted(source).syntax());
        assert!(has_node(&tree, "FeatureDeclaration"), "{source}: {tree}");
    }
}

#[test]
fn a_binding_connector_owns_what_its_production_writes() {
    let tree = render(&kerml_accepted("abstract binding b of a = c { }").syntax());
    assert!(has_node(&tree, "NamespaceFeatureMember"), "{tree}");
    assert!(has_node(&tree, "BindingConnector"), "{tree}");
    assert!(has_node(&tree, "FeaturePrefix"), "{tree}");
    assert!(
        !has_node(&tree, "Feature"),
        "a binding connector is not read as a Feature: {tree}"
    );
    assert_eq!(
        tree.lines()
            .filter(|line| line.trim().split(' ').next() == Some("ConnectorEndMember"))
            .count(),
        2,
        "{tree}"
    );
}

#[test]
fn a_binding_connector_is_bounded_by_its_rules() {
    // The ends are a pair: no source without `=` and a target.
    kerml_rejected("binding of a;");
    kerml_rejected("binding a =;");
    kerml_rejected("binding b of a;");
    // `of` belongs to the ends, not to the declaration: no second one.
    kerml_rejected("binding b of of a = c;");
    // TypeBody is not optional.
    kerml_rejected("binding a = b");
    // `bind` is SysML's BindingConnectorAsUsage, not a KerML keyword (ADR-0014).
    kerml_rejected("bind a = b;");
    // `binding of [1] a = b;` was here, rejected by absence as the succession's was.
}

#[test]
fn a_kerml_binding_connector_is_not_reachable_from_the_sysml_start_symbol() {
    // The SysML binding needs `bind`, and writes no `of`.
    for source in [
        "part def P { binding a = b; }",
        "part def P { binding b of a = c; }",
    ] {
        let sysml = parse(source, Language::SysMl);
        assert!(
            !sysml.errors().is_empty(),
            "SysML states no such binding: {source}"
        );
    }
}

#[test]
fn parsing_a_binding_connector_never_hangs_or_loses_bytes_on_truncated_input() {
    let source = "class C { abstract binding b : T of a.b = x references c { } \
                  binding all d = e; binding; binding { end feature references f; } }";
    for end in 0..=source.len() {
        if let Some(prefix) = source.get(..end) {
            assert_eq!(parse(prefix, Language::KerMl).text(), prefix);
        }
    }
}

// -- Connector, KerML 8.2.5.5.1 ----------------------------------------------------
//
//   Connector = FeaturePrefix 'connector'
//               ( FeatureDeclaration? ValuePart? | ConnectorDeclaration ) TypeBody
//   ConnectorDeclaration       = BinaryConnectorDeclaration | NaryConnectorDeclaration
//   BinaryConnectorDeclaration = ( FeatureDeclaration? 'from' | 'all' 'from'? )?
//                                ConnectorEndMember 'to' ConnectorEndMember
//   NaryConnectorDeclaration   = FeatureDeclaration?
//                                '(' ConnectorEndMember ',' ConnectorEndMember
//                                    ( ',' ConnectorEndMember )* ')'

#[test]
fn the_connector_examples_of_7_4_6_2_parse() {
    // 7.4.6.2's connector declarations (receipt d8abbbc3), each in its example's struct.
    // The binary form with and without a declaration, `from` omitted, chained ends, cross
    // multiplicities, and the n-ary form with and without association end names.
    for (item, form) in [
        (
            "connector mount : Mounting from axle to wheels;",
            "BinaryConnectorDeclaration",
        ),
        ("connector axle to wheels;", "BinaryConnectorDeclaration"),
        (
            "connector mount[2] : Mounting\n    from mountingAxle ::> axle\n      to mountedWheel ::> wheels;",
            "BinaryConnectorDeclaration",
        ),
        (
            "connector mount[2] : Mounting from [1] halfAxles to [1] wheels;",
            "BinaryConnectorDeclaration",
        ),
        (
            "connector mount : Mounting from axle.halfAxles to wheels.hub;",
            "BinaryConnectorDeclaration",
        ),
        (
            "connector mount[2] : Mounting (axle, wheels);",
            "NaryConnectorDeclaration",
        ),
        (
            "connector mount[2] : Mounting (\n    mountingAxle ::> axle,\n    mountedWheel ::> wheels\n);",
            "NaryConnectorDeclaration",
        ),
    ] {
        let source = format!("struct WheelAssembly {{ {item} }}");
        let tree = render(&kerml_accepted(&source).syntax());
        assert_eq!(
            child_kinds(&tree, "Connector"),
            ["FeaturePrefix", "KwConnector", form, "TypeBody"],
            "{source}\n{tree}"
        );
    }
}

#[test]
fn a_binary_connector_declaration_owns_what_its_production_writes() {
    let decl = |item: &str| {
        let tree = render(&kerml_accepted(&format!("struct S {{ {item} }}")).syntax());
        child_kinds(&tree, "BinaryConnectorDeclaration")
    };
    let ends = ["ConnectorEndMember", "KwTo", "ConnectorEndMember"];
    // examples/Simple Tests/Connectors.kerml:7 — declared, then `from`.
    assert_eq!(
        decl("connector c1 from a to b;"),
        [&["FeatureDeclaration", "KwFrom"][..], &ends].concat()
    );
    // `from` with nothing declared before it.
    assert_eq!(
        decl("connector from a to b;"),
        [&["KwFrom"][..], &ends].concat()
    );
    // Named Collection Members Example/VehicleTanks.kerml:27 — the ends alone.
    assert_eq!(decl("connector eng to tanks.main1;"), ends);
    // `all`, with and without the optional `from`.
    assert_eq!(
        decl("connector all from a to b;"),
        [&["KwAll", "KwFrom"][..], &ends].concat()
    );
    assert_eq!(
        decl("connector all a to b;"),
        [&["KwAll"][..], &ends].concat()
    );
    // `all` before a declaration is the declaration's own.
    assert_eq!(
        decl("connector all c from a to b;"),
        [&["FeatureDeclaration", "KwFrom"][..], &ends].concat()
    );
    // Simple Tests/ArgumentResolution.kerml:15 — an end that names itself.
    kerml_accepted("struct S { connector a ::> a.x to b; }");
}

#[test]
fn a_connector_with_no_connector_declaration_is_a_feature_declaration_and_value() {
    // The first alternative, examples/Simple Tests/Connectors.kerml:8-9: a value and no
    // ends, the ends declared in the body or not at all.
    for (item, kinds) in [
        (
            "abstract connector c2 = c1;",
            &[
                "FeaturePrefix",
                "KwConnector",
                "FeatureDeclaration",
                "ValuePart",
                "TypeBody",
            ][..],
        ),
        (
            "connector = c2 { }",
            &["FeaturePrefix", "KwConnector", "ValuePart", "TypeBody"],
        ),
        ("connector;", &["FeaturePrefix", "KwConnector", "TypeBody"]),
        // A `(` after the `=` is the value's, an invocation: the n-ary form has no
        // ValuePart.
        (
            "connector c = f(a, b);",
            &[
                "FeaturePrefix",
                "KwConnector",
                "FeatureDeclaration",
                "ValuePart",
                "TypeBody",
            ],
        ),
        (
            "connector c : T;",
            &[
                "FeaturePrefix",
                "KwConnector",
                "FeatureDeclaration",
                "TypeBody",
            ],
        ),
    ] {
        let tree = render(&kerml_accepted(&format!("struct S {{ {item} }}")).syntax());
        assert_eq!(child_kinds(&tree, "Connector"), kinds, "{item}\n{tree}");
    }
}

#[test]
fn a_connector_keeps_every_byte() {
    let source = "struct S {\n\tconnector /* c */ m[2] : M\n\t\tfrom [1] a . b // n\n\t\tto b ;\n\tconnector ( a , b , c ) { }\n}\n";
    assert_eq!(kerml_accepted(source).text(), source);
}

#[test]
fn a_connector_is_bounded_by_its_rules() {
    // A binary connector names two ends, `to` between them. Held as a file by
    // tests/rejection/kerml-binary-connector-needs-to.kerml.
    kerml_rejected("struct S { connector c from a; }");
    kerml_rejected("struct S { connector c from a to b to d; }");
    // An n-ary list has at least two ends. Held as a file by
    // tests/rejection/kerml-nary-connector-needs-two-ends.kerml.
    kerml_rejected("struct S { connector (a); }");
    kerml_rejected("struct S { connector (a, b }");
    // TypeBody is not optional.
    kerml_rejected("struct S { connector a to b }");
    // `disjoint from` is a FeatureDeclaration's DisjoiningPart (KerML 8.2.4.1.1), not a
    // binary connector's `from`: the connector is the first alternative, a declaration
    // alone, and never begins a BinaryConnectorDeclaration.
    let tree = render(&kerml_accepted("struct S { connector c disjoint from d; }").syntax());
    assert!(!has_node(&tree, "BinaryConnectorDeclaration"), "{tree}");
    assert_eq!(
        child_kinds(&tree, "FeatureDeclaration"),
        ["FeatureIdentification", "DisjoiningPart"],
        "{tree}"
    );
    // A `from` after the part is the binary form's, and the part stays in its declaration.
    let binary =
        render(&kerml_accepted("struct S { connector c disjoint from d from a to b; }").syntax());
    assert_eq!(
        child_kinds(&binary, "BinaryConnectorDeclaration"),
        [
            "FeatureDeclaration",
            "KwFrom",
            "ConnectorEndMember",
            "KwTo",
            "ConnectorEndMember"
        ],
        "{binary}"
    );
    // SysML's connector is `connection`; `connector` is KerML's (ADR-0014).
    assert!(
        !parse("part def P { connector a to b; }", Language::SysMl)
            .errors()
            .is_empty()
    );
}

// -- ConnectionUsage is SysML's alone ---------------------------------------------

#[test]
fn a_connection_usage_is_not_kerml() {
    // SysML 8.2.2.13.1 states it; KerML's own connector is `connector` (8.2.5.5.1), and no
    // KerML production writes the terminal `connect` (KerML does not reserve the word, so
    // there it would be a NAME; see pending decision [keyword-table-per-language]). Held as a file by
    // tests/rejection/connection-usage-is-not-kerml.kerml.
    kerml_rejected("package P { connect a to b; }");
}

// -- ConstructorExpression, KerML 8.2.5.8.3 ----------------------------------------

#[test]
fn a_constructor_expression_is_kerml_too() {
    // A shared unit, stated in KerML's own clause: vendor/corpus/kerml/src/examples/
    // Simple Tests/Expressions.kerml:73 writes `feature l = new L();`.
    let tree = render(&kerml_accepted("package P { feature l = new L(); }").syntax());
    assert!(has_node(&tree, "ConstructorExpression"), "{tree}");
    kerml_rejected("package P { feature l = new L; }");
}

#[test]
fn an_invocation_through_a_feature_chain_is_kerml_too() {
    // InstantiatedTypeMember is a shared unit, and KerML's OwnedFeatureChainMember
    // (8.2.5.8.2) owns a FeatureChain (8.2.4.3.5) of the same text as SysML's
    // OwnedFeatureChain: vendor/corpus/kerml/src/examples/Simple Tests/Expressions.kerml:56
    // writes `bb : Boolean = f.s(1);`.
    let tree = render(&kerml_accepted("package P { feature bb : Boolean = f.s(1); }").syntax());
    assert!(has_node(&tree, "OwnedFeatureChainMember"), "{tree}");
    assert!(has_node(&tree, "InvocationExpression"), "{tree}");
    kerml_rejected("package P { feature bb = f.(1); }");
}

// -- FunctionOperationExpression, KerML 8.2.5.8.2 -----------------------------------

#[test]
fn a_function_operation_is_kerml_too() {
    // A shared unit. Its argument-list and function-reference forms reach nothing
    // SysML-specific: vendor/corpus/kerml/src/examples/Simple Tests/Expressions.kerml:19
    // ends `->reduce '+'`.
    let tree = render(&kerml_accepted("package P { feature e = x->reduce '+'; }").syntax());
    assert!(has_node(&tree, "FunctionReferenceArgumentMember"), "{tree}");
    let tree = render(&kerml_accepted("package P { feature s = x->size(); }").syntax());
    assert!(has_node(&tree, "FunctionOperationExpression"), "{tree}");
}

// -- IndexExpression, KerML 8.2.5.8.2 ---------------------------------------------

#[test]
fn an_index_expression_is_kerml_too() {
    // A shared unit: vendor/corpus/kerml/src/examples/Vehicle Example/VehicleUsages.kerml:49
    // indexes a qualified name, `vehicle_C1::frontAxleAssembly::frontWheel#(1)`.
    let tree = render(
        &kerml_accepted("package P { feature w = vehicle_C1::frontAxleAssembly::frontWheel#(1); }")
            .syntax(),
    );
    assert!(has_node(&tree, "IndexExpression"), "{tree}");
}

// -- FilterPackage, KerML 8.2.3.4.2 ----------------------------------------------

#[test]
fn a_kerml_filter_package_takes_its_declaration_directly() {
    // KerML's FilterPackage is `ImportDeclaration FilterPackageMember+` (8.2.3.4.2), with
    // no FilterPackageImport between: that production is SysML's (8.2.2.5.1).
    // vendor/corpus/kerml/src/examples/Simple Tests/Filtering.kerml:34-36, less its
    // `as` conditions.
    let tree = render(
        &kerml_accepted("package P { private import DesignModel::**[@Structure][x > 1]; }")
            .syntax(),
    );
    assert_eq!(
        child_kinds(&tree, "FilterPackage"),
        [
            "ImportDeclaration",
            "FilterPackageMember",
            "FilterPackageMember"
        ],
        "{tree}"
    );
    assert!(!tree.contains("FilterPackageImport"), "{tree}");
    kerml_rejected("package P { private import A::**[]; }");
}

#[test]
fn a_kerml_expression_body_is_not_read_yet() {
    // KerML's ExpressionBody is `'{' FunctionBodyPart '}'` (8.2.5.8.3), whose
    // FunctionBodyPart@kerml is unimplemented. SysML's reading, CalculationBody, is SysML's
    // alone (deviation ExpressionBody), so a .kerml body is reported rather than read as
    // SysML's. Expressions.kerml:15 writes `x->collect {in xx; xx + 1}`. Held as a file by
    // tests/rejection/kerml-expression-body-is-not-implemented.kerml.
    kerml_rejected("package P { feature c = x->collect {in xx; xx + 1}; }");
    // A body SysML's CalculationBody would read whole, in both positions that reach one,
    // so what rejects it is the `{` and not an item inside: the report is AT the brace.
    for source in [
        "package P { feature c = x->collect { 1 }; }",
        "package P { feature c = { 1 }; }",
        // Expressions.kerml:18 writes a select, `x.?{in xx; xx != null}`.
        "package P { feature d = x.?{ 1 }; }",
        // Expressions.kerml:16 writes a collect, `x.{in xx; xx + 1}`.
        "package P { feature c1 = x.{ 1 }; }",
    ] {
        let parsed = kerml_rejected(source);
        // The second `{`: the first is the package body's.
        let brace = source.match_indices('{').nth(1).map(|(at, _)| at);
        let first = parsed
            .errors()
            .first()
            .map(|d| usize::from(d.range().start()));
        assert_eq!(first, brace, "{source}: {:?}", parsed.errors());
    }
}

// -- multiplicity, KerML 8.2.5.11 ------------------------------------------------
//
//   OwnedMultiplicity      = ownedRelatedElement += OwnedMultiplicityRange
//   OwnedMultiplicityRange : MultiplicityRange = MultiplicityBounds
//   MultiplicityBounds     = '[' ( MultiplicityExpressionMember '..' )?
//                                 MultiplicityExpressionMember ']'
//
// SysML's OwnedMultiplicity owns a MultiplicityRange of the same text (SysML 8.2.2.6.6);
// in KerML the name MultiplicityRange is the named `multiplicity` declaration, so the
// language decides the node (ADR-0015).

#[test]
fn a_kerml_multiplicity_is_an_owned_multiplicity_range() {
    // A feature's MultiplicityPart (KerML 8.2.4.3.1), in KerML's own shape.
    let tree = render(&kerml_accepted("package P { feature x : T[0..*]; }").syntax());
    assert_eq!(
        child_kinds(&tree, "OwnedMultiplicity"),
        ["OwnedMultiplicityRange"],
        "{tree}"
    );
    // MultiplicityBounds is a fragment of the range it is written into: no node.
    assert_eq!(
        child_kinds(&tree, "OwnedMultiplicityRange"),
        [
            "LBracket",
            "MultiplicityExpressionMember",
            "DotDot",
            "MultiplicityExpressionMember",
            "RBracket"
        ],
        "{tree}"
    );
    assert!(!has_node(&tree, "MultiplicityRange"), "{tree}");
    // The upper bound alone.
    let one = render(&kerml_accepted("package P { feature x[1]; }").syntax());
    assert_eq!(
        child_kinds(&one, "OwnedMultiplicityRange"),
        ["LBracket", "MultiplicityExpressionMember", "RBracket"],
        "{one}"
    );
    // SysML keeps its own: the same text is a MultiplicityRange there.
    let sysml = render(&parse("part p[1];", Language::SysMl).syntax());
    assert!(has_node(&sysml, "MultiplicityRange"), "{sysml}");
    assert!(!has_node(&sysml, "OwnedMultiplicityRange"), "{sysml}");
}

#[test]
fn a_classifier_declaration_takes_a_multiplicity() {
    // ClassifierDeclaration's OwnedMultiplicity, after the Identification (KerML
    // 8.2.4.2.1): KerML Spec Annex A Examples/A-2-ModelingInstances.kerml:8 and
    // Individuals Examples/JohnIndividualExample.kerml:52.
    let tree = render(
        &kerml_accepted("package P { classifier MyBike [1] specializes Bicycle; }").syntax(),
    );
    assert_eq!(
        child_kinds(&tree, "ClassifierDeclaration"),
        ["Identification", "OwnedMultiplicity", "SuperclassingPart"],
        "{tree}"
    );
    kerml_accepted("package P { class all JohnLife[0..1] specializes John, Occurrences::Life; }");
    kerml_accepted("package P { struct S [*]; }");
    // After the Identification, once. Held as files by
    // tests/rejection/kerml-classifier-multiplicity-follows-identification.kerml and
    // tests/rejection/kerml-classifier-takes-one-multiplicity.kerml.
    kerml_rejected("package P { classifier [1] MyBike; }");
    kerml_rejected("package P { classifier MyBike [1] [2]; }");
    // A bound is a literal or a name, not an operator expression. Held as a file by
    // tests/rejection/kerml-multiplicity-bound-is-not-an-operator-expression.kerml.
    kerml_rejected("package P { classifier MyBike [1 + 1]; }");
}

#[test]
fn a_kerml_connector_end_takes_a_cross_multiplicity() {
    // ConnectorEnd's OwnedCrossMultiplicityMember, now read in KerML as in SysML: KerML
    // Spec Annex A Examples/A-3-6-Sequences.kerml:10 writes it on its successions.
    let tree = render(
        &kerml_accepted("class C { succession p_before_d first [1] paint then [1] dry; }").syntax(),
    );
    assert_eq!(
        child_kinds(&tree, "ConnectorEnd"),
        ["OwnedCrossMultiplicityMember", "OwnedReferenceSubsetting"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "OwnedCrossMultiplicity"),
        ["OwnedMultiplicity"],
        "{tree}"
    );
    assert!(has_node(&tree, "OwnedMultiplicityRange"), "{tree}");
    kerml_accepted("package P { binding of [1] a = b; }");
    // The multiplicity AFTER an end is SysML's deviation ConnectorEnd-trailing-multiplicity
    // alone, not KerML's.
    kerml_rejected("class C { succession first a[1] then b; }");
}

// -- TypeRelationshipPart, KerML 8.2.4.1.1 ----------------------------------------

#[test]
fn a_classifier_takes_type_relationship_parts() {
    // vendor/corpus/kerml/src/examples/Simple Tests/Classifiers.kerml:13-15, whole.
    let tree = render(
        &kerml_accepted(
            "classifier D disjoint from C differences A, B;\n\
             classifier E specializes C intersects A, B;\n\
             classifier F unions A unions B;",
        )
        .syntax(),
    );
    // TypeRelationshipPart is an alternation and builds no node; the part says which.
    assert!(!has_node(&tree, "TypeRelationshipPart"), "{tree}");
    assert_eq!(
        child_kinds(&tree, "ClassifierDeclaration"),
        ["Identification", "DisjoiningPart", "DifferencingPart"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "DisjoiningPart"),
        ["KwDisjoint", "KwFrom", "OwnedDisjoining"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "DifferencingPart"),
        ["KwDifferences", "Differencing", "Comma", "Differencing"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "Differencing"),
        ["QualifiedName"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "IntersectingPart"),
        ["KwIntersects", "Intersecting", "Comma", "Intersecting"],
        "{tree}"
    );
    // After a SuperclassingPart, and repeated: TypeRelationshipPart* (8.2.4.2.1).
    assert!(has_node(&tree, "SuperclassingPart"), "{tree}");
    assert_eq!(tree.matches("UnioningPart").count(), 2, "{tree}");
    // KerML Spec Annex A Examples/A-2-ModelingInstances.kerml:9 and :32, after an
    // OwnedMultiplicity and a SuperclassingPart.
    kerml_accepted("classifier YourBike [1] specializes Bicycle disjoint from MyBike;");
    kerml_accepted("classifier OurBicycle unions MyBike, YourBike;");
    // Every classifier keyword reaches the one ClassifierDeclaration: A-3-5's `struct`.
    kerml_accepted("struct MyBikeTimeCoincident unions MyWheel, MyBikeFork, MyBike;");
}

#[test]
fn a_feature_takes_type_relationship_parts() {
    // vendor/corpus/kerml/src/examples/Simple Tests/Features.kerml:20, 21 and 28.
    let tree = render(
        &kerml_accepted(
            "feature z unions f, g disjoint from y;\n\
             feature z1 intersects f,g differences y, y1, z;\n\
             feature adult differences person, child;",
        )
        .syntax(),
    );
    assert_eq!(
        child_kinds(&tree, "FeatureDeclaration"),
        ["FeatureIdentification", "UnioningPart", "DisjoiningPart"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "UnioningPart"),
        ["KwUnions", "Unioning", "Comma", "Unioning"],
        "{tree}"
    );
    // After a FeatureSpecializationPart, and in the declaration that is one alone.
    kerml_accepted("feature x : T [1] unions a, b;");
    kerml_accepted("feature : T disjoint from a;");
}

#[test]
fn a_type_relationship_target_may_be_a_feature_chain() {
    // Unioning, Intersecting and Differencing own an OwnedFeatureChain, and
    // OwnedDisjoining a FeatureChain, in their second alternatives (8.2.4.1.4, 8.2.4.1.5).
    // vendor/corpus/kerml/src/examples/Simple Tests/FeatureChains.kerml:30-31.
    let tree = render(
        &kerml_accepted(
            "feature h1 unions f, b.f, b.a;\n\
             feature h2 differences b.f, b.a intersects f.a, g disjoint from h1;",
        )
        .syntax(),
    );
    assert_eq!(child_kinds(&tree, "Unioning"), ["QualifiedName"], "{tree}");
    assert_eq!(
        tree.lines()
            .filter(|l| l.trim_start() == "OwnedFeatureChain")
            .count(),
        5,
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "Differencing"),
        ["OwnedFeatureChain"],
        "{tree}"
    );
    kerml_accepted("classifier C disjoint from a.b;");
}

#[test]
fn a_type_relationship_part_is_bounded_by_its_rules() {
    // `disjoint` alone is not the part: `'disjoint' 'from'`. Held as a file by
    // tests/rejection/kerml-disjoining-part-needs-from.kerml.
    kerml_rejected("classifier C disjoint A;");
    // Each part names at least one type, and a comma one more. Held as a file by
    // tests/rejection/kerml-type-relationship-part-needs-a-target.kerml.
    kerml_rejected("classifier C unions;");
    kerml_rejected("classifier C intersects A, ;");
    kerml_rejected("feature f differences;");
    // The parts follow the specialization, never precede it (8.2.4.2.1, 8.2.4.3.1). Held
    // as a file by tests/rejection/kerml-type-relationship-part-follows-specialization.kerml.
    kerml_rejected("classifier C unions A specializes B;");
    kerml_rejected("feature f unions a : T;");
    // A part cannot open a FeatureDeclaration: it needs a name, a specialization or a
    // conjugation first (8.2.4.3.1).
    kerml_rejected("feature disjoint from a;");
    // SysML states no TypeRelationshipPart: its definitions and usages end their
    // declarations elsewhere (ADR-0014). Held as a file by
    // tests/rejection/type-relationship-part-is-not-sysml.sysml.
    assert!(
        !parse("part def P unions A, B;", Language::SysMl)
            .errors()
            .is_empty()
    );
}

#[test]
fn a_type_relationship_part_is_read_before_it_is_validated() {
    // validateTypeOwnedUnioningNotOne, ...IntersectingNotOne, ...DifferencingNotOne and
    // the three ...TypesNotSelf constraints (KerML 8.3.3.1.10, receipt 5200b0a5) are
    // validity, not syntax: the text parses and the element carries its diagnostic
    // downstream (ADR-0002).
    kerml_accepted("classifier C unions A;");
    kerml_accepted("classifier C intersects C, D;");
    kerml_accepted("feature f differences f, g;");
}

// -- ConjugationPart, KerML 8.2.4.1.1 ---------------------------------------------

#[test]
fn a_classifier_takes_a_conjugation_part() {
    // vendor/corpus/kerml/src/examples/Simple Tests/Conjugation.kerml:6.
    let tree = render(&kerml_accepted("class B conjugates A;").syntax());
    assert_eq!(
        child_kinds(&tree, "ClassifierDeclaration"),
        ["Identification", "ConjugationPart"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "ConjugationPart"),
        ["KwConjugates", "OwnedConjugation"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "OwnedConjugation"),
        ["QualifiedName"],
        "{tree}"
    );
    // CONJUGATES = '~' | 'conjugates' (KerML 8.2.2.7).
    let tilde = render(&kerml_accepted("class B ~ A;").syntax());
    assert_eq!(
        child_kinds(&tilde, "ConjugationPart"),
        ["Tilde", "OwnedConjugation"],
        "{tilde}"
    );
    // OwnedConjugation's FeatureChain alternative (8.2.4.1.3) on a classifier too: the
    // specification states one OwnedConjugation for both declarations, where the Pilot's
    // ClassifierConjugation takes a name alone (deviation ClassifierConjugation,
    // follow_spec).
    let chain = render(&kerml_accepted("class B conjugates a.b;").syntax());
    assert_eq!(
        child_kinds(&chain, "OwnedConjugation"),
        ["OwnedFeatureChain"],
        "{chain}"
    );
    // After an OwnedMultiplicity, and before TypeRelationshipPart* (8.2.4.2.1).
    let tail = render(&kerml_accepted("classifier C [1] ~ A disjoint from D;").syntax());
    assert_eq!(
        child_kinds(&tail, "ClassifierDeclaration"),
        [
            "Identification",
            "OwnedMultiplicity",
            "ConjugationPart",
            "DisjoiningPart"
        ],
        "{tail}"
    );
}

#[test]
fn a_feature_takes_a_conjugation_part() {
    // vendor/corpus/kerml/src/examples/Simple Tests/Conjugation.kerml:8 and
    // Features.kerml:36, after a FeatureIdentification.
    let tree = render(&kerml_accepted("feature g ~ B::f;").syntax());
    assert_eq!(
        child_kinds(&tree, "FeatureDeclaration"),
        ["FeatureIdentification", "ConjugationPart"],
        "{tree}"
    );
    kerml_accepted("class C { feature fuelOutPort ~ fuelInPort; }");
    // The third alternative, a ConjugationPart alone (8.2.4.3.1).
    let alone = render(&kerml_accepted("feature conjugates g;").syntax());
    assert_eq!(
        child_kinds(&alone, "FeatureDeclaration"),
        ["ConjugationPart"],
        "{alone}"
    );
    kerml_accepted("feature ~ g;");
    // OwnedConjugation's second alternative, a FeatureChain (8.2.4.1.3):
    // vendor/corpus/kerml/src/examples/Simple Tests/FeatureChains.kerml:35.
    let chain = render(&kerml_accepted("feature x conjugates f.a;").syntax());
    assert_eq!(
        child_kinds(&chain, "OwnedConjugation"),
        ["OwnedFeatureChain"],
        "{chain}"
    );
    // FeatureRelationshipPart* follows it.
    kerml_accepted("feature x ~ f unions a, b;");
}

#[test]
fn a_conjugation_part_is_bounded_by_its_rules() {
    // A conjugated type "may not also be the specific Type in any Specialization"
    // (KerML 8.3.3.1.2, receipt eabb0d9b), and the grammar says so by alternation:
    // ( SuperclassingPart | ConjugationPart )? on a classifier (8.2.4.2.1), and
    // FeatureIdentification ( FeatureSpecializationPart | ConjugationPart )? on a feature
    // (8.2.4.3.1). Held as a file by
    // tests/rejection/kerml-conjugation-part-is-not-a-specialization.kerml.
    kerml_rejected("class B :> A conjugates C;");
    kerml_rejected("class B conjugates C :> A;");
    kerml_rejected("feature f : T ~ g;");
    kerml_rejected("feature f ~ g : T;");
    // A multiplicity is a FeatureSpecializationPart's, so it cannot sit beside one either.
    kerml_rejected("feature f [1] ~ g;");
    // One conjugation, of one type: `CONJUGATES OwnedConjugation`, no list, no repeat
    // ("at most one Conjugation", 8.3.3.1.2). Held as a file by
    // tests/rejection/kerml-conjugation-part-names-one-type.kerml.
    kerml_rejected("class B conjugates A, C;");
    kerml_rejected("class B ~ A ~ C;");
    kerml_rejected("class B conjugates;");
    // A `~` after `:` is still SysML's ConjugatedPortTyping, not this part.
    kerml_rejected("feature f : ~T;");
    // SysML states no ConjugationPart on a definition: DefinitionDeclaration =
    // Identification SubclassificationPart? (SysML 8.2.2.6.1). Held as a file by
    // tests/rejection/conjugation-part-is-not-sysml.sysml.
    assert!(
        !parse("part def B conjugates A;", Language::SysMl)
            .errors()
            .is_empty()
    );
}

// -- prefix metadata, KerML 8.2.5.12 ---------------------------------------------
//
//   PrefixMetadataMember     = '#' PrefixMetadataFeature
//   PrefixMetadataAnnotation = '#' PrefixMetadataFeature
//   PrefixMetadataFeature    = OwnedFeatureTyping

#[test]
fn a_classifier_takes_prefix_metadata() {
    // KerML Spec Annex A Examples/A-2-ModelingInstances.kerml:22-23: the `#atom` on the
    // line before the classifier's keyword, in its TypePrefix (8.2.4.1.1).
    let tree = render(
        &kerml_accepted("package P {\n\t#atom\n\tclassifier MyBike specializes Bicycle;\n}")
            .syntax(),
    );
    assert_eq!(
        child_kinds(&tree, "TypePrefix"),
        ["PrefixMetadataMember"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "PrefixMetadataMember"),
        ["Hash", "PrefixMetadataFeature"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "PrefixMetadataFeature"),
        ["OwnedFeatureTyping"],
        "{tree}"
    );
    // KerML's element, not SysML's (ADR-0014).
    assert!(!has_node(&tree, "PrefixMetadataUsage"), "{tree}");
    // "It is also possible to include more than one user defined-keyword in a
    // declaration" (7.4.13, receipt 5e755297), after `abstract`.
    let two = render(&kerml_accepted("abstract #SecurityRelated #command behavior Save;").syntax());
    assert_eq!(
        child_kinds(&two, "TypePrefix"),
        ["KwAbstract", "PrefixMetadataMember", "PrefixMetadataMember"],
        "{two}"
    );
    // The type is a GeneralType, a name or a feature chain (8.2.4.1.2).
    kerml_accepted("#Meta::tags.kind struct S;");
}

#[test]
fn a_package_and_a_dependency_take_prefix_metadata() {
    // Package and LibraryPackage own PrefixMetadataMembers (8.2.5.13); Dependency owns
    // PrefixMetadataAnnotations (8.2.3.2).
    let package = render(&kerml_accepted("#X package Q;").syntax());
    assert_eq!(
        child_kinds(&package, "Package"),
        ["PrefixMetadataMember", "PackageDeclaration", "PackageBody"],
        "{package}"
    );
    kerml_accepted("library #X package L;");
    kerml_accepted("package P { #X package Q; }");
    let dependency = render(&kerml_accepted("#X dependency a to b;").syntax());
    assert_eq!(
        child_kinds(&dependency, "PrefixMetadataAnnotation"),
        ["Hash", "PrefixMetadataFeature"],
        "{dependency}"
    );
}

#[test]
fn a_feature_takes_prefix_metadata() {
    // FeaturePrefix ends in PrefixMetadataMember* (8.2.4.3.1): before `feature`, all of
    // them are the prefix's.
    let keyword = render(&kerml_accepted("#M feature f;").syntax());
    assert_eq!(
        child_kinds(&keyword, "FeaturePrefix"),
        ["BasicFeaturePrefix", "PrefixMetadataMember"],
        "{keyword}"
    );
    assert_eq!(
        child_kinds(&keyword, "Feature"),
        [
            "FeaturePrefix",
            "KwFeature",
            "FeatureDeclaration",
            "TypeBody"
        ],
        "{keyword}"
    );
    // Feature = FeaturePrefix ( 'feature' | PrefixMetadataMember ) FeatureDeclaration?
    // ...: with no keyword the last `#` stands in its place, and the rest are the prefix's.
    let replaced = render(&kerml_accepted("#A #B f : T;").syntax());
    assert_eq!(
        child_kinds(&replaced, "Feature"),
        [
            "FeaturePrefix",
            "PrefixMetadataMember",
            "FeatureDeclaration",
            "TypeBody"
        ],
        "{replaced}"
    );
    assert_eq!(
        child_kinds(&replaced, "FeaturePrefix"),
        ["BasicFeaturePrefix", "PrefixMetadataMember"],
        "{replaced}"
    );
    // The declaration is optional there, as after the keyword.
    let bare = render(&kerml_accepted("class C { #M; }").syntax());
    assert_eq!(
        child_kinds(&bare, "Feature"),
        ["FeaturePrefix", "PrefixMetadataMember", "TypeBody"],
        "{bare}"
    );
    kerml_accepted("#M = 1;");
    kerml_accepted("#M ~ g;");
    // After a BasicFeaturePrefix, and before the other FeatureElements' keywords.
    kerml_accepted("in #M feature x;");
    kerml_accepted("struct S { #M connector c from a to b; }");
    kerml_accepted("#M succession s first a then b;");
    kerml_accepted("#M binding a = b;");
}

#[test]
fn prefix_metadata_is_bounded_by_its_rules() {
    // The `#` follows the prefix keywords, never precedes them: TypePrefix is
    // `'abstract'? PrefixMetadataMember*` (8.2.4.1.1), FeaturePrefix puts its
    // PrefixMetadataMember* after the Basic or End prefix (8.2.4.3.1). Held as files by
    // tests/rejection/kerml-prefix-metadata-follows-abstract.kerml and
    // kerml-prefix-metadata-follows-feature-direction.kerml.
    kerml_rejected("#M abstract class C;");
    kerml_rejected("#M in feature x;");
    // PrefixMetadataFeature is an OwnedFeatureTyping: a `#` names a type. Held as a file
    // by tests/rejection/kerml-prefix-metadata-names-a-type.kerml.
    kerml_rejected("# class C;");
    kerml_rejected("#1 class C;");
    // A `#` alone is no feature: the Feature alternative wants a name after it, and a
    // name is what makes it a PrefixMetadataMember at all.
    kerml_rejected("class C { #; }");
    // FeatureElements this parser does not read stay reported with a `#` before them.
    kerml_rejected("#M inv { true }");
}

// -- a keywordless feature declared by its specialization, KerML 8.2.4.3.1 ----------
//
//   Feature = ... | ( EndFeaturePrefix | BasicFeaturePrefix ) FeatureDeclaration ...
//   FeatureDeclaration = 'all'? ( FeatureIdentification ... | FeatureSpecializationPart
//                               | ConjugationPart ) FeatureRelationshipPart*

#[test]
fn a_keywordless_feature_may_be_declared_by_its_specialization_alone() {
    // Variable Feature Examples/Enhancements/ExtendedOccurrences.kerml:6, with no prefix.
    let tree = render(&kerml_accepted("class C {\n\t:>> self : Timeslice;\n}").syntax());
    assert_eq!(
        child_kinds(&tree, "Feature"),
        ["FeaturePrefix", "FeatureDeclaration", "TypeBody"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "FeatureDeclaration"),
        ["FeatureSpecializationPart"],
        "{tree}"
    );
    // After a BasicFeaturePrefix: Variable Feature Examples/TimeVaryingFeatures.kerml:5
    // and Mass Roll-up Example/Vehicles_3.kerml:34.
    kerml_accepted("class C { portion :>> startShot { } }");
    kerml_accepted("class C { composite :>> engine = e; }");
    // The word forms: Variable Feature Examples/Enhancements/Moments.kerml:8 and KerML
    // Spec Annex A Examples/A-3-8-ChangingFeatureValues.kerml:13.
    kerml_accepted("class C { redefines predecessors [0]; }");
    kerml_accepted("class C { redefines objectToPaint = objectToFinish; }");
    // A FeatureSpecializationPart may open on its MultiplicityPart (8.2.4.3.1), and the
    // declaration may be a ConjugationPart alone.
    kerml_accepted("class C { [0..1] : T; }");
    kerml_accepted("class C { ~ g; }");
    kerml_accepted("class C { conjugates g; }");
    // Still no feature called `package` or `class`: a keyword is not a name.
    let package = render(&kerml_accepted("package P;").syntax());
    assert!(!has_node(&package, "Feature"), "{package}");
}

#[test]
fn a_keywordless_feature_still_needs_its_declaration() {
    // With no `feature` and no `#` in its place, the FeatureDeclaration is what says a
    // feature is here, and it is required (8.2.4.3.1). Held as a file by
    // tests/rejection/kerml-keywordless-feature-needs-a-declaration.kerml.
    kerml_rejected("class C { composite = e; }");
    kerml_rejected("class C { portion { } }");
    kerml_rejected("class C { portion; }");
}

// -- the owned cross feature of an end feature, KerML 8.2.4.3.1 ---------------------
//
//   FeaturePrefix           = ( EndFeaturePrefix OwnedCrossFeatureMember?
//                             | BasicFeaturePrefix ) PrefixMetadataMember*
//   OwnedCrossFeatureMember = OwnedCrossFeature
//   OwnedCrossFeature       = BasicFeaturePrefix FeatureDeclaration

#[test]
fn an_end_feature_may_own_a_cross_feature() {
    // Association Examples/ProductSelection_N_ary.kerml:9: the cross feature is the
    // multiplicity between `end` and `feature`.
    let tree = render(
        &kerml_accepted("assoc A {\n\tend [0..1] feature cart: ShoppingCart[1];\n}").syntax(),
    );
    assert_eq!(
        child_kinds(&tree, "FeaturePrefix"),
        ["EndFeaturePrefix", "OwnedCrossFeatureMember"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "OwnedCrossFeatureMember"),
        ["OwnedCrossFeature"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "OwnedCrossFeature"),
        ["BasicFeaturePrefix", "FeatureDeclaration"],
        "{tree}"
    );
    // Simple Tests/Associations.kerml:6, a named cross feature; and Massed Thing
    // Example/MassedThings.kerml:10, after a visibility.
    kerml_accepted("assoc A { end x_cross [1..1] feature x : X; }");
    kerml_accepted("assoc A { public end [0..1] feature assembly: MassedThing; }");
    // Its own BasicFeaturePrefix, and the other FeatureElements' keywords and a `#`
    // after it: FeaturePrefix is theirs too (8.2.5.5.1, 8.2.5.6.2).
    kerml_accepted("assoc A { const end derived x : T feature y; }");
    kerml_accepted("assoc A { end x step s; }");
    kerml_accepted("assoc A { end x #M feature y; }");
    // With nothing between `end` and the keyword there is no cross feature, and with no
    // keyword the `end` is the keywordless Feature's EndFeaturePrefix, whose declaration
    // follows it directly.
    let plain = render(&kerml_accepted("assoc A { end feature f; }").syntax());
    assert!(!has_node(&plain, "OwnedCrossFeatureMember"), "{plain}");
    let keywordless = render(&kerml_accepted("assoc A { end f : T; }").syntax());
    assert!(
        !has_node(&keywordless, "OwnedCrossFeatureMember"),
        "{keywordless}"
    );
}

#[test]
fn an_owned_cross_feature_is_bounded_by_its_rules() {
    // A cross feature is a BasicFeaturePrefix AND a FeatureDeclaration: a prefix alone is
    // none. EndFeaturePrefix is `'const'? 'end'` and writes no `derived`, so the word
    // can only open a cross feature that then declares nothing. Held as a file by
    // tests/rejection/kerml-owned-cross-feature-needs-a-declaration.kerml.
    kerml_rejected("assoc A { end derived feature y; }");
    // `const` is EndFeaturePrefix's before `end`, and the cross feature's own after it;
    // it is not both at once before the `end`.
    kerml_rejected("assoc A { const const end feature y; }");
}

// -- TypeFeaturingPart, KerML 8.2.4.3.1 ---------------------------------------------
//
//   TypeFeaturingPart  = 'featured' 'by' OwnedTypeFeaturing ( ',' OwnedTypeFeaturing )*
//   OwnedTypeFeaturing = featuringType = [QualifiedName]        (8.2.4.3.7)

#[test]
fn a_feature_takes_a_type_featuring_part() {
    // Variable Feature Examples/Enhancements/Moments.kerml:36-38, after a
    // FeatureSpecializationPart whose subsetting names two features.
    let tree = render(
        &kerml_accepted(
            "package P {\n\tfeature coincidentUEPortion : Occurrence [1] subsets \
             spaceTimeCoincidentOccurrences,\n\t\tuniversalEternity.portions\n\t\tfeatured by \
             Occurrence;\n}",
        )
        .syntax(),
    );
    assert_eq!(
        child_kinds(&tree, "FeatureDeclaration"),
        [
            "FeatureIdentification",
            "FeatureSpecializationPart",
            "TypeFeaturingPart"
        ],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "TypeFeaturingPart"),
        ["KwFeatured", "KwBy", "OwnedTypeFeaturing"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "OwnedTypeFeaturing"),
        ["QualifiedName"],
        "{tree}"
    );
    // More than one featuring type, a qualified one: `featured by CC1::t::t1::startShot`
    // (Variable Feature Examples/TimeVaryingFeatures.kerml).
    let two = render(&kerml_accepted("feature x featured by A, CC1::t::t1::startShot;").syntax());
    assert_eq!(
        child_kinds(&two, "TypeFeaturingPart"),
        [
            "KwFeatured",
            "KwBy",
            "OwnedTypeFeaturing",
            "Comma",
            "OwnedTypeFeaturing"
        ],
        "{two}"
    );
    // FeatureRelationshipPart* takes its parts in any order and number (8.2.4.3.1).
    kerml_accepted("feature f featured by A unions b, c featured by B;");
    kerml_accepted("feature f unions b featured by A = 0;");
    // Every FeatureDeclaration ends in the parts, a binary connector's before its `from`
    // too (8.2.5.5.1): the text of the commented-out Variable Feature
    // Examples/Enhancements/TimeVaryingFeaturesEnhanced.kerml:98, less its `member`.
    let connector = render(
        &kerml_accepted(
            "struct Car { connector drive featured by Car_snapshots from engine to transmission; }",
        )
        .syntax(),
    );
    assert!(
        has_node(&connector, "BinaryConnectorDeclaration"),
        "{connector}"
    );
    assert!(has_node(&connector, "TypeFeaturingPart"), "{connector}");
}

#[test]
fn a_type_featuring_part_is_bounded_by_its_rules() {
    // `featured` alone is not the part: `'featured' 'by'`.
    kerml_rejected("feature f featured A;");
    // OwnedTypeFeaturing names its featuring type by QualifiedName alone: unlike the
    // relationship parts, it has no feature chain alternative (8.2.4.3.7). Held as a
    // file by tests/rejection/kerml-owned-type-featuring-names-a-qualified-name.kerml.
    kerml_rejected("feature f featured by a.b;");
    kerml_rejected("feature f featured by;");
    // A feature's part, not a classifier's: ClassifierDeclaration ends in
    // TypeRelationshipPart*, which does not reach it (8.2.4.2.1). Held as a file by
    // tests/rejection/kerml-type-featuring-part-is-a-feature-s.kerml.
    kerml_rejected("class C featured by D;");
}

// -- ChainingPart, KerML 8.2.4.3.1 --------------------------------------------------
//
//   ChainingPart         = 'chains' ( OwnedFeatureChaining | FeatureChain )
//   FeatureChain         = OwnedFeatureChaining ( '.' OwnedFeatureChaining )+   (8.2.4.3.5)
//   OwnedFeatureChaining = chainingFeature = [QualifiedName]                     (8.2.4.3.5)

#[test]
fn a_feature_takes_a_chaining_part() {
    // KerML Spec Annex A Examples/A-3-8-ChangingFeatureValues.kerml:150, before a
    // FeatureValue. Neither alternative assigns the chain to a feature of its own: both
    // add the FeatureChainings to the declared feature's ownedRelationship, so the links
    // are the part's children and no OwnedFeatureChain node stands between them.
    let tree = render(
        &kerml_accepted("feature obPiP chains objectToFinish.beforePaint.isPainted = false;")
            .syntax(),
    );
    assert_eq!(
        child_kinds(&tree, "FeatureDeclaration"),
        ["FeatureIdentification", "ChainingPart"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "ChainingPart"),
        [
            "KwChains",
            "OwnedFeatureChaining",
            "Dot",
            "OwnedFeatureChaining",
            "Dot",
            "OwnedFeatureChaining"
        ],
        "{tree}"
    );
    assert!(!has_node(&tree, "OwnedFeatureChain"), "{tree}");
    assert!(has_node(&tree, "FeatureValue"), "{tree}");
    // The first alternative is one link. validateFeatureChainingFeatureNotOne
    // (8.3.3.3.4) makes a single chainingFeature invalid, but validity gates writes, not
    // reads (ADR-0002): it parses and carries its diagnostic downstream. The corpus
    // writes it, twice in one declaration: Simple Tests/FeatureChains.kerml:33.
    let two = render(&kerml_accepted("feature b_f_a chains b chains f.a;").syntax());
    assert_eq!(
        child_kinds(&two, "FeatureDeclaration"),
        ["FeatureIdentification", "ChainingPart", "ChainingPart"],
        "{two}"
    );
    // A link is a QualifiedName, so it may be qualified itself.
    kerml_accepted("feature f chains P::a.Q::b;");
    // After a FeatureSpecializationPart, across lines: A-3-8-ChangingFeatureValues.kerml
    // :158-160, as a type body's feature.
    kerml_accepted(
        "behavior B {\n\tfeature subsets objectToFinish.beforePaint.immediateSuccessors,\n\t\t\
         objectToFinish.whilePainting.startShot.timeCoincidentOccurrences\n\t\tchains \
         paint.painting.endShot;\n}",
    );
    // FeatureRelationshipPart* takes its parts in any order (8.2.4.3.1).
    kerml_accepted("feature f unions g chains a.b featured by A;");
}

#[test]
fn a_chaining_part_is_bounded_by_its_rules() {
    // `chains` needs at least one link.
    kerml_rejected("feature f chains;");
    // Every `.` is followed by a link (8.2.4.3.5). Held as a file by
    // tests/rejection/kerml-feature-chain-needs-a-link-after-every-dot.kerml.
    kerml_rejected("feature f chains a.;");
    // One chain, not a list: FeatureChain is `.`-separated, never `,`-separated.
    kerml_rejected("feature f chains a, b;");
    // A feature's part, not a classifier's: ClassifierDeclaration ends in
    // TypeRelationshipPart*, which does not reach it (8.2.4.2.1). Held as a file by
    // tests/rejection/kerml-chaining-part-is-a-feature-s.kerml.
    kerml_rejected("class C chains a.b;");
    // A FeatureDeclaration opens on an identification, a specialization or a conjugation
    // (8.2.4.3.1); FeatureRelationshipPart* only ends one, so a part cannot stand alone.
    kerml_rejected("feature chains a.b;");
    kerml_rejected("feature featured by A;");
}

// -- InvertingPart, KerML 8.2.4.3.1 -------------------------------------------------
//
//   InvertingPart         = 'inverse' 'of' OwnedFeatureInverting
//   OwnedFeatureInverting = invertingFeature = [QualifiedName]
//                         | invertingFeature = OwnedFeatureChain            (8.2.4.3.6)

#[test]
fn a_feature_takes_an_inverting_part() {
    // Simple Tests/Inverses.kerml:3, before a DisjoiningPart.
    let tree = render(&kerml_accepted("feature f : B inverse of B::g disjoint from h;").syntax());
    assert_eq!(
        child_kinds(&tree, "FeatureDeclaration"),
        [
            "FeatureIdentification",
            "FeatureSpecializationPart",
            "InvertingPart",
            "DisjoiningPart"
        ],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "InvertingPart"),
        ["KwInverse", "KwOf", "OwnedFeatureInverting"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "OwnedFeatureInverting"),
        ["QualifiedName"],
        "{tree}"
    );
    // After a TypeFeaturingPart: Inverses.kerml:14.
    kerml_accepted("feature gg : A featured by B inverse of A::f;");
    // KerML 7.3.4.7's own examples, after a multiplicity and in nested bodies.
    kerml_accepted(
        "classifier Person {\n    feature children : Person[*];\n    feature parents : \
         Person[*] inverse of children;\n}",
    );
    kerml_accepted("classifier C { feature b2: B { feature a2: A inverse of A::b1::c1; } }");
    // The second alternative: the inverting feature is an OwnedFeatureChain, an owned
    // related element of the FeatureInverting, so here the chain IS a node of its own,
    // unlike a ChainingPart's links (8.2.4.3.6, 8.2.4.3.5).
    let chain = render(&kerml_accepted("feature f inverse of a.b.c;").syntax());
    assert_eq!(
        child_kinds(&chain, "OwnedFeatureInverting"),
        ["OwnedFeatureChain"],
        "{chain}"
    );
    assert_eq!(
        child_kinds(&chain, "OwnedFeatureChain"),
        [
            "OwnedFeatureChaining",
            "Dot",
            "OwnedFeatureChaining",
            "Dot",
            "OwnedFeatureChaining"
        ],
        "{chain}"
    );
    // FeatureRelationshipPart* admits the part more than once (8.2.4.3.1); KerML 7.3.4.7
    // says it is "generally not useful", which is not a rule.
    kerml_accepted("feature f inverse of g inverse of h chains a.b;");
    // Every FeatureDeclaration ends in the parts, a binary connector's before its `from`
    // too (8.2.5.5.1), and a keywordless feature's (8.2.4.3.1).
    let connector = render(&kerml_accepted("connector c inverse of g from a to b;").syntax());
    assert!(
        has_node(&connector, "BinaryConnectorDeclaration"),
        "{connector}"
    );
    assert!(has_node(&connector, "InvertingPart"), "{connector}");
    kerml_accepted("class A { f : B inverse of B::g; }");
}

#[test]
fn an_inverting_part_is_bounded_by_its_rules() {
    // Both keywords: `'inverse' 'of'`.
    kerml_rejected("feature f inverse g;");
    kerml_rejected("feature f of g;");
    kerml_rejected("feature f inverse of;");
    // One inverting feature, not a list (8.2.4.3.6). Held as a file by
    // tests/rejection/kerml-inverting-part-names-one-feature.kerml.
    kerml_rejected("feature f inverse of g, h;");
    kerml_rejected("feature f inverse of a.;");
    // A feature's part, not a classifier's: ClassifierDeclaration ends in
    // TypeRelationshipPart*, which does not reach it (8.2.4.2.1). Held as a file by
    // tests/rejection/kerml-inverting-part-is-a-feature-s.kerml.
    kerml_rejected("class C inverse of g;");
    kerml_rejected("feature inverse of g;");
}

// -- FeatureInverting, KerML 8.2.4.3.6 ----------------------------------------------
//
//   FeatureInverting = ( 'inverting' Identification? )?
//                      'inverse' ( [QualifiedName] | OwnedFeatureChain )
//                      'of'      ( [QualifiedName] | OwnedFeatureChain )
//                      RelationshipBody
//
// A NonFeatureElement (8.2.3.4.3): the standalone declaration of the relationship an
// InvertingPart owns.

#[test]
fn a_feature_inverting_reads_the_corpus_forms() {
    // Simple Tests/Inverses.kerml:11-12, in a package body: both targets names, then
    // `inverting` with a name and a chain for the first target.
    let tree = render(
        &kerml_accepted(
            "package Inverses {\n\tinverse B::g of A::f;\n\tinverting Invert inverse B::g.f of \
             A::h;\n}",
        )
        .syntax(),
    );
    assert_eq!(
        tree.lines()
            .filter(|l| l.trim() == "FeatureInverting")
            .count(),
        2,
        "{tree}"
    );
    let bare = render(&kerml_accepted("inverse B::g of A::f;").syntax());
    assert_eq!(
        child_kinds(&bare, "FeatureInverting"),
        [
            "KwInverse",
            "QualifiedName",
            "KwOf",
            "QualifiedName",
            "RelationshipBody"
        ],
        "{bare}"
    );
    // A target written as a chain is an OwnedFeatureChain, an ownedRelatedElement of
    // the relationship (8.2.4.3.6), on either side.
    let named = render(&kerml_accepted("inverting Invert inverse B::g.f of A::h;").syntax());
    assert_eq!(
        child_kinds(&named, "FeatureInverting"),
        [
            "KwInverting",
            "Identification",
            "KwInverse",
            "OwnedFeatureChain",
            "KwOf",
            "QualifiedName",
            "RelationshipBody"
        ],
        "{named}"
    );
    let chained = render(&kerml_accepted("inverse a of b.c;").syntax());
    assert_eq!(
        child_kinds(&chained, "FeatureInverting"),
        [
            "KwInverse",
            "QualifiedName",
            "KwOf",
            "OwnedFeatureChain",
            "RelationshipBody"
        ],
        "{chained}"
    );
}

#[test]
fn a_feature_inverting_takes_the_optional_parts_and_positions_its_production_admits() {
    // KerML 7.3.4.7's example: a short name is optional, and so is the whole
    // Identification after `inverting` (8.2.3.1), and the body may hold an annotation.
    kerml_accepted(
        "inverting parent_child inverse Person::parent of Person::child {\n    doc /* A \
         Person is the parent of their children. */\n}",
    );
    kerml_accepted("inverting <pc> parent_child inverse a of b;");
    // `inverting` with nothing after it still has its Identification, empty, as a
    // PayloadFeature's has: the production writes one whenever `inverting` is taken.
    let anonymous = render(&kerml_accepted("inverting inverse a of b;").syntax());
    assert_eq!(
        child_kinds(&anonymous, "FeatureInverting"),
        [
            "KwInverting",
            "Identification",
            "KwInverse",
            "QualifiedName",
            "KwOf",
            "QualifiedName",
            "RelationshipBody"
        ],
        "{anonymous}"
    );
    // A NonFeatureMember takes a MemberPrefix (8.2.3.4.1), and a type body and a
    // relationship body reach NonFeatureElement too (8.2.4.1.1, 8.2.3.1).
    kerml_accepted("package P { private inverse a of b; }");
    kerml_accepted("class C { inverse a of b; }");
    kerml_accepted("dependency x to y { inverse a of b; }");
}

#[test]
fn a_feature_inverting_is_bounded_by_its_rules() {
    // Both targets and both keywords.
    kerml_rejected("inverse a;");
    kerml_rejected("inverse of b;");
    kerml_rejected("inverse a of;");
    kerml_rejected("inverting x;");
    // One feature on each side, not a list. Held as a file by
    // tests/rejection/kerml-feature-inverting-relates-two-features.kerml.
    kerml_rejected("inverse a, b of c;");
    kerml_rejected("inverse a of b, c;");
    // An Identification is one short name and one name, at most (8.2.3.1).
    kerml_rejected("inverting a b inverse c of d;");
    // A relationship ends in its RelationshipBody.
    kerml_rejected("inverse a of b");
    // SysML states no FeatureInverting: .sysml text never reaches it (ADR-0014). Held
    // as a file by tests/rejection/feature-inverting-is-not-sysml.sysml.
    let sysml = parse("inverse a of b;", Language::SysMl);
    assert!(!sysml.errors().is_empty(), "{:?}", sysml.errors());
}

// -- Specialization, KerML 8.2.4.1.2 ------------------------------------------------
//
//   Specialization = ( 'specialization' Identification )?
//                    'subtype' SpecificType SPECIALIZES GeneralType RelationshipBody
//   SpecificType   = [QualifiedName] | OwnedFeatureChain
//   GeneralType    = [QualifiedName] | OwnedFeatureChain
//   SPECIALIZES    = ':>' | 'specializes'                                   (8.2.2.7)
//
// A NonFeatureElement (8.2.3.4.3).

#[test]
fn a_specialization_reads_the_corpus_forms() {
    // Simple Tests/Types.kerml:17-18: named with the word, then unnamed with the symbol.
    let named = render(&kerml_accepted("specialization Gen subtype A specializes B;").syntax());
    assert_eq!(
        child_kinds(&named, "Specialization"),
        [
            "KwSpecialization",
            "Identification",
            "KwSubtype",
            "QualifiedName",
            "KwSpecializes",
            "QualifiedName",
            "RelationshipBody"
        ],
        "{named}"
    );
    let unnamed = render(&kerml_accepted("specialization subtype x :> Base::things;").syntax());
    assert_eq!(
        child_kinds(&unnamed, "Specialization"),
        [
            "KwSpecialization",
            "Identification",
            "KwSubtype",
            "QualifiedName",
            "ColonGt",
            "QualifiedName",
            "RelationshipBody"
        ],
        "{unnamed}"
    );
    // Simple Tests/FeatureChains.kerml:26: either type may be a feature chain.
    let chained = render(&kerml_accepted("subtype g.g specializes b.f.a;").syntax());
    assert_eq!(
        child_kinds(&chained, "Specialization"),
        [
            "KwSubtype",
            "OwnedFeatureChain",
            "KwSpecializes",
            "OwnedFeatureChain",
            "RelationshipBody"
        ],
        "{chained}"
    );
    // KerML 7.3.2.3's examples: a body with an annotation, and the keyword omitted.
    kerml_accepted(
        "specialization subtype x :> Base::things {\n    doc /* This specialization is \
         unnamed. */\n}",
    );
    kerml_accepted("package P { subtype C specializes A; subtype C specializes B; }");
    // A member (8.2.3.4.1), of a type body too (8.2.4.1.1), and an owned related
    // element (8.2.3.1).
    kerml_accepted("package P { private specialization <s> S subtype A :> B; }");
    kerml_accepted("class C { subtype a :> b; }");
    kerml_accepted("dependency x to y { subtype a :> b; }");
}

#[test]
fn a_specialization_is_bounded_by_its_rules() {
    kerml_rejected("specialization Gen A specializes B;");
    kerml_rejected("specialization;");
    kerml_rejected("subtype A;");
    kerml_rejected("subtype A specializes;");
    kerml_rejected("subtype specializes B;");
    // One general type, not a list: a list is a type declaration's owned
    // specializations, `type C specializes A, B;` (7.3.2.3). Held as a file by
    // tests/rejection/kerml-specialization-relates-one-general-type.kerml.
    kerml_rejected("subtype C specializes A, B;");
    // SPECIALIZES is `:>` or `specializes`; `subsets` is SUBSETS's word (8.2.2.7).
    kerml_rejected("subtype A subsets B;");
    kerml_rejected("subtype A :> B");
    // SysML states no Specialization declaration (ADR-0014). Held as a file by
    // tests/rejection/specialization-declaration-is-not-sysml.sysml.
    let sysml = parse("subtype A :> B;", Language::SysMl);
    assert!(!sysml.errors().is_empty(), "{:?}", sysml.errors());
}

// -- Subclassification, KerML 8.2.4.2.2 ---------------------------------------------
//
//   Subclassification = ( 'specialization' Identification )?
//                       'subclassifier' subclassifier = [QualifiedName]
//                       SPECIALIZES superclassifier = [QualifiedName]
//                       RelationshipBody
//
// A NonFeatureElement (8.2.3.4.3). Names only: no feature chain on either side.

#[test]
fn a_subclassification_reads_the_corpus_forms() {
    // Simple Tests/Classifiers.kerml:5-9, the whole run.
    let tree = render(
        &kerml_accepted(
            "package Classifiers {\n\tspecialization Super subclassifier A specializes B;\n\t\
             specialization subclassifier B :> A;\n\t\n\tsubclassifier C specializes A;\n\t\
             subclassifier C specializes B;\n}",
        )
        .syntax(),
    );
    assert_eq!(
        tree.lines()
            .filter(|l| l.trim() == "Subclassification")
            .count(),
        4,
        "{tree}"
    );
    let named =
        render(&kerml_accepted("specialization Super subclassifier A specializes B;").syntax());
    assert_eq!(
        child_kinds(&named, "Subclassification"),
        [
            "KwSpecialization",
            "Identification",
            "KwSubclassifier",
            "QualifiedName",
            "KwSpecializes",
            "QualifiedName",
            "RelationshipBody"
        ],
        "{named}"
    );
    let bare = render(&kerml_accepted("subclassifier C :> A;").syntax());
    assert_eq!(
        child_kinds(&bare, "Subclassification"),
        [
            "KwSubclassifier",
            "QualifiedName",
            "ColonGt",
            "QualifiedName",
            "RelationshipBody"
        ],
        "{bare}"
    );
    // KerML 7.3.3.3's example, with a comment in the body.
    kerml_accepted(
        "specialization subclassifier B :> A {\n    /* This subclassification is unnamed. */\n}",
    );
    kerml_accepted("class K { subclassifier a::b :> c::d; }");
}

#[test]
fn a_subclassification_is_bounded_by_its_rules() {
    kerml_rejected("subclassifier C;");
    kerml_rejected("subclassifier C specializes;");
    kerml_rejected("specialization S C specializes A;");
    // A classifier is named, never chained: `subclassifier = [QualifiedName]` on both
    // sides (8.2.4.2.2). Held as a file by
    // tests/rejection/kerml-subclassification-names-classifiers-not-feature-chains.kerml.
    kerml_rejected("subclassifier C specializes a.b;");
    kerml_rejected("subclassifier c.d specializes A;");
    // One superclassifier: a list is a classifier's own SuperclassingPart (8.2.4.2.1).
    kerml_rejected("subclassifier C specializes A, B;");
    kerml_rejected("subclassifier C subsets A;");
    // SysML states no Subclassification declaration (ADR-0014).
    let sysml = parse("subclassifier C :> A;", Language::SysMl);
    assert!(!sysml.errors().is_empty(), "{:?}", sysml.errors());
}

// -- FeatureTyping, KerML 8.2.4.3.2 -------------------------------------------------
//
//   FeatureTyping = ( 'specialization' Identification )?
//                   'typing' typedFeature = [QualifiedName]
//                   TYPED_BY GeneralType RelationshipBody
//   TYPED_BY      = ':' | 'typed' 'by'                                    (8.2.2.7)
//
// A NonFeatureElement (8.2.3.4.3). NOT SysML's FeatureTyping (8.2.2.6.5), the typing a
// feature's `:` owns; the node is the same metaclass's, and its children tell them apart.

#[test]
fn a_feature_typing_reads_the_corpus_forms() {
    // Simple Tests/Features.kerml:42-43, one of each TYPED_BY spelling.
    let words = render(&kerml_accepted("specialization t1 typing f typed by B;").syntax());
    assert_eq!(
        child_kinds(&words, "FeatureTyping"),
        [
            "KwSpecialization",
            "Identification",
            "KwTyping",
            "QualifiedName",
            "KwTyped",
            "KwBy",
            "QualifiedName",
            "RelationshipBody"
        ],
        "{words}"
    );
    let colon = render(&kerml_accepted("specialization t2 typing g : A;").syntax());
    assert_eq!(
        child_kinds(&colon, "FeatureTyping"),
        [
            "KwSpecialization",
            "Identification",
            "KwTyping",
            "QualifiedName",
            "Colon",
            "QualifiedName",
            "RelationshipBody"
        ],
        "{colon}"
    );
    // The type is a GeneralType, so a feature chain too (8.2.4.1.2); the typed feature
    // is a name.
    let chained = render(&kerml_accepted("typing f : a.b;").syntax());
    assert_eq!(
        child_kinds(&chained, "FeatureTyping"),
        [
            "KwTyping",
            "QualifiedName",
            "Colon",
            "OwnedFeatureChain",
            "RelationshipBody"
        ],
        "{chained}"
    );
    // KerML 7.3.4.3's examples.
    kerml_accepted(
        "specialization t2 typing employer : Organization {\n    doc /* An employer is an \
         Organization. */\n}",
    );
    kerml_accepted(
        "package P { typing customer typed by Person; typing employer : Organization; }",
    );
    kerml_accepted("class C { private typing f : T; }");
}

#[test]
fn a_feature_typing_is_bounded_by_its_rules() {
    kerml_rejected("typing f;");
    kerml_rejected("typing f :;");
    kerml_rejected("typing f typed B;");
    // KerML spells it `typed by`; `defined by` is SysML's DEFINED_BY (8.2.2.1.2).
    kerml_rejected("typing f defined by B;");
    // The typed feature is `[QualifiedName]` alone (8.2.4.3.2). Held as a file by
    // tests/rejection/kerml-feature-typing-names-its-typed-feature.kerml.
    kerml_rejected("typing a.b : B;");
    // One type: a list is a feature's own typings (7.3.4.3).
    kerml_rejected("typing f : A, B;");
    // SysML states no FeatureTyping declaration (ADR-0014).
    let sysml = parse("typing f : B;", Language::SysMl);
    assert!(!sysml.errors().is_empty(), "{:?}", sysml.errors());
}

// -- Subsetting, KerML 8.2.4.3.3 ----------------------------------------------------
//
//   Subsetting = ( 'specialization' Identification )?
//                'subset' SpecificType SUBSETS GeneralType RelationshipBody
//   SUBSETS    = ':>' | 'subsets'                                        (8.2.2.7)
//
// A NonFeatureElement (8.2.3.4.3).

#[test]
fn a_subsetting_reads_the_corpus_forms() {
    // Simple Tests/Features.kerml:45-46.
    let named =
        render(&kerml_accepted("specialization Sub subset parent subsets person;").syntax());
    assert_eq!(
        child_kinds(&named, "Subsetting"),
        [
            "KwSpecialization",
            "Identification",
            "KwSubset",
            "QualifiedName",
            "KwSubsets",
            "QualifiedName",
            "RelationshipBody"
        ],
        "{named}"
    );
    kerml_accepted("specialization subset mother subsets parent;");
    // Simple Tests/FeatureChains.kerml:23: feature chains on both sides.
    let chained = render(&kerml_accepted("subset g.g subsets b.f.a;").syntax());
    assert_eq!(
        child_kinds(&chained, "Subsetting"),
        [
            "KwSubset",
            "OwnedFeatureChain",
            "KwSubsets",
            "OwnedFeatureChain",
            "RelationshipBody"
        ],
        "{chained}"
    );
    // KerML 7.3.4.4's examples, and the symbol.
    kerml_accepted(
        "specialization subset mother subsets parent {\n    doc /* All mothers are parents. */\n}",
    );
    kerml_accepted(
        "package P { subset rearWheels subsets wheels; subset rearWheels :> driveWheels; }",
    );
}

#[test]
fn a_subsetting_is_bounded_by_its_rules() {
    kerml_rejected("subset f;");
    kerml_rejected("subset f subsets;");
    kerml_rejected("specialization S f subsets g;");
    // SUBSETS is `:>` or `subsets`; `specializes` is SPECIALIZES's word (8.2.2.7).
    kerml_rejected("subset f specializes g;");
    // One subsetted feature: a list is a feature's own subsettings (7.3.4.4). Held as a
    // file by tests/rejection/kerml-subsetting-relates-one-subsetted-feature.kerml.
    kerml_rejected("subset f subsets g, h;");
    // SysML states no Subsetting declaration (ADR-0014).
    let sysml = parse("subset f :> g;", Language::SysMl);
    assert!(!sysml.errors().is_empty(), "{:?}", sysml.errors());
}

// -- Redefinition, KerML 8.2.4.3.4 --------------------------------------------------
//
//   Redefinition = ( 'specialization' Identification )?
//                  'redefinition' SpecificType REDEFINES GeneralType RelationshipBody
//   REDEFINES    = ':>>' | 'redefines'                                   (8.2.2.7)
//
// A NonFeatureElement (8.2.3.4.3).

#[test]
fn a_redefinition_reads_the_corpus_forms() {
    // Simple Tests/Features.kerml:68-71.
    let named = render(
        &kerml_accepted(
            "specialization Redef redefinition LegalRecord::guardian redefines parent;",
        )
        .syntax(),
    );
    assert_eq!(
        child_kinds(&named, "Redefinition"),
        [
            "KwSpecialization",
            "Identification",
            "KwRedefinition",
            "QualifiedName",
            "KwRedefines",
            "QualifiedName",
            "RelationshipBody"
        ],
        "{named}"
    );
    kerml_accepted(
        "specialization redefinition Vehicle::vin redefines RegisteredAsset::identifier;",
    );
    kerml_accepted("redefinition Vehicle::vin redefines legalIdentification;");
    // Simple Tests/FeatureChains.kerml:24: feature chains on both sides.
    let chained = render(&kerml_accepted("redefinition b.f redefines b.a;").syntax());
    assert_eq!(
        child_kinds(&chained, "Redefinition"),
        [
            "KwRedefinition",
            "OwnedFeatureChain",
            "KwRedefines",
            "OwnedFeatureChain",
            "RelationshipBody"
        ],
        "{chained}"
    );
    // KerML 7.3.4.5's example with its body, and the symbol.
    kerml_accepted(
        "specialization redefinition Vehicle::vin redefines RegisteredAsset::identifier {\n    \
         doc /* A \"vin\" is a Vehicle Identification Number. */\n}",
    );
    let symbol = render(&kerml_accepted("redefinition a :>> b;").syntax());
    assert_eq!(
        child_kinds(&symbol, "Redefinition"),
        [
            "KwRedefinition",
            "QualifiedName",
            "ColonGtGt",
            "QualifiedName",
            "RelationshipBody"
        ],
        "{symbol}"
    );
}

#[test]
fn a_redefinition_is_bounded_by_its_rules() {
    kerml_rejected("redefinition f;");
    kerml_rejected("redefinition f redefines;");
    kerml_rejected("specialization R f redefines g;");
    // REDEFINES is `:>>` or `redefines`; `:>` is SUBSETS's and SPECIALIZES's (8.2.2.7).
    // Held as a file by tests/rejection/kerml-redefinition-is-written-with-redefines.kerml.
    kerml_rejected("redefinition f :> g;");
    kerml_rejected("redefinition f subsets g;");
    // One redefined feature: a list is a feature's own redefinitions (7.3.4.5).
    kerml_rejected("redefinition f redefines g, h;");
    // SysML states no Redefinition declaration (ADR-0014).
    let sysml = parse("redefinition f :>> g;", Language::SysMl);
    assert!(!sysml.errors().is_empty(), "{:?}", sysml.errors());
}

// -- Disjoining, KerML 8.2.4.1.4 ----------------------------------------------------
//
//   Disjoining = ( 'disjoining' Identification )?
//                'disjoint' ( [QualifiedName] | FeatureChain )
//                'from'     ( [QualifiedName] | FeatureChain )
//                RelationshipBody
//
// A NonFeatureElement (8.2.3.4.3). NOT the DisjoiningPart a declaration ends in,
// `classifier D disjoint from C`, which has no first target (8.2.4.1.1).

#[test]
fn a_disjoining_reads_the_corpus_forms() {
    // Simple Tests/FeatureChains.kerml:28: a feature chain on both sides.
    let chained = render(&kerml_accepted("disjoint b.f.a from b.a;").syntax());
    assert_eq!(
        child_kinds(&chained, "Disjoining"),
        [
            "KwDisjoint",
            "OwnedFeatureChain",
            "KwFrom",
            "OwnedFeatureChain",
            "RelationshipBody"
        ],
        "{chained}"
    );
    // KerML 7.3.2.5's examples, every one.
    let named = render(&kerml_accepted("disjoining Disj disjoint A from B;").syntax());
    assert_eq!(
        child_kinds(&named, "Disjoining"),
        [
            "KwDisjoining",
            "Identification",
            "KwDisjoint",
            "QualifiedName",
            "KwFrom",
            "QualifiedName",
            "RelationshipBody"
        ],
        "{named}"
    );
    kerml_accepted("disjoining disjoint Mammal from Mineral;");
    kerml_accepted(
        "disjoining disjoint Person::parents from Person::children {\n    doc /* No Person can \
         have a parent as a child. */\n}",
    );
    kerml_accepted(
        "package P {\n\tdisjoint A from B;\n\tdisjoint Mammal from Mineral;\n\tdisjoint \
         Person::parents from Person::children;\n}",
    );
    // Beside a classifier's DisjoiningPart, which reads the same two words after a
    // declaration: Simple Tests/Classifiers.kerml:13.
    let both = render(
        &kerml_accepted("classifier D disjoint from C differences A, B; disjoint C from D;")
            .syntax(),
    );
    assert!(has_node(&both, "DisjoiningPart"), "{both}");
    assert!(has_node(&both, "Disjoining"), "{both}");
}

#[test]
fn a_disjoining_is_bounded_by_its_rules() {
    kerml_rejected("disjoint A;");
    kerml_rejected("disjoint A from;");
    // The first type is written: `disjoint from B` is a DisjoiningPart's text, and a
    // part only ends a declaration (8.2.4.1.1).
    kerml_rejected("disjoint from B;");
    kerml_rejected("disjoining D A from B;");
    // One type on each side: a list is a type's own DisjoiningPart (7.3.2.5). Held as a
    // file by tests/rejection/kerml-disjoining-relates-two-types.kerml.
    kerml_rejected("disjoint A from B, C;");
    // SysML states no Disjoining declaration (ADR-0014).
    let sysml = parse("disjoint A from B;", Language::SysMl);
    assert!(!sysml.errors().is_empty(), "{:?}", sysml.errors());
}

// -- Conjugation, KerML 8.2.4.1.3 ---------------------------------------------------
//
//   Conjugation = ( 'conjugation' Identification )?
//                 'conjugate' ( [QualifiedName] | FeatureChain )
//                 CONJUGATES  ( [QualifiedName] | FeatureChain )
//                 RelationshipBody
//   CONJUGATES  = '~' | 'conjugates'                                      (8.2.2.7)
//
// A NonFeatureElement (8.2.3.4.3). NOT the ConjugationPart a declaration may write,
// `feature f conjugates g`, which has no first target (8.2.4.1.1).

#[test]
fn a_conjugation_reads_the_corpus_forms() {
    // Simple Tests/Types.kerml:25-26, one of each CONJUGATES spelling.
    let word = render(
        &kerml_accepted("conjugation c1 conjugate Conjugate1 conjugates Original;").syntax(),
    );
    assert_eq!(
        child_kinds(&word, "Conjugation"),
        [
            "KwConjugation",
            "Identification",
            "KwConjugate",
            "QualifiedName",
            "KwConjugates",
            "QualifiedName",
            "RelationshipBody"
        ],
        "{word}"
    );
    let tilde = render(&kerml_accepted("conjugation c2 conjugate Conjugate2 ~ Original;").syntax());
    assert_eq!(
        child_kinds(&tilde, "Conjugation"),
        [
            "KwConjugation",
            "Identification",
            "KwConjugate",
            "QualifiedName",
            "Tilde",
            "QualifiedName",
            "RelationshipBody"
        ],
        "{tilde}"
    );
    // KerML 7.3.2.4's examples: a body, and the keyword omitted.
    kerml_accepted(
        "conjugation c2 conjugate Conjugate2 ~ Original {\n    doc /* This conjugation is \
         equivalent to c1. */\n}",
    );
    kerml_accepted(
        "package P { conjugate Conjugate1 conjugates Original; conjugate Conjugate2 ~ Original; }",
    );
    // Either type may be a feature chain (8.2.4.1.3).
    let chained = render(&kerml_accepted("conjugate a.b ~ c.d;").syntax());
    assert_eq!(
        child_kinds(&chained, "Conjugation"),
        [
            "KwConjugate",
            "OwnedFeatureChain",
            "Tilde",
            "OwnedFeatureChain",
            "RelationshipBody"
        ],
        "{chained}"
    );
}

#[test]
fn a_conjugation_is_bounded_by_its_rules() {
    kerml_rejected("conjugate A;");
    kerml_rejected("conjugate A ~;");
    kerml_rejected("conjugation c A ~ B;");
    // `conjugates B;` is not a Conjugation missing its first type but a keywordless
    // Feature whose FeatureDeclaration is a bare ConjugationPart: `( EndFeaturePrefix |
    // BasicFeaturePrefix ) FeatureDeclaration`, the prefix empty (8.2.4.3.1).
    let feature = render(&kerml_accepted("conjugates B;").syntax());
    assert!(has_node(&feature, "ConjugationPart"), "{feature}");
    assert!(!has_node(&feature, "Conjugation"), "{feature}");
    // One original type: the production writes no list (8.2.4.1.3). Held as a file by
    // tests/rejection/kerml-conjugation-relates-two-types.kerml.
    kerml_rejected("conjugate A ~ B, C;");
    // CONJUGATES is `~` or `conjugates`; `:>` is SPECIALIZES's (8.2.2.7).
    kerml_rejected("conjugate A :> B;");
    // SysML states no Conjugation declaration (ADR-0014).
    let sysml = parse("conjugate A ~ B;", Language::SysMl);
    assert!(!sysml.errors().is_empty(), "{:?}", sysml.errors());
}

// -- FeatureMember, KerML 8.2.4.1.6 -----------------------------------------------
//
//   TypeBodyElement    = NonFeatureMember | FeatureMember | AliasMember | Import
//   FeatureMember      = TypeFeatureMember | OwnedFeatureMember
//   TypeFeatureMember  = MemberPrefix 'member' FeatureElement
//   OwnedFeatureMember = MemberPrefix FeatureElement

#[test]
fn a_type_body_owns_a_feature_through_a_feature_member() {
    // A TypeBody's feature is an OwnedFeatureMember (8.2.4.1.6), a FeatureMembership,
    // not the namespace body's NamespaceFeatureMember (8.2.3.4.1): KerML Spec Annex A
    // Examples/A-3-6-Sequences.kerml:8, in a behavior's body.
    let tree = render(&kerml_accepted("behavior B {\n\tstep paint : Paint [1];\n}").syntax());
    assert_eq!(
        child_kinds(&tree, "OwnedFeatureMember"),
        ["MemberPrefix", "Step"],
        "{tree}"
    );
    assert!(!has_node(&tree, "NamespaceFeatureMember"), "{tree}");
    // Every FeatureElement this parser reads, in every classifier's body.
    let class = render(
        &kerml_accepted(
            "class C { feature f; private x : T; connector c from a to b; binding a = b; \
             succession a then b; }",
        )
        .syntax(),
    );
    assert_eq!(class.matches("OwnedFeatureMember").count(), 5, "{class}");
    // A package body keeps the namespace's member.
    let package = render(&kerml_accepted("package P { feature f; }").syntax());
    assert!(has_node(&package, "NamespaceFeatureMember"), "{package}");
    assert!(!has_node(&package, "OwnedFeatureMember"), "{package}");
}

#[test]
fn a_type_body_owns_a_feature_through_member_too() {
    // Variable Feature Examples/TimeVaryingCarDriver.kerml:65, a TypeFeatureMember: the
    // feature is a member of the type without being its owned feature (8.2.4.1.6).
    let tree = render(
        &kerml_accepted(
            "struct Driver {\n\tmember feature isLicensed : Boolean [1] featured by \
             Person_snapshots { }\n}",
        )
        .syntax(),
    );
    assert_eq!(
        child_kinds(&tree, "TypeFeatureMember"),
        ["MemberPrefix", "KwMember", "Feature"],
        "{tree}"
    );
    // Variable Feature Examples/Enhancements/TimeVaryingSteps.kerml:11, a step, here after
    // a visibility. (Its line 4, `member step merge : ...`, waits on pending decision
    // keyword-table-per-language: `merge` is SysML's reserved word, not KerML's.)
    kerml_accepted(
        "behavior TakePicture { private member step focus [0..1] featured by \
         TakePicture_snapshots { } }",
    );
}

#[test]
fn a_type_feature_member_is_bounded_by_its_rules() {
    // `member` is a TypeBody's: NamespaceBodyElement reaches NamespaceMember, which has no
    // such alternative (8.2.3.4.1). Held as a file by
    // tests/rejection/kerml-type-feature-member-is-a-type-body-s.kerml.
    kerml_rejected("package P { member feature x; }");
    kerml_rejected("member feature x;");
    // It owns a FeatureElement, not a classifier or a package. Held as a file by
    // tests/rejection/kerml-type-feature-member-owns-a-feature-element.kerml.
    kerml_rejected("class C { member class D; }");
    // The MemberPrefix comes first.
    kerml_rejected("class C { member private feature x; }");
}

// -- Step, KerML 8.2.5.6.2 ---------------------------------------------------------
//
//   Step = FeaturePrefix 'step' FeatureDeclaration ValuePart? TypeBody
//   (FeatureDeclaration? by deviation Step, follow_xtext)

#[test]
fn a_step_is_a_feature_element() {
    // KerML Spec Annex A Examples/A-3-6-Sequences.kerml:8. In a package body it is owned
    // through a NamespaceFeatureMember (8.2.3.4.1); in a behavior's body, a TypeBody,
    // through a FeatureMember instead (8.2.4.1.1, 8.2.4.1.6), so the membership is asserted here.
    let tree = render(&kerml_accepted("package P {\n\tstep paint : Paint [1];\n}").syntax());
    assert_eq!(
        child_kinds(&tree, "NamespaceFeatureMember"),
        ["MemberPrefix", "Step"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "Step"),
        ["FeaturePrefix", "KwStep", "FeatureDeclaration", "TypeBody"],
        "{tree}"
    );
    // A-3-8-ChangingFeatureValues.kerml:12, with a body; and the declaration that is a
    // FeatureSpecializationPart alone, `step redefines paint : MyPaint {` (A-3-8:155),
    // here with the `;` TypeBody.
    kerml_accepted("behavior B { step paint : Paint [1] { in x; } }");
    kerml_accepted("behavior B { step redefines paint : MyPaint { } }");
    kerml_accepted("behavior B { step redefines paint : MyPaint; }");
    // Behavior Examples/TakePicture.kerml:11, no space before the `:`.
    kerml_accepted("behavior TakePicture { step step1: Focus[1]; }");
    // A FeaturePrefix and a ValuePart (8.2.5.6.2), and prefix metadata before the
    // keyword: `#command step previousAction[1];` (7.4.13, receipt 5e755297).
    kerml_accepted("in step s : S = t;");
    kerml_accepted("#command step previousAction[1];");
    // A step is a member of a namespace too, not only of a behavior (8.2.3.4.1).
    kerml_accepted("package P { step s; }");
}

#[test]
fn a_step_without_a_declaration_is_admitted_by_deviation() {
    // The clause writes FeatureDeclaration without `?`; deviation Step (follow_xtext)
    // makes it optional, as Connector's is (8.2.5.5.1). The text parses and carries a
    // PARSE-DEVIATION note naming the entry (ADR-0022).
    let parsed = kerml_accepted("behavior B { step; }");
    let notes: Vec<String> = parsed
        .deviations()
        .iter()
        .map(|d| d.message().to_owned())
        .collect();
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(notes[0].contains("deviation Step"), "{notes:?}");
    // A declared step is the specification's, and carries none.
    assert!(
        kerml_accepted("behavior B { step s; }")
            .deviations()
            .is_empty()
    );
}

#[test]
fn a_step_is_bounded_by_its_rules() {
    // TypeBody is not optional. Held as a file by
    // tests/rejection/kerml-step-needs-a-type-body.kerml.
    kerml_rejected("behavior B { step s : S }");
    // The FeaturePrefix comes before `step`, never after it.
    kerml_rejected("behavior B { step in s; }");
    // SysML has no `step`: its steps are action usages (SysML 8.2.2.17). Held as a file
    // by tests/rejection/step-is-not-sysml.sysml.
    assert!(
        !parse("action def A { step s; }", Language::SysMl)
            .errors()
            .is_empty()
    );
}

// -- the invariants, under this grammar too ---------------------------------------

#[test]
fn parsing_kerml_never_panics_on_truncated_input() {
    let source = "package <C> Classes { public import A::B::*; alias X for Y; filter @S; }";
    for end in 0..=source.len() {
        if let Some(prefix) = source.get(..end) {
            assert_eq!(parse(prefix, Language::KerMl).text(), prefix);
        }
    }
}

/// The tree as indented text, so a membership node can be asserted on by name.
fn render(node: &SyntaxNode) -> String {
    let mut out = String::new();
    write_element(&mut out, node.clone().into(), 0);
    out
}

/// The DIRECT children of the first `kind` node in `rendered`, trivia skipped.
///
/// The helper tests/parser.rs states at length: a production says what it OWNS, which
/// is one level down, and the author's spacing is no part of it. `RegularComment` is
/// kept, because in an annotating position it is a token.
fn child_kinds(rendered: &str, kind: &str) -> Vec<String> {
    const TRIVIA: [&str; 3] = ["Whitespace", "SingleLineNote", "MultilineNote"];
    let mut lines = rendered.lines().skip_while(|l| l.trim_start() != kind);
    let Some(head) = lines.next() else {
        return Vec::new();
    };
    let depth = head.len() - head.trim_start().len();
    lines
        .take_while(|l| l.len() - l.trim_start().len() > depth)
        .filter(|l| l.len() - l.trim_start().len() == depth + 2)
        .map(|l| l.split_whitespace().next().unwrap_or_default().to_owned())
        .filter(|name| !TRIVIA.contains(&name.as_str()))
        .collect()
}

fn write_element(out: &mut String, element: SyntaxElement, depth: usize) {
    let pad = "  ".repeat(depth);
    match element {
        SyntaxElement::Node(node) => {
            let _ = writeln!(out, "{pad}{:?}", node.kind());
            for child in node.children_with_tokens() {
                write_element(out, child, depth + 1);
            }
        }
        SyntaxElement::Token(token) => {
            let _ = writeln!(out, "{pad}{:?} {:?}", token.kind(), token.text());
        }
    }
}
