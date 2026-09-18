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
//! `Package` is the one `NonFeatureElement` implemented, and it is a shared unit — the
//! same production in both grammars. `FeatureElement`'s ten alternatives and the other
//! `NonFeatureElement`s are unimplemented, and the cases below say so rather than
//! pretending they parse.

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
