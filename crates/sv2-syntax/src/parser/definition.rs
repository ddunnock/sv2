// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Definitions, `SysML` 8.2.2.6: definition prefixes and declarations, the simple
//! definitions, enumerations, individuals, and dependencies.

use crate::generated::kinds::SyntaxKind;
use crate::grammar::Language;
use crate::parser::Parser;
use crate::parser::body::Body;
use crate::parser::case::is_case_head;
use crate::parser::lookahead::{MemberHead, keyword};

/// A `SysML` definition production: one keyword over a shared spine.
///
/// Eight productions of `SysML` 8.2.2 are stated as `<prefix> KEYWORD 'def' Definition`,
/// differing in the keyword and in which prefix they take. Matching rule shapes across
/// the verified units finds twenty-two productions with a `def` keyword; the other
/// fourteen end in a specialised body — `ActionBody`, `CaseBody`, `CalculationBody`,
/// `RequirementBody`, `StateDefBody`, `InterfaceBody`, `ViewDefinitionBody` — and they
/// name the declaration and the body separately rather than taking a `Definition`, so
/// none of them is here whether or not its body is implemented. `RequirementBody` is
/// implemented and `RequirementDefinition` still has a method of its own for exactly
/// that reason. `PortDefinition` does take a `Definition`, and is out for the opposite
/// reason: it carries a trailing `ConjugatedPortDefinitionMember` the eight do not.
#[derive(Clone, Copy)]
pub(super) struct SimpleDefinition {
    /// The one keyword that says which production this is, before the `def`.
    keyword: &'static str,
    /// The node the production builds.
    node: SyntaxKind,
    /// Whether the prefix is an `OccurrenceDefinitionPrefix` rather than a
    /// `DefinitionPrefix`.
    ///
    /// The same distinction `SimpleUsage::is_occurrence` draws, one level up: only an
    /// occurrence may be `individual` (`SysML` 8.2.2.9.1). An attribute is not an
    /// occurrence, so `individual attribute def A;` is two errors rather than a prefix.
    is_occurrence: bool,
}

/// Every definition production sharing the `'def' Definition` spine.
///
/// The keywords are disjoint, so the order decides nothing.
pub(super) const SIMPLE_DEFINITIONS: [SimpleDefinition; 8] = [
    SimpleDefinition {
        keyword: "attribute",
        node: SyntaxKind::AttributeDefinition,
        is_occurrence: false,
    },
    SimpleDefinition {
        keyword: "occurrence",
        node: SyntaxKind::OccurrenceDefinition,
        is_occurrence: true,
    },
    SimpleDefinition {
        keyword: "item",
        node: SyntaxKind::ItemDefinition,
        is_occurrence: true,
    },
    SimpleDefinition {
        keyword: "part",
        node: SyntaxKind::PartDefinition,
        is_occurrence: true,
    },
    SimpleDefinition {
        keyword: "connection",
        node: SyntaxKind::ConnectionDefinition,
        is_occurrence: true,
    },
    SimpleDefinition {
        keyword: "flow",
        node: SyntaxKind::FlowDefinition,
        is_occurrence: true,
    },
    SimpleDefinition {
        keyword: "allocation",
        node: SyntaxKind::AllocationDefinition,
        is_occurrence: true,
    },
    SimpleDefinition {
        keyword: "rendering",
        node: SyntaxKind::RenderingDefinition,
        is_occurrence: true,
    },
];

