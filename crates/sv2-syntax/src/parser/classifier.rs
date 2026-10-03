// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Types, classifiers and their relationships, `KerML` 8.2.4: specialization,
//! subclassification, conjugation, disjoining, feature inverting, type featuring, and the
//! `KerML` namespace.

use crate::generated::kinds::SyntaxKind;
use crate::parser::{Body, Parser};

/// Which standalone `KerML` relationship declaration is written, of the
/// `NonFeatureElement` alternatives that declare a relationship on its own
/// (`KerML` 8.2.3.4.3) and that this parser reads.
///
/// Each opens on its own reserved word, looked past an optional `( 'specialization'
/// Identification )?` for the ones that write it, so the word decides.
#[derive(Clone, Copy)]
pub(super) enum RelationshipDeclaration {
    /// `FeatureInverting`, `inverting` or `inverse` (8.2.4.3.6).
    FeatureInverting,
    /// `Specialization`, `subtype` (8.2.4.1.2).
    Specialization,
    /// `Subclassification`, `subclassifier` (8.2.4.2.2).
    Subclassification,
    /// `FeatureTyping`, `typing` (8.2.4.3.2).
    FeatureTyping,
    /// `Subsetting`, `subset` (8.2.4.3.3).
    Subsetting,
    /// `Redefinition`, `redefinition` (8.2.4.3.4).
    Redefinition,
    /// `Disjoining`, `disjoining` or `disjoint` (8.2.4.1.4).
    Disjoining,
    /// `Conjugation`, `conjugation` or `conjugate` (8.2.4.1.3).
    Conjugation,
    /// `TypeFeaturing`, `featuring` (8.2.4.3.7).
    TypeFeaturing,
}

/// A `KerML` classifier production: its keywords over a shared spine.
///
/// Nine productions of `KerML` 8.2.4.2 and 8.2.5.4 are stated as `TypePrefix KEYWORDS
/// ClassifierDeclaration TypeBody`, differing in the keywords and nothing else. The
/// derived units say so mechanically, so this table is a transcription rather than a
/// judgment, and writing nine near-identical methods would hide that they agree.
///
/// `Function` and `Predicate` share the shape but take a `FunctionBody`, and are not
/// implemented; `Type` takes a `TypeDeclaration` rather than a `ClassifierDeclaration`,
/// and is read by `kerml_type`. None is in this table, because the table is exactly the
/// set whose spine is shared.
#[derive(Clone, Copy)]
pub(super) struct Classifier {
    /// The keywords that say which production this is, in order: one, or `assoc
    /// struct`'s two.
    keywords: &'static [&'static str],
    /// The node the production builds.
    node: SyntaxKind,
}

/// Every classifier production sharing the `ClassifierDeclaration TypeBody` spine.
///
/// ORDER MATTERS once: `assoc struct` is before `assoc`, whose one keyword is its first,
/// so the first match is the longest. Every other first keyword is its own.
pub(super) const CLASSIFIERS: [Classifier; 9] = [
    Classifier {
        keywords: &["classifier"],
        node: SyntaxKind::Classifier,
    },
    Classifier {
        keywords: &["class"],
        node: SyntaxKind::Class,
    },
    Classifier {
        keywords: &["struct"],
        node: SyntaxKind::Structure,
    },
    Classifier {
        keywords: &["datatype"],
        node: SyntaxKind::DataType,
    },
    Classifier {
        keywords: &["metaclass"],
        node: SyntaxKind::Metaclass,
    },
    Classifier {
        keywords: &["assoc", "struct"],
        node: SyntaxKind::AssociationStructure,
    },
    Classifier {
        keywords: &["assoc"],
        node: SyntaxKind::Association,
    },
    Classifier {
        keywords: &["behavior"],
        node: SyntaxKind::Behavior,
    },
    Classifier {
        keywords: &["interaction"],
        node: SyntaxKind::Interaction,
    },
];

