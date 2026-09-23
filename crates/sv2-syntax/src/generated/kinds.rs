// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Token and node kinds. GENERATED — do not edit.
//!
//! Written by `scripts/gen_syntax_kinds.py` from `.claude/state/grammar/keywords.json`,
//! the token set extracted from the pinned Pilot Xtext. Keyword and operator
//! variants track that file; lexical and node variants are authored in the
//! generator. Change the generator or re-pin the grammar, then regenerate.

/// Every kind of token and node in the `SysML` v2 / `KerML` syntax tree.
///
/// `repr(u16)` because rowan stores a kind as a `u16`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
#[non_exhaustive]
pub enum SyntaxKind {
    // -- lexical tokens, named by the specification at KerML 8.2.2 --
    /// `WHITE_SPACE`, `KerML` 8.2.2.1 (trivia).
    Whitespace,
    /// `SINGLE_LINE_NOTE`, `KerML` 8.2.2.2 (trivia).
    SingleLineNote,
    /// `MULTILINE_NOTE`, `KerML` 8.2.2.2 (trivia).
    MultilineNote,
    /// `REGULAR_COMMENT`, `KerML` 8.2.2.2 (trivia).
    RegularComment,
    /// `BASIC_NAME`, `KerML` 8.2.2.3 (token).
    BasicName,
    /// `UNRESTRICTED_NAME`, `KerML` 8.2.2.3 (token).
    UnrestrictedName,
    /// `DECIMAL_VALUE`, `KerML` 8.2.2.4 (token).
    DecimalValue,
    /// `EXPONENTIAL_VALUE`, `KerML` 8.2.2.4 (token).
    ExponentialValue,
    /// `STRING_VALUE`, `KerML` 8.2.2.5 (token).
    StringValue,

    // -- 173 keywords, from the pinned token set --
    /// `about`
    KwAbout,
    /// `abstract`
    KwAbstract,
    /// `accept`
    KwAccept,
    /// `action`
    KwAction,
    /// `actor`
    KwActor,
    /// `after`
    KwAfter,
    /// `alias`
    KwAlias,
    /// `all`
    KwAll,
    /// `allocate`
    KwAllocate,
    /// `allocation`
    KwAllocation,
    /// `analysis`
    KwAnalysis,
    /// `and`
    KwAnd,
    /// `as`
    KwAs,
    /// `assert`
    KwAssert,
    /// `assign`
    KwAssign,
    /// `assoc`
    KwAssoc,
    /// `assume`
    KwAssume,
    /// `at`
    KwAt,
    /// `attribute`
    KwAttribute,
    /// `behavior`
    KwBehavior,
    /// `bind`
    KwBind,
    /// `binding`
    KwBinding,
    /// `bool`
    KwBool,
    /// `by`
    KwBy,
    /// `calc`
    KwCalc,
    /// `case`
    KwCase,
    /// `chains`
    KwChains,
    /// `class`
    KwClass,
    /// `classifier`
    KwClassifier,
    /// `comment`
    KwComment,
    /// `composite`
    KwComposite,
    /// `concern`
    KwConcern,
    /// `conjugate`
    KwConjugate,
    /// `conjugates`
    KwConjugates,
    /// `conjugation`
    KwConjugation,
    /// `connect`
    KwConnect,
    /// `connection`
    KwConnection,
    /// `connector`
    KwConnector,
    /// `const`
    KwConst,
    /// `constant`
    KwConstant,
    /// `constraint`
    KwConstraint,
    /// `crosses`
    KwCrosses,
    /// `datatype`
    KwDatatype,
    /// `decide`
    KwDecide,
    /// `def`
    KwDef,
    /// `default`
    KwDefault,
    /// `defined`
    KwDefined,
    /// `dependency`
    KwDependency,
    /// `derived`
    KwDerived,
    /// `differences`
    KwDifferences,
    /// `disjoining`
    KwDisjoining,
    /// `disjoint`
    KwDisjoint,
    /// `do`
    KwDo,
    /// `doc`
    KwDoc,
    /// `else`
    KwElse,
    /// `end`
    KwEnd,
    /// `entry`
    KwEntry,
    /// `enum`
    KwEnum,
    /// `event`
    KwEvent,
    /// `exhibit`
    KwExhibit,
    /// `exit`
    KwExit,
    /// `expose`
    KwExpose,
    /// `expr`
    KwExpr,
    /// `false`
    KwFalse,
    /// `feature`
    KwFeature,
    /// `featured`
    KwFeatured,
    /// `featuring`
    KwFeaturing,
    /// `filter`
    KwFilter,
    /// `first`
    KwFirst,
    /// `flow`
    KwFlow,
    /// `for`
    KwFor,
    /// `fork`
    KwFork,
    /// `frame`
    KwFrame,
    /// `from`
    KwFrom,
    /// `function`
    KwFunction,
    /// `hastype`
    KwHastype,
    /// `if`
    KwIf,
    /// `implies`
    KwImplies,
    /// `import`
    KwImport,
    /// `in`
    KwIn,
    /// `include`
    KwInclude,
    /// `individual`
    KwIndividual,
    /// `inout`
    KwInout,
    /// `interaction`
    KwInteraction,
    /// `interface`
    KwInterface,
    /// `intersects`
    KwIntersects,
    /// `inv`
    KwInv,
    /// `inverse`
    KwInverse,
    /// `inverting`
    KwInverting,
    /// `istype`
    KwIstype,
    /// `item`
    KwItem,
    /// `join`
    KwJoin,
    /// `language`
    KwLanguage,
    /// `library`
    KwLibrary,
    /// `locale`
    KwLocale,
    /// `loop`
    KwLoop,
    /// `member`
    KwMember,
    /// `merge`
    KwMerge,
    /// `message`
    KwMessage,
    /// `meta`
    KwMeta,
    /// `metaclass`
    KwMetaclass,
    /// `metadata`
    KwMetadata,
    /// `multiplicity`
    KwMultiplicity,
    /// `namespace`
    KwNamespace,
    /// `new`
    KwNew,
    /// `nonunique`
    KwNonunique,
    /// `not`
    KwNot,
    /// `null`
    KwNull,
    /// `objective`
    KwObjective,
    /// `occurrence`
    KwOccurrence,
    /// `of`
    KwOf,
    /// `or`
    KwOr,
    /// `ordered`
    KwOrdered,
    /// `out`
    KwOut,
    /// `package`
    KwPackage,
    /// `parallel`
    KwParallel,
    /// `part`
    KwPart,
    /// `perform`
    KwPerform,
    /// `port`
    KwPort,
    /// `portion`
    KwPortion,
    /// `predicate`
    KwPredicate,
    /// `private`
    KwPrivate,
    /// `protected`
    KwProtected,
    /// `public`
    KwPublic,
    /// `redefines`
    KwRedefines,
    /// `redefinition`
    KwRedefinition,
    /// `ref`
    KwRef,
    /// `references`
    KwReferences,
    /// `render`
    KwRender,
    /// `rendering`
    KwRendering,
    /// `rep`
    KwRep,
    /// `require`
    KwRequire,
    /// `requirement`
    KwRequirement,
    /// `return`
    KwReturn,
    /// `satisfy`
    KwSatisfy,
    /// `send`
    KwSend,
    /// `snapshot`
    KwSnapshot,
    /// `specialization`
    KwSpecialization,
    /// `specializes`
    KwSpecializes,
    /// `stakeholder`
    KwStakeholder,
    /// `standard`
    KwStandard,
    /// `state`
    KwState,
    /// `step`
    KwStep,
    /// `struct`
    KwStruct,
    /// `subclassifier`
    KwSubclassifier,
    /// `subject`
    KwSubject,
    /// `subset`
    KwSubset,
    /// `subsets`
    KwSubsets,
    /// `subtype`
    KwSubtype,
    /// `succession`
    KwSuccession,
    /// `terminate`
    KwTerminate,
    /// `then`
    KwThen,
    /// `timeslice`
    KwTimeslice,
    /// `to`
    KwTo,
    /// `transition`
    KwTransition,
    /// `true`
    KwTrue,
    /// `type`
    KwType,
    /// `typed`
    KwTyped,
    /// `typing`
    KwTyping,
    /// `unions`
    KwUnions,
    /// `until`
    KwUntil,
    /// `use`
    KwUse,
    /// `var`
    KwVar,
    /// `variant`
    KwVariant,
    /// `variation`
    KwVariation,
    /// `verification`
    KwVerification,
    /// `verify`
    KwVerify,
    /// `via`
    KwVia,
    /// `view`
    KwView,
    /// `viewpoint`
    KwViewpoint,
    /// `when`
    KwWhen,
    /// `while`
    KwWhile,
    /// `xor`
    KwXor,

    // -- 44 operators, from the pinned token set --
    /// `!=`
    BangEq,
    /// `!==`
    BangEqEq,
    /// `#`
    Hash,
    /// `$`
    Dollar,
    /// `%`
    Percent,
    /// `&`
    Amp,
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `*`
    Star,
    /// `**`
    StarStar,
    /// `+`
    Plus,
    /// `,`
    Comma,
    /// `-`
    Minus,
    /// `->`
    ThinArrow,
    /// `.`
    Dot,
    /// `..`
    DotDot,
    /// `.?`
    DotQuestion,
    /// `/`
    Slash,
    /// `:`
    Colon,
    /// `::`
    ColonColon,
    /// `::>`
    ColonColonGt,
    /// `:=`
    ColonEq,
    /// `:>`
    ColonGt,
    /// `:>>`
    ColonGtGt,
    /// `;`
    Semicolon,
    /// `<`
    Lt,
    /// `<=`
    LtEq,
    /// `=`
    Eq,
    /// `==`
    EqEq,
    /// `===`
    EqEqEq,
    /// `=>`
    FatArrow,
    /// `>`
    Gt,
    /// `>=`
    GtEq,
    /// `?`
    Question,
    /// `??`
    QuestionQuestion,
    /// `@`
    At,
    /// `@@`
    AtAt,
    /// `[`
    LBracket,
    /// `]`
    RBracket,
    /// `^`
    Caret,
    /// `{`
    LBrace,
    /// `|`
    Pipe,
    /// `}`
    RBrace,
    /// `~`
    Tilde,

