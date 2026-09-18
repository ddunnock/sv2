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
//! Of `NonFeatureElement`'s alternatives, `Package` and the eight classifiers of
//! `KerML` 8.2.4.2 are implemented. `Package` is a shared unit — the same production in
//! both grammars — and the classifiers are `KerML`'s alone. `FeatureElement`'s ten
//! alternatives are unimplemented, as are `Type`, `Function` and `Predicate`, and the
//! cases below say so rather than pretending they parse.

use sv2_syntax::{Language, Parse, parse};

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
fn a_feature_element_is_unimplemented_rather_than_accepted() {
    // NamespaceFeatureMember reaches FeatureElement's ten alternatives, none of which
    // is implemented. Reporting them is the honest state; accepting them would be a
    // parser that says it understands KerML when it does not.
    for source in [
        "feature f : A;",
        "connector c from a to b;",
        "succession s;",
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
    // differently. KerML says MetadataFeature; neither spelling is read.
    kerml_rejected("metadata M about X;");
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
