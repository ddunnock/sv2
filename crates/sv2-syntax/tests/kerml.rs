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
//! Every one of `NonFeatureElement`'s alternatives is implemented, the nine standalone
//! relationship declarations `TypeFeaturing` last among them, and so is every one of
//! `FeatureElement`'s ten. `Package` is a shared unit — the same production in both
//! grammars — `Dependency` is stated in each, and the rest are `KerML`'s alone.

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

// -- classifiers, KerML 8.2.4.2 ---------------------------------------------------
//
// Classifier  = TypePrefix 'classifier'  ClassifierDeclaration TypeBody
// Class       = TypePrefix 'class'       ClassifierDeclaration TypeBody
// Structure   = TypePrefix 'struct'      ClassifierDeclaration TypeBody
// DataType    = TypePrefix 'datatype'    ClassifierDeclaration TypeBody
// Metaclass   = TypePrefix 'metaclass'   ClassifierDeclaration TypeBody
// Association = TypePrefix 'assoc'       ClassifierDeclaration TypeBody
// AssociationStructure = TypePrefix 'assoc' 'struct' ClassifierDeclaration TypeBody
//                                                                     (8.2.5.4)
// Behavior    = TypePrefix 'behavior'    ClassifierDeclaration TypeBody
// Interaction = TypePrefix 'interaction' ClassifierDeclaration TypeBody