    // -- nodes, authored --
    /// The whole file, and one of the two places the grammars disagree: `PackageBodyElement*` in `SysML` 8.2.2.5.1, `NamespaceBodyElement*` in `KerML` 8.2.3.4.1 (ADR-0014).
    RootNamespace,
    /// `PrefixMetadataMember* PackageDeclaration PackageBody`. `SysML` 8.2.2.5.1.
    Package,
    /// `'package' Identification`. `SysML` 8.2.2.5.1.
    PackageDeclaration,
    /// `'standard'? 'library' PrefixMetadataMember* PackageDeclaration PackageBody`. `SysML` 8.2.2.5.1, `KerML` 8.2.5.13, as deviation `LibraryPackage` reads it.
    LibraryPackage,
    /// `';' | '{' PackageBodyElement* '}'` in `SysML` 8.2.2.5.1; `';' | '{' ( NamespaceBodyElement | ElementFilterMember )* '}'` in `KerML` 8.2.3.4.1.
    PackageBody,
    /// `( '<' NAME '>' )? ( NAME )?`. `SysML` 8.2.2.2.
    Identification,
    /// `( '$' '::' )? ( NAME '::' )* NAME`. `KerML` 8.2.3.4.1.
    QualifiedName,
    /// `VisibilityIndicator 'import' 'all'? ImportDeclaration RelationshipBody`. `SysML` 8.2.2.5.1.
    Import,
    /// `'public' | 'private' | 'protected'`. `SysML` 8.2.2.5.1.
    VisibilityIndicator,
    /// `MembershipImport | NamespaceImport`. `SysML` 8.2.2.5.1.
    ImportDeclaration,
    /// `[QualifiedName] ( '::' '**' )?`. `SysML` 8.2.2.5.1.
    MembershipImport,
    /// `[QualifiedName] '::' '*' ( '::' '**' )? | FilterPackage`. `SysML` 8.2.2.5.1.
    NamespaceImport,
    /// `FilterPackageImport FilterPackageMember+` in `SysML` 8.2.2.5.1; `ImportDeclaration FilterPackageMember+` in `KerML` 8.2.3.4.2.
    FilterPackage,
    /// `ImportDeclaration { visibility = 'public' }`. `SysML` 8.2.2.5.1.
    FilterPackageImport,
    /// `'[' OwnedExpression ']'`. `SysML` 8.2.2.5.1, `KerML` 8.2.3.4.2.
    FilterPackageMember,
    /// `';' | '{' OwnedAnnotation* '}'`. `SysML` 8.2.2.2.
    RelationshipBody,
    /// `MemberPrefix ( DefinitionElement | UsageElement )`. `SysML` 8.2.2.5.1.
    PackageMember,
    /// `TypePrefix 'classifier' ClassifierDeclaration TypeBody`. `KerML` 8.2.4.2.
    Classifier,
    /// `TypePrefix 'class' ClassifierDeclaration TypeBody`. `KerML` 8.2.4.2.
    Class,
    /// `TypePrefix 'struct' ClassifierDeclaration TypeBody`. `KerML` 8.2.4.2.
    Structure,
    /// `TypePrefix 'datatype' ClassifierDeclaration TypeBody`. `KerML` 8.2.4.2.
    DataType,
    /// `TypePrefix 'metaclass' ClassifierDeclaration TypeBody`. `KerML` 8.2.4.2.
    Metaclass,
    /// `TypePrefix 'assoc' ClassifierDeclaration TypeBody`. `KerML` 8.2.4.2.
    Association,
    /// `TypePrefix 'behavior' ClassifierDeclaration TypeBody`. `KerML` 8.2.4.2.
    Behavior,
    /// `TypePrefix 'interaction' ClassifierDeclaration TypeBody`. `KerML` 8.2.4.2.
    Interaction,
    /// `'abstract'? PrefixMetadataMember*`. `KerML` 8.2.4.1.
    TypePrefix,
    /// `'all'? Identification OwnedMultiplicity? ( SuperclassingPart | ConjugationPart )? TypeRelationshipPart*`. `KerML` 8.2.4.2.
    ClassifierDeclaration,
    /// `';' | '{' TypeBodyElement* '}'`. `KerML` 8.2.4.1.
    TypeBody,
    /// `SPECIALIZES OwnedSubclassification ( ',' OwnedSubclassification )*`. `KerML` 8.2.4.2.
    SuperclassingPart,
    /// `( FeaturePrefix ( 'feature' | PrefixMetadataMember ) FeatureDeclaration? | ( EndFeaturePrefix | BasicFeaturePrefix ) FeatureDeclaration ) ValuePart? TypeBody`. `KerML` 8.2.4.3.1.
    Feature,
    /// `( EndFeaturePrefix OwnedCrossFeatureMember? | BasicFeaturePrefix ) PrefixMetadataMember*`. `KerML` 8.2.4.3.1.
    FeaturePrefix,
    /// `FeatureDirection? 'derived'? 'abstract'? ( 'composite' | 'portion' )? ( 'var' | 'const' )?`. `KerML` 8.2.4.3.1.
    BasicFeaturePrefix,
    /// `'const'? 'end'`. `KerML` 8.2.4.3.1.
    EndFeaturePrefix,
    /// `'all'? ( FeatureIdentification ( FeatureSpecializationPart | ConjugationPart )? | FeatureSpecializationPart | ConjugationPart ) FeatureRelationshipPart*`. `KerML` 8.2.4.3.1.
    FeatureDeclaration,
    /// `'<' NAME '>' NAME? | NAME`. `KerML` 8.2.4.3.1 — NOT `Identification`, whose parts are both optional. A feature declaration must name something.
    FeatureIdentification,
    /// `FeaturePrefix 'connector' ( FeatureDeclaration? ValuePart? | ConnectorDeclaration ) TypeBody`. `KerML` 8.2.5.5.1.
    Connector,
    /// `( FeatureDeclaration? 'from' | 'all' 'from'? )? ConnectorEndMember 'to' ConnectorEndMember`. `KerML` 8.2.5.5.1.
    BinaryConnectorDeclaration,
    /// `FeatureDeclaration? '(' ConnectorEndMember ',' ConnectorEndMember ( ',' ConnectorEndMember )* ')'`. `KerML` 8.2.5.5.1.
    NaryConnectorDeclaration,
    /// `FeaturePrefix 'succession' SuccessionDeclaration TypeBody`. `KerML` 8.2.5.5.3.
    Succession,
    /// `FeatureDeclaration ( 'first' ConnectorEndMember 'then' ConnectorEndMember )? | 'all'? ( 'first'? ConnectorEndMember 'then' ConnectorEndMember )?`. `KerML` 8.2.5.5.3.
    SuccessionDeclaration,
    /// `FeaturePrefix 'binding' BindingConnectorDeclaration TypeBody`. `KerML` 8.2.5.5.2.
    BindingConnector,
    /// `FeatureDeclaration ( 'of' ConnectorEndMember '=' ConnectorEndMember )? | 'all'? ( 'of'? ConnectorEndMember '=' ConnectorEndMember )?`. `KerML` 8.2.5.5.2.
    BindingConnectorDeclaration,
    /// `MemberPrefix FeatureElement`. `KerML` 8.2.3.4.1.
    NamespaceFeatureMember,
    /// `MemberPrefix MemberElement`. `KerML` 8.2.3.4.1 — what a `PackageMember` is in a `KerML` file, where the members are `MemberElement` and `FeatureElement` rather than `DefinitionElement` and `UsageElement`.
    NonFeatureMember,
    /// `( visibility = VisibilityIndicator )?`. `SysML` 8.2.2.5.1.
    MemberPrefix,
    /// `MemberPrefix 'alias' ( '<' NAME '>' )? NAME? 'for' [QualifiedName] RelationshipBody`. `SysML` 8.2.2.5.1.
    AliasMember,
    /// `ownedRelatedElement += AnnotatingElement`. `SysML` 8.2.2.4.1.
    OwnedAnnotation,
    /// `annotatedElement = [QualifiedName]`. `SysML` 8.2.2.4.1.
    Annotation,
    /// `( 'comment' Identification ( 'about' Annotation ( ',' Annotation )* )? )? ( 'locale' STRING_VALUE )? REGULAR_COMMENT`. `SysML` 8.2.2.4.2.
    Comment,
    /// `'doc' Identification ( 'locale' STRING_VALUE )? REGULAR_COMMENT`. `SysML` 8.2.2.4.2.
    Documentation,
    /// `( 'rep' Identification )? 'language' STRING_VALUE REGULAR_COMMENT`. `SysML` 8.2.2.4.3.
    TextualRepresentation,
    /// `OccurrenceDefinitionPrefix 'part' 'def' Definition`. `SysML` 8.2.2.11.
    PartDefinition,
    /// `DefinitionPrefix 'attribute' 'def' Definition`. `SysML` 8.2.2.7.
    AttributeDefinition,
    /// `OccurrenceDefinitionPrefix 'occurrence' 'def' Definition`. `SysML` 8.2.2.9.1.
    OccurrenceDefinition,
    /// `OccurrenceDefinitionPrefix 'item' 'def' Definition`. `SysML` 8.2.2.10.
    ItemDefinition,
    /// `OccurrenceDefinitionPrefix 'connection' 'def' Definition`. `SysML` 8.2.2.13.
    ConnectionDefinition,
    /// `OccurrenceDefinitionPrefix 'flow' 'def' Definition`. `SysML` 8.2.2.16.
    FlowDefinition,
    /// `OccurrenceUsagePrefix 'flow' FlowDeclaration DefinitionBody`. `SysML` 8.2.2.16.
    FlowUsage,
    /// `OccurrenceUsagePrefix 'succession' 'flow' FlowDeclaration DefinitionBody`. `SysML` 8.2.2.16.
    SuccessionFlowUsage,
    /// `OccurrenceUsagePrefix 'message' MessageDeclaration DefinitionBody`. `SysML` 8.2.2.16 — the metaclass is `FlowUsage`.
    Message,
    /// `UsageDeclaration ValuePart? ( 'of' FlowPayloadFeatureMember )? ( 'from' MessageEventMember 'to' MessageEventMember )? | MessageEventMember 'to' MessageEventMember`. `SysML` 8.2.2.16 — returns `FlowUsage`.
    MessageDeclaration,
    /// `ownedRelatedElement += MessageEvent`. `SysML` 8.2.2.16 — a `ParameterMembership`.
    MessageEventMember,
    /// `ownedRelationship += OwnedReferenceSubsetting`. `SysML` 8.2.2.16 — the metaclass is `EventOccurrenceUsage`.
    MessageEvent,
    /// `UsageDeclaration ValuePart? ( 'of' FlowPayloadFeatureMember )? ( 'from' FlowEndMember 'to' FlowEndMember )? | FlowEndMember 'to' FlowEndMember`. `SysML` 8.2.2.16 — returns `FlowUsage`.
    FlowDeclaration,
    /// `FlowEnd`. `SysML` 8.2.2.16 — the metaclass is `EndFeatureMembership`.
    FlowEndMember,
    /// `FlowEndSubsetting? FlowFeatureMember`. `SysML` 8.2.2.16.
    FlowEnd,
    /// `[QualifiedName] '.' | FeatureChainPrefix`. `SysML` 8.2.2.16, with the '.' of deviation `FlowEndSubsetting` — the metaclass is `ReferenceSubsetting`.
    FlowEndSubsetting,
    /// `( OwnedFeatureChaining '.' )+ OwnedFeatureChaining '.'`. `SysML` 8.2.2.16.
    FeatureChainPrefix,
    /// `FlowFeature`. `SysML` 8.2.2.16 — the metaclass is `FeatureMembership`.
    FlowFeatureMember,
    /// `FlowFeatureRedefinition`. `SysML` 8.2.2.16 — the metaclass is `ReferenceUsage`.
    FlowFeature,
    /// `[QualifiedName]`. `SysML` 8.2.2.16, stated alike in `KerML` 8.2.5.9.2 — `Redefinition`.
    FlowFeatureRedefinition,
    /// `FlowPayloadFeature`. `SysML` 8.2.2.16 — the metaclass is `FeatureMembership`.
    FlowPayloadFeatureMember,
    /// `PayloadFeature`. `SysML` 8.2.2.16 — the metaclass is `PayloadFeature`.
    FlowPayloadFeature,
    /// `Identification? PayloadFeatureSpecializationPart ValuePart? | OwnedFeatureTyping OwnedMultiplicity? | OwnedMultiplicity OwnedFeatureTyping`. `SysML` 8.2.2.16.
    PayloadFeature,
    /// `FeatureSpecialization+ MultiplicityPart? FeatureSpecialization* | MultiplicityPart FeatureSpecialization+`. `SysML` 8.2.2.16.
    PayloadFeatureSpecializationPart,
    /// `OccurrenceDefinitionPrefix 'allocation' 'def' Definition`. `SysML` 8.2.2.15.
    AllocationDefinition,
    /// `OccurrenceDefinitionPrefix 'rendering' 'def' Definition`. `SysML` 8.2.2.26.3.
    RenderingDefinition,
    /// `DefinitionPrefix 'port' 'def' Definition ConjugatedPortDefinitionMember`. `SysML` 8.2.2.12.
    PortDefinition,
    /// `ownedRelatedElement += ConjugatedPortDefinition`. `SysML` 8.2.2.12.
    ConjugatedPortDefinitionMember,
    /// `ownedRelationship += PortConjugation`. `SysML` 8.2.2.12.
    ConjugatedPortDefinition,
    /// `{ }`, which consumes no tokens. `SysML` 8.2.2.12.
    PortConjugation,
    /// `BasicDefinitionPrefix? DefinitionExtensionKeyword*`. `SysML` 8.2.2.6.1 — `OccurrenceDefinitionPrefix` without the `individual` part, for the definitions that are not occurrences.
    DefinitionPrefix,
    /// `BasicDefinitionPrefix? ( 'individual' EmptyMultiplicityMember )? DefinitionExtensionKeyword*`. `SysML` 8.2.2.9.1.
    OccurrenceDefinitionPrefix,
    /// `'abstract' | 'variation'`. `SysML` 8.2.2.6.1.
    BasicDefinitionPrefix,
    /// `BasicDefinitionPrefix? 'individual' DefinitionExtensionKeyword* 'def' Definition EmptyMultiplicityMember`. `SysML` 8.2.2.9.1 — the metaclass is `OccurrenceDefinition`.
    IndividualDefinition,
    /// `ownedRelatedElement += EmptyMultiplicity`. `SysML` 8.2.2.9.1.
    EmptyMultiplicityMember,
    /// `{ }`, a Multiplicity that consumes no tokens. `SysML` 8.2.2.9.1.
    EmptyMultiplicity,
    /// `DefinitionDeclaration DefinitionBody`. `SysML` 8.2.2.6.1.
    Definition,
    /// `Identification SubclassificationPart?`. `SysML` 8.2.2.6.1.
    DefinitionDeclaration,
    /// `SPECIALIZES OwnedSubclassification ( ',' OwnedSubclassification )*`. `SysML` 8.2.2.6.5.
    SubclassificationPart,
    /// `superClassifier = [QualifiedName]`. `SysML` 8.2.2.6.5.
    OwnedSubclassification,
    /// `';' | '{' DefinitionBodyItem* '}'`. `SysML` 8.2.2.6.1.
    DefinitionBody,
    /// `MemberPrefix DefinitionElement`. `SysML` 8.2.2.6.1.
    DefinitionMember,
    /// `MemberPrefix NonOccurrenceUsageElement`. `SysML` 8.2.2.6.1.
    NonOccurrenceUsageMember,
    /// `MemberPrefix OccurrenceUsageElement`. `SysML` 8.2.2.6.1.
    OccurrenceUsageMember,
    /// `MemberPrefix StructureUsageElement`. `SysML` 8.2.2.6.1.
    StructureUsageMember,
    /// `MemberPrefix BehaviorUsageElement`. `SysML` 8.2.2.6.1.
    BehaviorUsageMember,
    /// `OccurrenceDefinitionPrefix 'requirement' 'def' DefinitionDeclaration RequirementBody`. `SysML` 8.2.2.21.1.
    RequirementDefinition,
    /// `';' | '{' RequirementBodyItem* '}'`. `SysML` 8.2.2.21.1.
    RequirementBody,
    /// `MemberPrefix ownedRelatedElement += SubjectUsage`. `SysML` 8.2.2.21.1.
    SubjectMember,
    /// `'subject' UsageExtensionKeyword* Usage`. `SysML` 8.2.2.21.1.
    SubjectUsage,
    /// `MemberPrefix ownedRelatedElement += ActorUsage`. `SysML` 8.2.2.21.1.
    ActorMember,
    /// `'actor' UsageExtensionKeyword* Usage`. `SysML` 8.2.2.21.1 — the metaclass is `PartUsage`.
    ActorUsage,
    /// `MemberPrefix ownedRelatedElement += StakeholderUsage`. `SysML` 8.2.2.21.1.
    StakeholderMember,
    /// `'stakeholder' UsageExtensionKeyword* Usage`. `SysML` 8.2.2.21.1 — the metaclass is `PartUsage`.
    StakeholderUsage,
    /// `MemberPrefix? 'frame' ownedRelatedElement += FramedConcernUsage`. `SysML` 8.2.2.21.1 — a `FramedConcernMembership`.
    FramedConcernMember,
    /// `OwnedReferenceSubsetting FeatureSpecializationPart? RequirementBody | ( UsageExtensionKeyword* 'concern' | UsageExtensionKeyword+ ) ConstraintUsageDeclaration RequirementBody`, as deviation `FramedConcernUsage` reads it. `SysML` 8.2.2.21.1 — the metaclass is `ConcernUsage`.
    FramedConcernUsage,
    /// `OccurrenceDefinitionPrefix 'concern' 'def' DefinitionDeclaration RequirementBody`. `SysML` 8.2.2.21.3.
    ConcernDefinition,
    /// `OccurrenceUsagePrefix 'concern' ConstraintUsageDeclaration RequirementBody`. `SysML` 8.2.2.21.3.
    ConcernUsage,
    /// `OccurrenceDefinitionPrefix 'view' 'def' DefinitionDeclaration ViewDefinitionBody`. `SysML` 8.2.2.26.1.
    ViewDefinition,
    /// `';' | '{' ViewDefinitionBodyItem* '}'`. `SysML` 8.2.2.26.1.
    ViewDefinitionBody,
    /// `MemberPrefix 'render' ViewRenderingUsage`. `SysML` 8.2.2.26.1.
    ViewRenderingMember,
    /// `OwnedReferenceSubsetting FeatureSpecializationPart? UsageBody | ( UsageExtensionKeyword* 'rendering' | UsageExtensionKeyword+ ) Usage`. `SysML` 8.2.2.26.1.
    ViewRenderingUsage,
    /// `OccurrenceUsagePrefix 'view' UsageDeclaration? ValuePart? ViewBody`. `SysML` 8.2.2.26.2.
    ViewUsage,
    /// `';' | '{' ViewBodyItem* '}'`. `SysML` 8.2.2.26.2.
    ViewBody,
    /// `'expose' ( MembershipExpose | NamespaceExpose ) RelationshipBody`. `SysML` 8.2.2.26.2.
    Expose,
    /// `MembershipImport`. `SysML` 8.2.2.26.2.
    MembershipExpose,
    /// `NamespaceImport`. `SysML` 8.2.2.26.2.
    NamespaceExpose,
    /// `OccurrenceDefinitionPrefix 'viewpoint' 'def' DefinitionDeclaration RequirementBody`. `SysML` 8.2.2.26.3.
    ViewpointDefinition,
    /// `OccurrenceUsagePrefix 'viewpoint' ConstraintUsageDeclaration RequirementBody`. `SysML` 8.2.2.26.3.
    ViewpointUsage,
    /// `MemberPrefix? RequirementKind ownedRelatedElement += RequirementConstraintUsage`. `SysML` 8.2.2.21.1.
    RequirementConstraintMember,
    /// `'assume' | 'require'`. `SysML` 8.2.2.21.1.
    RequirementKind,
    /// `MemberPrefix 'verify' ownedRelatedElement += RequirementVerificationUsage`. `SysML` 8.2.2.24.
    RequirementVerificationMember,
    /// `OwnedReferenceSubsetting FeatureSpecialization* RequirementBody` or `'requirement' ConstraintUsageDeclaration RequirementBody`. `SysML` 8.2.2.24 — the metaclass is `RequirementUsage`.
    RequirementVerificationUsage,
    /// `OwnedReferenceSubsetting FeatureSpecializationPart? RequirementBody` or `'constraint' ConstraintUsageDeclaration CalculationBody`. `SysML` 8.2.2.21.1 — the metaclass is `ConstraintUsage`.
    RequirementConstraintUsage,
    /// `UsageDeclaration ValuePart?`. `SysML` 8.2.2.20.
    ConstraintUsageDeclaration,
    /// `OccurrenceUsagePrefix 'requirement' ConstraintUsageDeclaration RequirementBody`. `SysML` 8.2.2.21.2.
    RequirementUsage,
    /// `OccurrenceUsagePrefix 'assert' 'not' 'satisfy' ( OwnedReferenceSubsetting FeatureSpecializationPart? | 'requirement' UsageDeclaration ) ValuePart? ( 'by' SatisfactionSubjectMember )? RequirementBody`, `assert` and `not` optional by deviation. `SysML` 8.2.2.21.2.
    SatisfyRequirementUsage,
    /// `ownedRelatedElement += SatisfactionParameter`. `SysML` 8.2.2.21.2 — a `SubjectMembership`.
    SatisfactionSubjectMember,
    /// `ownedRelationship += SatisfactionFeatureValue`. `SysML` 8.2.2.21.2 — the metaclass is `ReferenceUsage`.
    SatisfactionParameter,
    /// `ownedRelatedElement += SatisfactionReferenceExpression`. `SysML` 8.2.2.21.2 — the metaclass is `FeatureValue`.
    SatisfactionFeatureValue,
    /// `ownedRelationship += FeatureChainMember`. `SysML` 8.2.2.21.2 — the metaclass is `FeatureReferenceExpression`.
    SatisfactionReferenceExpression,
    /// `OccurrenceUsagePrefix 'constraint' ConstraintUsageDeclaration CalculationBody`. `SysML` 8.2.2.20.
    ConstraintUsage,
    /// `OccurrenceUsagePrefix 'assert' 'not'? ( OwnedReferenceSubsetting FeatureSpecializationPart? | 'constraint' ConstraintUsageDeclaration ) CalculationBody`. `SysML` 8.2.2.20.
    AssertConstraintUsage,
    /// `OccurrenceDefinitionPrefix 'constraint' 'def' DefinitionDeclaration CalculationBody`. `SysML` 8.2.2.20.
    ConstraintDefinition,
    /// `MemberPrefix? 'return' ownedRelatedElement += UsageElement`. `SysML` 8.2.2.19 — the metaclass is `ReturnParameterMembership`.
    ReturnParameterMember,
    /// `MemberPrefix 'first' memberFeature = [QualifiedName] RelationshipBody`. `SysML` 8.2.2.17.1 — the metaclass is `FeatureMembership`.
    InitialNodeMember,
    /// `MemberPrefix ownedRelatedElement += ActionTargetSuccession`. `SysML` 8.2.2.17.1 — the metaclass is `FeatureMembership`.
    ActionTargetSuccessionMember,
    /// `( TargetSuccession | GuardedTargetSuccession | DefaultTargetSuccession ) UsageBody`. `SysML` 8.2.2.17.8.
    ActionTargetSuccession,
    /// `SourceEndMember 'then' ConnectorEndMember`. `SysML` 8.2.2.17.8 — the metaclass is `SuccessionAsUsage`.
    TargetSuccession,
    /// `GuardExpressionMember 'then' TransitionSuccessionMember`. `SysML` 8.2.2.17.8 — the metaclass is `TransitionUsage`.
    GuardedTargetSuccession,
    /// `MemberPrefix ownedRelatedElement += GuardedSuccession`. `SysML` 8.2.2.17.1 — the metaclass is `FeatureMembership`.
    GuardedSuccessionMember,
    /// `( 'succession' UsageDeclaration )? 'first' FeatureChainMember GuardExpressionMember 'then' TransitionSuccessionMember UsageBody`. `SysML` 8.2.2.17.8 — the metaclass is `TransitionUsage`.
    GuardedSuccession,
    /// `UsagePrefix ( 'succession' UsageDeclaration )? 'first' ConnectorEndMember 'then' ConnectorEndMember UsageBody`. `SysML` 8.2.2.13.3 — the metaclass is `SuccessionAsUsage`.
    SuccessionAsUsage,
    /// `UsagePrefix ( 'binding' UsageDeclaration )? 'bind' ConnectorEndMember '=' ConnectorEndMember UsageBody`. `SysML` 8.2.2.13.2 — the metaclass is `BindingConnectorAsUsage`.
    BindingConnectorAsUsage,
    /// `memberElement = [QualifiedName] | OwnedFeatureChainMember`. `SysML` 8.2.2.17.5 — the metaclass is `Membership`.
    FeatureChainMember,
    /// `ownedRelatedElement += OwnedFeatureChain`. `SysML` 8.2.2.17.5 — the metaclass is `OwningMembership`.
    OwnedFeatureChainMember,
    /// `'else' ownedRelationship += TransitionSuccessionMember`. `SysML` 8.2.2.17.8 — the metaclass is `TransitionUsage`.
    DefaultTargetSuccession,
    /// `'if' { kind = 'guard' } ownedRelatedElement += OwnedExpression`. `SysML` 8.2.2.18.3 — the metaclass is `TransitionFeatureMembership`.
    GuardExpressionMember,
    /// `ownedRelatedElement += TransitionSuccession`. `SysML` 8.2.2.18.3 — the metaclass is `OwningMembership`.
    TransitionSuccessionMember,
    /// `EmptyEndMember ConnectorEndMember`. `SysML` 8.2.2.18.3 — the metaclass is `Succession`, so its source end is empty rather than a `SourceEnd`.
    TransitionSuccession,
    /// `ownedRelatedElement += EmptyFeature`. `SysML` 8.2.2.18.3 — the metaclass is `EndFeatureMembership`.
    EmptyEndMember,
    /// `ownedRelatedElement += SourceEnd`. `SysML` 8.2.2.9.3 — the metaclass is `EndFeatureMembership`.
    SourceEndMember,
    /// `( ownedRelationship += OwnedMultiplicity )?`. `SysML` 8.2.2.9.3 — the metaclass is `ReferenceUsage`.
    SourceEnd,
    /// `'then' SourceSuccession`. `SysML` 8.2.2.9.3 — the metaclass is `FeatureMembership`.
    SourceSuccessionMember,
    /// `SourceEndMember`. `SysML` 8.2.2.9.3 — the metaclass is `SuccessionAsUsage`.
    SourceSuccession,
    /// `ownedRelatedElement += ConnectorEnd`. `SysML` 8.2.2.13.1 — the metaclass is `EndFeatureMembership`.
    ConnectorEndMember,
    /// `OwnedCrossMultiplicityMember? ( NAME REFERENCES )? OwnedReferenceSubsetting`. `SysML` 8.2.2.13.1 — the metaclass is `ReferenceUsage`.
    ConnectorEnd,
    /// `OwnedCrossMultiplicity`. `SysML` 8.2.2.13.1.
    OwnedCrossMultiplicityMember,
    /// `'end' OwnedCrossFeatureMember?`. `SysML` 8.2.2.6.2.
    EndUsagePrefix,
    /// `OwnedCrossFeature`. `SysML` 8.2.2.6.2, `KerML` 8.2.4.3.1.
    OwnedCrossFeatureMember,
    /// `BasicUsagePrefix UsageDeclaration`, a `ReferenceUsage`. `SysML` 8.2.2.6.2.
    OwnedCrossFeature,
    /// `OwnedMultiplicity`, a `Feature`. `SysML` 8.2.2.13.1, `KerML` 8.2.5.5.1.
    OwnedCrossMultiplicity,
    /// `OccurrenceUsagePrefix ( 'connection' UsageDeclaration ValuePart? ( 'connect' ConnectorPart )? | 'connect' ConnectorPart ) UsageBody`. `SysML` 8.2.2.13.1.
    ConnectionUsage,
    /// `OccurrenceUsagePrefix AllocationUsageDeclaration UsageBody`. `SysML` 8.2.2.15.
    AllocationUsage,
    /// `'allocation' UsageDeclaration ( 'allocate' ConnectorPart )? | 'allocate' ConnectorPart`. `SysML` 8.2.2.15 — returns `AllocationUsage`.
    AllocationUsageDeclaration,
    /// `ConnectorEndMember 'to' ConnectorEndMember`. `SysML` 8.2.2.13.1.
    BinaryConnectorPart,
    /// `'(' ConnectorEndMember ',' ConnectorEndMember ( ',' ConnectorEndMember )* ')'`. `SysML` 8.2.2.13.1.
    NaryConnectorPart,
    /// `OccurrenceDefinitionPrefix 'calc' 'def' DefinitionDeclaration CalculationBody`. `SysML` 8.2.2.19.
    CalculationDefinition,
    /// `OccurrenceDefinitionPrefix 'action' 'def' DefinitionDeclaration ActionBody`. `SysML` 8.2.2.17.1.
    ActionDefinition,
    /// `';' | '{' ActionBodyItem* '}'`. `SysML` 8.2.2.17.1.
    ActionBody,
    /// `OwnedFeatureChaining ( '.' OwnedFeatureChaining )+`. `SysML` 8.2.2.6.5.
    OwnedFeatureChain,
    /// `chainingFeature = [QualifiedName]`. `SysML` 8.2.2.6.5.
    OwnedFeatureChaining,
    /// `OccurrenceUsagePrefix 'action' ActionUsageDeclaration ActionBody`. `SysML` 8.2.2.17.2.
    ActionUsage,
    /// `UsageDeclaration ValuePart?`. `SysML` 8.2.2.17.2.
    ActionUsageDeclaration,
    /// `OccurrenceUsagePrefix 'calc' ActionUsageDeclaration CalculationBody`. `SysML` 8.2.2.19.
    CalculationUsage,
    /// `OccurrenceUsagePrefix 'perform' PerformActionUsageDeclaration ActionBody`. `SysML` 8.2.2.17.2.
    PerformActionUsage,
    /// `( OwnedReferenceSubsetting FeatureSpecializationPart? | 'action' UsageDeclaration ) ValuePart?`. `SysML` 8.2.2.17.2.
    PerformActionUsageDeclaration,
    /// `MemberPrefix ownedRelatedElement += ActionNode`. `SysML` 8.2.2.17.1 — the metaclass is `FeatureMembership`.
    ActionNodeMember,
    /// `RefPrefix 'individual'? PortionKind? UsageExtensionKeyword*`. `SysML` 8.2.2.17.3.
    ControlNodePrefix,
    /// `ControlNodePrefix 'merge' UsageDeclaration ActionBody`. `SysML` 8.2.2.17.3.
    MergeNode,
    /// `ControlNodePrefix 'decide' UsageDeclaration ActionBody`. `SysML` 8.2.2.17.3.
    DecisionNode,
    /// `ControlNodePrefix 'join' UsageDeclaration ActionBody`. `SysML` 8.2.2.17.3.
    JoinNode,
    /// `ControlNodePrefix 'fork' UsageDeclaration ActionBody`. `SysML` 8.2.2.17.3.
    ForkNode,
    /// `';' | '{' CalculationBodyPart '}'`. `SysML` 8.2.2.19.
    CalculationBody,
    /// `CalculationBodyItem* ResultExpressionMember?`. `SysML` 8.2.2.19.
    CalculationBodyPart,
    /// `';' | '{' CaseBodyItem* ResultExpressionMember? '}'`. `SysML` 8.2.2.22.
    CaseBody,
    /// `OccurrenceDefinitionPrefix 'case' 'def' DefinitionDeclaration CaseBody`. `SysML` 8.2.2.22.
    CaseDefinition,
    /// `OccurrenceUsagePrefix 'case' ConstraintUsageDeclaration CaseBody`. `SysML` 8.2.2.22.
    CaseUsage,
    /// `MemberPrefix 'objective' ObjectiveRequirementUsage`. `SysML` 8.2.2.22.
    ObjectiveMember,
    /// `UsageExtensionKeyword* ConstraintUsageDeclaration RequirementBody`. `SysML` 8.2.2.22 — the metaclass is `RequirementUsage`.
    ObjectiveRequirementUsage,
    /// `OccurrenceDefinitionPrefix 'analysis' 'def' DefinitionDeclaration CaseBody`. `SysML` 8.2.2.23.
    AnalysisCaseDefinition,
    /// `OccurrenceUsagePrefix 'analysis' ConstraintUsageDeclaration CaseBody`. `SysML` 8.2.2.23.
    AnalysisCaseUsage,
    /// `OccurrenceDefinitionPrefix 'verification' 'def' DefinitionDeclaration CaseBody`. `SysML` 8.2.2.24.
    VerificationCaseDefinition,
    /// `OccurrenceUsagePrefix 'verification' ConstraintUsageDeclaration CaseBody`. `SysML` 8.2.2.24.
    VerificationCaseUsage,
    /// `OccurrenceDefinitionPrefix 'use' 'case' 'def' DefinitionDeclaration CaseBody`. `SysML` 8.2.2.25.
    UseCaseDefinition,
    /// `OccurrenceUsagePrefix 'use' 'case' ConstraintUsageDeclaration CaseBody`. `SysML` 8.2.2.25.
    UseCaseUsage,
    /// `OccurrenceUsagePrefix 'include' ( OwnedReferenceSubsetting FeatureSpecializationPart? | 'use' 'case' UsageDeclaration ) ValuePart? CaseBody`. `SysML` 8.2.2.25.
    IncludeUseCaseUsage,
    /// `NonFeatureChainPrimaryArgumentMember '.' FeatureChainMember`. `KerML` 8.2.5.8.2 — the metaclass is an `OperatorExpression`, 8.3.4.8.4.
    FeatureChainExpression,
    /// `ownedMemberParameter = PrimaryArgument`. `KerML` 8.2.5.8.2.
    NonFeatureChainPrimaryArgumentMember,
    /// `PrimaryArgumentMember '[' SequenceExpressionListMember ']'`. `KerML` 8.2.5.8.2 — the quantity form, `1200 [kg]`.
    BracketExpression,
    /// `ownedMemberParameter = PrimaryArgument`. `KerML` 8.2.5.8.2.
    PrimaryArgumentMember,
    /// `ownedRelationship += PrimaryArgumentValue`. `KerML` 8.2.5.8.2.
    PrimaryArgument,
    /// `value = PrimaryExpression`. `KerML` 8.2.5.8.2.
    PrimaryArgumentValue,
    /// `PrimaryArgumentMember '#' '(' SequenceExpressionListMember ')'`. `KerML` 8.2.5.8.2.
    IndexExpression,
    /// `PrimaryArgumentMember '.' BodyArgumentMember`. `KerML` 8.2.5.8.2.
    CollectExpression,
    /// `PrimaryArgumentMember '.?' BodyArgumentMember`. `KerML` 8.2.5.8.2.
    SelectExpression,
    /// `PrimaryArgumentMember '->' InstantiatedTypeMember ( BodyArgumentMember | FunctionReferenceArgumentMember | ArgumentList ) EmptyResultMember`. `KerML` 8.2.5.8.2.
    FunctionOperationExpression,
    /// `ownedMemberParameter = BodyArgument`. `KerML` 8.2.5.8.2.
    BodyArgumentMember,
    /// `ownedRelationship += BodyArgumentValue`. `KerML` 8.2.5.8.2.
    BodyArgument,
    /// `value = BodyExpression`. `KerML` 8.2.5.8.2.
    BodyArgumentValue,
    /// `ownedMemberParameter = FunctionReferenceArgument`. `KerML` 8.2.5.8.2.
    FunctionReferenceArgumentMember,
    /// `ownedRelationship += FunctionReferenceArgumentValue`. `KerML` 8.2.5.8.2.
    FunctionReferenceArgument,
    /// `value = FunctionReferenceExpression`. `KerML` 8.2.5.8.2.
    FunctionReferenceArgumentValue,
    /// `ownedRelationship += FunctionReferenceMember`. `KerML` 8.2.5.8.2.
    FunctionReferenceExpression,
    /// `ownedMemberFeature = FunctionReference`. `KerML` 8.2.5.8.2.
    FunctionReferenceMember,
    /// `ownedRelationship += ReferenceTyping`. `KerML` 8.2.5.8.2.
    FunctionReference,
    /// `ownedRelationship += ExpressionBodyMember`. `KerML` 8.2.5.8.3.
    BodyExpression,
    /// `ownedMemberFeature = ExpressionBody`. `KerML` 8.2.5.8.3.
    ExpressionBodyMember,
    /// `'{' FunctionBodyPart '}'` in `KerML` 8.2.5.8.3; `CalculationBody` in `SysML`, by deviation `ExpressionBody`.
    ExpressionBody,
    /// `MemberPrefix? ownedRelatedElement += OwnedExpression`. `SysML` 8.2.2.19 — the metaclass is `KerML`'s `ResultExpressionMembership`, 8.3.4.7.7.
    ResultExpressionMember,
    /// `OccurrenceUsagePrefix 'part' Usage`. `SysML` 8.2.2.11.
    PartUsage,
    /// `( EndUsagePrefix | RefPrefix ) 'ref' Usage`. `SysML` 8.2.2.6.2.
    ReferenceUsage,
    /// `'end'? RefPrefix ( Identification FeatureSpecializationPart? | FeatureSpecializationPart ) UsageCompletion`. `SysML` 8.2.2.6.2 — a usage with no keyword at all, carried by its declaration.
    DefaultReferenceUsage,
    /// `UsagePrefix 'attribute' Usage`. `SysML` 8.2.2.7.
    AttributeUsage,
    /// `OccurrenceUsagePrefix 'item' Usage`. `SysML` 8.2.2.10.
    ItemUsage,
    /// `OccurrenceUsagePrefix 'occurrence' Usage`. `SysML` 8.2.2.9.2.
    OccurrenceUsage,
    /// `BasicUsagePrefix 'individual' UsageExtensionKeyword* Usage`. `SysML` 8.2.2.9.2 — the metaclass is `OccurrenceUsage`.
    IndividualUsage,
    /// `BasicUsagePrefix 'individual'? PortionKind UsageExtensionKeyword* Usage`. `SysML` 8.2.2.9.2 — the metaclass is `OccurrenceUsage`.
    PortionUsage,
    /// `OccurrenceUsagePrefix 'event' ( OwnedReferenceSubsetting FeatureSpecializationPart? | 'occurrence' UsageDeclaration? ) UsageCompletion`. `SysML` 8.2.2.9.2.
    EventOccurrenceUsage,
    /// `OccurrenceUsagePrefix 'port' Usage`. `SysML` 8.2.2.12.
    PortUsage,
    /// `OccurrenceUsagePrefix 'rendering' Usage`. `SysML` 8.2.2.26.3.
    RenderingUsage,
    /// `UsagePrefix 'enum' Usage`. `SysML` 8.2.2.8.
    EnumerationUsage,
    /// `DefinitionExtensionKeyword* 'enum' 'def' DefinitionDeclaration EnumerationBody`. `SysML` 8.2.2.8.
    EnumerationDefinition,
    /// `';' | '{' ( AnnotatingMember | EnumerationUsageMember )* '}'`. `SysML` 8.2.2.8.
    EnumerationBody,
    /// `MemberPrefix EnumeratedValue`. `SysML` 8.2.2.8.
    EnumerationUsageMember,
    /// `'enum'? Usage`, the metaclass `EnumerationUsage`. `SysML` 8.2.2.8.
    EnumeratedValue,
    /// `MemberPrefix AnnotatingElement`. `SysML` 8.2.2.4.1.
    AnnotatingMember,
    /// `MemberPrefix 'variant' ownedVariantUsage = VariantUsageElement`. `SysML` 8.2.2.6.1.
    VariantUsageMember,
    /// `OwnedReferenceSubsetting FeatureSpecialization* UsageBody`, the metaclass `ReferenceUsage`. `SysML` 8.2.2.6.3.
    VariantReference,
    /// `PrefixMetadataAnnotation* 'dependency' DependencyDeclaration RelationshipBody`. `SysML` 8.2.2.3; `KerML` 8.2.3.2 writes the declaration inline, with no `DependencyDeclaration` child.
    Dependency,
    /// `( Identification 'from' )? client ( ',' client )* 'to' supplier ( ',' supplier )*`. `SysML` 8.2.2.3.
    DependencyDeclaration,
    /// `UnextendedUsagePrefix UsageExtensionKeyword*`. `SysML` 8.2.2.6.2.
    UsagePrefix,
    /// `( EndUsagePrefix | BasicUsagePrefix 'individual'? PortionKind? ) UsageExtensionKeyword*`. `SysML` 8.2.2.9.2.
    OccurrenceUsagePrefix,
    /// `RefPrefix 'ref'?`. `SysML` 8.2.2.6.2.
    BasicUsagePrefix,
    /// `FeatureDirection? 'derived'? ( 'abstract' | 'variation' )? 'constant'?`. `SysML` 8.2.2.6.2.
    RefPrefix,
    /// `'in' | 'out' | 'inout'`. `SysML` 8.2.2.6.2.
    FeatureDirection,
    /// `'snapshot' | 'timeslice'`. `SysML` 8.2.2.9.2.
    PortionKind,
    /// `UsageDeclaration UsageCompletion`. `SysML` 8.2.2.6.2.
    Usage,
    /// `Identification FeatureSpecializationPart?`. `SysML` 8.2.2.6.2.
    UsageDeclaration,
    /// `ValuePart? UsageBody`. `SysML` 8.2.2.6.2.
    UsageCompletion,
    /// `DefinitionBody`. `SysML` 8.2.2.6.2.
    UsageBody,
    /// `FeatureSpecialization+ MultiplicityPart? FeatureSpecialization* | MultiplicityPart FeatureSpecialization*`. `KerML` 8.2.4.3.1.
    FeatureSpecializationPart,
    /// `TypedBy ( ',' FeatureTyping )*`. `SysML` 8.2.2.6.5.
    Typings,
    /// `Subsets ( ',' OwnedSubsetting )*`. `SysML` 8.2.2.6.5.
    Subsettings,
    /// `SUBSETS OwnedSubsetting`, `SUBSETS = ':>' | 'subsets'`. `SysML` 8.2.2.6.5.
    Subsets,
    /// `QualifiedName | OwnedFeatureChain`. `SysML` 8.2.2.6.5.
    OwnedSubsetting,
    /// `Redefines ( ',' OwnedRedefinition )*`. `SysML` 8.2.2.6.5.
    Redefinitions,
    /// `REDEFINES OwnedRedefinition`, `REDEFINES = ':>>' | 'redefines'`. `SysML` 8.2.2.6.5.
    Redefines,
    /// `QualifiedName | OwnedFeatureChain`. `SysML` 8.2.2.6.5.
    OwnedRedefinition,
    /// `REFERENCES OwnedReferenceSubsetting`, `REFERENCES = '::>' | 'references'`. `SysML` 8.2.2.6.5.
    References,
    /// `QualifiedName | OwnedFeatureChain`. `SysML` 8.2.2.6.5.
    OwnedReferenceSubsetting,
    /// `CROSSES OwnedCrossSubsetting`, `CROSSES = '=>' | 'crosses'`. `SysML` 8.2.2.6.5.
    Crosses,
    /// `QualifiedName | OwnedFeatureChain`. `SysML` 8.2.2.6.5.
    OwnedCrossSubsetting,
    /// `( ':' | 'defined' 'by' ) FeatureTyping`. `SysML` 8.2.2.6.5.
    TypedBy,
    /// `OwnedFeatureTyping | ConjugatedPortTyping`. `SysML` 8.2.2.6.5.
    FeatureTyping,
    /// `QualifiedName | OwnedFeatureChain`. `SysML` 8.2.2.6.5.
    OwnedFeatureTyping,
    /// `OccurrenceDefinitionPrefix 'state' 'def' DefinitionDeclaration StateDefBody`. `SysML` 8.2.2.18.1.
    StateDefinition,
    /// `';' | 'parallel'? '{' StateBodyItem* '}'`. `SysML` 8.2.2.18.1.
    StateDefBody,
    /// `OccurrenceUsagePrefix 'state' ActionUsageDeclaration StateUsageBody`. `SysML` 8.2.2.18.2.
    StateUsage,
    /// `';' | 'parallel'? '{' StateBodyItem* '}'`. `SysML` 8.2.2.18.2.
    StateUsageBody,
    /// `MemberPrefix TransitionUsage`. `SysML` 8.2.2.18.1.
    TransitionUsageMember,
    /// `'transition' ( UsageDeclaration 'first' )? FeatureChainMember EmptyParameterMember ( EmptyParameterMember TriggerActionMember )? GuardExpressionMember? EffectBehaviorMember? 'then' TransitionSuccessionMember ActionBody`. `SysML` 8.2.2.18.3.
    TransitionUsage,
    /// `MemberPrefix TargetTransitionUsage`. `SysML` 8.2.2.18.1.
    TargetTransitionUsageMember,
    /// `EmptyParameterMember ( trigger, guard, effect )? 'then' TransitionSuccessionMember ActionBody`. `SysML` 8.2.2.18.3.
    TargetTransitionUsage,
    /// `'accept' TriggerAction`. `SysML` 8.2.2.18.3.
    TriggerActionMember,
    /// `AcceptParameterPart`, an `AcceptActionUsage`. `SysML` 8.2.2.18.3.
    TriggerAction,
    /// `PayloadParameterMember ( 'via' NodeParameterMember )?`. `SysML` 8.2.2.17.4.
    AcceptParameterPart,
    /// `PayloadParameter`. `SysML` 8.2.2.17.4.
    PayloadParameterMember,
    /// `PayloadFeature | Identification PayloadFeatureSpecializationPart? TriggerValuePart`. `SysML` 8.2.2.17.4.
    PayloadParameter,
    /// `NodeParameter`. `SysML` 8.2.2.17.4.
    NodeParameterMember,
    /// `FeatureBinding`, a `ReferenceUsage`. `SysML` 8.2.2.17.4.
    NodeParameter,
    /// `OwnedExpression`, a `FeatureValue`. `SysML` 8.2.2.17.4.
    FeatureBinding,
    /// `MemberPrefix 'entry' StateActionUsage`. `SysML` 8.2.2.18.1.
    EntryActionMember,
    /// `MemberPrefix 'do' StateActionUsage`. `SysML` 8.2.2.18.1.
    DoActionMember,
    /// `MemberPrefix 'exit' StateActionUsage`. `SysML` 8.2.2.18.1.
    ExitActionMember,
    /// `MemberPrefix ( GuardedTargetSuccession | 'then' TransitionSuccession ) ';'`. `SysML` 8.2.2.18.1, with the `EntryTransitionMember` deviation.
    EntryTransitionMember,
    /// `{}`. `SysML` 8.2.2.18.1.
    EmptyActionUsage,
    /// `PerformActionUsageDeclaration ActionBody`. `SysML` 8.2.2.18.1.
    StatePerformActionUsage,
    /// `AcceptNodeDeclaration ActionBody`. `SysML` 8.2.2.18.1.
    StateAcceptActionUsage,
    /// `'do' EffectBehaviorUsage`. `SysML` 8.2.2.18.3.
    EffectBehaviorMember,
    /// `PerformActionUsageDeclaration ( '{' ActionBodyItem* '}' )?`. `SysML` 8.2.2.18.3.
    TransitionPerformActionUsage,
    /// `AcceptNodeDeclaration ( '{' ActionBodyItem* '}' )?`. `SysML` 8.2.2.18.3.
    TransitionAcceptActionUsage,
    /// `ActionNodeUsageDeclaration? 'accept' AcceptParameterPart`. `SysML` 8.2.2.17.4.
    AcceptNodeDeclaration,
    /// `'action' UsageDeclaration?`. `SysML` 8.2.2.17.2.
    ActionNodeUsageDeclaration,
    /// `OccurrenceUsagePrefix AcceptNodeDeclaration ActionBody`. `SysML` 8.2.2.17.4.
    AcceptNode,
    /// `OccurrenceUsagePrefix ActionNodeUsageDeclaration? 'send' ( NodeParameterMember SenderReceiverPart? | EmptyParameterMember SenderReceiverPart )? ActionBody`. `SysML` 8.2.2.17.4, with the `SendNode` deviation.
    SendNode,
    /// `ActionNodeUsageDeclaration? 'send' NodeParameterMember SenderReceiverPart?`. `SysML` 8.2.2.17.4.
    SendNodeDeclaration,
    /// `'via' NodeParameterMember ( 'to' NodeParameterMember )? | EmptyParameterMember 'to' NodeParameterMember`. `SysML` 8.2.2.17.4.
    SenderReceiverPart,
    /// `ActionNodePrefix ( 'while' ExpressionParameterMember | 'loop' EmptyParameterMember ) ActionBodyParameterMember ( 'until' ExpressionParameterMember ';' )?`. `SysML` 8.2.2.17.7 — returns `WhileLoopActionUsage`.
    WhileLoopNode,
    /// `ActionNodePrefix 'for' ForVariableDeclarationMember 'in' NodeParameterMember ActionBodyParameterMember`. `SysML` 8.2.2.17.7 — returns `ForLoopActionUsage`.
    ForLoopNode,
    /// `ownedRelatedElement += ForVariableDeclaration`, by deviation `ForVariableDeclarationMember` (`follow_xtext`). `SysML` 8.2.2.17.7 — a `FeatureMembership`.
    ForVariableDeclarationMember,
    /// `UsageDeclaration`. `SysML` 8.2.2.17.7 — returns `ReferenceUsage`.
    ForVariableDeclaration,
    /// `ActionNodePrefix 'if' ExpressionParameterMember ActionBodyParameterMember ( 'else' ( ActionBodyParameterMember | IfNodeParameterMember ) )?`. `SysML` 8.2.2.17.7 — returns `IfActionUsage`.
    IfNode,
    /// `ownedRelatedElement += IfNode`. `SysML` 8.2.2.17.7 — a `ParameterMembership`.
    IfNodeParameterMember,
    /// `ownedRelatedElement += OwnedExpression`. `SysML` 8.2.2.17.7 — a `ParameterMembership`.
    ExpressionParameterMember,
    /// `ownedRelatedElement += ActionBodyParameter`. `SysML` 8.2.2.17.7 — a `ParameterMembership`.
    ActionBodyParameterMember,
    /// `( 'action' UsageDeclaration? )? '{' ActionBodyItem* '}'`. `SysML` 8.2.2.17.7 — returns `ActionUsage`.
    ActionBodyParameter,
    /// `OccurrenceUsagePrefix ActionNodeUsageDeclaration? 'terminate' NodeParameterMember? ActionBody`. `SysML` 8.2.2.17.6 — returns `TerminateActionUsage`.
    TerminateNode,
    /// `OccurrenceUsagePrefix AssignmentNodeDeclaration ActionBody`. `SysML` 8.2.2.17.5.
    AssignmentNode,
    /// `ActionNodeUsageDeclaration? 'assign' AssignmentTargetMember FeatureChainMember ':=' NodeParameterMember`. `SysML` 8.2.2.17.5.
    AssignmentNodeDeclaration,
    /// `AssignmentTargetParameter`. `SysML` 8.2.2.17.5.
    AssignmentTargetMember,
    /// `( AssignmentTargetBinding '.' )?`. `SysML` 8.2.2.17.5.
    AssignmentTargetParameter,
    /// `NonFeatureChainPrimaryExpression`, a `FeatureValue`. `SysML` 8.2.2.17.5.
    AssignmentTargetBinding,
    /// `SendNodeDeclaration ActionBody`. `SysML` 8.2.2.18.1.
    StateSendActionUsage,
    /// `AssignmentNodeDeclaration ActionBody`. `SysML` 8.2.2.18.1.
    StateAssignmentActionUsage,
    /// `SendNodeDeclaration ( '{' ActionBodyItem* '}' )?`. `SysML` 8.2.2.18.3.
    TransitionSendActionUsage,
    /// `AssignmentNodeDeclaration ( '{' ActionBodyItem* '}' )?`. `SysML` 8.2.2.18.3.
    TransitionAssignmentActionUsage,
    /// `TriggerFeatureValue`. `SysML` 8.2.2.17.4.
    TriggerValuePart,
    /// `TriggerExpression`, a `FeatureValue`. `SysML` 8.2.2.17.4.
    TriggerFeatureValue,
    /// `( 'at' | 'after' ) ArgumentMember | 'when' ArgumentExpressionMember`. `SysML` 8.2.2.17.4.
    TriggerExpression,
    /// `OccurrenceUsagePrefix 'exhibit' ( OwnedReferenceSubsetting FeatureSpecializationPart? | 'state' UsageDeclaration ) ValuePart? StateUsageBody`. `SysML` 8.2.2.18.2.
    ExhibitStateUsage,
    /// `EmptyUsage`. `SysML` 8.2.2.17.4.
    EmptyParameterMember,
    /// `{}`. `SysML` 8.2.2.17.4.
    EmptyUsage,
    /// `'~' originalPortDefinition = ~[QualifiedName]`. `SysML` 8.2.2.12.
    ConjugatedPortTyping,
    /// `OwnedMultiplicity | OwnedMultiplicity? ( 'ordered' 'nonunique'? | 'nonunique' 'ordered'? )`. `SysML` 8.2.2.6.6.
    MultiplicityPart,
    /// `ownedRelatedElement += MultiplicityRange`. `SysML` 8.2.2.6.6.
    OwnedMultiplicity,
    /// `'[' ( MultiplicityExpressionMember '..' )? MultiplicityExpressionMember ']'`. `SysML` 8.2.2.6.6.
    MultiplicityRange,
    /// `MultiplicityBounds`, `'[' ( MultiplicityExpressionMember '..' )? MultiplicityExpressionMember ']'`. `KerML` 8.2.5.11 — returns `MultiplicityRange`.
    OwnedMultiplicityRange,
    /// `LiteralExpression | FeatureReferenceExpression`. `SysML` 8.2.2.6.6.
    MultiplicityExpressionMember,
    /// `ownedRelationship += FeatureValue`. `SysML` 8.2.2.6.2.
    ValuePart,
    /// `( '=' | ':=' | 'default' ( '=' | ':=' )? ) OwnedExpression`. `SysML` 8.2.2.6.2.
    FeatureValue,
    /// `MemberPrefix 'filter' OwnedExpression ';'`. `SysML` 8.2.2.5.1.
    ElementFilterMember,
    /// `'if' ArgumentMember '?' ArgumentExpressionMember 'else' ArgumentExpressionMember EmptyResultMember`. `KerML` 8.2.5.8.1.
    ConditionalExpression,
    /// `ArgumentMember ConditionalBinaryOperator ArgumentExpressionMember EmptyResultMember`. `KerML` 8.2.5.8.1.
    ConditionalBinaryOperatorExpression,
    /// `ArgumentMember BinaryOperator ArgumentMember EmptyResultMember`. `KerML` 8.2.5.8.1.
    BinaryOperatorExpression,
    /// `UnaryOperator ArgumentMember EmptyResultMember`. `KerML` 8.2.5.8.1.
    UnaryOperatorExpression,
    /// `ArgumentMember? ( ClassificationTestOperator TypeReferenceMember | CastOperator TypeResultMember ) EmptyResultMember`. `KerML` 8.2.5.8.1.
    ClassificationExpression,
    /// `MetadataArgumentMember ( MetaclassificationTestOperator TypeReferenceMember | MetaCastOperator TypeResultMember ) EmptyResultMember`. `KerML` 8.2.5.8.1.
    MetaclassificationExpression,
    /// `'all' TypeReferenceMember`. `KerML` 8.2.5.8.1.
    ExtentExpression,
    /// `ownedMemberParameter = Argument`. `KerML` 8.2.5.8.1.
    ArgumentMember,
    /// `ownedRelationship += ArgumentValue`. `KerML` 8.2.5.8.1.
    Argument,
    /// `value = OwnedExpression`. `KerML` 8.2.5.8.1.
    ArgumentValue,
    /// `ownedRelatedElement += ArgumentExpression`. `KerML` 8.2.5.8.1.
    ArgumentExpressionMember,
    /// `ownedRelationship += ArgumentExpressionValue`. `KerML` 8.2.5.8.1.
    ArgumentExpression,
    /// `value = OwnedExpressionReference`. `KerML` 8.2.5.8.1.
    ArgumentExpressionValue,
    /// `ownedRelationship += OwnedExpressionMember`. `KerML` 8.2.5.8.1.
    OwnedExpressionReference,
    /// `ownedFeatureMember = OwnedExpression`. `KerML` 8.2.5.8.1.
    OwnedExpressionMember,
    /// `ownedRelatedElement += MetadataArgument`. `KerML` 8.2.5.8.1.
    MetadataArgumentMember,
    /// `ownedRelationship += MetadataValue`. `KerML` 8.2.5.8.1.
    MetadataArgument,
    /// `value = MetadataReference`. `KerML` 8.2.5.8.1.
    MetadataValue,
    /// `ownedRelationship += ElementReferenceMember`. `KerML` 8.2.5.8.1.
    MetadataReference,
    /// `memberElement = [QualifiedName]`. `KerML` 8.2.5.8.3.
    ElementReferenceMember,
    /// `ownedRelatedElement += EmptyFeature`. `KerML` 8.2.5.8.1.
    EmptyResultMember,
    /// `{ }`, a Feature that consumes no tokens. `KerML` 8.2.5.8.1.
    EmptyFeature,
    /// `ownedMemberFeature = TypeReference`. `KerML` 8.2.5.8.1.
    TypeReferenceMember,
    /// `ownedMemberFeature = TypeReference`. `KerML` 8.2.5.8.1.
    TypeResultMember,
    /// `ownedRelationship += ReferenceTyping`. `KerML` 8.2.5.8.1.
    TypeReference,
    /// `type = [QualifiedName]`. `KerML` 8.2.5.8.1.
    ReferenceTyping,
    /// `'(' SequenceExpressionList ')'`. `KerML` 8.2.5.8.2.
    SequenceExpression,
    /// `OwnedExpression ','? | SequenceOperatorExpression`. `KerML` 8.2.5.8.2.
    SequenceExpressionList,
    /// `OwnedExpressionMember ',' SequenceExpressionListMember`. `KerML` 8.2.5.8.2.
    SequenceOperatorExpression,
    /// `ownedRelatedElement += SequenceExpressionList`. `KerML` 8.2.5.8.2.
    SequenceExpressionListMember,
    /// `'null' | '(' ')'`. `KerML` 8.2.5.8.3.
    NullExpression,
    /// `FeatureReferenceMember EmptyResultMember`. `KerML` 8.2.5.8.3.
    FeatureReferenceExpression,
    /// `memberElement = FeatureReference`. `KerML` 8.2.5.8.3.
    FeatureReferenceMember,
    /// `[QualifiedName]`. `KerML` 8.2.5.8.3.
    FeatureReference,
    /// `InstantiatedTypeMember ArgumentList EmptyResultMember`. `KerML` 8.2.5.8.3.
    InvocationExpression,
    /// `memberElement = InstantiatedTypeReference | OwnedFeatureChainMember`. `KerML` 8.2.5.8.3 — only the first alternative is read.
    InstantiatedTypeMember,
    /// `[QualifiedName]`. `KerML` 8.2.5.8.3.
    InstantiatedTypeReference,
    /// `'new' InstantiatedTypeMember ConstructorResultMember`. `KerML` 8.2.5.8.3.
    ConstructorExpression,
    /// `ConstructorResult`. `KerML` 8.2.5.8.3.
    ConstructorResultMember,
    /// `ArgumentList`. `KerML` 8.2.5.8.3.
    ConstructorResult,
    /// `'(' ( PositionalArgumentList | NamedArgumentList )? ')'`. `KerML` 8.2.5.8.3.
    ArgumentList,
    /// `ArgumentMember ( ',' ArgumentMember )*`. `KerML` 8.2.5.8.3.
    PositionalArgumentList,
    /// `NamedArgumentMember ( ',' NamedArgumentMember )*`. `KerML` 8.2.5.8.3.
    NamedArgumentList,
    /// `ownedMemberFeature = NamedArgument`. `KerML` 8.2.5.8.3.
    NamedArgumentMember,
    /// `ParameterRedefinition '=' ArgumentValue`. `KerML` 8.2.5.8.3.
    NamedArgument,
    /// `redefinedFeature = [QualifiedName]`. `KerML` 8.2.5.8.3.
    ParameterRedefinition,
    /// `'true' | 'false'`. `KerML` 8.2.5.8.4.
    LiteralBoolean,
    /// `STRING_VALUE`. `KerML` 8.2.5.8.4.
    LiteralString,
    /// `DECIMAL_VALUE`. `KerML` 8.2.5.8.4.
    LiteralInteger,
    /// `DECIMAL_VALUE? '.' ( DECIMAL_VALUE | EXPONENTIAL_VALUE ) | EXPONENTIAL_VALUE`. `KerML` 8.2.5.8.4.
    LiteralReal,
    /// `'*'`. `KerML` 8.2.5.8.4.
    LiteralInfinity,
    /// `'abstract'? DefinitionExtensionKeyword* 'metadata' 'def' Definition`. `SysML` 8.2.2.27.
    MetadataDefinition,
    /// `UsageExtensionKeyword* ( '@' | 'metadata' ) MetadataUsageDeclaration ( 'about' Annotation ( ',' Annotation )* )? MetadataBody`. `SysML` 8.2.2.27.
    MetadataUsage,
    /// `( Identification ( ':' | 'defined' 'by' ) )? OwnedFeatureTyping`. `SysML` 8.2.2.27, as deviation `MetadataUsageDeclaration` reads it.
    MetadataUsageDeclaration,
    /// `';' | '{' ( DefinitionMember | MetadataBodyUsageMember | AliasMember | Import )* '}'`. `SysML` 8.2.2.27.
    MetadataBody,
    /// `ownedMemberFeature = MetadataBodyUsage`. `SysML` 8.2.2.27.
    MetadataBodyUsageMember,
    /// `'ref'? ( ':>>' | 'redefines' )? OwnedRedefinition FeatureSpecializationPart? ValuePart? MetadataBody`. `SysML` 8.2.2.27 — the metaclass is `ReferenceUsage`.
    MetadataBodyUsage,
    /// `'#' ownedRelatedElement = PrefixMetadataUsage`. `SysML` 8.2.2.27 — an `OwningMembership`.
    PrefixMetadataMember,
    /// `'#' annotatingElement = PrefixMetadataUsage`. `SysML` 8.2.2.27 — an `Annotation`, on a `Dependency`.
    PrefixMetadataAnnotation,
    /// `ownedRelationship += OwnedFeatureTyping`. `SysML` 8.2.2.27 — a `MetadataUsage`.
    PrefixMetadataUsage,
    /// `ownedRelationship += PrefixMetadataMember`. `SysML` 8.2.2.6.2.
    UsageExtensionKeyword,
    /// `ownedRelationship += PrefixMetadataMember`. `SysML` 8.2.2.6.1.
    DefinitionExtensionKeyword,
    /// `BasicDefinitionPrefix? DefinitionExtensionKeyword+ 'def' Definition`. `SysML` 8.2.2.27.
    ExtendedDefinition,
    /// `UnextendedUsagePrefix UsageExtensionKeyword+ Usage`. `SysML` 8.2.2.27.
    ExtendedUsage,
    /// `OccurrenceDefinitionPrefix 'interface' 'def' DefinitionDeclaration InterfaceBody`. `SysML` 8.2.2.14.1.
    InterfaceDefinition,
    /// `';' | '{' InterfaceBodyItem* '}'`. `SysML` 8.2.2.14.1.
    InterfaceBody,
    /// `MemberPrefix ownedRelatedElement += InterfaceNonOccurrenceUsageElement`. `SysML` 8.2.2.14.1.
    InterfaceNonOccurrenceUsageMember,
    /// `MemberPrefix ownedRelatedElement += InterfaceOccurrenceUsageElement`. `SysML` 8.2.2.14.1.
    InterfaceOccurrenceUsageMember,
    /// `isEnd ?= 'end' Usage`. `SysML` 8.2.2.14.1 — the metaclass is `PortUsage`.
    DefaultInterfaceEnd,
    /// `OccurrenceUsagePrefix 'interface' InterfaceUsageDeclaration InterfaceBody`. `SysML` 8.2.2.14.2.
    InterfaceUsage,
    /// `UsageDeclaration ValuePart? ( 'connect' InterfacePart )? | InterfacePart`. `SysML` 8.2.2.14.2.
    InterfaceUsageDeclaration,
    /// `InterfaceEndMember 'to' InterfaceEndMember`. `SysML` 8.2.2.14.2.
    BinaryInterfacePart,
    /// `'(' InterfaceEndMember ',' InterfaceEndMember ( ',' InterfaceEndMember )* ')'`. `SysML` 8.2.2.14.2.
    NaryInterfacePart,
    /// `ownedRelatedElement += InterfaceEnd`. `SysML` 8.2.2.14.2.
    InterfaceEndMember,
    /// `OwnedCrossMultiplicityMember? ( NAME REFERENCES )? OwnedReferenceSubsetting`. `SysML` 8.2.2.14.2 — the metaclass is `PortUsage`.
    InterfaceEnd,
    /// recovered-over text; carries its bytes so the tree stays lossless
    Error,