impl Parser<'_> {
    // production: Classifier
    // production: Class
    // production: Structure
    // production: DataType
    // production: Metaclass
    // production: Association
    // production: Behavior
    // production: Interaction
    // production: AssociationStructure
    //
    // Classifier  = TypePrefix 'classifier'  ClassifierDeclaration TypeBody
    // Class       = TypePrefix 'class'       ClassifierDeclaration TypeBody
    // Structure   = TypePrefix 'struct'      ClassifierDeclaration TypeBody
    // DataType    = TypePrefix 'datatype'    ClassifierDeclaration TypeBody
    // Metaclass   = TypePrefix 'metaclass'   ClassifierDeclaration TypeBody
    // Association = TypePrefix 'assoc'       ClassifierDeclaration TypeBody
    // Behavior    = TypePrefix 'behavior'    ClassifierDeclaration TypeBody
    // Interaction = TypePrefix 'interaction' ClassifierDeclaration TypeBody
    //                                                            (KerML 8.2.4.2)
    // AssociationStructure =
    //     TypePrefix 'assoc' 'struct' ClassifierDeclaration TypeBody (KerML 8.2.5.4)
    //
    // AssociationStructure is both an Association and a Structure (8.3.4.4.3, receipt
    // 544b43ab), declared "like a regular association ..., but using the keyword assoc
    // struct" (7.4.5.3, receipt ae11cb00).
    //
    // implied specialization: Objects::BinaryLinkObject for a binary AssociationStructure,
    //     else Objects::LinkObject, where its own superclassifications do not reach one
    //     (checkAssociationStructureBinarySpecialization,
    //     checkAssociationStructureSpecialization, KerML 8.3.4.4.3), "implicitly given a
    //     default superclassification" (7.4.5.3). sv2-hir's to inject; nothing is written
    //     into the tree.
    //
    // Nine productions, one method, as the seven usages share `simple_usage`. Each is
    // marked separately because each IS fully implemented: what none of them implements
    // lives below, in ClassifierDeclaration's optional parts and in TypePrefix, and is
    // recorded there.
    pub(super) fn classifier(&mut self, classifier: Classifier) {
        self.eat_trivia();
        self.start_node(classifier.node);
        self.type_prefix();
        for word in classifier.keywords {
            self.expect_keyword(word);
        }
        self.classifier_declaration();
        self.type_body();
        self.finish_node();
    }

    /// Which of `CLASSIFIERS` starts at the `n`th meaningful token, if any.
    pub(super) fn at_classifier(&self, n: usize) -> Option<Classifier> {
        let after = self.skip_type_prefix(n);
        CLASSIFIERS.iter().copied().find(|c| {
            c.keywords
                .iter()
                .enumerate()
                .all(|(i, word)| self.nth_is_keyword(after + i, word))
        })
    }

    /// Whether a `KerML` `Namespace` starts at the `n`th meaningful token: `namespace`,
    /// reserved (`KerML` 8.2.2.6), after any `PrefixMetadataMember`s.
    pub(super) fn at_kerml_namespace(&self, n: usize) -> bool {
        self.nth_is_keyword(self.skip_prefix_metadata(n), "namespace")
    }

    // production: Namespace@kerml
    //
    // Namespace = ( ownedRelationship += PrefixMetadataMember )*
    //             NamespaceDeclaration NamespaceBody             (KerML 8.2.3.4.1)
    //
    // production: NamespaceDeclaration@kerml
    //
    // NamespaceDeclaration : Namespace = 'namespace' Identification
    //
    // production: NamespaceBody@kerml
    //
    // NamespaceBody : Namespace = ';' | '{' NamespaceBodyElement* '}'
    //
    // "A namespace that is not a root namespace ..., and does not represent any more
    // specialized modeling construct ... is declared using the keyword namespace,
    // optionally followed by a short name and/or name" (7.2.5.2, receipt 5a18c1e2); the
    // metaclass is Namespace (8.3.2.4.5, receipt 8d19e03e). Its body's elements are the
    // root's, NamespaceBodyElement (8.2.3.4.1), so Body::Root reads them: no
    // ElementFilterMember, which is a package body's alone (8.2.5.13).
    // NamespaceBodyElement@kerml is marked at `body_element`.
    //
    // constraint: Namespace::validateNamespaceDistinguishibility, and the derivations of
    //     8.3.2.4.5. Validity and derivation, sv2-resolve's.
    pub(super) fn kerml_namespace(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Namespace);
        while self.at(SyntaxKind::Hash) {
            self.prefix_metadata_member();
        }
        self.eat_trivia();
        self.start_node(SyntaxKind::NamespaceDeclaration);
        self.expect_keyword("namespace");
        self.identification();
        self.finish_node();
        self.eat_trivia();
        self.start_node(SyntaxKind::NamespaceBody);
        if self.at(SyntaxKind::Semicolon) {
            self.bump();
        } else if self.at(SyntaxKind::LBrace) {
            self.bump();
            self.depth += 1;
            self.body_elements(Some(SyntaxKind::RBrace), Body::Root);
            self.depth -= 1;
            self.expect(SyntaxKind::RBrace, "`}`");
        } else {
            self.error_expected("`;` or `{` after a namespace declaration");
        }
        self.finish_node();
        self.finish_node();
    }

    /// Whether a `KerML` `Type` starts at the `n`th meaningful token: `type`, reserved
    /// (`KerML` 8.2.2.6), after a `TypePrefix`.
    pub(super) fn at_kerml_type(&self, n: usize) -> bool {
        self.nth_is_keyword(self.skip_type_prefix(n), "type")
    }

    // production: Type@kerml
    //
    // Type = TypePrefix 'type' TypeDeclaration TypeBody          (KerML 8.2.4.1.1)
    //
    // A NonFeatureElement (8.2.3.4.3), KerML's alone, and NOT a Classifier: it declares
    // through TypeDeclaration, whose part is required, where the eight classifiers
    // declare through ClassifierDeclaration, whose part is optional and is a
    // SuperclassingPart. So it has its own recogniser beside `at_classifier` rather than a
    // row in CLASSIFIERS. The metaclass is Type (8.3.3.1.10, receipt 5200b0a5):
    // `abstract type A specializes Base::Anything;` (KerML 7.3.2.2, receipt 632d1e41;
    // Simple Tests/Types.kerml:2).
    //
    // implied specialization: none. checkTypeSpecialization (KerML 8.3.3.1.10) requires
    //     every Type to specialize Base::Anything, but "no implied relationship shall be
    //     inserted to satisfy this constraint for a Type that is not a Classifier or a
    //     Feature" (KerML 8.4.3.2, receipt fd895569): a bare Type satisfies it by what it
    //     writes or not at all. Checking it is sv2-resolve's, which does not do so yet.
    pub(super) fn kerml_type(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Type);
        self.type_prefix();
        self.expect_keyword("type");
        self.type_declaration();
        self.type_body();
        self.finish_node();
    }

    // production: TypeDeclaration@kerml
    //
    // TypeDeclaration : Type =
    //     ( isSufficient ?= 'all' )? Identification
    //     ( ownedRelationship += OwnedMultiplicity )?
    //     ( SpecializationPart | ConjugationPart )+
    //     TypeRelationshipPart*                                   (KerML 8.2.4.1.1)
    //
    // The clause's `+`, where the Pilot writes the group once (KerML.xtext:323-328):
    // deviation TypeDeclaration, follow_spec, keeps it and leaves the rules against two
    // conjugations, or a conjugation beside a specialization, to validation. So `type T
    // :> A conjugates B;` parses and `type T;` does not: "a type declaration defines
    // either one or more owned specializations ... or a conjugator" (7.3.2.2).
    //
    // constraint: Type::validateTypeAtMostOneConjugator (KerML 8.3.3.1.10), and
    //     Conjugation's rule that a conjugated type is no Specialization's specific
    //     (8.3.3.1.2, receipt eabb0d9b). Validity, not syntax (ADR-0002).
    fn type_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::TypeDeclaration);
        self.eat_optional_keyword("all");
        self.identification();
        if self.at(SyntaxKind::LBracket) {
            self.owned_multiplicity();
        }
        if !self.at_superclassing() && !self.at_conjugation_part() {
            self.error_expected("`specializes`, `:>`, `conjugates` or `~`");
        }
        loop {
            if self.at_superclassing() {
                self.specialization_part();
            } else if self.at_conjugation_part() {
                self.conjugation_part();
            } else {
                break;
            }
        }
        self.type_relationship_parts();
        self.finish_node();
    }

    // production: SpecializationPart@kerml
    //
    // SpecializationPart : Type =
    //     SPECIALIZES ownedRelationship += OwnedSpecialization
    //     ( ',' ownedRelationship += OwnedSpecialization )*      (KerML 8.2.4.1.1)
    //
    // A type's, where a classifier writes a SuperclassingPart of the same text over
    // OwnedSubclassifications, names only. `at_superclassing` asks for SPECIALIZES, which
    // both open on.
    fn specialization_part(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::SpecializationPart);
        self.terminal(SyntaxKind::ColonGt, "specializes", "`:>` or `specializes`");
        self.owned_specialization();
        while self.at(SyntaxKind::Comma) {
            self.bump();
            self.owned_specialization();
        }
        self.finish_node();
    }

    // production: OwnedSpecialization@kerml
    //
    // OwnedSpecialization : Specialization = GeneralType         (KerML 8.2.4.1.2)
    //
    // A Specialization the type owns, its general a name or a feature chain: `type T :>
    // a.b;`. GeneralType contributes to it, so the node wraps what `general_type` reads,
    // as `chainable_target` wraps the other owned relationships' targets.
    fn owned_specialization(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::OwnedSpecialization);
        self.general_type();
        self.finish_node();
    }

    /// The index just past a `TypePrefix` written from the `n`th token.
    pub(super) fn skip_type_prefix(&self, n: usize) -> usize {
        self.skip_prefix_metadata(n + usize::from(self.nth_is_keyword(n, "abstract")))
    }

    // production: TypePrefix@kerml
    //
    // TypePrefix : Type = ( isAbstract ?= 'abstract' )?
    //     ( ownedRelationship += PrefixMetadataMember )*          (KerML 8.2.4.1.1)
    //
    // `abstract`, then every `#X` "placed immediately before the language-defined
    // (reserved) keyword for the declaration" (7.4.13, receipt 5e755297): KerML Spec
    // Annex A Examples/A-2-ModelingInstances.kerml:22-23 writes `#atom` on the line
    // before `classifier MyBike`. Never the other way about: `#X abstract class C;` is
    // no TypePrefix.
    //
    // The node is built even when empty, as MemberPrefix's is.
    pub(super) fn type_prefix(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::TypePrefix);
        self.eat_optional_keyword("abstract");
        while self.at(SyntaxKind::Hash) {
            self.prefix_metadata_member();
        }
        self.finish_node();
    }

    // production: ClassifierDeclaration@kerml
    //
    // ClassifierDeclaration : Classifier =
    //     ( isSufficient ?= 'all' )? Identification
    //     ( ownedRelationship += OwnedMultiplicity )?
    //     ( SuperclassingPart | ConjugationPart )?
    //     TypeRelationshipPart*                                   (KerML 8.2.4.2.1)
    //
    // Scoped `kerml`: SysML's definitions declare through DefinitionDeclaration
    // (8.2.2.6.1), which has no conjugation and no relationship parts. Every part is read:
    // `all`, Identification, the OwnedMultiplicity (`classifier MyBike [1]`, KerML Spec
    // Annex A Examples/A-2-ModelingInstances.kerml:8), the alternation of
    // SuperclassingPart and ConjugationPart, and TypeRelationshipPart*.
    //
    // The alternation takes one side at most: a conjugated type "may not also be the
    // specific Type in any Specialization" (KerML 8.3.3.1.2, receipt eabb0d9b), so
    // `class B :> A conjugates C;` is a syntax error, not a validity one.
    //
    // constraint: Type::validateTypeAtMostOneConjugator (KerML 8.3.3.1.10, receipt
    //     5200b0a5). The grammar already admits one ConjugationPart of one target; the
    //     constraint also reaches conjugations a type owns by other routes, for sv2-hir.
    pub(super) fn classifier_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ClassifierDeclaration);
        self.eat_optional_keyword("all");
        self.identification();
        if self.at(SyntaxKind::LBracket) {
            self.owned_multiplicity();
        }
        if self.at_superclassing() {
            self.superclassing_part();
        } else if self.at_conjugation_part() {
            self.conjugation_part();
        }
        self.type_relationship_parts();
        self.finish_node();
    }

    /// Whether a `ConjugationPart` is written here: `CONJUGATES = '~' | 'conjugates'`
    /// (`KerML` 8.2.2.7). The symbol has its own kind; the word arrives as a `BasicName`.
    pub(super) fn at_conjugation_part(&self) -> bool {
        self.at(SyntaxKind::Tilde) || self.at_keyword("conjugates")
    }

    // production: ConjugationPart@kerml
    //
    // ConjugationPart : Type =
    //     CONJUGATES ownedRelationship += OwnedConjugation       (KerML 8.2.4.1.1)
    //
    // One target, and no repetition: the part appears once in each declaration that
    // reaches it. The Pilot splits this in two, ClassifierConjugationPart over a name
    // alone and FeatureConjugationPart over a name or chain; both are deviations
    // xtext_only, follow_spec, so the one production of the clause is read in both
    // places, and a classifier may name a chain as the specification states.
    //
    // Scoped `kerml`: SysML writes a conjugation only as ConjugatedPortTyping's `~` after
    // `:` (8.2.2.12), which is a typing, not this part, and is read in .sysml alone.
    pub(super) fn conjugation_part(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ConjugationPart);
        self.terminal(SyntaxKind::Tilde, "conjugates", "`~` or `conjugates`");
        self.owned_conjugation();
        self.finish_node();
    }

    // production: OwnedConjugation@kerml
    //
    // OwnedConjugation : Conjugation =
    //       originalType = [QualifiedName]
    //     | originalType = FeatureChain
    //       { ownedRelatedElement += originalType }                (KerML 8.2.4.1.3)
    //
    // The metaclass is Conjugation (8.3.3.1.2, receipt eabb0d9b): its conjugatedType is
    // the declaring type, which "inherits all the Features of the originalType, but with
    // all input and output Features reversed". A derivation, sv2-resolve's; no implied
    // specialization attaches. The chain builds the OwnedFeatureChain node, as
    // OwnedDisjoining's does.
    fn owned_conjugation(&mut self) {
        self.chainable_target(SyntaxKind::OwnedConjugation);
    }

    // production: TypeRelationshipPart@kerml
    //
    // TypeRelationshipPart : Type =
    //     DisjoiningPart | UnioningPart | IntersectingPart | DifferencingPart
    //                                                            (KerML 8.2.4.1.1)
    //
    // Scoped `kerml`: SysML states no such part, and a .sysml file never reaches this
    // (ADR-0014). Read as the `TypeRelationshipPart*` that ends ClassifierDeclaration
    // (8.2.4.2.1) and, through FeatureRelationshipPart's first alternative, FeatureDeclaration
    // (8.2.4.3.1), and TypeDeclaration (8.2.4.1.1).
    //
    // An alternation with no node, as ConnectorDeclaration is; the part says which. The
    // four open on four reserved keywords (8.2.2.6), `disjoint`, `unions`, `intersects`
    // and `differences`, so one token chooses and none can be taken for a name. The star
    // admits a part more than once and in any order: `classifier F unions A unions B;`
    // (vendor/corpus/kerml/src/examples/Simple Tests/Classifiers.kerml:15).
    //
    // constraint: Type::validateTypeOwnedUnioningNotOne, validateTypeOwnedIntersectingNotOne,
    //     validateTypeOwnedDifferencingNotOne, validateTypeUnioningTypesNotSelf,
    //     validateTypeIntersectingTypesNotSelf and validateTypeDifferencingTypesNotSelf
    //     (KerML 8.3.3.1.10, receipt 5200b0a5). Validity, not syntax: `unions A;` with one
    //     target parses and carries its diagnostic downstream (ADR-0002). No implied
    //     specialization attaches to any of the four relationships.
    fn type_relationship_parts(&mut self) {
        while self.type_relationship_part() {}
    }

    /// One `TypeRelationshipPart`, if one is written here. Returns whether it was.
    fn type_relationship_part(&mut self) -> bool {
        if self.at_keyword("disjoint") {
            self.disjoining_part();
        } else if self.at_keyword("unions") {
            self.relationship_part(SyntaxKind::UnioningPart, "unions", SyntaxKind::Unioning);
        } else if self.at_keyword("intersects") {
            self.relationship_part(
                SyntaxKind::IntersectingPart,
                "intersects",
                SyntaxKind::Intersecting,
            );
        } else if self.at_keyword("differences") {
            self.relationship_part(
                SyntaxKind::DifferencingPart,
                "differences",
                SyntaxKind::Differencing,
            );
        } else {
            return false;
        }
        true
    }

    // production: FeatureRelationshipPart@kerml
    //
    // FeatureRelationshipPart : Feature =
    //     TypeRelationshipPart | ChainingPart | InvertingPart | TypeFeaturingPart
    //                                                            (KerML 8.2.4.3.1)
    //
    // `FeatureRelationshipPart*`, the end of a FeatureDeclaration, all four alternatives.
    // They open on four reserved words, so one token chooses, and the star takes them in
    // any order and number: `feature f featured by A unions b featured by B;`.
    pub(super) fn feature_relationship_parts(&mut self) {
        loop {
            if self.type_relationship_part() {
                continue;
            }
            if self.at_keyword("chains") {
                self.chaining_part();
                continue;
            }
            if self.at_keyword("inverse") {
                self.inverting_part();
                continue;
            }
            if self.at_keyword("featured") {
                self.type_featuring_part();
                continue;
            }
            return;
        }
    }

    // production: ChainingPart@kerml
    //
    // ChainingPart : Feature =
    //     'chains'
    //     ( ownedRelationship += OwnedFeatureChaining
    //     | FeatureChain )                                       (KerML 8.2.4.3.1)
    //
    // FeatureChain : Feature =
    //     ownedRelationship += OwnedFeatureChaining
    //     ( '.' ownedRelationship += OwnedFeatureChaining )+     (KerML 8.2.4.3.5)
    //
    // Both alternatives add FeatureChainings (8.3.3.3.5, receipt ed78282e) to the
    // DECLARED feature's ownedRelationship: FeatureChain is called here unassigned, so it
    // contributes to the Feature this part returns, where OwnedFeatureChainMember and the
    // relationship targets assign it to a Feature of its own. So the links are this node's
    // children, flat, with no OwnedFeatureChain between: `feature cousins chains
    // parents.siblings.children;` (KerML 7.3.4.6, receipt 8cd4056a). The two alternatives
    // share their first link and differ only in whether a `.` follows, so one loop reads
    // both, as `owned_feature_chain` does for its own.
    //
    // constraint: Feature::validateFeatureChainingFeatureNotOne (KerML 8.3.3.3.4). A
    //     single chainingFeature is invalid, and the first alternative writes exactly one:
    //     `feature b_f_a chains b chains f.a;` (Simple Tests/FeatureChains.kerml:33).
    //     Validity, not syntax, so it parses and carries its diagnostic downstream
    //     (ADR-0002); likewise validateFeatureChainingFeaturesNotSelf and
    //     validateFeatureChainingFeatureConformance. deriveFeatureChainingFeature and
    //     deriveFeatureOwnedFeatureChaining are derivations, sv2-resolve's. No implied
    //     specialization attaches.
    fn chaining_part(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ChainingPart);
        self.expect_keyword("chains");
        self.owned_feature_chaining();
        while self.at_feature_chain() {
            self.bump();
            self.owned_feature_chaining();
        }
        self.finish_node();
    }

    // production: InvertingPart@kerml
    //
    // InvertingPart : Feature =
    //     'inverse' 'of' ownedRelationship += OwnedFeatureInverting
    //                                                            (KerML 8.2.4.3.1)
    //
    // One target, not a list: "only a single feature identification is allowed after
    // inverse of" (KerML 7.3.4.7, receipt f0f6593d). A second is a second part.
    fn inverting_part(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::InvertingPart);
        self.expect_keyword("inverse");
        self.expect_keyword("of");
        self.owned_feature_inverting();
        self.finish_node();
    }

    // production: OwnedFeatureInverting@kerml
    //
    // OwnedFeatureInverting : FeatureInverting =
    //       invertingFeature = [QualifiedName]
    //     | invertingFeature = OwnedFeatureChain
    //       { ownedRelatedElement += invertingFeature }            (KerML 8.2.4.3.6)
    //
    // The metaclass is FeatureInverting (8.3.3.3.6, receipt 13f1b52b): its featureInverted
    // is the declared feature, which owns it, and its invertingFeature the target. The
    // target has the shape every chainable relationship target has, so
    // `chainable_target` reads it. Unlike a ChainingPart's links, the chain here is a
    // Feature of its own, an ownedRelatedElement of the relationship, and so builds the
    // OwnedFeatureChain node: `feature f inverse of a.b.c;`.
    //
    // constraint: Feature::deriveFeatureOwnedFeatureInverting (KerML 8.3.3.3.4). A
    //     derivation, sv2-resolve's. No implied specialization attaches.
    fn owned_feature_inverting(&mut self) {
        self.chainable_target(SyntaxKind::OwnedFeatureInverting);
    }

    /// Which standalone relationship declaration starts at the `n`th meaningful token, if
    /// one does. Asked by every site that asks for a `KerML` member element, and only by
    /// those, so none is reached from a .sysml file.
    pub(super) fn at_relationship_declaration(&self, n: usize) -> Option<RelationshipDeclaration> {
        if self.at_feature_inverting(n) {
            return Some(RelationshipDeclaration::FeatureInverting);
        }
        // `disjoining` and `disjoint` are reserved (8.2.2.6), and a DisjoiningPart's
        // `disjoint` only ever follows a declaration, never a member position.
        if self.nth_is_keyword(n, "disjoining") || self.nth_is_keyword(n, "disjoint") {
            return Some(RelationshipDeclaration::Disjoining);
        }
        // `conjugation` and `conjugate` likewise; a ConjugationPart writes `conjugates` or
        // `~`, and only after a declaration.
        if self.nth_is_keyword(n, "conjugation") || self.nth_is_keyword(n, "conjugate") {
            return Some(RelationshipDeclaration::Conjugation);
        }
        // `featuring` is reserved (8.2.2.6) and opens nothing else; a TypeFeaturingPart
        // writes `featured`, and only after a declaration.
        if self.nth_is_keyword(n, "featuring") {
            return Some(RelationshipDeclaration::TypeFeaturing);
        }
        let word = self.skip_specialization_prefix(n);
        if self.nth_is_keyword(word, "subtype") {
            Some(RelationshipDeclaration::Specialization)
        } else if self.nth_is_keyword(word, "subclassifier") {
            Some(RelationshipDeclaration::Subclassification)
        } else if self.nth_is_keyword(word, "typing") {
            Some(RelationshipDeclaration::FeatureTyping)
        } else if self.nth_is_keyword(word, "subset") {
            Some(RelationshipDeclaration::Subsetting)
        } else if self.nth_is_keyword(word, "redefinition") {
            Some(RelationshipDeclaration::Redefinition)
        } else {
            None
        }
    }

    /// The declaration `at_relationship_declaration` found.
    pub(super) fn relationship_declaration(&mut self, declaration: RelationshipDeclaration) {
        match declaration {
            RelationshipDeclaration::FeatureInverting => self.feature_inverting(),
            RelationshipDeclaration::Specialization => self.specialization(),
            RelationshipDeclaration::Subclassification => self.subclassification(),
            RelationshipDeclaration::FeatureTyping => self.kerml_feature_typing(),
            RelationshipDeclaration::Subsetting => self.subsetting(),
            RelationshipDeclaration::Redefinition => self.redefinition(),
            RelationshipDeclaration::Disjoining => self.disjoining(),
            RelationshipDeclaration::Conjugation => self.conjugation(),
            RelationshipDeclaration::TypeFeaturing => self.type_featuring(),
        }
    }

    /// Whether a `FeatureInverting` starts at the `n`th meaningful token.
    ///
    /// `inverting` or `inverse`, both reserved (`KerML` 8.2.2.6); neither opens any other
    /// element, and a `FeatureDeclaration`'s `InvertingPart` is reached only after one, so a
    /// member position decides on the one token.
    fn at_feature_inverting(&self, n: usize) -> bool {
        self.nth_is_keyword(n, "inverting") || self.nth_is_keyword(n, "inverse")
    }

    // production: FeatureInverting@kerml
    //
    // FeatureInverting =
    //     ( 'inverting' Identification? )?
    //     'inverse'
    //     ( featureInverted = [QualifiedName]
    //     | featureInverted = OwnedFeatureChain
    //       { ownedRelatedElement += featureInverted }
    //     )
    //     'of'
    //     ( invertingFeature = [QualifiedName]
    //     | ownedRelatedElement += OwnedFeatureChain
    //       { ownedRelatedElement += invertingFeature }
    //     )
    //     RelationshipBody                                       (KerML 8.2.4.3.6)
    //
    // A NonFeatureElement (8.2.3.4.3), KerML's alone: SysML states no such production, and
    // every dispatch to this is behind a KerML guard. The metaclass is FeatureInverting
    // (8.3.3.3.6, receipt 13f1b52b), relating featureInverted, the first target, to
    // invertingFeature, the second: `inverse Person::parents of Person::children;`
    // (KerML 7.3.4.7, receipt f0f6593d; Simple Tests/Inverses.kerml:11).
    //
    // The two targets have no node of their own, as Dependency's clients and suppliers
    // have none: the `of` between them says which is which. Each is a QualifiedName or an
    // OwnedFeatureChain, told apart by whether a `.` follows the first name.
    //
    // The clause's second chain alternative assigns `ownedRelatedElement +=
    // OwnedFeatureChain` and then `ownedRelatedElement += invertingFeature`, where the
    // first writes `featureInverted = OwnedFeatureChain`; the Pilot writes
    // `ownedRelatedElement += OwnedFeatureChain` on both sides (KerML.xtext:633-641). The
    // TEXT is the same under all three readings, so this parser is unaffected; which
    // element becomes invertingFeature is sv2-hir's to settle.
    //
    // Identification is optional after `inverting` and every part of it is optional
    // (8.2.3.1), so it is built whenever `inverting` is written, empty in `inverting
    // inverse a of b;`, as `payload_feature` builds its own: one shape is simpler to
    // consume than two.
    fn feature_inverting(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FeatureInverting);
        if self.at_keyword("inverting") {
            self.expect_keyword("inverting");
            self.identification();
        }
        self.expect_keyword("inverse");
        self.name_or_owned_feature_chain();
        self.expect_keyword("of");
        self.name_or_owned_feature_chain();
        self.relationship_body();
        self.finish_node();
    }

    /// The index just past a `( 'specialization' Identification )?` written from the
    /// `n`th meaningful token, or `n` itself when none is.
    ///
    /// The prefix the standalone `Specialization`, `Subclassification`, `FeatureTyping`,
    /// `Subsetting` and `Redefinition` share (`KerML` 8.2.4.1.2, 8.2.4.2.2, 8.2.4.3.2-4): the
    /// keyword after it is what says which, so a recogniser looks past it. Every part of
    /// the `Identification` is optional (8.2.3.1): `<` NAME `>`, then NAME.
    fn skip_specialization_prefix(&self, n: usize) -> usize {
        if !self.nth_is_keyword(n, "specialization") {
            return n;
        }
        let mut m = n + 1;
        if self.nth_is(m, SyntaxKind::Lt) {
            m += 3;
        }
        m + usize::from(self.nth_is_name(m))
    }

    /// `( 'specialization' Identification )?`, read if written.
    ///
    /// Unlike `FeatureInverting`'s `( 'inverting' Identification? )?`, the `Identification`
    /// is not itself optional here, which changes no text since it derives the empty
    /// string, and it is built whenever the keyword is written either way.
    fn specialization_prefix(&mut self) {
        if self.at_keyword("specialization") {
            self.expect_keyword("specialization");
            self.identification();
        }
    }

    // production: Specialization@kerml
    //
    // Specialization =
    //     ( 'specialization' Identification )?
    //     'subtype' SpecificType
    //     SPECIALIZES GeneralType
    //     RelationshipBody                                       (KerML 8.2.4.1.2)
    //
    // SPECIALIZES = ':>' | 'specializes'                        (KerML 8.2.2.7)
    //
    // A NonFeatureElement (8.2.3.4.3), KerML's alone, dispatched behind KerML guards as
    // FeatureInverting is. The metaclass is Specialization (8.3.3.1.8, receipt
    // dfc0f1ba), relating its specific type to its general one: `specialization Gen
    // subtype A specializes B;` (KerML 7.3.2.3, receipt e72b8e89; Simple
    // Tests/Types.kerml:17). One general type: a list is a type's owned specializations,
    // written in its own declaration.
    //
    // constraint: Specialization::validateSpecificationSpecificNotConjugated (KerML
    //     8.3.3.1.8). Validity, not syntax: `subtype A :> B;` parses whatever A is and
    //     carries its diagnostic downstream (ADR-0002). No implied specialization
    //     attaches: this IS the specialization, written out.
    fn specialization(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Specialization);
        self.specialization_prefix();
        self.expect_keyword("subtype");
        self.specific_type();
        self.terminal(SyntaxKind::ColonGt, "specializes", "`:>` or `specializes`");
        self.general_type();
        self.relationship_body();
        self.finish_node();
    }

    // production: Subclassification@kerml
    //
    // Subclassification =
    //     ( 'specialization' Identification )?
    //     'subclassifier' subclassifier = [QualifiedName]
    //     SPECIALIZES superclassifier = [QualifiedName]
    //     RelationshipBody                                       (KerML 8.2.4.2.2)
    //
    // A Specialization between two classifiers (8.3.3.2.3, receipt 3f715fda), its
    // subclassifier and superclassifier redefining specific and general: `specialization
    // Super subclassifier A specializes B;` (KerML 7.3.3.3, receipt b8c11363; Simple
    // Tests/Classifiers.kerml:5). NAMES on both sides, where a Specialization takes a
    // feature chain too: a Classifier is not a Feature, so there is nothing to chain.
    // One superclassifier; a list is a classifier's own SuperclassingPart (8.2.4.2.1).
    //
    // constraint: none on Subclassification itself; Classifier::
    //     deriveClassifierOwnedSubclassification (KerML 8.3.3.2.2) is a derivation,
    //     sv2-resolve's. No implied specialization attaches to this declaration. The
    //     one checkTypeSpecialization (KerML 8.3.3.1.10) implies, to Base::Anything, is
    //     the subclassifier's, and only where it has no explicit specialization.
    fn subclassification(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Subclassification);
        self.specialization_prefix();
        self.expect_keyword("subclassifier");
        self.qualified_name();
        self.terminal(SyntaxKind::ColonGt, "specializes", "`:>` or `specializes`");
        self.qualified_name();
        self.relationship_body();
        self.finish_node();
    }

    // production: FeatureTyping@kerml
    //
    // FeatureTyping =
    //     ( 'specialization' Identification )?
    //     'typing' typedFeature = [QualifiedName]
    //     TYPED_BY GeneralType
    //     RelationshipBody                                       (KerML 8.2.4.3.2)
    //
    // TYPED_BY = ':' | 'typed' 'by'                              (KerML 8.2.2.7)
    //
    // A Specialization whose specific is a Feature and whose general its type (8.3.3.3.7,
    // receipt a58abb3e): `specialization t1 typing customer typed by Person;` (KerML
    // 7.3.4.3, receipt de9b153b; Simple Tests/Features.kerml:42). The typed feature is a
    // name, the type a GeneralType, so a name or a feature chain. One type; a list is a
    // feature's own typings. The Pilot writes `FeatureType` for GeneralType
    // (KerML.xtext:664-669); deviations.json records FeatureType xtext_only,
    // follow_spec, so no production is added for it.
    //
    // SCOPED IN THE NAME because SysML states a production of the same name, `FeatureTyping
    // = OwnedFeatureTyping | ConjugatedPortTyping` (8.2.2.6.5), read by `feature_typing`
    // (ADR-0015). The NODE is shared, a decision taken knowingly against the nearest
    // precedent: OwnedMultiplicityRange got a kind of its own beside MultiplicityRange,
    // but that production has a name of its own, and every node kind here is a
    // production's name. Two productions of ONE name have no second name to give, and
    // both are the metaclass FeatureTyping. The first child says which built the node:
    // `specialization` or `typing` here, where the first QualifiedName is the
    // typedFeature, and an OwnedFeatureTyping or ConjugatedPortTyping there, whose name
    // is the type. A typed accessor in sv2-ast reads that child before any other; the
    // kind's entry in scripts/gen_syntax_kinds.py states both shapes.
    //
    // constraint: none on FeatureTyping itself (8.3.3.3.7 lists none); those it inherits
    //     from Specialization still apply downstream, among them
    //     validateSpecificationSpecificNotConjugated (8.3.3.1.8). No implied
    //     specialization attaches: this IS the typing, written out.
    fn kerml_feature_typing(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::FeatureTyping);
        self.specialization_prefix();
        self.expect_keyword("typing");
        self.qualified_name();
        if self.at(SyntaxKind::Colon) {
            self.bump();
        } else {
            self.expect_keyword("typed");
            self.expect_keyword("by");
        }
        self.general_type();
        self.relationship_body();
        self.finish_node();
    }

    // production: Subsetting@kerml
    //
    // Subsetting =
    //     ( 'specialization' Identification )?
    //     'subset' SpecificType
    //     SUBSETS GeneralType
    //     RelationshipBody                                       (KerML 8.2.4.3.3)
    //
    // SUBSETS = ':>' | 'subsets'                                 (KerML 8.2.2.7)
    //
    // A Specialization between two features (8.3.3.3.10, receipt 4738b7f2), its
    // subsettingFeature and subsettedFeature: `specialization Sub subset parent subsets
    // person;` (KerML 7.3.4.4, receipt aa7a8838; Simple Tests/Features.kerml:45). Both
    // sides a name or a feature chain, as a Specialization's are: `subset g.g subsets
    // b.f.a;` (Simple Tests/FeatureChains.kerml:23). One subsetted feature; a list is a
    // feature's own subsettings. `:>` is SPECIALIZES's symbol too, and the keyword before
    // the first target, not the symbol, is what says which relationship this is.
    //
    // constraint: Subsetting::validateSubsettingConstantConformance,
    //     validateSubsettingFeaturingTypes and validateSubsettingUniquenessConformance
    //     (KerML 8.3.3.3.10). Validity, not syntax: they carry their diagnostics
    //     downstream (ADR-0002). No implied specialization attaches: this IS the
    //     subsetting, written out.
    fn subsetting(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Subsetting);
        self.specialization_prefix();
        self.expect_keyword("subset");
        self.specific_type();
        self.terminal(SyntaxKind::ColonGt, "subsets", "`:>` or `subsets`");
        self.general_type();
        self.relationship_body();
        self.finish_node();
    }

    // production: Redefinition@kerml
    //
    // Redefinition =
    //     ( 'specialization' Identification )?
    //     'redefinition' SpecificType
    //     REDEFINES GeneralType
    //     RelationshipBody                                       (KerML 8.2.4.3.4)
    //
    // REDEFINES = ':>>' | 'redefines'                            (KerML 8.2.2.7)
    //
    // A Subsetting whose two features have the same values (8.3.3.3.8, receipt 7b56885c),
    // its redefiningFeature and redefinedFeature: `specialization Redef redefinition
    // LegalRecord::guardian redefines parent;` (KerML 7.3.4.5, receipt 8d8e645c; Simple
    // Tests/Features.kerml:68). Both sides a name or a feature chain: `redefinition b.f
    // redefines b.a;` (Simple Tests/FeatureChains.kerml:24). One redefined feature; a
    // list is a feature's own redefinitions.
    //
    // constraint: Redefinition::validateRedefinitionDirectionConformance,
    //     validateRedefinitionEndConformance and validateRedefinitionFeaturingTypes
    //     (KerML 8.3.3.3.8). Validity, not syntax: they carry their diagnostics
    //     downstream (ADR-0002). No implied specialization attaches: this IS the
    //     redefinition, written out.
    fn redefinition(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Redefinition);
        self.specialization_prefix();
        self.expect_keyword("redefinition");
        self.specific_type();
        self.terminal(SyntaxKind::ColonGtGt, "redefines", "`:>>` or `redefines`");
        self.general_type();
        self.relationship_body();
        self.finish_node();
    }

    // production: Disjoining@kerml
    //
    // Disjoining =
    //     ( 'disjoining' Identification )?
    //     'disjoint'
    //     ( typeDisjoined = [QualifiedName]
    //     | typeDisjoined = FeatureChain
    //       { ownedRelatedElement += typeDisjoined }
    //     )
    //     'from'
    //     ( disjoiningType = [QualifiedName]
    //     | disjoiningType = FeatureChain
    //       { ownedRelatedElement += disjoiningType }
    //     )
    //     RelationshipBody                                       (KerML 8.2.4.1.4)
    //
    // A Relationship between two types that share no instances (8.3.3.1.4, receipt
    // 9029a37f), typeDisjoined and disjoiningType: `disjoining Disj disjoint A from B;`
    // (KerML 7.3.2.5, receipt 2c9c122c). Each side a name or a feature chain, and the
    // chain, an ownedRelatedElement, builds the OwnedFeatureChain node as OwnedDisjoining's
    // does: `disjoint b.f.a from b.a;` (Simple Tests/FeatureChains.kerml:28). One type on
    // each side; a list is a type's own DisjoiningPart. Its prefix is `disjoining`, not
    // `specialization`: Disjoining is no Specialization.
    //
    // constraint: none on Disjoining itself (8.3.3.1.4 lists none);
    //     Type::deriveTypeOwnedDisjoining (KerML 8.3.3.1.10) is a derivation,
    //     sv2-resolve's. No implied specialization attaches.
    fn disjoining(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Disjoining);
        if self.at_keyword("disjoining") {
            self.expect_keyword("disjoining");
            self.identification();
        }
        self.expect_keyword("disjoint");
        self.name_or_owned_feature_chain();
        self.expect_keyword("from");
        self.name_or_owned_feature_chain();
        self.relationship_body();
        self.finish_node();
    }

    // production: Conjugation@kerml
    //
    // Conjugation =
    //     ( 'conjugation' Identification )?
    //     'conjugate'
    //     ( conjugatedType = [QualifiedName]
    //     | conjugatedType = FeatureChain
    //       { ownedRelatedElement += conjugatedType }
    //     )
    //     CONJUGATES
    //     ( originalType = [QualifiedName]
    //     | originalType = FeatureChain
    //       { ownedRelatedElement += originalType }
    //     )
    //     RelationshipBody                                       (KerML 8.2.4.1.3)
    //
    // CONJUGATES = '~' | 'conjugates'                            (KerML 8.2.2.7)
    //
    // A Relationship whose conjugatedType inherits the originalType's features with
    // their directions reversed (8.3.3.1.2, receipt eabb0d9b): `conjugation c1 conjugate
    // Conjugate1 conjugates Original;` (KerML 7.3.2.4, receipt 2107419a; Simple
    // Tests/Types.kerml:25). Each side a name or a feature chain, as Disjoining's are.
    // One original type: the production writes no list.
    //
    // constraint: none on Conjugation itself (8.3.3.1.2 lists none);
    //     Type::validateTypeAtMostOneConjugator (KerML 8.3.3.1.10) limits a type's OWNED
    //     conjugations, a validity check downstream (ADR-0002). No implied specialization
    //     attaches.
    fn conjugation(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Conjugation);
        if self.at_keyword("conjugation") {
            self.expect_keyword("conjugation");
            self.identification();
        }
        self.expect_keyword("conjugate");
        self.name_or_owned_feature_chain();
        self.terminal(SyntaxKind::Tilde, "conjugates", "`~` or `conjugates`");
        self.name_or_owned_feature_chain();
        self.relationship_body();
        self.finish_node();
    }

    // production: SpecificType@kerml
    //
    // SpecificType : Specialization =
    //       specific = [QualifiedName]
    //     | specific += OwnedFeatureChain
    //       { ownedRelatedElement += specific }                    (KerML 8.2.4.1.2)
    //
    // production: GeneralType@kerml
    //
    // GeneralType : Specialization =
    //       general = [QualifiedName]
    //     | general += OwnedFeatureChain
    //       { ownedRelatedElement += general }                     (KerML 8.2.4.1.2)
    //
    // Called unassigned, so each contributes to the Specialization that calls it and
    // builds no node: the SPECIALIZES between them says which is which, as the `of` does
    // in FeatureInverting. The clause's SpecificType line is malformed; deviations.json
    // records reading it as `SpecificType : Specialization =`, follow_spec, and GeneralType
    // spec_only, follow_spec.
    fn specific_type(&mut self) {
        self.name_or_owned_feature_chain();
    }

    fn general_type(&mut self) {
        self.name_or_owned_feature_chain();
    }

    // production: TypeFeaturingPart@kerml
    //
    // TypeFeaturingPart : Feature =
    //     'featured' 'by' ownedRelationship += OwnedTypeFeaturing
    //     ( ',' ownedTypeFeaturing += OwnedTypeFeaturing )*        (KerML 8.2.4.3.1)
    //
    // The clause names the repeated slot `ownedTypeFeaturing` and the first
    // `ownedRelationship`; the Pilot writes `ownedRelationship` for both, and the text is
    // the same either way. Each is a TypeFeaturing (8.3.3.3.11, receipt 8e4c93c3) whose
    // featureOfType is the declared feature: `featured by Occurrence` (Variable Feature
    // Examples/Enhancements/Moments.kerml:38).
    //
    // production: OwnedTypeFeaturing@kerml
    //
    // OwnedTypeFeaturing : TypeFeaturing =
    //     featuringType = [QualifiedName]                          (KerML 8.2.4.3.7)
    //
    // A name only: unlike the relationship parts' targets (8.2.4.1.4, 8.2.4.1.5), no
    // feature chain, so `featured by a.b` is reported.
    //
    // constraint: Feature::deriveFeatureOwnedTypeFeaturing and deriveFeatureFeaturingType
    //     (KerML 8.3.3.3.4). Derivations, sv2-resolve's.
    fn type_featuring_part(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::TypeFeaturingPart);
        self.expect_keyword("featured");
        self.expect_keyword("by");
        self.owned_type_featuring();
        while self.at(SyntaxKind::Comma) {
            self.bump();
            self.owned_type_featuring();
        }
        self.finish_node();
    }

    fn owned_type_featuring(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::OwnedTypeFeaturing);
        self.qualified_name();
        self.finish_node();
    }

    // production: TypeFeaturing@kerml
    //
    // TypeFeaturing =
    //     'featuring' ( Identification 'of' )?
    //     featureOfType = [QualifiedName]
    //     'by' featuringType = [QualifiedName]
    //     RelationshipBody                                       (KerML 8.2.4.3.7)
    //
    // A NonFeatureElement (8.2.3.4.3), KerML's alone, dispatched behind KerML guards as
    // FeatureInverting is. The metaclass is TypeFeaturing (8.3.3.3.11, receipt 8e4c93c3),
    // relating featureOfType, its source, to featuringType, its target: `featuring F of y
    // by C;` (Simple Tests/Features.kerml:16). Both are names, as OwnedTypeFeaturing's
    // target is: no feature chain on either side. One featuring type; a list is a
    // feature's own TypeFeaturingPart.
    //
    // The keyword before the featuring type is the clause's bare `by`. KerML 7.3.4.8's
    // examples (receipt 5065873b) write `featured by`, the owned form's pair; the clause,
    // the Pilot and the corpus agree on `by`, and deviations.json entry TypeFeaturing
    // records the examples as the error (conflict, follow_spec), so `featured by` here is
    // reported, as tests/rejection/kerml-type-featuring-is-featuring-by.kerml holds.
    //
    // `( Identification 'of' )?` is decided by looking past an Identification, `<` NAME
    // `>` then NAME, each optional (8.2.3.1), for `of`, which is reserved and so is never
    // the featured feature's name. Without it the first QualifiedName is featureOfType.
    // The Pilot writes `( Identification? 'of' )?` (KerML.xtext:651-656), the same text,
    // since Identification derives the empty string.
    // The Identification is built whenever the group is written, empty in `featuring of y
    // by C;`, as `feature_inverting` builds its own.
    //
    // constraint: none on TypeFeaturing itself (8.3.3.3.11 lists none). No implied
    //     specialization attaches.
    fn type_featuring(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::TypeFeaturing);
        self.expect_keyword("featuring");
        let mut m = 0;
        if self.nth_is(m, SyntaxKind::Lt) {
            m += 3;
        }
        m += usize::from(self.nth_is_name(m));
        if self.nth_is_keyword(m, "of") {
            self.identification();
            self.expect_keyword("of");
        }
        self.qualified_name();
        self.expect_keyword("by");
        self.qualified_name();
        self.relationship_body();
        self.finish_node();
    }

    // production: DisjoiningPart@kerml
    //
    // DisjoiningPart : Type =
    //     'disjoint' 'from' ownedRelationship += OwnedDisjoining
    //     ( ',' ownedRelationship += OwnedDisjoining )*          (KerML 8.2.4.1.1)
    //
    // The one part of two keywords. `from` is a binary connector's word too, which is
    // why `connector_from_follows` does not count a `from` directly after `disjoint`.
    fn disjoining_part(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::DisjoiningPart);
        self.expect_keyword("disjoint");
        self.expect_keyword("from");
        self.owned_disjoining();
        while self.at(SyntaxKind::Comma) {
            self.bump();
            self.owned_disjoining();
        }
        self.finish_node();
    }

    // production: OwnedDisjoining@kerml
    //
    // OwnedDisjoining : Disjoining =
    //       disjoiningType = [QualifiedName]
    //     | disjoiningType = FeatureChain
    //       { ownedRelatedElement += disjoiningType }             (KerML 8.2.4.1.4)
    //
    // The metaclass is Disjoining (8.3.3.1.4, receipt 9029a37f), its typeDisjoined the
    // declaring type. The chain is the same text as the OwnedFeatureChain the other three
    // own (8.2.4.3.5 states `OwnedFeatureChain : Feature = FeatureChain`), and builds the
    // same node, as `instantiated_type_member` explains for FeatureChain@kerml.
    fn owned_disjoining(&mut self) {
        self.chainable_target(SyntaxKind::OwnedDisjoining);
    }

    // production: UnioningPart@kerml
    // production: IntersectingPart@kerml
    // production: DifferencingPart@kerml
    //
    // UnioningPart : Type =
    //     'unions' ownedRelationship += Unioning
    //     ( ',' ownedRelationship += Unioning )*
    // IntersectingPart : Type =
    //     'intersects' ownedRelationship += Intersecting
    //     ( ',' ownedRelationship += Intersecting )*
    // DifferencingPart : Type =
    //     'differences' ownedRelationship += Differencing
    //     ( ',' ownedRelationship += Differencing )*              (KerML 8.2.4.1.1)
    //
    // production: Unioning@kerml
    // production: Intersecting@kerml
    // production: Differencing@kerml
    //
    // Unioning : Unioning =
    //     unioningType = [QualifiedName] | ownedRelatedElement += OwnedFeatureChain
    // Intersecting : Intersecting =
    //     intersectingType = [QualifiedName] | ownedRelatedElement += OwnedFeatureChain
    // Differencing : Differencing =
    //     differencingType = [QualifiedName] | ownedRelatedElement += OwnedFeatureChain
    //                                                            (KerML 8.2.4.1.5)
    //
    // Three productions of one shape, one method, as the eight classifiers share
    // `classifier`: a keyword, then one or more targets. The metaclasses are Unioning
    // (8.3.3.1.11, receipt bc28c9b0), Intersecting (8.3.3.1.7, receipt ebe08a05) and
    // Differencing (8.3.3.1.3, receipt 631855b4), each a Relationship whose source is the
    // declaring type. The target is `chainable_target`'s name-or-chain shape.
    fn relationship_part(&mut self, part: SyntaxKind, word: &str, target: SyntaxKind) {
        self.eat_trivia();
        self.start_node(part);
        self.expect_keyword(word);
        self.chainable_target(target);
        while self.at(SyntaxKind::Comma) {
            self.bump();
            self.chainable_target(target);
        }
        self.finish_node();
    }

    /// Whether a `SuperclassingPart` is written here.
    ///
    /// `SPECIALIZES = ':>' | 'specializes'` (`KerML` 8.2.4.2). The symbol is checked
    /// before the word because the lexer gives the symbol its own kind while the word
    /// arrives as a `BasicName`.
    fn at_superclassing(&self) -> bool {
        self.at(SyntaxKind::ColonGt) || self.at_keyword("specializes")
    }

    // production: SuperclassingPart
    //
    // SuperclassingPart : Classifier =
    //     SPECIALIZES ownedRelationship += OwnedSubclassification
    //     ( ',' ownedRelationship += OwnedSubclassification )*    (KerML 8.2.4.2)
    //
    // OwnedSubclassification is a shared unit — the same production SysML's
    // SubclassificationPart owns — so the target is read by the method that already
    // exists for it rather than by a second one written here.
    fn superclassing_part(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::SuperclassingPart);
        if self.at(SyntaxKind::ColonGt) {
            self.bump();
        } else {
            self.expect_keyword("specializes");
        }
        self.owned_subclassification();
        while self.at(SyntaxKind::Comma) {
            self.bump();
            self.owned_subclassification();
        }
        self.finish_node();
    }

    // production: TypeBody
    //
    // TypeBody : Type = ';' | '{' TypeBodyElement* '}'            (KerML 8.2.4.1)
    //
    // production: TypeBodyElement@kerml
    //
    // TypeBodyElement : Type =
    //     ownedRelationship += NonFeatureMember | ownedRelationship += FeatureMember
    //   | ownedRelationship += AliasMember | ownedRelationship += Import
    //                                                            (KerML 8.2.4.1.1)
    //
    // An alternation, read by `body_elements` under `Body::Type`; all four alternatives
    // are, FeatureMember through `feature_member`.
    pub(super) fn type_body(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::TypeBody);
        if self.at(SyntaxKind::Semicolon) {
            self.bump();
        } else if self.at(SyntaxKind::LBrace) {
            self.bump();
            self.depth += 1;
            self.body_elements(Some(SyntaxKind::RBrace), Body::Type);
            self.depth -= 1;
            self.expect(SyntaxKind::RBrace, "`}`");
        } else {
            self.error_expected("`;` or `{` after a classifier declaration");
        }
        self.finish_node();
    }
}
