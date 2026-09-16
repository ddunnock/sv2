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
    /// `PackageBodyElement*` — the whole file. `SysML` 8.2.2.5.1.
    RootNamespace,
    /// `PrefixMetadataMember* PackageDeclaration PackageBody`. `SysML` 8.2.2.5.1.
    Package,
    /// `'package' Identification`. `SysML` 8.2.2.5.1.
    PackageDeclaration,
    /// `';' | '{' PackageBodyElement* '}'`. `SysML` 8.2.2.5.1.
    PackageBody,
    /// `( '<' NAME '>' )? ( NAME )?`. `SysML` 8.2.2.2.
    Identification,
    /// `( '$' '::' )? ( NAME '::' )* NAME`. `KerML` 8.2.3.4.1.
    QualifiedName,
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
    SyntaxKind::PackageBody,
    SyntaxKind::Identification,
    SyntaxKind::QualifiedName,
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