    /// Sentinel for rowan's `from_raw`; never produced by the lexer.
    Tombstone,
}

/// Every kind, in declaration order, so `ALL[n as usize]` is the kind whose
/// discriminant is `n`.
///
/// rowan stores a kind as a `u16` and hands it back as one. Recovering the
/// variant by index is how that round-trip stays safe: the workspace forbids
/// `unsafe_code`, so a transmute is not available and would be wrong anyway —
/// an out-of-range `u16` has to be representable as `None`, not as a variant
/// that was never constructed.
pub const ALL: &[SyntaxKind] = &[
    SyntaxKind::Whitespace,
    SyntaxKind::SingleLineNote,
    SyntaxKind::MultilineNote,
    SyntaxKind::RegularComment,
    SyntaxKind::BasicName,
    SyntaxKind::UnrestrictedName,
    SyntaxKind::DecimalValue,
    SyntaxKind::ExponentialValue,
    SyntaxKind::StringValue,
    SyntaxKind::KwAbout,
    SyntaxKind::KwAbstract,
    SyntaxKind::KwAccept,
    SyntaxKind::KwAction,
    SyntaxKind::KwActor,
    SyntaxKind::KwAfter,
    SyntaxKind::KwAlias,
    SyntaxKind::KwAll,
    SyntaxKind::KwAllocate,
    SyntaxKind::KwAllocation,
    SyntaxKind::KwAnalysis,
    SyntaxKind::KwAnd,
    SyntaxKind::KwAs,
    SyntaxKind::KwAssert,
    SyntaxKind::KwAssign,
    SyntaxKind::KwAssoc,
    SyntaxKind::KwAssume,
    SyntaxKind::KwAt,
    SyntaxKind::KwAttribute,
    SyntaxKind::KwBehavior,
    SyntaxKind::KwBind,
    SyntaxKind::KwBinding,
    SyntaxKind::KwBool,
    SyntaxKind::KwBy,
    SyntaxKind::KwCalc,
    SyntaxKind::KwCase,
    SyntaxKind::KwChains,
    SyntaxKind::KwClass,
    SyntaxKind::KwClassifier,
    SyntaxKind::KwComment,
    SyntaxKind::KwComposite,
    SyntaxKind::KwConcern,
    SyntaxKind::KwConjugate,
    SyntaxKind::KwConjugates,
    SyntaxKind::KwConjugation,
    SyntaxKind::KwConnect,
    SyntaxKind::KwConnection,
    SyntaxKind::KwConnector,
    SyntaxKind::KwConst,
    SyntaxKind::KwConstant,
    SyntaxKind::KwConstraint,
    SyntaxKind::KwCrosses,
    SyntaxKind::KwDatatype,
    SyntaxKind::KwDecide,
    SyntaxKind::KwDef,
    SyntaxKind::KwDefault,
    SyntaxKind::KwDefined,
    SyntaxKind::KwDependency,
    SyntaxKind::KwDerived,
    SyntaxKind::KwDifferences,
    SyntaxKind::KwDisjoining,
    SyntaxKind::KwDisjoint,
    SyntaxKind::KwDo,
    SyntaxKind::KwDoc,
    SyntaxKind::KwElse,
    SyntaxKind::KwEnd,
    SyntaxKind::KwEntry,
    SyntaxKind::KwEnum,
    SyntaxKind::KwEvent,
    SyntaxKind::KwExhibit,
    SyntaxKind::KwExit,
    SyntaxKind::KwExpose,
    SyntaxKind::KwExpr,
    SyntaxKind::KwFalse,
    SyntaxKind::KwFeature,
    SyntaxKind::KwFeatured,
    SyntaxKind::KwFeaturing,
    SyntaxKind::KwFilter,
    SyntaxKind::KwFirst,
    SyntaxKind::KwFlow,
    SyntaxKind::KwFor,
    SyntaxKind::KwFork,
    SyntaxKind::KwFrame,
    SyntaxKind::KwFrom,
    SyntaxKind::KwFunction,
    SyntaxKind::KwHastype,
    SyntaxKind::KwIf,
    SyntaxKind::KwImplies,
    SyntaxKind::KwImport,
    SyntaxKind::KwIn,
    SyntaxKind::KwInclude,
    SyntaxKind::KwIndividual,
    SyntaxKind::KwInout,
    SyntaxKind::KwInteraction,
    SyntaxKind::KwInterface,
    SyntaxKind::KwIntersects,
    SyntaxKind::KwInv,
    SyntaxKind::KwInverse,
    SyntaxKind::KwInverting,
    SyntaxKind::KwIstype,
    SyntaxKind::KwItem,
    SyntaxKind::KwJoin,
    SyntaxKind::KwLanguage,
    SyntaxKind::KwLibrary,
    SyntaxKind::KwLocale,
    SyntaxKind::KwLoop,
    SyntaxKind::KwMember,
    SyntaxKind::KwMerge,
    SyntaxKind::KwMessage,
    SyntaxKind::KwMeta,
    SyntaxKind::KwMetaclass,
    SyntaxKind::KwMetadata,
    SyntaxKind::KwMultiplicity,
    SyntaxKind::KwNamespace,
    SyntaxKind::KwNew,
    SyntaxKind::KwNonunique,
    SyntaxKind::KwNot,
    SyntaxKind::KwNull,
    SyntaxKind::KwObjective,
    SyntaxKind::KwOccurrence,
    SyntaxKind::KwOf,
    SyntaxKind::KwOr,
    SyntaxKind::KwOrdered,
    SyntaxKind::KwOut,
    SyntaxKind::KwPackage,
    SyntaxKind::KwParallel,
    SyntaxKind::KwPart,
    SyntaxKind::KwPerform,
    SyntaxKind::KwPort,
    SyntaxKind::KwPortion,
    SyntaxKind::KwPredicate,
    SyntaxKind::KwPrivate,
    SyntaxKind::KwProtected,
    SyntaxKind::KwPublic,
    SyntaxKind::KwRedefines,
    SyntaxKind::KwRedefinition,
    SyntaxKind::KwRef,
    SyntaxKind::KwReferences,
    SyntaxKind::KwRender,
    SyntaxKind::KwRendering,
    SyntaxKind::KwRep,
    SyntaxKind::KwRequire,
    SyntaxKind::KwRequirement,
    SyntaxKind::KwReturn,
    SyntaxKind::KwSatisfy,
    SyntaxKind::KwSend,
    SyntaxKind::KwSnapshot,
    SyntaxKind::KwSpecialization,
    SyntaxKind::KwSpecializes,
    SyntaxKind::KwStakeholder,
    SyntaxKind::KwStandard,
    SyntaxKind::KwState,
    SyntaxKind::KwStep,
    SyntaxKind::KwStruct,
    SyntaxKind::KwSubclassifier,
    SyntaxKind::KwSubject,
    SyntaxKind::KwSubset,
    SyntaxKind::KwSubsets,
    SyntaxKind::KwSubtype,
    SyntaxKind::KwSuccession,
    SyntaxKind::KwTerminate,
    SyntaxKind::KwThen,
    SyntaxKind::KwTimeslice,
    SyntaxKind::KwTo,
    SyntaxKind::KwTransition,
    SyntaxKind::KwTrue,
    SyntaxKind::KwType,
    SyntaxKind::KwTyped,
    SyntaxKind::KwTyping,
    SyntaxKind::KwUnions,
    SyntaxKind::KwUntil,
    SyntaxKind::KwUse,
    SyntaxKind::KwVar,
    SyntaxKind::KwVariant,
    SyntaxKind::KwVariation,
    SyntaxKind::KwVerification,
    SyntaxKind::KwVerify,
    SyntaxKind::KwVia,
    SyntaxKind::KwView,
    SyntaxKind::KwViewpoint,
    SyntaxKind::KwWhen,
    SyntaxKind::KwWhile,
    SyntaxKind::KwXor,
    SyntaxKind::BangEq,
    SyntaxKind::BangEqEq,
    SyntaxKind::Hash,
    SyntaxKind::Dollar,
    SyntaxKind::Percent,
    SyntaxKind::Amp,
    SyntaxKind::LParen,
    SyntaxKind::RParen,
    SyntaxKind::Star,
    SyntaxKind::StarStar,
    SyntaxKind::Plus,
    SyntaxKind::Comma,
    SyntaxKind::Minus,
    SyntaxKind::ThinArrow,
    SyntaxKind::Dot,
    SyntaxKind::DotDot,
    SyntaxKind::DotQuestion,
    SyntaxKind::Slash,
    SyntaxKind::Colon,
    SyntaxKind::ColonColon,
    SyntaxKind::ColonColonGt,
    SyntaxKind::ColonEq,
    SyntaxKind::ColonGt,
    SyntaxKind::ColonGtGt,
    SyntaxKind::Semicolon,
    SyntaxKind::Lt,
    SyntaxKind::LtEq,
    SyntaxKind::Eq,
    SyntaxKind::EqEq,
    SyntaxKind::EqEqEq,
    SyntaxKind::FatArrow,
    SyntaxKind::Gt,
    SyntaxKind::GtEq,
    SyntaxKind::Question,
    SyntaxKind::QuestionQuestion,
    SyntaxKind::At,
    SyntaxKind::AtAt,
    SyntaxKind::LBracket,
    SyntaxKind::RBracket,
    SyntaxKind::Caret,
    SyntaxKind::LBrace,
    SyntaxKind::Pipe,
    SyntaxKind::RBrace,
    SyntaxKind::Tilde,
    SyntaxKind::RootNamespace,
    SyntaxKind::Package,
    SyntaxKind::PackageDeclaration,
    SyntaxKind::LibraryPackage,
    SyntaxKind::PackageBody,
    SyntaxKind::Identification,
    SyntaxKind::QualifiedName,
    SyntaxKind::Import,
    SyntaxKind::VisibilityIndicator,
    SyntaxKind::ImportDeclaration,
    SyntaxKind::MembershipImport,
    SyntaxKind::NamespaceImport,
    SyntaxKind::FilterPackage,
    SyntaxKind::FilterPackageImport,
    SyntaxKind::FilterPackageMember,
    SyntaxKind::RelationshipBody,
    SyntaxKind::PackageMember,
    SyntaxKind::Classifier,
    SyntaxKind::Class,
    SyntaxKind::Structure,
    SyntaxKind::DataType,
    SyntaxKind::Metaclass,
    SyntaxKind::Association,
    SyntaxKind::Behavior,
    SyntaxKind::Interaction,
    SyntaxKind::TypePrefix,
    SyntaxKind::ClassifierDeclaration,
    SyntaxKind::TypeBody,
    SyntaxKind::SuperclassingPart,
    SyntaxKind::Feature,
    SyntaxKind::FeaturePrefix,
    SyntaxKind::BasicFeaturePrefix,
    SyntaxKind::EndFeaturePrefix,
    SyntaxKind::FeatureDeclaration,
    SyntaxKind::FeatureIdentification,
    SyntaxKind::Connector,
    SyntaxKind::BinaryConnectorDeclaration,
    SyntaxKind::NaryConnectorDeclaration,
    SyntaxKind::Succession,
    SyntaxKind::SuccessionDeclaration,
    SyntaxKind::BindingConnector,
    SyntaxKind::BindingConnectorDeclaration,
    SyntaxKind::NamespaceFeatureMember,
    SyntaxKind::NonFeatureMember,
    SyntaxKind::MemberPrefix,
    SyntaxKind::AliasMember,
    SyntaxKind::OwnedAnnotation,
    SyntaxKind::Annotation,
    SyntaxKind::Comment,
    SyntaxKind::Documentation,
    SyntaxKind::TextualRepresentation,
    SyntaxKind::PartDefinition,
    SyntaxKind::AttributeDefinition,
    SyntaxKind::OccurrenceDefinition,
    SyntaxKind::ItemDefinition,
    SyntaxKind::ConnectionDefinition,
    SyntaxKind::FlowDefinition,
    SyntaxKind::FlowUsage,
    SyntaxKind::SuccessionFlowUsage,
    SyntaxKind::Message,
    SyntaxKind::MessageDeclaration,
    SyntaxKind::MessageEventMember,
    SyntaxKind::MessageEvent,
    SyntaxKind::FlowDeclaration,
    SyntaxKind::FlowEndMember,
    SyntaxKind::FlowEnd,
    SyntaxKind::FlowEndSubsetting,
    SyntaxKind::FeatureChainPrefix,
    SyntaxKind::FlowFeatureMember,
    SyntaxKind::FlowFeature,
    SyntaxKind::FlowFeatureRedefinition,
    SyntaxKind::FlowPayloadFeatureMember,
    SyntaxKind::FlowPayloadFeature,
    SyntaxKind::PayloadFeature,
    SyntaxKind::PayloadFeatureSpecializationPart,
    SyntaxKind::AllocationDefinition,
    SyntaxKind::RenderingDefinition,
    SyntaxKind::PortDefinition,
    SyntaxKind::ConjugatedPortDefinitionMember,
    SyntaxKind::ConjugatedPortDefinition,
    SyntaxKind::PortConjugation,
    SyntaxKind::DefinitionPrefix,
    SyntaxKind::OccurrenceDefinitionPrefix,
    SyntaxKind::BasicDefinitionPrefix,
    SyntaxKind::IndividualDefinition,
    SyntaxKind::EmptyMultiplicityMember,
    SyntaxKind::EmptyMultiplicity,
    SyntaxKind::Definition,
    SyntaxKind::DefinitionDeclaration,
    SyntaxKind::SubclassificationPart,
    SyntaxKind::OwnedSubclassification,
    SyntaxKind::DefinitionBody,
    SyntaxKind::DefinitionMember,
    SyntaxKind::NonOccurrenceUsageMember,
    SyntaxKind::OccurrenceUsageMember,
    SyntaxKind::StructureUsageMember,
    SyntaxKind::BehaviorUsageMember,
    SyntaxKind::RequirementDefinition,
    SyntaxKind::RequirementBody,
    SyntaxKind::SubjectMember,
    SyntaxKind::SubjectUsage,
    SyntaxKind::ActorMember,
    SyntaxKind::ActorUsage,
    SyntaxKind::StakeholderMember,
    SyntaxKind::StakeholderUsage,
    SyntaxKind::FramedConcernMember,
    SyntaxKind::FramedConcernUsage,
    SyntaxKind::ConcernDefinition,
    SyntaxKind::ConcernUsage,
    SyntaxKind::ViewDefinition,
    SyntaxKind::ViewDefinitionBody,
    SyntaxKind::ViewRenderingMember,
    SyntaxKind::ViewRenderingUsage,
    SyntaxKind::ViewUsage,
    SyntaxKind::ViewBody,
    SyntaxKind::Expose,
    SyntaxKind::MembershipExpose,
    SyntaxKind::NamespaceExpose,
    SyntaxKind::ViewpointDefinition,
    SyntaxKind::ViewpointUsage,
    SyntaxKind::RequirementConstraintMember,
    SyntaxKind::RequirementKind,
    SyntaxKind::RequirementVerificationMember,
    SyntaxKind::RequirementVerificationUsage,
    SyntaxKind::RequirementConstraintUsage,
    SyntaxKind::ConstraintUsageDeclaration,
    SyntaxKind::RequirementUsage,
    SyntaxKind::SatisfyRequirementUsage,
    SyntaxKind::SatisfactionSubjectMember,
    SyntaxKind::SatisfactionParameter,
    SyntaxKind::SatisfactionFeatureValue,
    SyntaxKind::SatisfactionReferenceExpression,
    SyntaxKind::ConstraintUsage,
    SyntaxKind::AssertConstraintUsage,
    SyntaxKind::ConstraintDefinition,
    SyntaxKind::ReturnParameterMember,
    SyntaxKind::InitialNodeMember,
    SyntaxKind::ActionTargetSuccessionMember,
    SyntaxKind::ActionTargetSuccession,
    SyntaxKind::TargetSuccession,
    SyntaxKind::GuardedTargetSuccession,
    SyntaxKind::GuardedSuccessionMember,
    SyntaxKind::GuardedSuccession,
    SyntaxKind::SuccessionAsUsage,
    SyntaxKind::BindingConnectorAsUsage,
    SyntaxKind::FeatureChainMember,
    SyntaxKind::OwnedFeatureChainMember,
    SyntaxKind::DefaultTargetSuccession,
    SyntaxKind::GuardExpressionMember,
    SyntaxKind::TransitionSuccessionMember,
    SyntaxKind::TransitionSuccession,
    SyntaxKind::EmptyEndMember,
    SyntaxKind::SourceEndMember,
    SyntaxKind::SourceEnd,
    SyntaxKind::SourceSuccessionMember,
    SyntaxKind::SourceSuccession,
    SyntaxKind::ConnectorEndMember,
    SyntaxKind::ConnectorEnd,
    SyntaxKind::OwnedCrossMultiplicityMember,
    SyntaxKind::EndUsagePrefix,
    SyntaxKind::OwnedCrossFeatureMember,
    SyntaxKind::OwnedCrossFeature,
    SyntaxKind::OwnedCrossMultiplicity,
    SyntaxKind::ConnectionUsage,
    SyntaxKind::AllocationUsage,
    SyntaxKind::AllocationUsageDeclaration,
    SyntaxKind::BinaryConnectorPart,
    SyntaxKind::NaryConnectorPart,
    SyntaxKind::CalculationDefinition,
    SyntaxKind::ActionDefinition,
    SyntaxKind::ActionBody,
    SyntaxKind::OwnedFeatureChain,
    SyntaxKind::OwnedFeatureChaining,
    SyntaxKind::ActionUsage,
    SyntaxKind::ActionUsageDeclaration,
    SyntaxKind::CalculationUsage,
    SyntaxKind::PerformActionUsage,
    SyntaxKind::PerformActionUsageDeclaration,
    SyntaxKind::ActionNodeMember,
    SyntaxKind::ControlNodePrefix,
    SyntaxKind::MergeNode,
    SyntaxKind::DecisionNode,
    SyntaxKind::JoinNode,
    SyntaxKind::ForkNode,
    SyntaxKind::CalculationBody,
    SyntaxKind::CalculationBodyPart,
    SyntaxKind::CaseBody,
    SyntaxKind::CaseDefinition,
    SyntaxKind::CaseUsage,
    SyntaxKind::ObjectiveMember,
    SyntaxKind::ObjectiveRequirementUsage,
    SyntaxKind::AnalysisCaseDefinition,
    SyntaxKind::AnalysisCaseUsage,
    SyntaxKind::VerificationCaseDefinition,
    SyntaxKind::VerificationCaseUsage,
    SyntaxKind::UseCaseDefinition,
    SyntaxKind::UseCaseUsage,
    SyntaxKind::IncludeUseCaseUsage,
    SyntaxKind::FeatureChainExpression,
    SyntaxKind::NonFeatureChainPrimaryArgumentMember,
    SyntaxKind::BracketExpression,
    SyntaxKind::PrimaryArgumentMember,
    SyntaxKind::PrimaryArgument,
    SyntaxKind::PrimaryArgumentValue,
    SyntaxKind::IndexExpression,
    SyntaxKind::CollectExpression,
    SyntaxKind::SelectExpression,
    SyntaxKind::FunctionOperationExpression,
    SyntaxKind::BodyArgumentMember,
    SyntaxKind::BodyArgument,
    SyntaxKind::BodyArgumentValue,
    SyntaxKind::FunctionReferenceArgumentMember,
    SyntaxKind::FunctionReferenceArgument,
    SyntaxKind::FunctionReferenceArgumentValue,
    SyntaxKind::FunctionReferenceExpression,
    SyntaxKind::FunctionReferenceMember,
    SyntaxKind::FunctionReference,
    SyntaxKind::BodyExpression,
    SyntaxKind::ExpressionBodyMember,
    SyntaxKind::ExpressionBody,
    SyntaxKind::ResultExpressionMember,
    SyntaxKind::PartUsage,
    SyntaxKind::ReferenceUsage,
    SyntaxKind::DefaultReferenceUsage,
    SyntaxKind::AttributeUsage,
    SyntaxKind::ItemUsage,
    SyntaxKind::OccurrenceUsage,
    SyntaxKind::IndividualUsage,
    SyntaxKind::PortionUsage,
    SyntaxKind::EventOccurrenceUsage,
    SyntaxKind::PortUsage,
    SyntaxKind::RenderingUsage,
    SyntaxKind::EnumerationUsage,
    SyntaxKind::EnumerationDefinition,
    SyntaxKind::EnumerationBody,
    SyntaxKind::EnumerationUsageMember,
    SyntaxKind::EnumeratedValue,
    SyntaxKind::AnnotatingMember,
    SyntaxKind::VariantUsageMember,
    SyntaxKind::VariantReference,
    SyntaxKind::Dependency,
    SyntaxKind::DependencyDeclaration,
    SyntaxKind::UsagePrefix,
    SyntaxKind::OccurrenceUsagePrefix,
    SyntaxKind::BasicUsagePrefix,
    SyntaxKind::RefPrefix,
    SyntaxKind::FeatureDirection,
    SyntaxKind::PortionKind,
    SyntaxKind::Usage,
    SyntaxKind::UsageDeclaration,
    SyntaxKind::UsageCompletion,
    SyntaxKind::UsageBody,
    SyntaxKind::FeatureSpecializationPart,
    SyntaxKind::Typings,
    SyntaxKind::Subsettings,
    SyntaxKind::Subsets,
    SyntaxKind::OwnedSubsetting,
    SyntaxKind::Redefinitions,
    SyntaxKind::Redefines,
    SyntaxKind::OwnedRedefinition,
    SyntaxKind::References,
    SyntaxKind::OwnedReferenceSubsetting,
    SyntaxKind::Crosses,
    SyntaxKind::OwnedCrossSubsetting,
    SyntaxKind::TypedBy,
    SyntaxKind::FeatureTyping,
    SyntaxKind::OwnedFeatureTyping,
    SyntaxKind::StateDefinition,
    SyntaxKind::StateDefBody,
    SyntaxKind::StateUsage,
    SyntaxKind::StateUsageBody,
    SyntaxKind::TransitionUsageMember,
    SyntaxKind::TransitionUsage,
    SyntaxKind::TargetTransitionUsageMember,
    SyntaxKind::TargetTransitionUsage,
    SyntaxKind::TriggerActionMember,
    SyntaxKind::TriggerAction,
    SyntaxKind::AcceptParameterPart,
    SyntaxKind::PayloadParameterMember,
    SyntaxKind::PayloadParameter,
    SyntaxKind::NodeParameterMember,
    SyntaxKind::NodeParameter,
    SyntaxKind::FeatureBinding,
    SyntaxKind::EntryActionMember,
    SyntaxKind::DoActionMember,
    SyntaxKind::ExitActionMember,
    SyntaxKind::EntryTransitionMember,
    SyntaxKind::EmptyActionUsage,
    SyntaxKind::StatePerformActionUsage,
    SyntaxKind::StateAcceptActionUsage,
    SyntaxKind::EffectBehaviorMember,
    SyntaxKind::TransitionPerformActionUsage,
    SyntaxKind::TransitionAcceptActionUsage,
    SyntaxKind::AcceptNodeDeclaration,
    SyntaxKind::ActionNodeUsageDeclaration,
    SyntaxKind::AcceptNode,
    SyntaxKind::SendNode,
    SyntaxKind::SendNodeDeclaration,
    SyntaxKind::SenderReceiverPart,
    SyntaxKind::WhileLoopNode,
    SyntaxKind::ForLoopNode,
    SyntaxKind::ForVariableDeclarationMember,
    SyntaxKind::ForVariableDeclaration,
    SyntaxKind::IfNode,
    SyntaxKind::IfNodeParameterMember,
    SyntaxKind::ExpressionParameterMember,
    SyntaxKind::ActionBodyParameterMember,
    SyntaxKind::ActionBodyParameter,
    SyntaxKind::TerminateNode,
    SyntaxKind::AssignmentNode,
    SyntaxKind::AssignmentNodeDeclaration,
    SyntaxKind::AssignmentTargetMember,
    SyntaxKind::AssignmentTargetParameter,
    SyntaxKind::AssignmentTargetBinding,
    SyntaxKind::StateSendActionUsage,
    SyntaxKind::StateAssignmentActionUsage,
    SyntaxKind::TransitionSendActionUsage,
    SyntaxKind::TransitionAssignmentActionUsage,
    SyntaxKind::TriggerValuePart,
    SyntaxKind::TriggerFeatureValue,
    SyntaxKind::TriggerExpression,
    SyntaxKind::ExhibitStateUsage,
    SyntaxKind::EmptyParameterMember,
    SyntaxKind::EmptyUsage,
    SyntaxKind::ConjugatedPortTyping,
    SyntaxKind::MultiplicityPart,
    SyntaxKind::OwnedMultiplicity,
    SyntaxKind::MultiplicityRange,
    SyntaxKind::OwnedMultiplicityRange,
    SyntaxKind::MultiplicityExpressionMember,
    SyntaxKind::ValuePart,
    SyntaxKind::FeatureValue,
    SyntaxKind::ElementFilterMember,
    SyntaxKind::ConditionalExpression,
    SyntaxKind::ConditionalBinaryOperatorExpression,
    SyntaxKind::BinaryOperatorExpression,
    SyntaxKind::UnaryOperatorExpression,
    SyntaxKind::ClassificationExpression,
    SyntaxKind::MetaclassificationExpression,
    SyntaxKind::ExtentExpression,
    SyntaxKind::ArgumentMember,
    SyntaxKind::Argument,
    SyntaxKind::ArgumentValue,
    SyntaxKind::ArgumentExpressionMember,
    SyntaxKind::ArgumentExpression,
    SyntaxKind::ArgumentExpressionValue,
    SyntaxKind::OwnedExpressionReference,
    SyntaxKind::OwnedExpressionMember,
    SyntaxKind::MetadataArgumentMember,
    SyntaxKind::MetadataArgument,
    SyntaxKind::MetadataValue,
    SyntaxKind::MetadataReference,
    SyntaxKind::ElementReferenceMember,
    SyntaxKind::EmptyResultMember,
    SyntaxKind::EmptyFeature,
    SyntaxKind::TypeReferenceMember,
    SyntaxKind::TypeResultMember,
    SyntaxKind::TypeReference,
    SyntaxKind::ReferenceTyping,
    SyntaxKind::SequenceExpression,
    SyntaxKind::SequenceExpressionList,
    SyntaxKind::SequenceOperatorExpression,
    SyntaxKind::SequenceExpressionListMember,
    SyntaxKind::NullExpression,
    SyntaxKind::FeatureReferenceExpression,
    SyntaxKind::FeatureReferenceMember,
    SyntaxKind::FeatureReference,
    SyntaxKind::InvocationExpression,
    SyntaxKind::InstantiatedTypeMember,
    SyntaxKind::InstantiatedTypeReference,
    SyntaxKind::ConstructorExpression,
    SyntaxKind::ConstructorResultMember,
    SyntaxKind::ConstructorResult,
    SyntaxKind::ArgumentList,
    SyntaxKind::PositionalArgumentList,
    SyntaxKind::NamedArgumentList,
    SyntaxKind::NamedArgumentMember,
    SyntaxKind::NamedArgument,
    SyntaxKind::ParameterRedefinition,
    SyntaxKind::LiteralBoolean,
    SyntaxKind::LiteralString,
    SyntaxKind::LiteralInteger,
    SyntaxKind::LiteralReal,
    SyntaxKind::LiteralInfinity,
    SyntaxKind::MetadataDefinition,
    SyntaxKind::MetadataUsage,
    SyntaxKind::MetadataUsageDeclaration,
    SyntaxKind::MetadataBody,
    SyntaxKind::MetadataBodyUsageMember,
    SyntaxKind::MetadataBodyUsage,
    SyntaxKind::PrefixMetadataMember,
    SyntaxKind::PrefixMetadataAnnotation,
    SyntaxKind::PrefixMetadataUsage,
    SyntaxKind::UsageExtensionKeyword,
    SyntaxKind::DefinitionExtensionKeyword,
    SyntaxKind::ExtendedDefinition,
    SyntaxKind::ExtendedUsage,
    SyntaxKind::InterfaceDefinition,
    SyntaxKind::InterfaceBody,
    SyntaxKind::InterfaceNonOccurrenceUsageMember,
    SyntaxKind::InterfaceOccurrenceUsageMember,
    SyntaxKind::DefaultInterfaceEnd,
    SyntaxKind::InterfaceUsage,
    SyntaxKind::InterfaceUsageDeclaration,
    SyntaxKind::BinaryInterfacePart,
    SyntaxKind::NaryInterfacePart,
    SyntaxKind::InterfaceEndMember,
    SyntaxKind::InterfaceEnd,
    SyntaxKind::Error,
    SyntaxKind::Tombstone,
];