/// Every keyword of the shared spine, taken from the derived units rather than from
/// the clause prose, because the units are what the parser's table transcribes.
const CLASSIFIER_KEYWORDS: [&str; 9] = [
    "classifier",
    "class",
    "struct",
    "datatype",
    "metaclass",
    "assoc",
    "assoc struct",
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
    // FunctionBody, so they are not the same spine; they are next (`function f;` and
    // `predicate p;` were here, rejected by absence).
    //
    // Type takes a TypeDeclaration rather than a ClassifierDeclaration, which requires
    // a SpecializationPart or a ConjugationPart where a classifier's are optional
    // (8.2.4.1.1; deviation TypeDeclaration, follow_spec): `type T;` is rejected by that
    // rule, and read by
    // `a_type_declaration_takes_its_parts_as_its_production_writes_them` once it writes
    // one.
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

// AnnotatingElement = Comment | Documentation | TextualRepresentation | MetadataFeature
//                                                                     (KerML 8.2.3.3.1)
//
// Reached as a MemberElement (8.2.3.4.1) and as a relationship body's OwnedAnnotation
// (8.2.3.1). One instance of each alternative, with the node its production builds, at
// both. The Comment is written with its keyword: a bare REGULAR_COMMENT at member
// position is still trivia here (see `at_annotating_member` in the parser), which is
// MemberElement's gap, not this alternation's.
const ANNOTATING_ELEMENTS: [(&str, &str); 4] = [
    ("comment C about X /* on X */", "Comment"),
    ("doc /* on the owner */", "Documentation"),
    ("rep r language \"alf\" /* f(); */", "TextualRepresentation"),
    // KerML's fourth alternative is the clause's own MetadataFeature (8.2.5.12); SysML's
    // is a MetadataUsage by deviation AnnotatingElement.
    ("metadata m : M;", "MetadataFeature"),
];

#[test]
fn every_annotating_element_is_a_member_and_an_owned_annotation() {
    for (element, kind) in ANNOTATING_ELEMENTS {
        let member = render(&kerml_accepted(&format!("package P {{ {element} }}")).syntax());
        assert!(has_node(&member, kind), "{kind}: {member}");
        let owned = render(&kerml_accepted(&format!("featuring y by C {{ {element} }}")).syntax());
        assert!(has_node(&owned, "OwnedAnnotation"), "{owned}");
        assert!(has_node(&owned, kind), "{kind}: {owned}");
    }
    // `@` is MetadataFeature's other keyword (8.2.5.12), at both sites.
    kerml_accepted("package P { @M; }");
    kerml_accepted("featuring y by C { @M; }");
    // Of the four only MetadataFeature takes PrefixMetadataMembers (8.2.5.12);
    // Documentation opens on `doc` (8.2.3.3.2).
    kerml_accepted("featuring y by C { #S metadata m : M; }");
    kerml_rejected("featuring y by C { #S doc /* d */ }");
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

// Typings            = TypedBy ( ',' ownedRelationship += OwnedFeatureTyping )*
// TypedBy            = TYPED_BY ownedRelationship += OwnedFeatureTyping      (8.2.4.3.1)
// OwnedFeatureTyping = GeneralType                                           (8.2.4.3.2)
//
// KerML's TypedBy owns its OwnedFeatureTyping directly. SysML's owns a FeatureTyping,
// `OwnedFeatureTyping | ConjugatedPortTyping` (SysML 8.2.2.6.5), an alternation KerML
// does not state, so no FeatureTyping node stands between them in a .kerml file.

#[test]
fn a_kerml_typed_by_owns_its_feature_typing_directly() {
    // Simple Tests/Features.kerml:8, both spellings of TYPED_BY and a second typing.
    let words = render(&kerml_accepted("feature x typed by A, B;").syntax());
    assert_eq!(
        child_kinds(&words, "Typings"),
        ["TypedBy", "Comma", "OwnedFeatureTyping"],
        "{words}"
    );
    assert_eq!(
        child_kinds(&words, "TypedBy"),
        ["KwTyped", "KwBy", "OwnedFeatureTyping"],
        "{words}"
    );
    let colon = render(&kerml_accepted("feature parent[1..2] : Person;").syntax());
    assert_eq!(
        child_kinds(&colon, "TypedBy"),
        ["Colon", "OwnedFeatureTyping"],
        "{colon}"
    );
    // OwnedFeatureTyping = GeneralType, a name or an OwnedFeatureChain (8.2.4.1.2).
    let chained = render(&kerml_accepted("feature f : a.b;").syntax());
    assert_eq!(
        child_kinds(&chained, "OwnedFeatureTyping"),
        ["OwnedFeatureChain"],
        "{chained}"
    );
    // The standalone declaration keeps its FeatureTyping node (8.2.4.3.2).
    assert!(has_node(
        &render(&kerml_accepted("typing f : T;").syntax()),
        "FeatureTyping"
    ));
    // SysML's TypedBy owns a FeatureTyping (SysML 8.2.2.6.5).
    let sysml = parse("part p : P;", Language::SysMl);
    assert!(sysml.errors().is_empty(), "{:?}", sysml.errors());
    assert_eq!(
        child_kinds(&render(&sysml.syntax()), "TypedBy"),
        ["Colon", "FeatureTyping"]
    );
    // A ConjugatedPortTyping is SysML's alone (8.2.2.12).
    kerml_rejected("feature f : ~T;");
    kerml_rejected("feature f : A, ~T;");
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

// -- ExpressionBody, KerML 8.2.5.8.3 -------------------------------------------------
//
//   BodyExpression       = ownedRelationship += ExpressionBodyMember
//   ExpressionBodyMember = ownedMemberFeature = ExpressionBody
//   ExpressionBody       = '{' FunctionBodyPart '}'
//
// KerML's own, over the FunctionBodyPart of 8.2.5.7.1. SysML states none and reads a
// CalculationBody by deviation ExpressionBody; a .kerml body is read as KerML's, with no
// deviation note.

#[test]
fn a_kerml_expression_body_reads_the_corpus_forms() {
    // Simple Tests/Expressions.kerml:15, a collect: a parameter, then the result.
    let parsed = kerml_accepted("package P { c = x->collect {in xx; xx + 1}; }");
    assert!(parsed.deviations().is_empty(), "{:?}", parsed.deviations());
    let tree = render(&parsed.syntax());
    assert_eq!(
        child_kinds(&tree, "ExpressionBody"),
        ["LBrace", "FunctionBodyPart", "RBrace"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "FunctionBodyPart"),
        ["OwnedFeatureMember", "ResultExpressionMember"],
        "{tree}"
    );
    assert!(!has_node(&tree, "CalculationBody"), "{tree}");
    // Expressions.kerml:16-19: the collect and select shorthands, a select, and a
    // reduce whose body is followed by a second reduce by function reference.
    kerml_accepted(
        "package P {\n\tc1 = x.{in xx; xx + 1}; \n\td = x->select {in xx; xx != null};\n\t\
         d1 = x.?{in xx; xx != null};\n\te = x->reduce {in s; in t; s + t}->reduce '+';\n}",
    );
    // Expansion.kerml:3: three parameters.
    kerml_accepted("package Expansion { feature x = x->select {in y; in w; in z; w+1}; }");
    // BaseExpression's BodyExpression alternative (8.2.5.8.3), where an operand is.
    let base = render(&kerml_accepted("feature c = { 1 };").syntax());
    assert!(has_node(&base, "BodyExpression"), "{base}");
}

#[test]
fn a_kerml_expression_body_nests_in_a_result_expression() {
    // A result expression may hold a body of its own: the inner `{` opens a
    // BodyExpression after `->select` (8.2.5.8.2), and completes no feature, so `y` is
    // the select's operand and not a keywordless feature named `y`.
    let tree = render(
        &kerml_accepted("package P { feature c = x->collect { in xx; y->select { in z; z } }; }")
            .syntax(),
    );
    assert_eq!(
        tree.lines()
            .filter(|l| l.trim() == "ExpressionBody")
            .count(),
        2,
        "{tree}"
    );
    assert_eq!(
        tree.lines()
            .filter(|l| l.trim() == "ResultExpressionMember")
            .count(),
        2,
        "{tree}"
    );
    // And a keywordless feature valued by a body is still an item, not the result: its
    // `;` completes it after the body closes.
    let item = render(&kerml_accepted("inv { c = { 1 }; c }").syntax());
    assert_eq!(
        child_kinds(&item, "FunctionBodyPart"),
        ["OwnedFeatureMember", "ResultExpressionMember"],
        "{item}"
    );
}

#[test]
fn a_kerml_expression_body_is_bounded_by_its_rules() {
    // Its items are KerML's: no SysML usage (ADR-0014). Held as a file by
    // tests/rejection/kerml-expression-body-owns-kerml-elements.kerml.
    kerml_rejected("feature c = x->collect { part p; 1 };");
    // The result expression takes no `;` (8.2.5.7.1).
    kerml_rejected("feature c = x->collect { in xx; xx + 1; };");
    kerml_rejected("feature c = x->collect { in xx; xx + 1 ;");
    // Braced only: `;` is FunctionBody's other form, not ExpressionBody's (8.2.5.8.3).
    kerml_rejected("feature c = x->collect ;;");
}

// -- Function and Predicate, KerML 8.2.5.7.1, 8.2.5.7.3 ------------------------------
//
//   Function  = TypePrefix 'function'  ClassifierDeclaration FunctionBody
//   Predicate = TypePrefix 'predicate' ClassifierDeclaration FunctionBody

#[test]
fn a_function_reads_the_corpus_forms() {
    // Simple Tests/Expressions.kerml:46-48: parameters, then a result expression whose
    // collect body nests in it.
    let tree = render(
        &kerml_accepted(
            "function TotalMass { in partMass; in subparts;\n\t\tpartMass + (subparts->collect \
             {in p; totalMass(partMass, subparts)}->reduce '+' ?? 0.0)\n\t}",
        )
        .syntax(),
    );
    assert_eq!(
        child_kinds(&tree, "Function"),
        [
            "TypePrefix",
            "KwFunction",
            "ClassifierDeclaration",
            "FunctionBody"
        ],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "FunctionBodyPart")[2..],
        ["ResultExpressionMember"],
        "{tree}"
    );
}

#[test]
fn a_function_reads_the_examples_of_7_4_8_2() {
    kerml_accepted(
        "// Specializes Performances::Evaluation by default.\nfunction Velocity {\n    in v_i : \
         VelocityValue;\n    in a : AccelerationValue;\n    in dt : TimeValue;\n    return v_f : \
         VelocityValue;\n}",
    );
    kerml_accepted(
        "abstract function Dynamics {\n    in initialState : DynamicState;\n    in time : \
         TimeValue;\n    return : DynamicState;\n}\nfunction VehicleDynamics specializes \
         Dynamics {\n    // Each parameter redefines the corresponding superclassifier \
         parameter\n    in initialState : VehicleState;\n    in time : TimeValue;\n    return : \
         VehicleState;\n}",
    );
    kerml_accepted(
        "function Average {\n    in scores[1..*] : Rational;\n    return : Rational;\n\n    \
         sum(scores) / size(scores)\n}",
    );
    kerml_accepted(
        "function Average {\n    in scores[1..*] : Rational;\n    return : Rational = \
         sum(scores) / size(scores);\n}",
    );
}

#[test]
fn a_predicate_reads_the_examples_of_7_4_8_4() {
    let tree = render(
        &kerml_accepted(
            "predicate isAssembled {\n    in assembly : Assembly;\n    in subassemblies[*] : \
             Assembly;\n}",
        )
        .syntax(),
    );
    assert_eq!(
        child_kinds(&tree, "Predicate"),
        [
            "TypePrefix",
            "KwPredicate",
            "ClassifierDeclaration",
            "FunctionBody"
        ],
        "{tree}"
    );
    kerml_accepted(
        "predicate isFull {\n    in tank : FuelTank;\n    tank.fuelLevel == tank.maxFuelLevel\n}",
    );
    // A predicate is a Function (8.3.4.7.6); both are NonFeatureElements, owned as
    // members and in a type body alike.
    kerml_accepted("package P { abstract #M function f; class C { private predicate p; } }");
}

#[test]
fn a_function_is_bounded_by_its_rules() {
    // A FunctionBody, `;` or braced (8.2.5.7.1). Held as a file by
    // tests/rejection/kerml-function-ends-in-a-function-body.kerml.
    kerml_rejected("function f");
    kerml_rejected("function f { 1; }");
    // ClassifierDeclaration, not FeatureDeclaration: a classifier's part, not a typing.
    kerml_rejected("function f : T;");
    // SysML's are `calc def` and `constraint def` (ADR-0014).
    let sysml = parse("function f;", Language::SysMl);
    assert!(!sysml.errors().is_empty(), "{:?}", sysml.errors());
}

// -- Flow and SuccessionFlow, KerML 8.2.5.9.2 ---------------------------------------
//
//   Flow            = FeaturePrefix 'flow' FlowDeclaration TypeBody
//   SuccessionFlow  = FeaturePrefix 'succession' 'flow' FlowDeclaration TypeBody
//   FlowDeclaration = FeatureDeclaration? ValuePart? ( 'of' PayloadFeatureMember )?
//                     ( 'from' FlowEndMember 'to' FlowEndMember )?
//                   | 'all'? FlowEndMember 'to' FlowEndMember
//   FlowEnd         = ( OwnedReferenceSubsetting '.' )? FlowFeatureMember
//
// The first alternative's `?` is deviation FlowDeclaration's (follow_xtext).

#[test]
fn a_flow_reads_the_corpus_forms() {
    // Simple Tests/Behaviors.kerml:18, the second alternative with two-segment ends,
    // and :20, a declared flow with a payload typing alone.
    let ends = render(&kerml_accepted("behavior B { flow a.y to b.x1; }").syntax());
    assert_eq!(
        child_kinds(&ends, "FlowDeclaration"),
        ["FlowEndMember", "KwTo", "FlowEndMember"],
        "{ends}"
    );
    assert_eq!(
        child_kinds(&ends, "FlowEnd"),
        ["OwnedReferenceSubsetting", "Dot", "FlowFeatureMember"],
        "{ends}"
    );
    let payload = render(&kerml_accepted("behavior B { abstract flow msg of C; }").syntax());
    assert_eq!(
        child_kinds(&payload, "FlowDeclaration"),
        ["FeatureDeclaration", "KwOf", "PayloadFeatureMember"],
        "{payload}"
    );
    assert_eq!(
        child_kinds(&payload, "PayloadFeatureMember"),
        ["PayloadFeature"],
        "{payload}"
    );
    // Behavior Examples/TakePicture.kerml:14, a succession flow with everything.
    let succession = render(
        &kerml_accepted(
            "behavior TakePicture {\n\tsuccession flow exposure[1] of Exposure from step1.xrsl \
             to step2.xsf;\n}",
        )
        .syntax(),
    );
    assert_eq!(
        child_kinds(&succession, "SuccessionFlow"),
        [
            "FeaturePrefix",
            "KwSuccession",
            "KwFlow",
            "FlowDeclaration",
            "TypeBody"
        ],
        "{succession}"
    );
}

#[test]
fn a_flow_reads_the_examples_of_7_4_10_3() {
    kerml_accepted(
        "struct Vehicle {\n    composite feature fuelTank[1] {\n        out var feature \
         fuelOut[1] : Fuel;\n    }\n    composite feature engine {\n        in var feature \
         fuelIn[1] : Fuel;\n    }\n    // The flow actually connects the fuelTank to the \
         engine.\n    // The transfer moves Fuel from fuelOut to fuelIn.\n    flow fuelFlow from \
         fuelTank::fuelOut to engine::fuelIn;\n}",
    );
    kerml_accepted(
        "feature vehicle : Vehicle {\n    // The flow actually connects the inherited \
         fuelTank\n    // feature to the inherited engine feature.\n    flow fuelFlow from \
         fuelTank.fuelOut to engine.fuelIn;\n}",
    );
    kerml_accepted("flow fuelTank.fuelOut to engine.fuelIn;");
    kerml_accepted(
        "behavior TakePicture {\n    composite step focus : Focus { out image[1] : Image; }\n    \
         composite step shoot : Shoot { in image[1] : Image; }\n    // The use of a succession \
         flow means that focus must complete before\n    // the image is transferred, after \
         which shoot can begin.\n    succession flow focus.image to shoot.image;\n}",
    );
}

#[test]
fn a_flow_end_of_three_segments_subsets_a_feature_chain() {
    // KerML's FlowEnd has no FeatureChainPrefix: its OwnedReferenceSubsetting is a name
    // or an OwnedFeatureChain (8.2.4.3.3), so `a.b.c` is the chain `a.b`, a `.`, and `c`.
    let tree = render(&kerml_accepted("flow a.b.c to d.e;").syntax());
    assert_eq!(
        child_kinds(&tree, "OwnedReferenceSubsetting"),
        ["OwnedFeatureChain"],
        "{tree}"
    );
    assert!(!has_node(&tree, "FeatureChainPrefix"), "{tree}");
    // And `all` before the ends, the second alternative's `isSufficient`.
    kerml_accepted("flow all a.b to c.d;");
    // A payload's KerML-only alternative, `Identification ValuePart` (8.2.5.9.2), with
    // every Identification and every FeatureValue spelling; a short name opens it as
    // well as the declared alternative, `<p> : T`.
    for payload in [
        "p = 1",
        "<p> = 1",
        "<p> q := 1",
        "p default 1",
        "<p> : T = 1",
    ] {
        kerml_accepted(&format!("flow f of {payload} from a.b to c.d;"));
    }
}

#[test]
fn a_flow_without_a_declaration_is_admitted_by_deviation() {
    // 7.4.10.3's `flow of flowingFuel : Fuel from ...` writes no FeatureDeclaration
    // before its payload: deviation FlowDeclaration (follow_xtext), noted (ADR-0022).
    let parsed =
        kerml_accepted("flow of flowingFuel : Fuel from fuelTank.fuelOut to engine.fuelIn;");
    let notes: Vec<String> = parsed
        .deviations()
        .iter()
        .map(|d| d.message().to_owned())
        .collect();
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(notes[0].contains("deviation FlowDeclaration"), "{notes:?}");
    // The second alternative writes no declaration by its own text, and a declared flow
    // is the specification's: neither carries a note.
    assert!(kerml_accepted("flow a.b to c.d;").deviations().is_empty());
    assert!(
        kerml_accepted("flow f from a.b to c.d;")
            .deviations()
            .is_empty()
    );
}

#[test]
fn a_flow_is_bounded_by_its_rules() {
    // `from` needs `to` (8.2.5.9.2). Held as a file by
    // tests/rejection/kerml-flow-from-needs-to.kerml.
    kerml_rejected("flow f from a.b;");
    kerml_rejected("flow a.b;");
    // The payload's multiplicity alone is no PayloadFeature (deviation PayloadFeature).
    kerml_rejected("flow of [2] from a.b to c.d;");
    // TypeBody is not optional.
    kerml_rejected("flow a.b to c.d");
    // SysML's is FlowUsage, over its own productions; `flow` in .sysml reads that, and
    // KerML's `all` before the ends is not SysML's (8.2.2.16).
    let sysml = parse("flow all a.b to c.d;", Language::SysMl);
    assert!(!sysml.errors().is_empty(), "{:?}", sysml.errors());
}

// -- FeatureElement, KerML 8.2.3.4.3 -------------------------------------------------

#[test]
fn every_feature_element_is_read_as_a_member_and_in_a_type_body() {
    // FeatureElement's ten alternatives (8.2.3.4.3), each after a FeaturePrefix, each as
    // a NamespaceFeatureMember in a package body and a FeatureMember in a type body.
    let elements = [
        ("feature f;", "Feature"),
        ("step s;", "Step"),
        ("expr e { 1 }", "Expression"),
        ("bool b { true }", "BooleanExpression"),
        ("inv i { true }", "Invariant"),
        ("connector c from a to b;", "Connector"),
        ("binding b of a = b;", "BindingConnector"),
        ("succession s first a then b;", "Succession"),
        ("flow f from a.x to b.y;", "Flow"),
        ("succession flow f from a.x to b.y;", "SuccessionFlow"),
    ];
    for (element, node) in elements {
        for (source, member) in [
            (
                format!("package P {{ abstract {element} }}"),
                "NamespaceFeatureMember",
            ),
            (
                format!("behavior B {{ abstract {element} }}"),
                "OwnedFeatureMember",
            ),
            // An owned related element of a relationship body (8.2.3.1), and a
            // function's result parameter (8.2.5.7.1): the other two sites that read one.
            (
                format!("dependency a to b {{ abstract {element} }}"),
                "RelationshipBody",
            ),
            (
                format!("function F {{ return abstract {element} }}"),
                "ReturnFeatureMember",
            ),
        ] {
            let tree = render(&kerml_accepted(&source).syntax());
            assert!(has_node(&tree, node), "{source}\n{tree}");
            assert!(has_node(&tree, member), "{source}\n{tree}");
        }
    }
}

// -- Namespace, KerML 8.2.3.4.1 ----------------------------------------------------
//
//   Namespace            = PrefixMetadataMember* NamespaceDeclaration NamespaceBody
//   NamespaceDeclaration = 'namespace' Identification
//   NamespaceBody        = ';' | '{' NamespaceBodyElement* '}'

#[test]
fn a_namespace_reads_the_examples_of_7_2_5_2() {
    let tree = render(
        &kerml_accepted(
            "namespace <'1.1'> N1; // This is an empty namespace.\nnamespace <'1.2'> N2 {\n    \
             doc /* This is an example of a namespace body. */\n    class C;\n    datatype D;\n    \
             feature f : C;\n    namespace N3; // This is a nested namespace.\n}",
        )
        .syntax(),
    );
    assert_eq!(
        tree.lines().filter(|l| l.trim() == "Namespace").count(),
        3,
        "{tree}"
    );
    let empty = render(&kerml_accepted("namespace <'1.1'> N1;").syntax());
    assert_eq!(
        child_kinds(&empty, "Namespace"),
        ["NamespaceDeclaration", "NamespaceBody"],
        "{empty}"
    );
    assert_eq!(
        child_kinds(&empty, "NamespaceDeclaration"),
        ["KwNamespace", "Identification"],
        "{empty}"
    );
    // Its body's feature is a NamespaceFeatureMember, as a package's is, and a
    // non-feature a NonFeatureMember, visibility and all.
    let body = render(
        &kerml_accepted(
            "namespace N3 {\n    public class C;\n    private datatype D;\n    feature f : C; \
             // public by default\n}",
        )
        .syntax(),
    );
    assert!(has_node(&body, "NamespaceFeatureMember"), "{body}");
    assert!(has_node(&body, "NonFeatureMember"), "{body}");
    // Prefix metadata before the keyword, an alias and an import in the body.
    kerml_accepted("#M namespace N { alias X for Y; private import A::*; }");
}

#[test]
fn a_namespace_is_bounded_by_its_rules() {
    // NamespaceBody writes no ElementFilterMember, a package body's alone (8.2.5.13). Held
    // as a file by tests/rejection/kerml-namespace-body-admits-no-filter.kerml.
    kerml_rejected("namespace N { filter true; }");
    kerml_rejected("namespace N");
    // TypePrefix's `abstract` is a type's, not a namespace's.
    kerml_rejected("abstract namespace N;");
    // SysML states no `namespace` declaration (ADR-0014).
    let sysml = parse("namespace N;", Language::SysMl);
    assert!(!sysml.errors().is_empty(), "{:?}", sysml.errors());
}

// -- Multiplicity declarations, KerML 8.2.5.11 --------------------------------------
//
//   Multiplicity       = MultiplicitySubset | MultiplicityRange
//   MultiplicitySubset = 'multiplicity' Identification Subsets TypeBody
//   MultiplicityRange  = 'multiplicity' Identification MultiplicityBounds TypeBody
//
// A NonFeatureElement (8.2.3.4.3). KerML's MultiplicityRange is this named declaration;
// the bracketed range a feature writes is OwnedMultiplicityRange (see below).

#[test]
fn a_multiplicity_reads_the_examples_of_7_4_12() {
    let range = render(&kerml_accepted("multiplicity zeroOrMore [0..*];").syntax());
    assert_eq!(
        child_kinds(&range, "MultiplicityRange"),
        // MultiplicityBounds is a fragment and builds no node (KerML.xtext:774).
        [
            "KwMultiplicity",
            "Identification",
            "LBracket",
            "MultiplicityExpressionMember",
            "DotDot",
            "MultiplicityExpressionMember",
            "RBracket",
            "TypeBody"
        ],
        "{range}"
    );
    let subset = render(&kerml_accepted("multiplicity m subsets zeroOrMore;").syntax());
    assert_eq!(
        child_kinds(&subset, "MultiplicitySubset"),
        ["KwMultiplicity", "Identification", "Subsets", "TypeBody"],
        "{subset}"
    );
    // In a feature's body, with no name: the multiplicity of the feature (7.4.12).
    let body = render(
        &kerml_accepted(
            "feature driveWheels subsets wheels {\n    multiplicity [2..n];\n}\nfeature \
             autoCollection {\n    multiplicity subsets zeroOrMore;\n}",
        )
        .syntax(),
    );
    assert!(has_node(&body, "MultiplicityRange"), "{body}");
    assert!(has_node(&body, "MultiplicitySubset"), "{body}");
    assert!(has_node(&body, "NonFeatureMember"), "{body}");
    // `:>` is SUBSETS's symbol too, and a body is a TypeBody.
    kerml_accepted("multiplicity <m> m :> zeroOrMore { doc /* d */ }");
    kerml_accepted("multiplicity one [1] { }");
}

#[test]
fn a_multiplicity_is_bounded_by_its_rules() {
    // A range or a subsetting, one of them (8.2.5.11). Held as a file by
    // tests/rejection/kerml-multiplicity-needs-a-range-or-a-subsetting.kerml.
    kerml_rejected("multiplicity m;");
    kerml_rejected("multiplicity m [1] subsets n;");
    // Subsets is one subsetting, not a list (SysML 8.2.2.6.5, shared).
    kerml_rejected("multiplicity m subsets a, b;");
    kerml_rejected("multiplicity m [1]");
    // SysML's multiplicity is the bracket alone (ADR-0014).
    let sysml = parse("multiplicity m [1];", Language::SysMl);
    assert!(!sysml.errors().is_empty(), "{:?}", sysml.errors());
}

// -- a feature's owned relationships, KerML 8.2.4.3.1-8.2.4.3.4 -----------------------
//
//   Subsets    = SUBSETS    OwnedSubsetting            OwnedSubsetting          = GeneralType
//   Redefines  = REDEFINES  OwnedRedefinition          OwnedRedefinition        = GeneralType
//   References = REFERENCES OwnedReferenceSubsetting   OwnedReferenceSubsetting = GeneralType
//   Crosses    = CROSSES    OwnedCrossSubsetting       OwnedCrossSubsetting     = GeneralType
//   GeneralType = [QualifiedName] | OwnedFeatureChain                          (8.2.4.1.2)

#[test]
fn a_kerml_feature_owns_its_specializations_over_a_general_type() {
    // Each owned relationship is a GeneralType contributed into it: a name, or a feature
    // chain, which is an ownedRelatedElement of the relationship.
    for (source, part, owned) in [
        ("feature f :> a.b;", "Subsets", "OwnedSubsetting"),
        ("feature f subsets a;", "Subsets", "OwnedSubsetting"),
        ("feature f :>> a.b;", "Redefines", "OwnedRedefinition"),
        ("feature f redefines a;", "Redefines", "OwnedRedefinition"),
        (
            "feature f ::> a.b;",
            "References",
            "OwnedReferenceSubsetting",
        ),
        (
            "feature f references a;",
            "References",
            "OwnedReferenceSubsetting",
        ),
        ("end feature f => a.b;", "Crosses", "OwnedCrossSubsetting"),
        (
            "end feature f crosses a;",
            "Crosses",
            "OwnedCrossSubsetting",
        ),
    ] {
        let tree = render(&kerml_accepted(&format!("assoc A {{ {source} }}")).syntax());
        assert!(has_node(&tree, part), "{source}\n{tree}");
        let target = child_kinds(&tree, owned);
        let expected = if source.contains('.') {
            "OwnedFeatureChain"
        } else {
            "QualifiedName"
        };
        assert_eq!(target, [expected], "{source}\n{tree}");
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

// -- TypeFeaturing, KerML 8.2.4.3.7 -------------------------------------------------
//
//   TypeFeaturing = 'featuring' ( Identification 'of' )?
//                   featureOfType = [QualifiedName]
//                   'by' featuringType = [QualifiedName]
//                   RelationshipBody
//
// A NonFeatureElement (8.2.3.4.3). NOT the TypeFeaturingPart a feature declaration ends
// in, `feature y1 featured by C`, whose keywords are `featured by` (8.2.4.3.1). The
// layer note's examples (7.3.4.8) write the standalone form with `featured by` too;
// deviations.json entry TypeFeaturing records that as an editing error in the examples
// (conflict, follow_spec), so they are read below REWRITTEN with the clause's bare `by`.

#[test]
fn a_type_featuring_reads_the_corpus_forms() {
    // Simple Tests/Features.kerml:16, with an Identification before `of`.
    let named = render(&kerml_accepted("featuring F of y by C;").syntax());
    assert_eq!(
        child_kinds(&named, "TypeFeaturing"),
        [
            "KwFeaturing",
            "Identification",
            "KwOf",
            "QualifiedName",
            "KwBy",
            "QualifiedName",
            "RelationshipBody"
        ],
        "{named}"
    );
    // Without `( Identification 'of' )?` the first name is the featured feature.
    let bare = render(&kerml_accepted("featuring y by C;").syntax());
    assert_eq!(
        child_kinds(&bare, "TypeFeaturing"),
        [
            "KwFeaturing",
            "QualifiedName",
            "KwBy",
            "QualifiedName",
            "RelationshipBody"
        ],
        "{bare}"
    );
    // A qualified featured feature is no Identification: `::` follows the name, not `of`.
    kerml_accepted("featuring P::y by Q::C;");
    // Every part of Identification is optional (8.2.3.1), so `of` may follow at once.
    kerml_accepted("featuring of y by C;");
    kerml_accepted("featuring <tf1> F of y by C;");
    // KerML 7.3.4.8's two examples, `featured by` rewritten to the clause's `by`.
    kerml_accepted("featuring engine_by_Vehicle of engine by Vehicle;");
    kerml_accepted(
        "featuring power by engine {\n    doc /* The engine of a Vehicle has power. */\n}",
    );
    // The corpus file's own neighbourhood: a feature, then the declaration, then a
    // feature ending in the owned form (Simple Tests/Features.kerml:15-18).
    let both = render(
        &kerml_accepted(
            "package Features {\n\tclassifier C;\n\tfeature y;\n\tfeaturing F of y by C;\n\t\
             feature y1 : A :> x featured by C;\n}",
        )
        .syntax(),
    );
    assert!(has_node(&both, "TypeFeaturing"), "{both}");
    assert!(has_node(&both, "TypeFeaturingPart"), "{both}");
}

#[test]
fn a_type_featuring_is_bounded_by_its_rules() {
    // The standalone form's keyword is `by` alone (8.2.4.3.7); `featured by` belongs to
    // the owned form. Held as a file by
    // tests/rejection/kerml-type-featuring-is-featuring-by.kerml.
    kerml_rejected("featuring engine_by_Vehicle of engine featured by Vehicle;");
    kerml_rejected("featuring power featured by engine;");
    kerml_rejected("featuring y;");
    kerml_rejected("featuring y by;");
    kerml_rejected("featuring by C;");
    kerml_rejected("featuring F of by C;");
    // One featuring type: a list is a feature's own TypeFeaturingPart (7.3.4.8).
    kerml_rejected("featuring y by C, D;");
    // Both targets are QualifiedNames; no feature chain is admitted on either side.
    kerml_rejected("featuring a.b by C;");
    kerml_rejected("featuring y by a.b;");
    // SysML states no TypeFeaturing declaration (ADR-0014).
    let sysml = parse("featuring F of y by C;", Language::SysMl);
    assert!(!sysml.errors().is_empty(), "{:?}", sysml.errors());
}

// -- NonFeatureElement, KerML 8.2.3.4.3 ---------------------------------------------
//
//   NonFeatureElement = Dependency | Namespace | Type | Classifier | DataType | Class
//                     | Structure | Metaclass | Association | AssociationStructure
//                     | Interaction | Behavior | Function | Predicate | Multiplicity
//                     | Package | LibraryPackage | Specialization | Conjugation
//                     | Subclassification | Disjoining | FeatureInverting
//                     | FeatureTyping | Subsetting | Redefinition | TypeFeaturing
//
// Owned two ways: as a NonFeatureMember's MemberElement (8.2.3.4.1), and with no
// membership as a relationship's OwnedRelatedElement (8.2.3.1). One minimal instance of
// each alternative, in the order the clause lists them, read at both.

// Each with the node its production builds; Multiplicity is an alternation with no node,
// and a bracket makes it a MultiplicityRange (8.2.5.11).
const NON_FEATURE_ELEMENTS: [(&str, &str); 26] = [
    ("dependency a to b;", "Dependency"),
    ("namespace N;", "Namespace"),
    // TypeDeclaration's `( SpecializationPart | ConjugationPart )+` (8.2.4.1.1).
    ("type T :> A;", "Type"),
    ("classifier K;", "Classifier"),
    ("datatype D;", "DataType"),
    ("class C;", "Class"),
    ("struct S;", "Structure"),
    ("metaclass M;", "Metaclass"),
    ("assoc A;", "Association"),
    ("assoc struct AS;", "AssociationStructure"),
    ("interaction I;", "Interaction"),
    ("behavior B;", "Behavior"),
    ("function F;", "Function"),
    ("predicate P;", "Predicate"),
    ("multiplicity zeroOrMore [0..*];", "MultiplicityRange"),
    ("package Q;", "Package"),
    // `standard` optional by deviation LibraryPackage (follow_xtext); the clause
    // writes it bare.
    ("library package L;", "LibraryPackage"),
    ("subtype A :> B;", "Specialization"),
    ("conjugate A ~ B;", "Conjugation"),
    ("subclassifier A :> B;", "Subclassification"),
    ("disjoint A from B;", "Disjoining"),
    ("inverse a of b;", "FeatureInverting"),
    ("typing f : T;", "FeatureTyping"),
    ("subset f :> g;", "Subsetting"),
    ("redefinition f :>> g;", "Redefinition"),
    ("featuring y by C;", "TypeFeaturing"),
];

#[test]
fn every_non_feature_element_is_a_member_and_an_owned_related_element() {
    for (element, kind) in NON_FEATURE_ELEMENTS {
        let member = render(&kerml_accepted(&format!("package P {{ {element} }}")).syntax());
        assert!(has_node(&member, kind), "{kind}: {member}");
        // The package is the root's member, and the element the package's.
        assert_eq!(member.matches("NonFeatureMember").count(), 2, "{member}");
        let owned = render(&kerml_accepted(&format!("featuring y by C {{ {element} }}")).syntax());
        assert!(has_node(&owned, kind), "{kind}: {owned}");
        // The outer declaration is the root's one member; its body adds none.
        assert_eq!(owned.matches("NonFeatureMember").count(), 1, "{owned}");
    }
    // A SysML definition is no NonFeatureElement (ADR-0014), at either site; and an
    // owned related element takes no MemberPrefix (8.2.3.1).
    kerml_rejected("package P { part def D; }");
    kerml_rejected("featuring y by C { part def D; }");
    kerml_rejected("featuring y by C { private class K; }");
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

// -- Type, KerML 8.2.4.1.1 ----------------------------------------------------------
//
//   Type               = TypePrefix 'type' TypeDeclaration TypeBody
//   TypeDeclaration    = 'all'? Identification OwnedMultiplicity?
//                        ( SpecializationPart | ConjugationPart )+ TypeRelationshipPart*
//   SpecializationPart = SPECIALIZES OwnedSpecialization ( ',' OwnedSpecialization )*
//   OwnedSpecialization = GeneralType                                  (8.2.4.1.2)

#[test]
fn a_type_reads_the_corpus_forms() {
    // Simple Tests/Types.kerml:1-15, 28-29, 31 and 33: every type form it writes, less the
    // repeats at 20-24 and 34-35 and the relationship declarations at 17-18 and 25-26,
    // which have their own tests.
    let tree = render(
        &kerml_accepted(
            "package Types {\n\tabstract type A specializes Base::Anything;\n\ttype all x \
             specializes A, Base::things;\n\t\n\t// This Type has exactly one instance.\n\t\
             type Singleton[1] specializes Base::Anything;\n\t\n\ttype Super specializes \
             Base::Anything {\n\t    private package P {\n\t        type Sub specializes \
             Super;\n\t    }\n\t    protected feature f : P::Sub;\n\t}\n\t\n\ttype B :> \
             Base::Anything;\n\ttype Conjugate3 conjugates Original;\n\ttype Conjugate4 ~ \
             Conjugate1;\n\ttype C :> B disjoint from A;\n\ttype D :> Base::Anything unions \
             A, B;\n}",
        )
        .syntax(),
    );
    assert_eq!(
        tree.lines().filter(|l| l.trim() == "Type").count(),
        10,
        "{tree}"
    );
    let abstract_type =
        render(&kerml_accepted("abstract type A specializes Base::Anything;").syntax());
    assert_eq!(
        child_kinds(&abstract_type, "Type"),
        ["TypePrefix", "KwType", "TypeDeclaration", "TypeBody"],
        "{abstract_type}"
    );
    let all = render(&kerml_accepted("type all x specializes A, Base::things;").syntax());
    assert_eq!(
        child_kinds(&all, "TypeDeclaration"),
        ["KwAll", "Identification", "SpecializationPart"],
        "{all}"
    );
    assert_eq!(
        child_kinds(&all, "SpecializationPart"),
        [
            "KwSpecializes",
            "OwnedSpecialization",
            "Comma",
            "OwnedSpecialization"
        ],
        "{all}"
    );
    let singleton =
        render(&kerml_accepted("type Singleton[1] specializes Base::Anything;").syntax());
    assert_eq!(
        child_kinds(&singleton, "TypeDeclaration"),
        ["Identification", "OwnedMultiplicity", "SpecializationPart"],
        "{singleton}"
    );
}

#[test]
fn a_type_declaration_owns_the_parts_it_writes() {
    // Simple Tests/Types.kerml:29, a conjugation in place of a specialization.
    let conjugate = render(&kerml_accepted("type Conjugate4 ~ Conjugate1;").syntax());
    assert_eq!(
        child_kinds(&conjugate, "TypeDeclaration"),
        ["Identification", "ConjugationPart"],
        "{conjugate}"
    );
    // KerML 7.3.2.2's first example: a part after the specialization.
    let disjoint =
        render(&kerml_accepted("type A specializes Base::Anything disjoint from B;").syntax());
    assert_eq!(
        child_kinds(&disjoint, "TypeDeclaration"),
        ["Identification", "SpecializationPart", "DisjoiningPart"],
        "{disjoint}"
    );
    // An OwnedSpecialization's general type may be a feature chain (8.2.4.1.2).
    let chained = render(&kerml_accepted("type T :> a.b;").syntax());
    assert_eq!(
        child_kinds(&chained, "OwnedSpecialization"),
        ["OwnedFeatureChain"],
        "{chained}"
    );
}

#[test]
fn a_type_declaration_takes_its_parts_as_its_production_writes_them() {
    // `( SpecializationPart | ConjugationPart )+`: the clause's `+`, kept where the
    // Pilot writes the group once (deviation TypeDeclaration, follow_spec), so two parts
    // and both kinds parse, and the rules against them are validation's:
    // validateTypeAtMostOneConjugator (8.3.3.1.10) and a conjugated type being no
    // Specialization's specific (8.3.3.1.2).
    let two = render(&kerml_accepted("type T :> A specializes B;").syntax());
    assert_eq!(
        child_kinds(&two, "TypeDeclaration"),
        ["Identification", "SpecializationPart", "SpecializationPart"],
        "{two}"
    );
    kerml_accepted("type T :> A conjugates B;");
    // Identification may be empty (8.2.3.1).
    kerml_accepted("type :> A;");
    kerml_accepted("#M type T :> A;");
    kerml_accepted("class C { type T :> A; }");
}

#[test]
fn a_type_is_bounded_by_its_rules() {
    // A part is required. Held as a file by
    // tests/rejection/kerml-type-declaration-requires-a-specialization-or-conjugation.kerml.
    kerml_rejected("type T;");
    kerml_rejected("type T { }");
    kerml_rejected("type T specializes;");
    // `:` is TYPED_BY, a feature's; a type specializes (8.2.2.7).
    kerml_rejected("type T : A;");
    // TypeRelationshipPart* follows the required part and is not one (8.2.4.1.1).
    kerml_rejected("type T disjoint from A;");
    // SysML states no Type (ADR-0014).
    let sysml = parse("type T :> A;", Language::SysMl);
    assert!(!sysml.errors().is_empty(), "{:?}", sysml.errors());
}

// -- AssociationStructure, KerML 8.2.5.4 --------------------------------------------
//
//   AssociationStructure = TypePrefix 'assoc' 'struct' ClassifierDeclaration TypeBody
//
// One of the classifiers, and the one of two keywords: `assoc` alone is Association.

#[test]
fn an_association_structure_reads_the_corpus_forms() {
    // Simple Tests/Associations.kerml:15-18.
    let tree = render(
        &kerml_accepted(
            "assoc struct C {\n\t\tconst end [1] feature a;\n\t\tconst end feature b;\n\t}",
        )
        .syntax(),
    );
    assert_eq!(
        child_kinds(&tree, "AssociationStructure"),
        [
            "TypePrefix",
            "KwAssoc",
            "KwStruct",
            "ClassifierDeclaration",
            "TypeBody"
        ],
        "{tree}"
    );
    assert!(!has_node(&tree, "Association"), "{tree}");
    // KerML 7.4.5.3's example, after the two structs it names.
    kerml_accepted(
        "struct LegalEntity {\n    var feature assetsOwned [*] ordered : Asset;\n}\nstruct \
         Asset {\n    var feature owningEntities [1..*] : LegalEntity;\n}\nassoc struct \
         ExtendedAssetOwnership { // Specializes Objects::BinaryLinkObject by default.\n    end \
         feature owner : LegalEntity crosses ownedAsset.owningEntities;\n    end feature \
         ownedAsset : Asset crosses owner.assetsOwned;\n    feature valuationOnPurchase : \
         MonetaryValue;\n    // The values of the feature \"revaluations\" may change over \
         time.\n    var feature revaluations[*] ordered : MonetaryValue;\n}",
    );
    // `assoc` alone is still an Association, and `struct` alone a Structure.
    let assoc = render(&kerml_accepted("assoc A;").syntax());
    assert!(has_node(&assoc, "Association"), "{assoc}");
    assert!(!has_node(&assoc, "AssociationStructure"), "{assoc}");
}

#[test]
fn an_association_structure_is_bounded_by_its_rules() {
    // The two words in that order (8.2.5.4). Held as a file by
    // tests/rejection/kerml-association-structure-is-assoc-then-struct.kerml.
    kerml_rejected("struct assoc C;");
    // TypePrefix before both words, never between.
    kerml_rejected("assoc abstract struct C;");
    // SysML's definitions are not KerML classifiers (ADR-0014).
    let sysml = parse("assoc struct C;", Language::SysMl);
    assert!(!sysml.errors().is_empty(), "{:?}", sysml.errors());
}

// -- MetadataFeature, KerML 8.2.5.12 ------------------------------------------------
//
//   MetadataFeature = PrefixMetadataMember* ( '@' | 'metadata' ) MetadataFeatureDeclaration
//                     ( 'about' Annotation ( ',' Annotation )* )? MetadataBody
//   MetadataFeatureDeclaration = ( Identification ( ':' | 'typed' 'by' ) )?
//                                OwnedFeatureTyping
//   MetadataBody        = ';' | '{' MetadataBodyElement* '}'
//   MetadataBodyElement = NonFeatureMember | MetadataBodyFeatureMember
//                       | AliasMember | Import
//   MetadataBodyFeature = 'feature'? ( ':>>' | 'redefines' )? OwnedRedefinition
//                         FeatureSpecializationPart? ValuePart? MetadataBody
//
// AnnotatingElement's fourth alternative in KerML (8.2.3.3.1). SysML's is MetadataUsage.

#[test]
fn a_metadata_feature_reads_the_corpus_forms() {
    // Simple Tests/MetadataTest.kerml:19-31, both spellings, bodies and none.
    let tree = render(
        &kerml_accepted(
            "feature x {\n\tmetadata Classified {\n\t\tclassificationLevel = conf;\n\t}\n\t\
             metadata : Security;\n}\nfeature y {\n\t@Classified {\n\t\tclassificationLevel = \
             conf;\n\t}\n\t@ : Security;\n}",
        )
        .syntax(),
    );
    assert_eq!(
        tree.lines()
            .filter(|l| l.trim() == "MetadataFeature")
            .count(),
        4,
        "{tree}"
    );
    assert!(!has_node(&tree, "MetadataUsage"), "{tree}");
    let colon = render(&kerml_accepted("@ : Security;").syntax());
    assert_eq!(
        child_kinds(&colon, "MetadataFeature"),
        ["At", "MetadataFeatureDeclaration", "MetadataBody"],
        "{colon}"
    );
    assert_eq!(
        child_kinds(&colon, "MetadataFeatureDeclaration"),
        ["Identification", "Colon", "OwnedFeatureTyping"],
        "{colon}"
    );
    // Its body's feature redefines a feature of the metaclass, with no keyword.
    let body = render(&kerml_accepted("@Classified { classificationLevel = conf; }").syntax());
    assert_eq!(
        child_kinds(&body, "MetadataFeatureDeclaration"),
        ["OwnedFeatureTyping"],
        "{body}"
    );
    assert_eq!(
        child_kinds(&body, "MetadataBody"),
        ["LBrace", "MetadataBodyFeatureMember", "RBrace"],
        "{body}"
    );
    assert_eq!(
        child_kinds(&body, "MetadataBodyFeatureMember"),
        ["MetadataBodyFeature"],
        "{body}"
    );
}

#[test]
fn a_metadata_feature_is_written_where_an_annotating_element_is() {
    // MetadataTest.kerml:37-39: prefix metadata before the keyword.
    let prefixed = render(
        &kerml_accepted(
            "feature z {\n    #Security #Classified metadata Classified {\n        \
             classificationLevel = secret;\n    }\n}",
        )
        .syntax(),
    );
    assert_eq!(
        child_kinds(&prefixed, "MetadataFeature"),
        [
            "PrefixMetadataMember",
            "PrefixMetadataMember",
            "KwMetadata",
            "MetadataFeatureDeclaration",
            "MetadataBody"
        ],
        "{prefixed}"
    );
    // Simple Tests/Associations.kerml:22-24 and Filtering.kerml:15-19.
    kerml_accepted("assoc XY {\n\tend [0..1] feature x : X {\n\t\t@M;\n\t}\n}");
    kerml_accepted(
        "struct System {\n     @ApprovalAnnotation {\n        approved = true;\n        \
         approver = \"John Smith\";\n        level = 2;\n    }\n}",
    );
}

#[test]
fn a_metadata_feature_reads_the_examples_of_7_4_13() {
    let about = render(
        &kerml_accepted(
            "metadata securityDesignAnnotation : SecurityRelated about SecurityDesign;",
        )
        .syntax(),
    );
    assert_eq!(
        child_kinds(&about, "MetadataFeature"),
        [
            "KwMetadata",
            "MetadataFeatureDeclaration",
            "KwAbout",
            "Annotation",
            "MetadataBody"
        ],
        "{about}"
    );
    let keywords = render(
        &kerml_accepted(
            "metadata ApprovalAnnotation about Design {\n    feature redefines approved = true;\n    \
             feature redefines approver = \"John Smith\";\n}",
        )
        .syntax(),
    );
    assert_eq!(
        child_kinds(&keywords, "MetadataBodyFeature")[..3],
        ["KwFeature", "KwRedefines", "OwnedRedefinition"],
        "{keywords}"
    );
    kerml_accepted(
        "metadata ApprovalAnnotation about Design {\n    approved = true;\n    approver = \"John \
         Smith\";\n}",
    );
    kerml_accepted(
        "class Design {\n    // This metadata feature is implicitly about the class Design.\n    \
         @ApprovalAnnotation {\n        approved = true;\n        approver = \"John Smith\";\n    \
         }\n}",
    );
    // KerML spells TYPED_BY `typed by` (8.2.2.7), and `:>>` is REDEFINES's symbol.
    kerml_accepted("metadata m typed by T;");
    kerml_accepted("metadata m : T about a, b { :>> x = 1; }");
    // A NonFeatureMember is a MetadataBodyElement: a nested annotation, a class.
    kerml_accepted("metadata M { doc /* d */ @N; private class C; }");
    // A nested body on a body feature.
    kerml_accepted("metadata M { x { y = 1; } }");
}

#[test]
fn a_metadata_feature_is_bounded_by_its_rules() {
    // A typing is required (8.2.5.12).
    kerml_rejected("metadata;");
    kerml_rejected("metadata M about;");
    // `defined by` is SysML's spelling, deviation MetadataUsageDeclaration's; KerML's
    // is `typed by`.
    kerml_rejected("metadata m defined by T;");
    // A FeatureElement is no MetadataBodyElement: a body's features are redefinitions
    // (8.2.5.12). Held as a file by
    // tests/rejection/kerml-metadata-body-owns-no-feature-element.kerml.
    kerml_rejected("metadata M { step s; }");
    kerml_rejected("metadata M { connector a to b; }");
    // A MetadataBodyFeatureMember has no MemberPrefix.
    kerml_rejected("metadata M { private x = 1; }");
    // SysML's usages are not KerML's.
    kerml_rejected("metadata M { ref x = 1; }");
    kerml_rejected("metadata M { part p; }");
}

// -- Invariant and FunctionBody, KerML 8.2.5.7 --------------------------------------
//
//   Invariant        = FeaturePrefix 'inv' ( 'true' | 'false' )?
//                      FeatureDeclaration? ValuePart? FunctionBody        (8.2.5.7.4)
//   FunctionBody     = ';' | '{' FunctionBodyPart '}'                     (8.2.5.7.1)
//   FunctionBodyPart = ( TypeBodyElement | ReturnFeatureMember )*
//                      ResultExpressionMember?
//   ReturnFeatureMember    = MemberPrefix 'return' FeatureElement
//   ResultExpressionMember = MemberPrefix OwnedExpression
//
// The declaration's `?` is deviation Invariant's (follow_xtext, KERML11-181).

#[test]
fn an_invariant_reads_the_corpus_forms() {
    // Individuals Examples/JohnIndividualExample.kerml:88-90: no declaration, a result
    // expression alone.
    let tree = render(
        &kerml_accepted(
            "class C {\n  \tfeature presidentOfUS[1] redefines presidentOfCountry {\n   \t\tinv \
             { age >= 35 } \n  \t}\n}",
        )
        .syntax(),
    );
    assert_eq!(
        child_kinds(&tree, "Invariant"),
        ["FeaturePrefix", "KwInv", "FunctionBody"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "FunctionBody"),
        ["LBrace", "FunctionBodyPart", "RBrace"],
        "{tree}"
    );
    assert_eq!(
        child_kinds(&tree, "FunctionBodyPart"),
        ["ResultExpressionMember"],
        "{tree}"
    );
    // Simple Tests/TextualRepresentation.kerml:4-10: a declaration, and a body holding
    // an annotation and no result expression.
    let rep = render(
        &kerml_accepted(
            "class C {\n    feature x: Real;\n    inv x_constraint {\n\t    rep inOCL language \
             \"ocl\" \n\t        /* self.x > 0.0 */\n    }\n}",
        )
        .syntax(),
    );
    assert_eq!(
        child_kinds(&rep, "Invariant"),
        [
            "FeaturePrefix",
            "KwInv",
            "FeatureDeclaration",
            "FunctionBody"
        ],
        "{rep}"
    );
    assert!(!has_node(&rep, "ResultExpressionMember"), "{rep}");
}

#[test]
fn an_invariant_reads_the_examples_of_7_4_8_5() {
    // KerML 7.4.8.5's FuelTank, less its `feature readonly maxFuelLevel`: `readonly` is no
    // KerML reserved word (8.2.2.6), so that line declares a feature named `readonly`
    // followed by a stray name.
    let tree = render(
        &kerml_accepted(
            "class FuelTank {\n    feature fuelLevel : Real;\n    // The invariant is asserted \
             true by default.\n    inv { fuelLevel >= 0 & fuelLevel <= maxFuelLevel }\n    // \
             The invariant is explicitly asserted false, that is, it is negated.\n    inv false \
             { fuelLevel > maxFuelLevel }\n}",
        )
        .syntax(),
    );
    assert_eq!(
        tree.lines().filter(|l| l.trim() == "Invariant").count(),
        2,
        "{tree}"
    );
    let negated = render(&kerml_accepted("inv false { a > b }").syntax());
    assert_eq!(
        child_kinds(&negated, "Invariant"),
        ["FeaturePrefix", "KwInv", "KwFalse", "FunctionBody"],
        "{negated}"
    );
    kerml_accepted("inv true i : B;");
}

#[test]
fn a_function_body_reads_items_then_its_result_expression() {
    // TypeBodyElements, a ReturnFeatureMember, then the expression, which has no `;`
    // (7.4.8.2). A TypeBodyElement's feature is a FeatureMember, an alternation with no
    // node, and a feature with no `member` is its OwnedFeatureMember (8.2.4.1.6). A keywordless feature and an expression both open on a name; the
    // feature reaches a `;` or `{` before the body's `}`, and the expression does not.
    let tree = render(
        &kerml_accepted("inv i { in x : Real; y : Real; return r : Boolean; x > y }").syntax(),
    );
    assert_eq!(
        child_kinds(&tree, "FunctionBodyPart"),
        [
            "OwnedFeatureMember",
            "OwnedFeatureMember",
            "ReturnFeatureMember",
            "ResultExpressionMember"
        ],
        "{tree}"
    );
    // `~` opens a keywordless feature's ConjugationPart and a unary expression alike.
    let tilde = render(&kerml_accepted("inv { ~ T; }").syntax());
    assert!(!has_node(&tilde, "ResultExpressionMember"), "{tilde}");
    let not = render(&kerml_accepted("inv { ~x }").syntax());
    assert!(has_node(&not, "ResultExpressionMember"), "{not}");
    // An invocation is an expression, not a feature named `sum`.
    let call = render(&kerml_accepted("inv { sum(scores) / size(scores) > 0 }").syntax());
    assert!(has_node(&call, "ResultExpressionMember"), "{call}");
    // Nested bodies and a MemberPrefix on the result.
    kerml_accepted("inv { class K { feature f; } private true }");
}

#[test]
fn an_invariant_without_a_declaration_is_admitted_by_deviation() {
    // Deviation Invariant (follow_xtext): the clause writes FeatureDeclaration bare, and
    // KERML11-181 says it should be optional. The text parses and carries a
    // PARSE-DEVIATION note naming the entry (ADR-0022).
    let parsed = kerml_accepted("inv { true }");
    let notes: Vec<String> = parsed
        .deviations()
        .iter()
        .map(|d| d.message().to_owned())
        .collect();
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(notes[0].contains("deviation Invariant"), "{notes:?}");
    assert!(kerml_accepted("inv i { true }").deviations().is_empty());
}

#[test]
fn an_invariant_is_bounded_by_its_rules() {
    // The result expression takes no `;` (7.4.8.2). Held as a file by
    // tests/rejection/kerml-result-expression-takes-no-semicolon.kerml.
    kerml_rejected("inv { true; }");
    kerml_rejected("inv { a b }");
    kerml_rejected("inv true false { x }");
    kerml_rejected("inv { true }  }");
    // FunctionBody is `;` or braced (8.2.5.7.1).
    kerml_rejected("inv i");
    // SysML has no `inv`; it asserts with AssertConstraintUsage (ADR-0014).
    let sysml = parse("inv { true }", Language::SysMl);
    assert!(!sysml.errors().is_empty(), "{:?}", sysml.errors());
}

// -- Expression and BooleanExpression, KerML 8.2.5.7.2, 8.2.5.7.4 -------------------
//
//   Expression        = FeaturePrefix 'expr' FeatureDeclaration? ValuePart? FunctionBody
//   BooleanExpression = FeaturePrefix 'bool' FeatureDeclaration? ValuePart? FunctionBody
//
// The declarations' `?` are deviations Expression and BooleanExpression's (follow_xtext,
// KERML11-181), as Invariant's is.

#[test]
fn an_expression_reads_the_corpus_forms() {
    // Simple Tests/Expressions.kerml:50 and :53, and :23 with a direction.
    let tree = render(&kerml_accepted("expr totalMass: TotalMass { in mass; in sub; }").syntax());
    assert_eq!(
        child_kinds(&tree, "Expression"),
        [
            "FeaturePrefix",
            "KwExpr",
            "FeatureDeclaration",
            "FunctionBody"
        ],
        "{tree}"
    );
    // :53 is in a feature's body; here in a class's, the same TypeBody (8.2.4.1.1).
    kerml_accepted("class C { expr s { in x; return : Boolean; } }");
    let directed = render(&kerml_accepted("behavior B { in expr whileTest {v > 3} }").syntax());
    assert!(has_node(&directed, "Expression"), "{directed}");
    assert!(has_node(&directed, "ResultExpressionMember"), "{directed}");
    // Variable Feature Examples/Enhancements/ExtendedOccurrences.kerml:16-23: a
    // redefinition, parameters, and bindings in the body, no result expression. Its name
    // `at` is written `'at'`, and :25's `while` `'while'`: neither is a KerML reserved
    // word (8.2.2.6), but SysML's are reserved in a .kerml file too until pending decision
    // keyword-table-per-language splits the table, as `'state'` is below.
    kerml_accepted(
        "class C {\n        expr 'at' {\n        \t:>> that : Timeslice;\n            in interval : \
         Interval;\n            return result : Timeslice;\n\n            binding \
         result.portionOf = that;\n            binding result.interval = interval;\n        }\n}",
    );
    // :25-26, whose parameter `timeslice` is SysML's portion keyword and no KerML one:
    // quoted the same way.
    kerml_accepted("class C { expr 'while' { in 'timeslice' : Timeslice; } }");
}

#[test]
fn an_expression_reads_the_examples_of_7_4_8_3() {
    // `state` is written `'state'`: it is no KerML reserved word (8.2.2.6), but SysML's
    // are reserved in a .kerml file too until pending decision keyword-table-per-language
    // splits the table, as the_while_until_example_of_7_17_12_parses writes `'step'`.
    kerml_accepted(
        "expr computation : ComputeDynamics {\n    // Parameters redefined parameters of \
         ComputeDynamics.\n    in 'state';\n    in dt;\n    return result;\n}\nexpr \
         vehicleComputation subsets computation {\n    // Input parameters are inherited, \
         result is redefined.\n    return : VehicleState;\n}",
    );
    let result = render(
        &kerml_accepted(
            "expr : VehicleDynamics {\n    in initialState;\n    in time;\n    return \
             result;\n\n    vehicleComputation(initialState, time)\n}",
        )
        .syntax(),
    );
    assert_eq!(
        child_kinds(&result, "FunctionBodyPart"),
        [
            "OwnedFeatureMember",
            "OwnedFeatureMember",
            "ReturnFeatureMember",
            "ResultExpressionMember"
        ],
        "{result}"
    );
    kerml_accepted(
        "expr : Dynamics {\n    in initialState;\n    in time;\n    return result : \
         VehicleState =\n        vehicleComputation(initialState, time);\n}",
    );
}

#[test]
fn a_boolean_expression_reads_the_examples_of_7_4_8_5() {
    // `bool assemblyChecks[*] : isAssembled;`, and FuelTank's `bool isFull`, less the
    // `readonly` feature (see `an_invariant_reads_the_examples_of_7_4_8_5`).
    let checks = render(&kerml_accepted("bool assemblyChecks[*] : isAssembled;").syntax());
    assert_eq!(
        child_kinds(&checks, "BooleanExpression"),
        [
            "FeaturePrefix",
            "KwBool",
            "FeatureDeclaration",
            "FunctionBody"
        ],
        "{checks}"
    );
    kerml_accepted(
        "class FuelTank {\n    feature fuelLevel : Real;\n    bool isFull { fuelLevel == \
         maxFuelLevel }\n}",
    );
}

#[test]
fn an_expression_without_a_declaration_is_admitted_by_deviation() {
    // KERML11-181 names both clauses. Each form carries its own entry's note.
    for (source, entry) in [
        ("expr { 1 }", "deviation Expression"),
        ("bool { true }", "deviation BooleanExpression"),
    ] {
        let notes: Vec<String> = kerml_accepted(source)
            .deviations()
            .iter()
            .map(|d| d.message().to_owned())
            .collect();
        assert_eq!(notes.len(), 1, "{source}: {notes:?}");
        assert!(notes[0].contains(entry), "{source}: {notes:?}");
    }
    assert!(kerml_accepted("expr e { 1 }").deviations().is_empty());
    // `expr : T { }` declares by its typing, a FeatureSpecializationPart: no deviation.
    assert!(kerml_accepted("expr : T { 1 }").deviations().is_empty());
}

#[test]
fn an_expression_is_bounded_by_its_rules() {
    // `true`/`false` are Invariant's alone (8.2.5.7.4).
    kerml_rejected("bool true { x }");
    kerml_rejected("expr false { x }");
    // FunctionBody is `;` or braced (8.2.5.7.1). Held as a file by
    // tests/rejection/kerml-expression-ends-in-a-function-body.kerml.
    kerml_rejected("expr e");
    kerml_rejected("expr e { 1; }");
    // SysML's expressions are usages: `calc`, not `expr` (ADR-0014).
    let sysml = parse("expr e { 1 }", Language::SysMl);
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
