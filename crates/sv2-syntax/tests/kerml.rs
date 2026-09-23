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
//! `KerML`'s alone. Of `FeatureElement`'s ten alternatives, `Feature`, `Succession` and
//! `BindingConnector` are implemented; the other seven are not, nor are `Type`,
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
    // Succession and BindingConnector are implemented; the other seven are not, and
    // reporting them is the honest state. `succession flow` is SuccessionFlow, one of
    // the seven.
    for source in [
        "connector c from a to b;",
        "succession flow f from a to b;",
        "step s;",
        "inv { true }",
    ] {
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
fn the_unimplemented_halves_of_a_classifier_declaration_are_reported() {
    // ConjugationPart is the other alternative of ClassifierDeclaration's one
    // alternation, and TypeRelationshipPart is its trailing star. Neither is
    // implemented, and each has a file in tests/rejection/ naming its clause.
    kerml_rejected("class B conjugates A;");
    kerml_rejected("class B ~ A;");
    kerml_rejected("classifier C unions A, B;");
    // OwnedMultiplicity on a classifier, the remaining unimplemented slot.
    kerml_rejected("classifier C [1..*];");
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

#[test]
fn the_unimplemented_halves_of_a_feature_declaration_are_reported() {
    kerml_rejected("feature f conjugates g;");
    kerml_rejected("feature f chains a.b;");
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
    // OwnedCrossMultiplicityMember, ConnectorEnd's first part, is unimplemented — rejected
    // BY ABSENCE, and tests/rejection/kerml-connector-end-cross-multiplicity-is-not-implemented.kerml
    // holds it.
    kerml_rejected("succession first [1] a then b;");
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
    // OwnedCrossMultiplicityMember, ConnectorEnd's first part, is unimplemented — rejected
    // BY ABSENCE, as for the succession.
    kerml_rejected("binding of [1] a = b;");
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