impl Parser<'_> {
    // production: UsageExtensionKeyword@sysml
    //
    // UsageExtensionKeyword : Usage =
    //     ownedRelationship += PrefixMetadataMember                (SysML 8.2.2.6.2)
    //
    // production: DefinitionExtensionKeyword@sysml
    //
    // DefinitionExtensionKeyword : Definition =
    //     ownedRelationship += PrefixMetadataMember                (SysML 8.2.2.6.1)
    //
    // Every `#X` a prefix may write, as many as are written: "It is also possible to
    // include more than one user defined-keyword in a declaration" (7.27.4, receipt
    // 0f2c5bd1). The two productions have one body and differ in the element that owns
    // the membership, so one method reads both and `node` says which it built.
    pub(super) fn extension_keywords(&mut self, node: SyntaxKind) {
        while self.at(SyntaxKind::Hash) {
            self.eat_trivia();
            self.start_node(node);
            self.prefix_metadata_member();
            self.finish_node();
        }
    }

    /// Whether a `PartDefinition` starts at the `n`th meaningful token.
    ///
    /// `PartDefinition = OccurrenceDefinitionPrefix 'part' 'def' Definition`
    /// (`SysML` 8.2.2.11), with `OccurrenceDefinitionPrefix = BasicDefinitionPrefix?
    /// ( 'individual' EmptyMultiplicityMember )? DefinitionExtensionKeyword*`
    /// (8.2.2.9.1). The prefix keywords are optional and also open usages
    /// (`abstract part x;` is a `PartUsage`), so only `part` followed by `def` decides.
    fn at_simple_definition(&self, n: usize) -> Option<SimpleDefinition> {
        SIMPLE_DEFINITIONS.iter().copied().find(|definition| {
            let after = if definition.is_occurrence {
                self.skip_occurrence_definition_prefix(n)
            } else {
                self.skip_definition_prefix(n)
            };
            self.nth_is_keyword(after, definition.keyword) && self.nth_is_keyword(after + 1, "def")
        })
    }

    /// The index just past a `DefinitionPrefix` written from the `n`th token.
    ///
    /// `DefinitionPrefix = BasicDefinitionPrefix? DefinitionExtensionKeyword*`
    /// (`SysML` 8.2.2.6.1).
    pub(super) fn skip_definition_prefix(&self, n: usize) -> usize {
        self.skip_prefix_metadata(self.skip_basic_definition_prefix(n))
    }

    /// The index just past a `BasicDefinitionPrefix` at the `n`th token, if one is there.
    fn skip_basic_definition_prefix(&self, n: usize) -> usize {
        n + usize::from(self.nth_is_keyword(n, "abstract") || self.nth_is_keyword(n, "variation"))
    }

    /// The index just past an `OccurrenceDefinitionPrefix` written from the `n`th token.
    ///
    /// A `DefinitionPrefix` with the one keyword only an occurrence may carry between its
    /// two parts: `BasicDefinitionPrefix? ( 'individual' EmptyMultiplicityMember )?
    /// DefinitionExtensionKeyword*` (`SysML` 8.2.2.9.1). The same pair
    /// `skip_basic_usage_prefix` and `skip_occurrence_usage_prefix` make one level down.
    pub(super) fn skip_occurrence_definition_prefix(&self, n: usize) -> usize {
        let n = self.skip_basic_definition_prefix(n);
        self.skip_prefix_metadata(n + usize::from(self.nth_is_keyword(n, "individual")))
    }

    /// Whether `at_definition_element` can accept at a member with this `head`.
    ///
    /// Each of its alternatives reads only prefix words and `#` metadata before one
    /// keyword, so that keyword is the head:
    /// - `package` (`at_package`), `standard` or `library` (`at_library_package`, which
    ///   reads nothing before them), and `dependency` (`at_dependency`);
    /// - the kind keyword before `def`: `port`, `requirement`, `constraint`, `calc`,
    ///   `metadata`, `action`, `state`, `enum`, `interface`, `concern`, `viewpoint`,
    ///   `view`, every `SIMPLE_DEFINITIONS` keyword, and the first of every `CASES`
    ///   entry's keywords;
    /// - `def` itself, for `at_extended_definition` (`#X def`) and
    ///   `at_individual_definition` (`individual def`), which write no kind keyword.
    ///
    /// `definition_element_heads_cover_every_definition` holds this to the recognisers.
    pub(super) fn opens_definition(head: MemberHead<'_>) -> bool {
        let Some(word) = head.word else {
            return false;
        };
        matches!(
            word,
            "package"
                | "standard"
                | "library"
                | "dependency"
                | "def"
                | "port"
                | "requirement"
                | "constraint"
                | "calc"
                | "metadata"
                | "action"
                | "state"
                | "enum"
                | "interface"
                | "concern"
                | "viewpoint"
                | "view"
        ) || SIMPLE_DEFINITIONS
            .iter()
            .any(|definition| definition.keyword == word)
            || is_case_head(word)
    }

    /// Whether an implemented `DefinitionElement` starts at the `n`th meaningful token.
    pub(super) fn at_definition_element(&self, n: usize) -> bool {
        self.at_package(n)
            || self.at_library_package(n)
            || self.at_dependency(n)
            || self.at_port_definition(n)
            || self.at_requirement_definition(n)
            || self.at_constraint_definition(n)
            || self.at_calculation_definition(n)
            || self.at_case_definition(n).is_some()
            || self.at_metadata_definition(n)
            || self.at_action_definition(n)
            || self.at_state_definition(n)
            || self.at_enumeration_definition(n)
            || self.at_simple_definition(n).is_some()
            || self.at_interface_definition(n)
            || self.at_concern_definition(n)
            || self.at_viewpoint_definition(n)
            || self.at_view_definition(n)
            || self.at_individual_definition(n)
            || self.at_extended_definition(n)
    }

    /// Whether an `ExtendedDefinition` starts at the `n`th meaningful token.
    ///
    /// `BasicDefinitionPrefix? DefinitionExtensionKeyword+ 'def'` (`SysML` 8.2.2.27): at
    /// least one `#` and then `def` with no kind keyword between, which is the whole of
    /// what separates it from a definition whose prefix carries prefix metadata.
    /// "A user-defined keyword for semantic metadata may also be used to declare a
    /// definition or usage without using any language-defined keyword" (7.27.4, receipt
    /// 0f2c5bd1).
    fn at_extended_definition(&self, n: usize) -> bool {
        let k = self.skip_basic_definition_prefix(n);
        let after = self.skip_prefix_metadata(k);
        after > k && self.nth_is_keyword(after, "def")
    }

    /// `SysML`'s `DefinitionElement`, minus the `package` `membership` takes first.
    /// Returns whether one was read.
    ///
    /// `DefinitionElement` gets no node, as `UsageElement` gets none: it is an
    /// alternation, and the alternative that matched says which was taken
    /// (`SysML` 8.2.2.5.2, Package Elements — where BOTH alternations are stated, not
    /// 8.2.2.6.1, which is Definitions and only USES `DefinitionElement` inside
    /// `DefinitionMember`). Written in the same order as `at_definition_element`, its
    /// recogniser, so that the two cannot silently disagree about what a member may be.
    ///
    /// Order is not load-bearing here — each alternative is introduced by its own
    /// keyword pair, and a keyword is not a name (`SysML` 8.2.2.1.2). It is in
    /// `usage_element`, which says why there.
    pub(super) fn definition_element(&mut self) -> bool {
        if self.at_dependency(0) {
            self.dependency();
        } else if self.at_port_definition(0) {
            self.port_definition();
        } else if self.at_enumeration_definition(0) {
            self.enumeration_definition();
        } else if self.requirement_family_definition() {
            // Read by the call, which answers whether it read one.
        } else if self.at_view_definition(0) {
            // Where it stands decides nothing: `view def` is its own keyword pair, and
            // IndividualDefinition writes `def` after `individual` and its extension
            // keywords (8.2.2.9.1), never `view`, so `individual view def` is not one.
            self.view_definition();
        } else if let Some(case) = self.at_case_definition(0) {
            self.case_definition(case);
        } else if self.at_metadata_definition(0) {
            self.metadata_definition();
        } else if self.at_action_definition(0) {
            self.action_definition();
        } else if self.at_state_definition(0) {
            self.state_definition();
        } else if let Some(definition) = self.at_simple_definition(0) {
            self.simple_definition_element(definition);
        } else if self.at_interface_definition(0) {
            self.interface_definition();
        } else if self.at_individual_definition(0) {
            self.individual_definition();
        } else if self.at_extended_definition(0) {
            self.extended_definition();
        } else {
            return false;
        }
        true
    }

    /// One of `SIMPLE_DEFINITIONS` read as a `DefinitionElement`. Split out of
    /// `definition_element` so that function stays within clippy's complexity budget, as
    /// `behavior_usage_element` is split out of `usage_element_of_class`.
    fn simple_definition_element(&mut self, definition: SimpleDefinition) {
        if definition.keyword == "allocation" {
            // 8.2.2.5.2's DefinitionElement lists 29 alternatives and not
            // AllocationDefinition, which 8.2.2.15 defines.
            // deviation: DefinitionElement
            self.note_deviation(
                "DefinitionElement",
                "an allocation definition as a DefinitionElement",
            );
        }
        self.simple_definition(definition);
    }

    // production: ExtendedDefinition@sysml
    //
    // ExtendedDefinition : Definition =
    //     BasicDefinitionPrefix? DefinitionExtensionKeyword+ 'def' Definition
    //                                                            (SysML 8.2.2.27)
    //
    // "A user-defined keyword for semantic metadata may also be used to declare a
    // definition or usage without using any language-defined keyword ... `#situation def
    // Failure;`" (7.27.4, receipt 0f2c5bd1). The metaclass is Definition itself; what it
    // specializes comes from the metadata's baseType (7.27.3, receipt 938f2744), which is
    // resolution's. The prefix is written inline, so there is no DefinitionPrefix node.
    //
    // implied specialization: the baseType of each SemanticMetadata keyword (7.27.3).
    fn extended_definition(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::ExtendedDefinition);
        if self.at_keyword("abstract") || self.at_keyword("variation") {
            self.basic_definition_prefix();
        }
        self.extension_keywords(SyntaxKind::DefinitionExtensionKeyword);
        self.expect_keyword("def");
        self.definition();
        self.finish_node();
    }

    // production: IndividualDefinition@sysml
    //
    // IndividualDefinition : OccurrenceDefinition =
    //     BasicDefinitionPrefix? isIndividual ?= 'individual'
    //     DefinitionExtensionKeyword* 'def' Definition
    //     ownedRelationship += EmptyMultiplicityMember           (SysML 8.2.2.9.1)
    //
    // "individual may be used in place of the kind keyword, in which case the declaration
    // is equivalent to individual occurrence" (7.9.4, receipt 8c84370d): `individual def
    // Flight_248 :> Flight;`. The metaclass is OccurrenceDefinition (8.3.9.3, receipt
    // 69f220d7). The prefix is written inline, as ExtendedDefinition's is, so there is no
    // OccurrenceDefinitionPrefix node.
    //
    // The EmptyMultiplicityMember is LAST, where the specification writes it. The Pilot
    // writes it straight after `individual` (SysML.xtext:816), as OccurrenceDefinitionPrefix
    // does in both. It consumes no tokens, so the two accept the same text and differ only
    // in where the node sits; the specification is followed, and no deviation is needed.
    //
    // implied specialization: the EmptyMultiplicity's, for an individual definition
    // constraint: OccurrenceDefinition::checkOccurrenceDefinitionIndividualSpecialization
    //     and checkOccurrenceDefinitionMultiplicitySpecialization (8.3.9.3). Injections,
    //     so sv2-hir's (ADR-0002).
    fn individual_definition(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::IndividualDefinition);
        if self.at_keyword("abstract") || self.at_keyword("variation") {
            self.basic_definition_prefix();
        }
        self.expect_keyword("individual");
        self.extension_keywords(SyntaxKind::DefinitionExtensionKeyword);
        self.expect_keyword("def");
        self.definition();
        self.empty_multiplicity_member();
        self.finish_node();
    }

    /// Whether an `IndividualDefinition` starts at the `n`th meaningful token.
    ///
    /// `BasicDefinitionPrefix? 'individual' DefinitionExtensionKeyword* 'def'` (`SysML`
    /// 8.2.2.9.1): `def` straight after the prefix, with no kind keyword between, which is
    /// what separates it from an occurrence definition whose prefix is individual.
    fn at_individual_definition(&self, n: usize) -> bool {
        let k = self.skip_basic_definition_prefix(n);
        self.nth_is_keyword(k, "individual")
            && self.nth_is_keyword(self.skip_prefix_metadata(k + 1), "def")
    }

    /// Whether a `Dependency` starts at the `n`th meaningful token.
    ///
    /// `dependency`, reserved in both grammars, after its `PrefixMetadataAnnotation*`,
    /// looked past in both.
    pub(super) fn at_dependency(&self, n: usize) -> bool {
        self.nth_is_keyword(self.skip_prefix_metadata(n), "dependency")
    }

    // Dependency =
    //     ( ownedRelationship += PrefixMetadataAnnotation )*
    //     'dependency' DependencyDeclaration RelationshipBody       (SysML 8.2.2.3)
    //
    // Dependency =
    //     ( ownedRelationship += PrefixMetadataAnnotation )*
    //     'dependency' ( Identification? 'from' )?
    //     client += [QualifiedName] ( ',' client += [QualifiedName] )* 'to'
    //     supplier += [QualifiedName] ( ',' supplier += [QualifiedName] )*
    //     RelationshipBody                                          (KerML 8.2.3.2)
    //
    // production: Dependency@sysml
    // production: Dependency@kerml
    //
    // Both read whole. The PrefixMetadataAnnotation is over each language's own element
    // (`prefix_metadata_annotation`); the corpus writes SysML's, `#refinement dependency`
    // (SimpleVehicleModel.sysml:937).
    //
    // The two languages state the same text: `Identification?` and `Identification`
    // accept the same strings, since every part of an Identification is optional. What
    // differs is the tree. SysML names the declaration as a production and KerML writes it
    // inline, so a .sysml dependency has a DependencyDeclaration child and a .kerml one
    // owns the same tokens directly (ADR-0015).
    //
    // The metaclass is Dependency, a Relationship whose client and supplier are each
    // 1..* (KerML 8.3.2.2.2, receipt ec1e3424) — which the grammar's two non-empty lists
    // already say. The references are [QualifiedName], resolved by sv2-resolve.
    //
    // The body is RelationshipBody, which `relationship_body` reads as the file's language
    // states it: annotations only in SysML (8.2.2.2), and owned related elements as well
    // in KerML (8.2.3.1).
    pub(super) fn dependency(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Dependency);
        while self.at(SyntaxKind::Hash) {
            self.prefix_metadata_annotation();
        }
        self.expect_keyword("dependency");
        match self.language {
            Language::SysMl => self.dependency_declaration(),
            Language::KerMl => self.dependency_declaration_parts(),
        }
        self.relationship_body();
        self.finish_node();
    }

    // production: DependencyDeclaration@sysml
    //
    // DependencyDeclaration =
    //     ( Identification 'from' )?
    //     client += [QualifiedName] ( ',' client += [QualifiedName] )* 'to'
    //     supplier += [QualifiedName] ( ',' supplier += [QualifiedName] )*
    //                                                               (SysML 8.2.2.3)
    //
    // Specification-only: the Pilot inlines it into Dependency, and deviations.json
    // records follow_spec for it.
    fn dependency_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::DependencyDeclaration);
        self.dependency_declaration_parts();
        self.finish_node();
    }

    /// The parts of a dependency's declaration, which `SysML` wraps in a
    /// `DependencyDeclaration` and `KerML` writes inline.
    ///
    /// Whether the `( Identification 'from' )?` group is taken is decided before it: a
    /// short name's `<`, a `from` with no name before it, or a NAME followed by `from`.
    /// Otherwise the first name is a client — "if no short name or name is given for the
    /// dependency, then the keyword from may be omitted" (7.3.2, receipt 65989bd2), so
    /// `dependency z to x;` names nothing and `z` is what depends. A NAME followed by
    /// anything else is not an Identification, which is what reports `dependency Use a
    /// to b;` rather than reading `Use` as a name whose `from` was forgotten.
    fn dependency_declaration_parts(&mut self) {
        if self.at(SyntaxKind::Lt)
            || self.at_keyword("from")
            || (self.at_name() && self.nth_is_keyword(1, "from"))
        {
            self.identification();
            self.expect_keyword("from");
        }
        self.qualified_name_list();
        self.expect_keyword("to");
        self.qualified_name_list();
    }

    /// Whether an `EnumerationDefinition` starts at the `n`th meaningful token.
    ///
    /// `enum def`, with only prefix metadata looked past before it. The production opens
    /// on `DefinitionExtensionKeyword*`, not a `DefinitionPrefix` (`SysML` 8.2.2.8), so
    /// `abstract enum def` is no enumeration definition -- "the keywords abstract and
    /// variation may not be used with an enumeration definition" (7.8.2).
    fn at_enumeration_definition(&self, n: usize) -> bool {
        let n = self.skip_prefix_metadata(n);
        self.nth_is_keyword(n, "enum") && self.nth_is_keyword(n + 1, "def")
    }

    // production: EnumerationDefinition@sysml
    //
    // EnumerationDefinition =
    //     DefinitionExtensionKeyword* 'enum' 'def'
    //     DefinitionDeclaration EnumerationBody                     (SysML 8.2.2.8)
    //
    // examples/Simple Tests/MetadataTest.sysml writes the extension keyword:
    // `#Security enum def ClassificationLevel :> ScalarValues::Natural {`.
    //
    // The metaclass is EnumerationDefinition (8.3.8.2, receipt 224a4a2e), an
    // AttributeDefinition "all of whose instances are given by an explicit list of
    // enumeratedValues".
    //
    // constraint: EnumerationDefinition::validateEnumerationDefinitionIsVariation,
    //     `isVariation` (8.3.8.2): "an EnumerationDefinition is also required to have
    //     isVariation = true, and its enumeratedValues are then just its variants" (8.4.4,
    //     receipt ca82a1f5). An attribute of the element, set by sv2-hir; the grammar
    //     already gives no place to write `variation`, which is the textual half of it.
    fn enumeration_definition(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::EnumerationDefinition);
        self.extension_keywords(SyntaxKind::DefinitionExtensionKeyword);
        self.expect_keyword("enum");
        self.expect_keyword("def");
        self.definition_declaration();
        self.enumeration_body();
        self.finish_node();
    }

    // production: EnumerationBody@sysml
    //
    // EnumerationBody : EnumerationDefinition =
    //     ';'
    //   | '{' ( ownedRelationship += AnnotatingMember
    //         | ownedRelationship += EnumerationUsageMember )*
    //     '}'                                                        (SysML 8.2.2.8)
    //
    // Its own item loop, not `body_elements`: "any owned members declared in the body of
    // an enumeration definition must be enumeration usages" (7.8.2, receipt a2406cd5), so
    // a part or an attribute is no item here, where every other definition body admits
    // one. Anything else is recovered over and reported, and a pass that consumes nothing
    // takes one token, as `body_elements` does (invariant 3).
    fn enumeration_body(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::EnumerationBody);
        if self.at(SyntaxKind::Semicolon) {
            self.bump();
        } else if self.at(SyntaxKind::LBrace) {
            self.bump();
            self.depth += 1;
            self.enumeration_body_items();
            self.depth -= 1;
            self.expect(SyntaxKind::RBrace, "`}`");
        } else {
            self.error_expected("`;` or `{` after an enumeration definition declaration");
        }
        self.finish_node();
    }

    /// The items of an `EnumerationBody`, up to its `}` or end of input.
    fn enumeration_body_items(&mut self) {
        while self.at_bare_comment_member() || (!self.at_end() && !self.at(SyntaxKind::RBrace)) {
            let start = self.pos;
            let n = usize::from(self.at_visibility());
            if self.at_bare_comment_member() {
                self.with_significant_comments(Self::annotating_member);
            } else if self.at_annotating_member(n) {
                self.annotating_member();
            } else if self.at_enumerated_value(n) {
                self.enumeration_usage_member();
            } else {
                self.recover_statement();
            }
            if self.pos == start {
                self.error_token();
            }
        }
    }

    /// Whether an `EnumeratedValue` starts at the `n`th meaningful token.
    ///
    /// `'enum'? Usage`, where `Usage = UsageDeclaration UsageCompletion` and every part of
    /// the declaration is optional (`SysML` 8.2.2.6.2). So a value opens on a name, a short
    /// name's `<` or a feature specialization -- or, with no declaration at all, on its
    /// completion: a `ValuePart`'s `=`, `:=` or `default`, or the `UsageBody`'s `;` or `{`.
    /// examples/Simple Tests/EnumerationTest.sysml:48-50 writes `= 60.0;` as a value.
    /// With the keyword, anything may follow that `Usage` may -- except `def`, which makes
    /// it a nested enumeration definition, and that is no item of an enumeration body.
    /// Without it, a keyword other than `default` is not claimed, so `in a;` and `part p;`
    /// are reported. Prefix metadata before either form is looked past, by deviation
    /// `EnumeratedValue`.
    fn at_enumerated_value(&self, n: usize) -> bool {
        let n = self.skip_prefix_metadata(n);
        if self.nth_is_keyword(n, "enum") {
            return !self.nth_is_keyword(n + 1, "def");
        }
        self.nth_is_name(n)
            || self.nth_is(n, SyntaxKind::Lt)
            || self.nth_at_feature_specialization(n)
            || self.nth_is(n, SyntaxKind::Eq)
            || self.nth_is(n, SyntaxKind::ColonEq)
            || self.nth_is_keyword(n, "default")
            || self.nth_is(n, SyntaxKind::Semicolon)
            || self.nth_is(n, SyntaxKind::LBrace)
    }

    // production: EnumerationUsageMember@sysml
    //
    // EnumerationUsageMember : VariantMembership =
    //     MemberPrefix ownedRelatedElement += EnumeratedValue           (SysML 8.2.2.8)
    //
    // A VariantMembership: the enumerated values are the definition's variants (8.4.4).
    // Marked although EnumeratedValue is not, as other members are over a production with
    // a gap of its own: this production's two parts are read.
    fn enumeration_usage_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::EnumerationUsageMember);
        self.member_prefix();
        self.enumerated_value();
        self.finish_node();
    }

    // production: EnumeratedValue@sysml
    //
    // EnumeratedValue : EnumerationUsage =
    //     UsageExtensionKeyword* 'enum'? Usage       (SysML 8.2.2.8, with the deviation)
    //
    // The clause writes `'enum'? Usage`; deviations.json records follow_xtext for
    // EnumeratedValue, adding the leading UsageExtensionKeyword* every other usage carries,
    // on the corpus's own `#Security enum secret ...` (examples/Simple
    // Tests/MetadataTest.sysml:9). The keywords are that departure, so they carry its note.
    //
    // "The declaration of an enumerated value may omit the enum keyword" (7.8.2).
    fn enumerated_value(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::EnumeratedValue);
        if self.at(SyntaxKind::Hash) {
            // deviation: EnumeratedValue
            self.note_deviation("EnumeratedValue", "prefix metadata on an enumerated value");
            self.extension_keywords(SyntaxKind::UsageExtensionKeyword);
        }
        self.eat_optional_keyword("enum");
        self.usage();
        self.finish_node();
    }

    // production: PartDefinition
    // production: AttributeDefinition
    // production: OccurrenceDefinition
    // production: ItemDefinition
    // production: ConnectionDefinition
    // production: FlowDefinition
    // production: AllocationDefinition
    // production: RenderingDefinition
    //
    // AttributeDefinition  = DefinitionPrefix           'attribute'  'def' Definition
    // OccurrenceDefinition = OccurrenceDefinitionPrefix 'occurrence' 'def' Definition
    // ItemDefinition       = OccurrenceDefinitionPrefix 'item'       'def' Definition
    // PartDefinition       = OccurrenceDefinitionPrefix 'part'       'def' Definition
    // ConnectionDefinition = OccurrenceDefinitionPrefix 'connection' 'def' Definition
    // FlowDefinition       = OccurrenceDefinitionPrefix 'flow'       'def' Definition
    // AllocationDefinition = OccurrenceDefinitionPrefix 'allocation' 'def' Definition
    // RenderingDefinition  = OccurrenceDefinitionPrefix 'rendering'  'def' Definition
    //                                          (SysML 8.2.2.7, .9.1, .10, .11, .13,
    //                                                 .15, .16, .26.3)
    //
    // Eight productions, one method, as the seven usages share `simple_usage` and the
    // eight classifiers share `classifier`. Each is marked because each IS fully
    // implemented; what none of them implements lives in the prefixes and in
    // DefinitionDeclaration, and is recorded there.
    //
    // The Pilot factors 'part' 'def' into PartDefKeyword; deviations.json records
    // that as xtext_only/follow_spec, so the literals are matched here directly.
    fn simple_definition(&mut self, definition: SimpleDefinition) {
        self.eat_trivia();
        self.start_node(definition.node);
        if definition.is_occurrence {
            self.occurrence_definition_prefix();
        } else {
            self.definition_prefix();
        }
        self.expect_keyword(definition.keyword);
        self.expect_keyword("def");
        self.definition();
        self.finish_node();
    }

    // production: DefinitionPrefix@sysml
    //
    // DefinitionPrefix : Definition =
    //     BasicDefinitionPrefix? DefinitionExtensionKeyword*      (SysML 8.2.2.6.1)
    //
    // This is OccurrenceDefinitionPrefix without the `individual` part. The two are
    // separate productions because only an occurrence may be individual, and keeping
    // them separate is what makes `individual attribute def A;` an error.
    pub(super) fn definition_prefix(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::DefinitionPrefix);
        if self.at_keyword("abstract") || self.at_keyword("variation") {
            self.basic_definition_prefix();
        }
        self.extension_keywords(SyntaxKind::DefinitionExtensionKeyword);
        self.finish_node();
    }

    // production: OccurrenceDefinitionPrefix
    //
    // OccurrenceDefinitionPrefix : OccurrenceDefinition =
    //     BasicDefinitionPrefix?
    //     ( isIndividual ?= 'individual' ownedRelationship += EmptyMultiplicityMember )?
    //     DefinitionExtensionKeyword*                            (SysML 8.2.2.9.1)
    //
    // The node is built even when every slot is empty, as MemberPrefix's is.
    pub(super) fn occurrence_definition_prefix(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::OccurrenceDefinitionPrefix);
        if self.at_keyword("abstract") || self.at_keyword("variation") {
            self.basic_definition_prefix();
        }
        if self.at_keyword("individual") {
            self.bump_as(keyword("individual").unwrap_or(SyntaxKind::BasicName));
            self.empty_multiplicity_member();
        }
        self.extension_keywords(SyntaxKind::DefinitionExtensionKeyword);
        self.finish_node();
    }

    // production: BasicDefinitionPrefix
    //
    // BasicDefinitionPrefix = isAbstract ?= 'abstract' | isVariation ?= 'variation'
    //                                                            (SysML 8.2.2.6.1)
    fn basic_definition_prefix(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::BasicDefinitionPrefix);
        match ["abstract", "variation"]
            .iter()
            .find(|word| self.at_keyword(word))
        {
            Some(word) => self.bump_as(keyword(word).unwrap_or(SyntaxKind::BasicName)),
            None => self.error_expected("`abstract` or `variation`"),
        }
        self.finish_node();
    }

    // production: Definition
    //
    // Definition = DefinitionDeclaration DefinitionBody          (SysML 8.2.2.6.1)
    pub(super) fn definition(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::Definition);
        self.definition_declaration();
        self.definition_body();
        self.finish_node();
    }

    // production: DefinitionDeclaration
    //
    // DefinitionDeclaration : Definition = Identification SubclassificationPart?
    //                                                            (SysML 8.2.2.6.1)
    pub(super) fn definition_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::DefinitionDeclaration);
        self.identification();
        if self.at(SyntaxKind::ColonGt) || self.at_keyword("specializes") {
            self.subclassification_part();
        }
        self.finish_node();
    }

    // production: SubclassificationPart
    //
    // SubclassificationPart : Classifier =
    //     SPECIALIZES ownedRelationship += OwnedSubclassification
    //     ( ',' ownedRelationship += OwnedSubclassification )*   (SysML 8.2.2.6.5)
    //
    // SPECIALIZES = ':>' | 'specializes'                         (KerML 8.2.2.7)
    fn subclassification_part(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::SubclassificationPart);
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

    // production: OwnedSubclassification
    //
    // OwnedSubclassification : Subclassification = superClassifier = [QualifiedName]
    //                                                            (SysML 8.2.2.6.5)
    pub(super) fn owned_subclassification(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::OwnedSubclassification);
        self.qualified_name();
        self.finish_node();
    }

    // production: DefinitionBody
    //
    // DefinitionBody : Type = ';' | '{' DefinitionBodyItem* '}'  (SysML 8.2.2.6.1)
    //
    // DefinitionBodyItem is marked at `body_element`, which reads all six of its
    // alternatives under Body::Definition.
    pub(super) fn definition_body(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::DefinitionBody);
        if self.at(SyntaxKind::Semicolon) {
            self.bump();
        } else if self.at(SyntaxKind::LBrace) {
            self.bump();
            self.depth += 1;
            self.body_elements(Some(SyntaxKind::RBrace), Body::Definition);
            self.depth -= 1;
            self.expect(SyntaxKind::RBrace, "`}`");
        } else {
            self.error_expected("`;` or `{` after a definition declaration");
        }
        self.finish_node();
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::grammar::Language;
    use crate::parser::Parser;
    use crate::parser::namespace::KEYWORD_MEMBERS;

    /// Every `.sysml` and `.kerml` file under `dir`, found without recursing in Rust.
    fn model_files(dir: PathBuf) -> Vec<PathBuf> {
        let mut found = Vec::new();
        let mut stack = vec![dir];
        while let Some(dir) = stack.pop() {
            let (dirs, files): (Vec<PathBuf>, Vec<PathBuf>) = std::fs::read_dir(&dir)
                .into_iter()
                .flatten()
                .flatten()
                .map(|entry| entry.path())
                .partition(|path| path.is_dir());
            stack.extend(dirs);
            found.extend(files.into_iter().filter(|path| {
                path.extension()
                    .is_some_and(|e| e == "sysml" || e == "kerml")
            }));
        }
        found
    }

    /// Every model file under the repository's `dir`, with its text and its language.
    fn sources_under(dir: &str) -> Vec<(String, String, Language)> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(dir);
        model_files(root)
            .into_iter()
            .filter_map(|path| {
                let source = std::fs::read_to_string(&path).ok()?;
                let language = Language::from_path(&path)?;
                Some((path.display().to_string(), source, language))
            })
            .collect()
    }

    /// How many times `at_definition_element` accepts in `source`, asserting at each
    /// that `opens_definition` admits the head.
    fn definitions_admitted(name: &str, source: &str, language: Language) -> usize {
        let mut parser = Parser::new(source, language);
        let mut accepted = 0;
        for significant in [false, true] {
            parser.comments_significant = significant;
            let positions = if significant {
                parser.meaningful_with_comments.len()
            } else {
                parser.meaningful.len()
            };
            let at = (0..=positions).filter(|&n| parser.at_definition_element(n));
            for n in at.collect::<Vec<_>>() {
                accepted += 1;
                assert!(
                    Parser::opens_definition(parser.member_head(n)),
                    "{name}: a definition at meaningful position {n} (comments \
                     significant: {significant}) has a head `opens_definition` turns away"
                );
            }
        }
        accepted
    }

    /// Forms the corpus may not write: every prefix in every order, each extension form,
    /// and a `#` that names nothing, which `skip_prefix_metadata` does not look past.
    const FORMS: &[&str] = &[
        "abstract #M individual #N def D;",
        "variation #a::b.c part def P;",
        "individual #M def I;",
        "#M #N def E;",
        "abstract #M metadata def M;",
        "standard library package L;",
        "library package L;",
        "#M dependency a to b;",
        "#M package P;",
        "abstract use case def U;",
        "individual verification def V;",
        "# def X;",
        "abstract # part def X;",
        "#$::M enum def E;",
        // The usage prefixes `member_head` skips: none opens a definition, and the three
        // that decide a usage are admitted by their flags.
        "ref part def P;",
        "snapshot part def P;",
        "in out derived constant ref x : T;",
        "ref x : T;",
        "ref #M x;",
        "#M ref x;",
        "in #M x;",
        "#M x;",
        "individual part i;",
        "snapshot s;",
        "timeslice t : T;",
        "individual #M snapshot x;",
        "end ref a;",
        "end part p;",
        "end x : T part p;",
        "in end part p;",
        "assert not satisfy r;",
        "not satisfy r;",
        "satisfy r by s;",
        "first a then b;",
        "succession s first a then b;",
        "use case u;",
        "derived abstract constant attribute a;",
    ];

    /// Every keyword member's `admits` must hold wherever its recogniser accepts, or the
    /// head would turn that member away in `at_sysml_keyword_member`. Asked as
    /// `definitions_admitted` asks, over the same sources.
    fn keyword_members_admitted(name: &str, source: &str, language: Language) -> usize {
        let mut parser = Parser::new(source, language);
        let mut accepted = 0;
        for significant in [false, true] {
            parser.comments_significant = significant;
            let positions = if significant {
                parser.meaningful_with_comments.len()
            } else {
                parser.meaningful.len()
            };
            let pairs = (0..=positions).flat_map(|n| KEYWORD_MEMBERS.map(|member| (n, member)));
            let at = pairs.filter(|&(n, member)| parser.at_keyword_member(member, n));
            for (n, member) in at.collect::<Vec<_>>() {
                accepted += 1;
                assert!(
                    member.admits(parser.member_head(n)),
                    "{name}: {member:?} accepts at meaningful position {n} (comments \
                     significant: {significant}) where its head does not admit it"
                );
            }
        }
        accepted
    }

    #[test]
    fn every_keyword_member_is_admitted_where_it_accepts() {
        let mut sources: Vec<(String, String, Language)> = FORMS
            .iter()
            .map(|form| ((*form).to_owned(), (*form).to_owned(), Language::SysMl))
            .collect();
        sources.extend(sources_under("vendor/corpus"));
        sources.extend(sources_under("tests/rejection"));
        let accepted: usize = sources
            .iter()
            .map(|(name, source, language)| keyword_members_admitted(name, source, *language))
            .sum();
        println!("{} sources, {accepted} acceptances", sources.len());
        // Inert without the corpus, but the forms alone accept dozens of times.
        assert!(accepted >= 30, "only {accepted} acceptances seen");
    }

    /// `opens_definition` must hold wherever `at_definition_element` accepts, or the
    /// gate in `at_sysml_keyword_member` would turn a definition away.
    ///
    /// Asked at every meaningful position of every corpus file, every rejection case and
    /// every form above, in both comment modes, which is every place the recognisers can
    /// be asked from. The implication is what makes the gate exact: where the head opens
    /// no definition, `at_definition_element` was always going to answer `false`.
    #[test]
    fn definition_element_heads_cover_every_definition() {
        let mut sources: Vec<(String, String, Language)> = FORMS
            .iter()
            .map(|form| ((*form).to_owned(), (*form).to_owned(), Language::SysMl))
            .collect();
        let corpus = sources_under("vendor/corpus");
        let rejection = sources_under("tests/rejection");
        let files = corpus.len() + rejection.len();
        sources.extend(corpus);
        sources.extend(rejection);
        let accepted: usize = sources
            .iter()
            .map(|(name, source, language)| definitions_admitted(name, source, *language))
            .sum();
        println!("{files} files, {accepted} definitions");
        // Inert without the corpus, but the forms alone accept eleven times.
        assert!(accepted >= 11, "only {accepted} definitions seen");
    }
}
