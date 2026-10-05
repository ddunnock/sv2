// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Metadata, `SysML` 8.2.2.27 and `KerML` 8.2.5.13: definitions, usages, prefix metadata,
//! and metadata bodies.

use crate::counter::{Counter, count};
use crate::generated::kinds::SyntaxKind;
use crate::grammar::Language;
use crate::parser::body::Body;
use crate::parser::lookahead::keyword;
use crate::parser::{MAX_DEPTH, Parser};

impl Parser<'_> {
    /// The index just past a run of `#` prefix metadata written from the `n`th token, or
    /// `n` itself when none is.
    ///
    /// Each is `'#' PrefixMetadataUsage` with `PrefixMetadataUsage = OwnedFeatureTyping`
    /// (`SysML` 8.2.2.27): a `QualifiedName`, or an `OwnedFeatureChain` of them by
    /// deviation `PrefixMetadataTyping-chain` (`follow_spec`), walked as `chainable_target`
    /// reads it. The one shape stands behind `PrefixMetadataMember`,
    /// `PrefixMetadataAnnotation` and both extension keywords, so every prefix recogniser
    /// looks past it here and nowhere else. A `#` with no name after it is not looked
    /// past, and the member it would have opened is reported.
    ///
    /// Both languages. `KerML`'s `#` is `'#' PrefixMetadataFeature` with
    /// `PrefixMetadataFeature = OwnedFeatureTyping` and `OwnedFeatureTyping = GeneralType`,
    /// a `QualifiedName` or an `OwnedFeatureChain` (`KerML` 8.2.5.12, 8.2.4.3.2, 8.2.4.1.2):
    /// the same text over a different element, so the same walk.
    pub(super) fn skip_prefix_metadata(&self, n: usize) -> usize {
        count(Counter::SkipPrefixMetadata);
        // Every prefix recogniser asks this, many times from one start (roadmap Phase 4),
        // and the answer depends only on the start and the comment mode.
        self.memoized(&self.prefix_metadata_ends, n, |parser, n| {
            count(Counter::SkipPrefixMetadataComputed);
            let mut n = n;
            while let Some(k) = parser.skip_one_prefix_metadata(n) {
                n = k;
            }
            n
        })
    }

    /// The index just past ONE `#` prefix metadata written at the `n`th token, if one is:
    /// see `skip_prefix_metadata`.
    pub(super) fn skip_one_prefix_metadata(&self, n: usize) -> Option<usize> {
        if !self.nth_is(n, SyntaxKind::Hash) {
            return None;
        }
        let mut k = self.skip_qualified_name(n + 1)?;
        while self.nth_is(k, SyntaxKind::Dot) {
            match self.skip_qualified_name(k + 1) {
                Some(next) => k = next,
                None => break,
            }
        }
        Some(k)
    }

    // production: PrefixMetadataMember@sysml
    //
    // PrefixMetadataMember : OwningMembership =
    //     '#' ownedRelatedElement = PrefixMetadataUsage            (SysML 8.2.2.27)
    //
    // production: PrefixMetadataMember@kerml
    //
    // PrefixMetadataMember : OwningMembership =
    //     '#' ownedRelatedElement += PrefixMetadataFeature         (KerML 8.2.5.12)
    //
    // One method, two markers: the text is the same `#X` and the element differs, a
    // MetadataUsage in SysML and a MetadataFeature in KerML, so the file's language
    // chooses the child as `owned_multiplicity` chooses its range.
    //
    // production: PrefixMetadataUsage@sysml
    //
    // PrefixMetadataUsage : MetadataUsage =
    //     ownedRelationship += OwnedFeatureTyping                  (SysML 8.2.2.27)
    //
    // "A user-defined keyword is the (possibly qualified) name (or short name) of a
    // metadata definition (or KerML metaclass) preceded by the symbol #. ... [It] specifies
    // a metadata annotation of the declared element" (7.27.4, receipt 0f2c5bd1). So the
    // `#X` is a MetadataUsage typed by X and owned by the declared element; that it
    // annotates its owner is a derivation, not text. The typing is the whole
    // OwnedFeatureTyping, chain included (deviation PrefixMetadataTyping-chain,
    // follow_spec): that X names a metaclass is validateMetadataFeatureMetaclass, not
    // grammar (ADR-0002).
    //
    // implied specialization: when X specializes SemanticMetadata, the declared element
    //     implicitly specializes its baseType (7.27.3, receipt 938f2744). Resolution's.
    // constraint: MetadataUsage::checkMetadataUsageSpecialization (8.3.27.3). sv2-hir's.
    pub(super) fn prefix_metadata_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::PrefixMetadataMember);
        self.expect(SyntaxKind::Hash, "`#`");
        self.prefix_metadata_element();
        self.finish_node();
    }

    /// The element after a prefix `#`, in this file's language: `PrefixMetadataUsage` in
    /// `SysML` (8.2.2.27), `PrefixMetadataFeature` in `KerML` (8.2.5.12).
    fn prefix_metadata_element(&mut self) {
        match self.language {
            Language::SysMl => self.prefix_metadata_usage(),
            Language::KerMl => self.prefix_metadata_feature(),
        }
    }

    // production: PrefixMetadataFeature@kerml
    //
    // PrefixMetadataFeature : MetadataFeature =
    //     ownedRelationship += OwnedFeatureTyping                  (KerML 8.2.5.12)
    //
    // "A user-defined keyword is a (possibly qualified) metaclass name or short name
    // preceded by the symbol #. [It] is placed immediately before the language-defined
    // (reserved) keyword for the declaration and specifies a metadata feature annotation
    // of the declared element" (7.4.13, receipt 5e755297). The metaclass is
    // MetadataFeature (8.3.4.12.3, receipt 2c4eda9f).
    //
    // KerML's OwnedFeatureTyping is `GeneralType` (8.2.4.3.2), `[QualifiedName] |
    // OwnedFeatureChain` (8.2.4.1.2): the text `owned_feature_typing` reads, and the node it
    // builds. That the type is a metaclass is validateMetadataFeatureMetaclass, not
    // grammar, as deviation PrefixMetadataTyping-chain (follow_spec) records for SysML.
    //
    // implied specialization: Metaobjects::metaobjects (checkMetadataFeatureSpecialization),
    //     and for SemanticMetadata the annotated type's specialization of its baseType
    //     (checkMetadataFeatureSemanticSpecialization, 8.4.4.13.3, receipt 50b0d5ac).
    //     Injections, sv2-hir's; this layer builds the tree only (ADR-0002).
    fn prefix_metadata_feature(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::PrefixMetadataFeature);
        self.owned_feature_typing();
        self.finish_node();
    }

    fn prefix_metadata_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::PrefixMetadataUsage);
        self.owned_feature_typing();
        self.finish_node();
    }

    /// Whether a `MetadataDefinition` starts at the `n`th meaningful token.
    ///
    /// `'abstract'? DefinitionExtensionKeyword* 'metadata' 'def'` (`SysML` 8.2.2.27): its
    /// own prefix, `abstract` alone and not `DefinitionPrefix`'s `variation`, so it is not
    /// on the `SIMPLE_DEFINITIONS` spine.
    pub(super) fn at_metadata_definition(&self, n: usize) -> bool {
        let after = self.skip_prefix_metadata(n + usize::from(self.nth_is_keyword(n, "abstract")));
        self.nth_is_keyword(after, "metadata") && self.nth_is_keyword(after + 1, "def")
    }

    /// Whether `AnnotatingElement`'s fourth alternative starts at the `n`th meaningful
    /// token, in this file's language.
    ///
    /// `SysML`'s `MetadataUsage` opens `UsageExtensionKeyword* ( '@' | 'metadata' )`
    /// (8.2.2.27), reached by deviation `AnnotatingElement`; `KerML`'s `MetadataFeature`
    /// opens `PrefixMetadataMember* ( '@' | 'metadata' )` (8.2.5.12). The same text, a
    /// `#X` run then the symbol or the word, so one recogniser serves both, and
    /// `metadata_annotating_element` builds the file's language's element. In `SysML`,
    /// `metadata def` is `MetadataDefinition` beside it, and `#X metadata def` too. In
    /// `KerML` it is not: `KerML` has no `MetadataDefinition` and does not reserve `def`
    /// (8.2.2.6), so `metadata def : T;` is a `MetadataFeature` named `def`, by
    /// `MetadataFeatureDeclaration = ( Identification ( ':' | 'typed' 'by' ) )?
    /// OwnedFeatureTyping` (8.2.5.12).
    ///
    /// `@` also opens an expression: a `ClassificationExpression` with no left operand
    /// (`KerML` 8.2.5.8.1). The two never meet at member position, but they do where a
    /// calculation body's items end in its result expression; `at_result_expression`
    /// settles that one.
    pub(super) fn at_metadata_element(&self, n: usize) -> bool {
        let n = self.skip_prefix_metadata(n);
        self.nth_is(n, SyntaxKind::At)
            || (self.nth_is_keyword(n, "metadata")
                && !(self.language == Language::SysMl && self.nth_is_keyword(n + 1, "def")))
    }

    /// Whether a metadata element opens here with comments significant, so that a
    /// regular comment before it is seen as the `Comment` it is rather than looked past.
    pub(super) fn at_metadata_element_significantly(&mut self) -> bool {
        let outer = self.comments_significant;
        self.comments_significant = true;
        let metadata = self.at_metadata_element(0);
        self.comments_significant = outer;
        metadata
    }

    // production: PrefixMetadataAnnotation@sysml
    //
    // PrefixMetadataAnnotation : Annotation =
    //     '#' annotatingElement = PrefixMetadataUsage
    //     { ownedRelatedElement += annotatingElement }            (SysML 8.2.2.27)
    //
    // production: PrefixMetadataAnnotation@kerml
    //
    // PrefixMetadataAnnotation : Annotation =
    //     '#' ownedRelatedElement += PrefixMetadataFeature         (KerML 8.2.5.12)
    //
    // An Annotation where Package's PrefixMetadataMember is an OwningMembership: the
    // Dependency is a Relationship (KerML 8.3.2.2.2, receipt ec1e3424), and the clause
    // owns the metadata through the annotation's ownedRelatedElement. The text is the same
    // `#X`, read by the same element as PrefixMetadataMember's in each language.
    pub(super) fn prefix_metadata_annotation(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::PrefixMetadataAnnotation);
        self.expect(SyntaxKind::Hash, "`#`");
        self.prefix_metadata_element();
        self.finish_node();
    }

    // production: MetadataDefinition@sysml
    //
    // MetadataDefinition = ( isAbstract ?= 'abstract' )? DefinitionExtensionKeyword*
    //                      'metadata' 'def' Definition                (SysML 8.2.2.27)
    //
    // "A metadata definition is declared like an item definition ..., but using the keyword
    // metadata def" (7.27.2, receipt 66e5d6b3). The prefix is its own and narrower than
    // DefinitionPrefix: `abstract` only, never `variation`, so `variation metadata def M;`
    // is reported. `abstract` is a keyword token of this node, as the clause writes it
    // inline, and there is no DefinitionPrefix node. The metaclass is MetadataDefinition
    // (8.3.27.2, receipt 29a5d69e), an ItemDefinition that is also a KerML Metaclass.
    // The DefinitionExtensionKeyword* is written in this production's own body, not in a
    // prefix production, so it is read here.
    //
    // implied specialization: Metadata::MetadataItem
    // constraint: MetadataDefinition::checkMetadataDefinitionSpecialization
    //     (8.3.27.2). An injection, so sv2-hir's (ADR-0002).
    pub(super) fn metadata_definition(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::MetadataDefinition);
        self.eat_optional_keyword("abstract");
        self.extension_keywords(SyntaxKind::DefinitionExtensionKeyword);
        self.expect_keyword("metadata");
        self.expect_keyword("def");
        self.definition();
        self.finish_node();
    }

    // production: MetadataUsage@sysml
    //
    // MetadataUsage : MetadataUsage =
    //     UsageExtensionKeyword* ( '@' | 'metadata' ) MetadataUsageDeclaration
    //     ( 'about' ownedRelationship += Annotation
    //       ( ',' ownedRelationship += Annotation )* )?
    //     MetadataBody                                           (SysML 8.2.2.27)
    //
    // Reached only as AnnotatingElement's fourth
    // alternative, by deviation AnnotatingElement, which adds no text and so
    // carries no note (see `metadata_annotating_element`). "A metadata usage
    // is declared like an item usage ... using the keyword metadata (or the symbol @)"
    // (7.27.2, receipt 66e5d6b3). The metaclass is MetadataUsage (8.3.27.3, receipt
    // 45313c9a), an ItemUsage that is also a KerML MetadataFeature.
    //
    // "If there is no declared name or short name, then the keyword defined by (or the
    // symbol :) may also be omitted" (7.27.2), and a usage with no `about` annotates "the
    // containing namespace" -- a derivation over the owner, not text.
    //
    // implied specialization: Metadata::metadataItems
    // constraint: MetadataUsage::checkMetadataUsageSpecialization (8.3.27.3). sv2-hir's.
    pub(super) fn metadata_usage(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::MetadataUsage);
        self.extension_keywords(SyntaxKind::UsageExtensionKeyword);
        if self.at(SyntaxKind::At) {
            self.bump();
        } else {
            self.expect_keyword("metadata");
        }
        self.metadata_usage_declaration();
        if self.at_keyword("about") {
            self.bump_as(keyword("about").unwrap_or(SyntaxKind::BasicName));
            self.annotation();
            while self.at(SyntaxKind::Comma) {
                self.bump();
                self.annotation();
            }
        }
        self.metadata_body();
        self.finish_node();
    }

    // production: MetadataUsageDeclaration
    //
    // MetadataUsageDeclaration : MetadataUsage =
    //     ( Identification ( ':' | 'typed' 'by' ) )?
    //     ownedRelationship += OwnedFeatureTyping                (SysML 8.2.2.27)
    //
    // Read as deviation MetadataUsageDeclaration (follow_xtext) writes it: `defined by`,
    // SysML's spelling, in place of KerML's unadapted `typed by`, as 7.27.2 itself says
    // ("followed by the keyword defined by (or the symbol :)", receipt 66e5d6b3). `typed`
    // is no SysML keyword, so `metadata m typed by T;` is reported. The typing is the
    // whole OwnedFeatureTyping, chain included, per deviation PrefixMetadataTyping-chain
    // (follow_spec); that it names a metaclass is validateMetadataFeatureMetaclass, not
    // grammar.
    //
    // The optional group is decided by looking past an Identification for the `:` or
    // `defined` after it; without one, what is here is the typing alone. Identification
    // may be empty, so `metadata : T;` takes the group with nothing before the `:`.
    fn metadata_usage_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::MetadataUsageDeclaration);
        if self.at_metadata_identification() {
            self.identification();
            if self.at_keyword("defined") {
                // deviation: MetadataUsageDeclaration
                self.note_deviation(
                    "MetadataUsageDeclaration",
                    "`defined by` in a metadata usage declaration",
                );
                self.bump_as(keyword("defined").unwrap_or(SyntaxKind::BasicName));
                self.expect_keyword("by");
            } else {
                self.expect(SyntaxKind::Colon, "`:` or `defined by`");
            }
        }
        self.owned_feature_typing();
        self.finish_node();
    }

    /// Whether `MetadataUsageDeclaration`'s optional `Identification ( ':' | 'defined'
    /// 'by' )` group is written here: an optional `<short name>` and name, then `:` or
    /// `defined by`.
    ///
    /// `KerML`'s `MetadataFeatureDeclaration` writes `typed by` there (8.2.5.12), so the
    /// word asked for is the file's language's.
    fn at_metadata_identification(&self) -> bool {
        let mut n = 0;
        if self.nth_is(n, SyntaxKind::Lt) {
            n += 3;
        }
        if self.peek_nth(n).is_some_and(|token| self.is_name(token)) {
            n += 1;
        }
        let word = match self.language {
            Language::SysMl => "defined",
            Language::KerMl => "typed",
        };
        self.nth_is(n, SyntaxKind::Colon)
            || (self.nth_is_keyword(n, word) && self.nth_is_keyword(n + 1, "by"))
    }

    // production: MetadataBody@sysml
    //
    // MetadataBody : Type =
    //     ';'
    //   | '{' ( ownedRelationship += DefinitionMember
    //         | ownedRelationship += MetadataBodyUsageMember
    //         | ownedRelationship += AliasMember
    //         | ownedRelationship += Import )*
    //     '}'                                                    (SysML 8.2.2.27)
    //
    // Its own loop rather than `body_elements`, because its item set is its own: a
    // DefinitionMember but NO usage member, and a keywordless redefinition that no other
    // body has. A DefinitionElement includes AnnotatingElement, so a `doc` or a nested
    // `@M;` is a DefinitionMember here; `attribute a;` is reported. Keywordless
    // redefinitions are asked last, since a definition opens on a keyword and a
    // keyword is not a name (8.2.2.1.2).
    fn metadata_body(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::MetadataBody);
        if self.at(SyntaxKind::Semicolon) {
            self.bump();
        } else if self.at(SyntaxKind::LBrace) {
            self.bump();
            self.depth += 1;
            self.metadata_body_items();
            self.depth -= 1;
            self.expect(SyntaxKind::RBrace, "`}`");
        } else {
            self.error_expected("`;` or `{` after a metadata usage declaration");
        }
        self.finish_node();
    }

    /// The items of a braced `MetadataBody`, up to its `}` or end of input.
    fn metadata_body_items(&mut self) {
        while self.at_bare_comment_member() || (!self.at_end() && !self.at(SyntaxKind::RBrace)) {
            let start = self.pos;
            let n = usize::from(self.at_visibility());
            if self.depth >= MAX_DEPTH {
                self.report_too_deep();
                self.error_token();
            } else if self.at_bare_comment_member() {
                self.bare_comment_member(Body::Definition);
            } else if self.at_import() {
                self.import();
            } else if self.at_element_keyword("alias") {
                self.alias_member();
            } else if self.at_annotating_member(n)
                || self.nth_is_keyword(n, "package")
                || self.at_definition_element(n)
            {
                // Body::Definition builds a DefinitionMember for what is not a usage,
                // and the recogniser above admits no usage.
                self.membership(Body::Definition);
            } else if self.at_metadata_body_usage() {
                self.metadata_body_usage_member();
            } else {
                self.recover_statement();
            }
            if self.pos == start {
                self.error_token();
            }
        }
    }

    /// Whether a `MetadataBodyUsage` starts here: `'ref'? ( ':>>' | 'redefines' )?` and
    /// then the name its `OwnedRedefinition` opens on (`SysML` 8.2.2.27).
    fn at_metadata_body_usage(&self) -> bool {
        let mut n = usize::from(self.nth_is_keyword(0, "ref"));
        if self.nth_is(n, SyntaxKind::ColonGtGt) || self.nth_is_keyword(n, "redefines") {
            n += 1;
        }
        self.peek_nth(n).is_some_and(|token| self.is_name(token))
            || (self.nth_is(n, SyntaxKind::Dollar) && self.nth_is(n + 1, SyntaxKind::ColonColon))
    }

    // production: MetadataBodyUsageMember
    //
    // MetadataBodyUsageMember : FeatureMembership =
    //     ownedMemberFeature = MetadataBodyUsage                 (SysML 8.2.2.27)
    //
    // production: MetadataBodyUsage
    //
    // MetadataBodyUsage : ReferenceUsage =
    //     'ref'? ( ':>>' | 'redefines' )? ownedRelationship += OwnedRedefinition
    //     FeatureSpecializationPart? ValuePart? MetadataBody      (SysML 8.2.2.27)
    //
    // "The keyword ref and/or redefines (or the equivalent symbol :>>) may be omitted in
    // the declaration of a feature of a metadata usage" (7.27.2, receipt 66e5d6b3), so
    // `approved = true;` redefines `approved`. The redefinition is always there; the
    // keywords before it are optional spellings.
    fn metadata_body_usage_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::MetadataBodyUsageMember);
        self.start_node(SyntaxKind::MetadataBodyUsage);
        self.eat_optional_keyword("ref");
        if self.at(SyntaxKind::ColonGtGt) {
            self.bump();
        } else {
            self.eat_optional_keyword("redefines");
        }
        self.owned_redefinition();
        self.optional_feature_specialization_part();
        if self.at_value_part() {
            self.value_part();
        }
        self.metadata_body();
        self.finish_node();
        self.finish_node();
    }

    // production: MetadataFeature@kerml
    //
    // MetadataFeature : MetadataFeature =
    //     ( ownedRelationship += PrefixMetadataMember )*
    //     ( '@' | 'metadata' )
    //     MetadataFeatureDeclaration
    //     ( 'about' ownedRelationship += Annotation
    //       ( ',' ownedRelationship += Annotation )*
    //     )?
    //     MetadataBody                                           (KerML 8.2.5.12)
    //
    // AnnotatingElement's fourth alternative in KerML (8.2.3.3.1), the clause's own. "A
    // metadata feature is declared using the keyword metadata (or the symbol @), optionally
    // followed by a short name and/or name, followed by the keyword typed by (or the
    // symbol :) and the qualified name of exactly one metaclass" (7.4.13, receipt
    // 5e755297). The metaclass is MetadataFeature (8.3.4.12.3, receipt 2c4eda9f). SysML's
    // MetadataUsage is the same text over its own productions; the file's language
    // chooses (`metadata_annotating_element`).
    //
    // With no `about`, the annotated element "is implicitly the containing namespace"
    // (7.4.13): a derivation over the owner, not text.
    //
    // implied specialization: Metaobjects::metaobjects (checkMetadataFeatureSpecialization,
    //     KerML 8.3.4.12.3), as for a PrefixMetadataFeature. sv2-hir's to inject; nothing
    //     is written into the tree.
    // constraint: MetadataFeature::validateMetadataFeatureMetaclass and
    //     validateMetadataFeatureBody (8.3.4.12.3): the typing names a metaclass, and the
    //     body's features redefine its features. Validity, not syntax (ADR-0002).
    pub(super) fn metadata_feature(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::MetadataFeature);
        while self.at(SyntaxKind::Hash) {
            self.prefix_metadata_member();
        }
        if self.at(SyntaxKind::At) {
            self.bump();
        } else {
            self.expect_keyword("metadata");
        }
        self.metadata_feature_declaration();
        if self.at_keyword("about") {
            self.bump_as(keyword("about").unwrap_or(SyntaxKind::BasicName));
            self.annotation();
            while self.at(SyntaxKind::Comma) {
                self.bump();
                self.annotation();
            }
        }
        self.kerml_metadata_body();
        self.finish_node();
    }

    // production: MetadataFeatureDeclaration@kerml
    //
    // MetadataFeatureDeclaration : MetadataFeature =
    //     ( Identification ( ':' | 'typed' 'by' ) )?
    //     ownedRelationship += OwnedFeatureTyping                (KerML 8.2.5.12)
    //
    // SysML's MetadataUsageDeclaration less its deviation: `typed by` is KerML's own
    // TYPED_BY word, so it is read with no note. The group is decided as SysML's is, by
    // looking past an Identification for the `:` or `typed` after it
    // (`at_metadata_identification`); Identification may be empty, so `@ : T;` takes it
    // with nothing before the `:`. The typing reads `owned_feature_typing`, whose text is
    // KerML's OwnedFeatureTyping, `GeneralType` (8.2.4.3.2), as `prefix_metadata_feature`
    // says.
    fn metadata_feature_declaration(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::MetadataFeatureDeclaration);
        if self.at_metadata_identification() {
            self.identification();
            if self.at(SyntaxKind::Colon) {
                self.bump();
            } else {
                self.expect_keyword("typed");
                self.expect_keyword("by");
            }
        }
        self.owned_feature_typing();
        self.finish_node();
    }

    // production: MetadataBody@kerml
    //
    // MetadataBody : Type =
    //     ';' | '{' ( ownedRelationship += MetadataBodyElement )* '}'
    //                                                            (KerML 8.2.5.12)
    //
    // production: MetadataBodyElement@kerml
    //
    // MetadataBodyElement : Membership =
    //       NonFeatureMember
    //     | MetadataBodyFeatureMember
    //     | AliasMember
    //     | Import                                               (KerML 8.2.5.12)
    //
    // The same node as SysML's MetadataBody (8.2.2.27), a production of the same name
    // with a different item set: NonFeatureMember where SysML writes DefinitionMember, and
    // KerML's feature member. MetadataBodyElement is an alternation with no node, as
    // TypeBodyElement is. A NonFeatureMember is any KerML member element but a
    // FeatureElement (`at_kerml_non_feature_element`), an AnnotatingElement included, so
    // `step s;` here is reported: a body's features are its own redefinitions.
    // NonFeatureMembers are asked first, as they open on keywords and a keyword is not a
    // name (8.2.2.6).
    fn kerml_metadata_body(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::MetadataBody);
        if self.at(SyntaxKind::Semicolon) {
            self.bump();
        } else if self.at(SyntaxKind::LBrace) {
            self.bump();
            self.depth += 1;
            self.kerml_metadata_body_elements();
            self.depth -= 1;
            self.expect(SyntaxKind::RBrace, "`}`");
        } else {
            self.error_expected("`;` or `{` after a metadata feature declaration");
        }
        self.finish_node();
    }

    /// The elements of a braced `KerML` `MetadataBody`, up to its `}` or end of input.
    fn kerml_metadata_body_elements(&mut self) {
        while self.at_bare_comment_member() || (!self.at_end() && !self.at(SyntaxKind::RBrace)) {
            let start = self.pos;
            let n = usize::from(self.at_visibility());
            if self.depth >= MAX_DEPTH {
                self.report_too_deep();
                self.error_token();
            } else if self.at_bare_comment_member() {
                self.bare_comment_member(Body::Type);
            } else if self.at_import() {
                self.import();
            } else if self.at_element_keyword("alias") {
                self.alias_member();
            } else if self.at_annotating_member(n) || self.at_kerml_non_feature_element(n) {
                // Body::Type builds a NonFeatureMember in KerML (see `Body::member`).
                self.membership(Body::Type);
            } else if self.at_metadata_body_feature() {
                self.metadata_body_feature_member();
            } else {
                self.recover_statement();
            }
            if self.pos == start {
                self.error_token();
            }
        }
    }

    /// Whether a `MetadataBodyFeature` starts here: `'feature'? ( ':>>' | 'redefines' )?`
    /// and then the name its `OwnedRedefinition` opens on (`KerML` 8.2.5.12).
    fn at_metadata_body_feature(&self) -> bool {
        let mut n = usize::from(self.nth_is_keyword(0, "feature"));
        if self.nth_is(n, SyntaxKind::ColonGtGt) || self.nth_is_keyword(n, "redefines") {
            n += 1;
        }
        self.nth_is_name(n)
    }

    // production: MetadataBodyFeatureMember@kerml
    //
    // MetadataBodyFeatureMember : FeatureMembership =
    //     ownedMemberFeature = MetadataBodyFeature               (KerML 8.2.5.12)
    //
    // production: MetadataBodyFeature@kerml
    //
    // MetadataBodyFeature : Feature =
    //     'feature'? ( ':>>' | 'redefines')? ownedRelationship += OwnedRedefinition
    //     FeatureSpecializationPart? ValuePart?
    //     MetadataBody                                           (KerML 8.2.5.12)
    //
    // "The keywords feature and/or redefines (or the equivalent symbol :>>) may be
    // omitted in the declaration of a metadata feature" (7.4.13, receipt 5e755297), so
    // `approved = true;` redefines `approved`. SysML's MetadataBodyUsage writes `ref`
    // where this writes `feature`. The redefinition reads `owned_redefinition`, whose
    // text is KerML's OwnedRedefinition, `GeneralType` (8.2.4.3.4). The member has no
    // MemberPrefix: `private x = 1;` is reported.
    fn metadata_body_feature_member(&mut self) {
        self.eat_trivia();
        self.start_node(SyntaxKind::MetadataBodyFeatureMember);
        self.start_node(SyntaxKind::MetadataBodyFeature);
        self.eat_optional_keyword("feature");
        if self.at(SyntaxKind::ColonGtGt) {
            self.bump();
        } else {
            self.eat_optional_keyword("redefines");
        }
        self.owned_redefinition();
        self.optional_feature_specialization_part();
        if self.at_value_part() {
            self.value_part();
        }
        self.kerml_metadata_body();
        self.finish_node();
        self.finish_node();
    }
}