/// Every keyword of the language, paired with its kind, sorted by text.
pub const KEYWORDS: &[(&str, SyntaxKind)] = &[
    ("about", SyntaxKind::KwAbout),
    ("abstract", SyntaxKind::KwAbstract),
    ("accept", SyntaxKind::KwAccept),
    ("action", SyntaxKind::KwAction),
    ("actor", SyntaxKind::KwActor),
    ("after", SyntaxKind::KwAfter),
    ("alias", SyntaxKind::KwAlias),
    ("all", SyntaxKind::KwAll),
    ("allocate", SyntaxKind::KwAllocate),
    ("allocation", SyntaxKind::KwAllocation),
    ("analysis", SyntaxKind::KwAnalysis),
    ("and", SyntaxKind::KwAnd),
    ("as", SyntaxKind::KwAs),
    ("assert", SyntaxKind::KwAssert),
    ("assign", SyntaxKind::KwAssign),
    ("assoc", SyntaxKind::KwAssoc),
    ("assume", SyntaxKind::KwAssume),
    ("at", SyntaxKind::KwAt),
    ("attribute", SyntaxKind::KwAttribute),
    ("behavior", SyntaxKind::KwBehavior),
    ("bind", SyntaxKind::KwBind),
    ("binding", SyntaxKind::KwBinding),
    ("bool", SyntaxKind::KwBool),
    ("by", SyntaxKind::KwBy),
    ("calc", SyntaxKind::KwCalc),
    ("case", SyntaxKind::KwCase),
    ("chains", SyntaxKind::KwChains),
    ("class", SyntaxKind::KwClass),
    ("classifier", SyntaxKind::KwClassifier),
    ("comment", SyntaxKind::KwComment),
    ("composite", SyntaxKind::KwComposite),
    ("concern", SyntaxKind::KwConcern),
    ("conjugate", SyntaxKind::KwConjugate),
    ("conjugates", SyntaxKind::KwConjugates),
    ("conjugation", SyntaxKind::KwConjugation),
    ("connect", SyntaxKind::KwConnect),
    ("connection", SyntaxKind::KwConnection),
    ("connector", SyntaxKind::KwConnector),
    ("const", SyntaxKind::KwConst),
    ("constant", SyntaxKind::KwConstant),
    ("constraint", SyntaxKind::KwConstraint),
    ("crosses", SyntaxKind::KwCrosses),
    ("datatype", SyntaxKind::KwDatatype),
    ("decide", SyntaxKind::KwDecide),
    ("def", SyntaxKind::KwDef),
    ("default", SyntaxKind::KwDefault),
    ("defined", SyntaxKind::KwDefined),
    ("dependency", SyntaxKind::KwDependency),
    ("derived", SyntaxKind::KwDerived),
    ("differences", SyntaxKind::KwDifferences),
    ("disjoining", SyntaxKind::KwDisjoining),
    ("disjoint", SyntaxKind::KwDisjoint),
    ("do", SyntaxKind::KwDo),
    ("doc", SyntaxKind::KwDoc),
    ("else", SyntaxKind::KwElse),
    ("end", SyntaxKind::KwEnd),
    ("entry", SyntaxKind::KwEntry),
    ("enum", SyntaxKind::KwEnum),
    ("event", SyntaxKind::KwEvent),
    ("exhibit", SyntaxKind::KwExhibit),
    ("exit", SyntaxKind::KwExit),
    ("expose", SyntaxKind::KwExpose),
    ("expr", SyntaxKind::KwExpr),
    ("false", SyntaxKind::KwFalse),
    ("feature", SyntaxKind::KwFeature),
    ("featured", SyntaxKind::KwFeatured),
    ("featuring", SyntaxKind::KwFeaturing),
    ("filter", SyntaxKind::KwFilter),
    ("first", SyntaxKind::KwFirst),
    ("flow", SyntaxKind::KwFlow),
    ("for", SyntaxKind::KwFor),
    ("fork", SyntaxKind::KwFork),
    ("frame", SyntaxKind::KwFrame),
    ("from", SyntaxKind::KwFrom),
    ("function", SyntaxKind::KwFunction),
    ("hastype", SyntaxKind::KwHastype),
    ("if", SyntaxKind::KwIf),
    ("implies", SyntaxKind::KwImplies),
    ("import", SyntaxKind::KwImport),
    ("in", SyntaxKind::KwIn),
    ("include", SyntaxKind::KwInclude),
    ("individual", SyntaxKind::KwIndividual),
    ("inout", SyntaxKind::KwInout),
    ("interaction", SyntaxKind::KwInteraction),
    ("interface", SyntaxKind::KwInterface),
    ("intersects", SyntaxKind::KwIntersects),
    ("inv", SyntaxKind::KwInv),
    ("inverse", SyntaxKind::KwInverse),
    ("inverting", SyntaxKind::KwInverting),
    ("istype", SyntaxKind::KwIstype),
    ("item", SyntaxKind::KwItem),
    ("join", SyntaxKind::KwJoin),
    ("language", SyntaxKind::KwLanguage),
    ("library", SyntaxKind::KwLibrary),
    ("locale", SyntaxKind::KwLocale),
    ("loop", SyntaxKind::KwLoop),
    ("member", SyntaxKind::KwMember),
    ("merge", SyntaxKind::KwMerge),
    ("message", SyntaxKind::KwMessage),
    ("meta", SyntaxKind::KwMeta),
    ("metaclass", SyntaxKind::KwMetaclass),
    ("metadata", SyntaxKind::KwMetadata),
    ("multiplicity", SyntaxKind::KwMultiplicity),
    ("namespace", SyntaxKind::KwNamespace),
    ("new", SyntaxKind::KwNew),
    ("nonunique", SyntaxKind::KwNonunique),
    ("not", SyntaxKind::KwNot),
    ("null", SyntaxKind::KwNull),
    ("objective", SyntaxKind::KwObjective),
    ("occurrence", SyntaxKind::KwOccurrence),
    ("of", SyntaxKind::KwOf),
    ("or", SyntaxKind::KwOr),
    ("ordered", SyntaxKind::KwOrdered),
    ("out", SyntaxKind::KwOut),
    ("package", SyntaxKind::KwPackage),
    ("parallel", SyntaxKind::KwParallel),
    ("part", SyntaxKind::KwPart),
    ("perform", SyntaxKind::KwPerform),
    ("port", SyntaxKind::KwPort),
    ("portion", SyntaxKind::KwPortion),
    ("predicate", SyntaxKind::KwPredicate),
    ("private", SyntaxKind::KwPrivate),
    ("protected", SyntaxKind::KwProtected),
    ("public", SyntaxKind::KwPublic),
    ("redefines", SyntaxKind::KwRedefines),
    ("redefinition", SyntaxKind::KwRedefinition),
    ("ref", SyntaxKind::KwRef),
    ("references", SyntaxKind::KwReferences),
    ("render", SyntaxKind::KwRender),
    ("rendering", SyntaxKind::KwRendering),
    ("rep", SyntaxKind::KwRep),
    ("require", SyntaxKind::KwRequire),
    ("requirement", SyntaxKind::KwRequirement),
    ("return", SyntaxKind::KwReturn),
    ("satisfy", SyntaxKind::KwSatisfy),
    ("send", SyntaxKind::KwSend),
    ("snapshot", SyntaxKind::KwSnapshot),
    ("specialization", SyntaxKind::KwSpecialization),
    ("specializes", SyntaxKind::KwSpecializes),
    ("stakeholder", SyntaxKind::KwStakeholder),
    ("standard", SyntaxKind::KwStandard),
    ("state", SyntaxKind::KwState),
    ("step", SyntaxKind::KwStep),
    ("struct", SyntaxKind::KwStruct),
    ("subclassifier", SyntaxKind::KwSubclassifier),
    ("subject", SyntaxKind::KwSubject),
    ("subset", SyntaxKind::KwSubset),
    ("subsets", SyntaxKind::KwSubsets),
    ("subtype", SyntaxKind::KwSubtype),
    ("succession", SyntaxKind::KwSuccession),
    ("terminate", SyntaxKind::KwTerminate),
    ("then", SyntaxKind::KwThen),
    ("timeslice", SyntaxKind::KwTimeslice),
    ("to", SyntaxKind::KwTo),
    ("transition", SyntaxKind::KwTransition),
    ("true", SyntaxKind::KwTrue),
    ("type", SyntaxKind::KwType),
    ("typed", SyntaxKind::KwTyped),
    ("typing", SyntaxKind::KwTyping),
    ("unions", SyntaxKind::KwUnions),
    ("until", SyntaxKind::KwUntil),
    ("use", SyntaxKind::KwUse),
    ("var", SyntaxKind::KwVar),
    ("variant", SyntaxKind::KwVariant),
    ("variation", SyntaxKind::KwVariation),
    ("verification", SyntaxKind::KwVerification),
    ("verify", SyntaxKind::KwVerify),
    ("via", SyntaxKind::KwVia),
    ("view", SyntaxKind::KwView),
    ("viewpoint", SyntaxKind::KwViewpoint),
    ("when", SyntaxKind::KwWhen),
    ("while", SyntaxKind::KwWhile),
    ("xor", SyntaxKind::KwXor),
];

/// Every operator, paired with its kind, sorted longest first.
///
/// Longest first is what makes maximal munch correct: `::>` must be tried
/// before `::`, and `::` before `:`, or the lexer splits a token.
pub const OPERATORS: &[(&str, SyntaxKind)] = &[
    ("!==", SyntaxKind::BangEqEq),
    ("::>", SyntaxKind::ColonColonGt),
    (":>>", SyntaxKind::ColonGtGt),
    ("===", SyntaxKind::EqEqEq),
    ("!=", SyntaxKind::BangEq),
    ("**", SyntaxKind::StarStar),
    ("->", SyntaxKind::ThinArrow),
    ("..", SyntaxKind::DotDot),
    (".?", SyntaxKind::DotQuestion),
    ("::", SyntaxKind::ColonColon),
    (":=", SyntaxKind::ColonEq),
    (":>", SyntaxKind::ColonGt),
    ("<=", SyntaxKind::LtEq),
    ("==", SyntaxKind::EqEq),
    ("=>", SyntaxKind::FatArrow),
    (">=", SyntaxKind::GtEq),
    ("??", SyntaxKind::QuestionQuestion),
    ("@@", SyntaxKind::AtAt),
    ("#", SyntaxKind::Hash),
    ("$", SyntaxKind::Dollar),
    ("%", SyntaxKind::Percent),
    ("&", SyntaxKind::Amp),
    ("(", SyntaxKind::LParen),
    (")", SyntaxKind::RParen),
    ("*", SyntaxKind::Star),
    ("+", SyntaxKind::Plus),
    (",", SyntaxKind::Comma),
    ("-", SyntaxKind::Minus),
    (".", SyntaxKind::Dot),
    ("/", SyntaxKind::Slash),
    (":", SyntaxKind::Colon),
    (";", SyntaxKind::Semicolon),
    ("<", SyntaxKind::Lt),
    ("=", SyntaxKind::Eq),
    (">", SyntaxKind::Gt),
    ("?", SyntaxKind::Question),
    ("@", SyntaxKind::At),
    ("[", SyntaxKind::LBracket),
    ("]", SyntaxKind::RBracket),
    ("^", SyntaxKind::Caret),
    ("{", SyntaxKind::LBrace),
    ("|", SyntaxKind::Pipe),
    ("}", SyntaxKind::RBrace),
    ("~", SyntaxKind::Tilde),
];
