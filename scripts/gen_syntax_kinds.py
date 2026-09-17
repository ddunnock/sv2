# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""Generate the SyntaxKind enum from the pinned token set.

docs/DERIVATION.md says keywords.json drives the lexer. This is what makes that
true rather than aspirational: the keyword and operator variants, and the lookup
tables the lexer uses, are emitted from .claude/state/grammar/keywords.json, so a
re-pin that changes the token set changes the enum and the gate notices.

Node kinds are authored here, not derived — a node is a decision about tree shape,
and nothing in the pinned inputs implies one. Lexical token kinds are authored too,
because the specification names them (KerML 8.2.2) while the Xtext hides them inside
terminal regexes.

    python3.11 scripts/gen_syntax_kinds.py           write the generated module
    python3.11 scripts/gen_syntax_kinds.py --check   fail if it is stale
"""

from __future__ import annotations

import argparse
import json
import os
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TOKENS = Path(".claude/state/grammar/keywords.json")
OUT = Path("crates/sv2-syntax/src/generated/kinds.rs")
GENERATED_BY = "scripts/gen_syntax_kinds.py"

# Symbol -> Rust variant name. Explicit rather than transliterated: a generated
# identifier nobody chose is a name that turns up in every match arm downstream.
# An operator absent from this table stops the generator instead of being renamed
# into something unreadable.
SYMBOLS = {
    "!=": "BangEq",
    "!==": "BangEqEq",
    "#": "Hash",
    "$": "Dollar",
    "%": "Percent",
    "&": "Amp",
    "(": "LParen",
    ")": "RParen",
    "*": "Star",
    "**": "StarStar",
    "+": "Plus",
    ",": "Comma",
    "-": "Minus",
    "->": "ThinArrow",
    ".": "Dot",
    "..": "DotDot",
    ".?": "DotQuestion",
    "/": "Slash",
    ":": "Colon",
    "::": "ColonColon",
    "::>": "ColonColonGt",
    ":=": "ColonEq",
    ":>": "ColonGt",
    ":>>": "ColonGtGt",
    ";": "Semicolon",
    "<": "Lt",
    "<=": "LtEq",
    "=": "Eq",
    "==": "EqEq",
    "===": "EqEqEq",
    "=>": "FatArrow",
    ">": "Gt",
    ">=": "GtEq",
    "?": "Question",
    "??": "QuestionQuestion",
    "@": "At",
    "@@": "AtAt",
    "[": "LBracket",
    "]": "RBracket",
    "^": "Caret",
    "{": "LBrace",
    "|": "Pipe",
    "}": "RBrace",
    "~": "Tilde",
}

# Lexical tokens the specification names at KerML 8.2.2, with the clause each is
# implemented from. Trivia is listed here too: it is attached to the tree, never
# skipped, or the losslessness invariant fails (ADR-0004).
LEXICAL = [
    ("Whitespace", "WHITE_SPACE", "8.2.2.1", "trivia"),
    ("SingleLineNote", "SINGLE_LINE_NOTE", "8.2.2.2", "trivia"),
    ("MultilineNote", "MULTILINE_NOTE", "8.2.2.2", "trivia"),
    ("RegularComment", "REGULAR_COMMENT", "8.2.2.2", "trivia"),
    ("BasicName", "BASIC_NAME", "8.2.2.3", "token"),
    ("UnrestrictedName", "UNRESTRICTED_NAME", "8.2.2.3", "token"),
    ("DecimalValue", "DECIMAL_VALUE", "8.2.2.4", "token"),
    ("ExponentialValue", "EXPONENTIAL_VALUE", "8.2.2.4", "token"),
    ("StringValue", "STRING_VALUE", "8.2.2.5", "token"),
]

# Tree shape. Authored, because no pinned input implies a node.
NODES = [
    ("RootNamespace", "`PackageBodyElement*` — the whole file. `SysML` 8.2.2.5.1."),
    ("Package", "`PrefixMetadataMember* PackageDeclaration PackageBody`. `SysML` 8.2.2.5.1."),
    ("PackageDeclaration", "`'package' Identification`. `SysML` 8.2.2.5.1."),
    ("PackageBody", "`';' | '{' PackageBodyElement* '}'`. `SysML` 8.2.2.5.1."),
    ("Identification", "`( '<' NAME '>' )? ( NAME )?`. `SysML` 8.2.2.2."),
    ("QualifiedName", "`( '$' '::' )? ( NAME '::' )* NAME`. `KerML` 8.2.3.4.1."),
    (
        "Import",
        (
            "`VisibilityIndicator 'import' 'all'? ImportDeclaration RelationshipBody`. "
            "`SysML` 8.2.2.5.1."
        ),
    ),
    ("VisibilityIndicator", "`'public' | 'private' | 'protected'`. `SysML` 8.2.2.5.1."),
    ("ImportDeclaration", "`MembershipImport | NamespaceImport`. `SysML` 8.2.2.5.1."),
    ("MembershipImport", "`[QualifiedName] ( '::' '**' )?`. `SysML` 8.2.2.5.1."),
    ("NamespaceImport", "`[QualifiedName] '::' '*' ( '::' '**' )?`. `SysML` 8.2.2.5.1."),
    ("RelationshipBody", "`';' | '{' OwnedAnnotation* '}'`. `SysML` 8.2.2.2."),
    (
        "PackageMember",
        "`MemberPrefix ( DefinitionElement | UsageElement )`. `SysML` 8.2.2.5.1.",
    ),
    ("MemberPrefix", "`( visibility = VisibilityIndicator )?`. `SysML` 8.2.2.5.1."),
    (
        "AliasMember",
        (
            "`MemberPrefix 'alias' ( '<' NAME '>' )? NAME? 'for' [QualifiedName] "
            "RelationshipBody`. `SysML` 8.2.2.5.1."
        ),
    ),
    ("OwnedAnnotation", "`ownedRelatedElement += AnnotatingElement`. `SysML` 8.2.2.4.1."),
    ("Annotation", "`annotatedElement = [QualifiedName]`. `SysML` 8.2.2.4.1."),
    (
        "Comment",
        (
            "`( 'comment' Identification ( 'about' Annotation ( ',' Annotation )* )? )? "
            "( 'locale' STRING_VALUE )? REGULAR_COMMENT`. `SysML` 8.2.2.4.2."
        ),
    ),
    (
        "Documentation",
        "`'doc' Identification ( 'locale' STRING_VALUE )? REGULAR_COMMENT`. `SysML` 8.2.2.4.2.",
    ),
    (
        "TextualRepresentation",
        ("`( 'rep' Identification )? 'language' STRING_VALUE REGULAR_COMMENT`. `SysML` 8.2.2.4.3."),
    ),
    (
        "PartDefinition",
        "`OccurrenceDefinitionPrefix 'part' 'def' Definition`. `SysML` 8.2.2.11.",
    ),
    (
        "OccurrenceDefinitionPrefix",
        (
            "`BasicDefinitionPrefix? ( 'individual' EmptyMultiplicityMember )? "
            "DefinitionExtensionKeyword*`. `SysML` 8.2.2.9.1."
        ),
    ),
    ("BasicDefinitionPrefix", "`'abstract' | 'variation'`. `SysML` 8.2.2.6.1."),
    ("EmptyMultiplicityMember", "`ownedRelatedElement += EmptyMultiplicity`. `SysML` 8.2.2.9.1."),
    ("EmptyMultiplicity", "`{ }`, a Multiplicity that consumes no tokens. `SysML` 8.2.2.9.1."),
    ("Definition", "`DefinitionDeclaration DefinitionBody`. `SysML` 8.2.2.6.1."),
    ("DefinitionDeclaration", "`Identification SubclassificationPart?`. `SysML` 8.2.2.6.1."),
    (
        "SubclassificationPart",
        "`SPECIALIZES OwnedSubclassification ( ',' OwnedSubclassification )*`. `SysML` 8.2.2.6.5.",
    ),
    ("OwnedSubclassification", "`superClassifier = [QualifiedName]`. `SysML` 8.2.2.6.5."),
    ("DefinitionBody", "`';' | '{' DefinitionBodyItem* '}'`. `SysML` 8.2.2.6.1."),
    ("DefinitionMember", "`MemberPrefix DefinitionElement`. `SysML` 8.2.2.6.1."),
    ("PartUsage", "`OccurrenceUsagePrefix 'part' Usage`. `SysML` 8.2.2.11."),
    ("AttributeUsage", "`UsagePrefix 'attribute' Usage`. `SysML` 8.2.2.7."),
    ("ItemUsage", "`OccurrenceUsagePrefix 'item' Usage`. `SysML` 8.2.2.10."),
    ("OccurrenceUsage", "`OccurrenceUsagePrefix 'occurrence' Usage`. `SysML` 8.2.2.9.2."),
    ("PortUsage", "`OccurrenceUsagePrefix 'port' Usage`. `SysML` 8.2.2.12."),
    ("RenderingUsage", "`OccurrenceUsagePrefix 'rendering' Usage`. `SysML` 8.2.2.26.3."),
    ("EnumerationUsage", "`UsagePrefix 'enum' Usage`. `SysML` 8.2.2.8."),
    ("UsagePrefix", "`UnextendedUsagePrefix UsageExtensionKeyword*`. `SysML` 8.2.2.6.2."),
    (
        "OccurrenceUsagePrefix",
        (
            "`( EndUsagePrefix | BasicUsagePrefix 'individual'? PortionKind? ) "
            "UsageExtensionKeyword*`. `SysML` 8.2.2.9.2."
        ),
    ),
    ("BasicUsagePrefix", "`RefPrefix 'ref'?`. `SysML` 8.2.2.6.2."),
    (
        "RefPrefix",
        (
            "`FeatureDirection? 'derived'? ( 'abstract' | 'variation' )? 'constant'?`. "
            "`SysML` 8.2.2.6.2."
        ),
    ),
    ("FeatureDirection", "`'in' | 'out' | 'inout'`. `SysML` 8.2.2.6.2."),
    ("PortionKind", "`'snapshot' | 'timeslice'`. `SysML` 8.2.2.9.2."),
    ("Usage", "`UsageDeclaration UsageCompletion`. `SysML` 8.2.2.6.2."),
    ("UsageDeclaration", "`Identification FeatureSpecializationPart?`. `SysML` 8.2.2.6.2."),
    ("UsageCompletion", "`ValuePart? UsageBody`. `SysML` 8.2.2.6.2."),
    ("UsageBody", "`DefinitionBody`. `SysML` 8.2.2.6.2."),
    (
        "FeatureSpecializationPart",
        (
            "`FeatureSpecialization+ MultiplicityPart? FeatureSpecialization* "
            "| MultiplicityPart FeatureSpecialization*`. `KerML` 8.2.4.3.1."
        ),
    ),
    ("Typings", "`TypedBy ( ',' FeatureTyping )*`. `SysML` 8.2.2.6.5."),
    ("Subsettings", "`Subsets ( ',' OwnedSubsetting )*`. `SysML` 8.2.2.6.5."),
    ("Subsets", "`SUBSETS OwnedSubsetting`, `SUBSETS = ':>' | 'subsets'`. `SysML` 8.2.2.6.5."),
    ("OwnedSubsetting", "`QualifiedName | OwnedFeatureChain`. `SysML` 8.2.2.6.5."),
    ("Redefinitions", "`Redefines ( ',' OwnedRedefinition )*`. `SysML` 8.2.2.6.5."),
    (
        "Redefines",
        "`REDEFINES OwnedRedefinition`, `REDEFINES = ':>>' | 'redefines'`. `SysML` 8.2.2.6.5.",
    ),
    ("OwnedRedefinition", "`QualifiedName | OwnedFeatureChain`. `SysML` 8.2.2.6.5."),
    (
        "References",
        (
            "`REFERENCES OwnedReferenceSubsetting`, `REFERENCES = '::>' | 'references'`. "
            "`SysML` 8.2.2.6.5."
        ),
    ),
    ("OwnedReferenceSubsetting", "`QualifiedName | OwnedFeatureChain`. `SysML` 8.2.2.6.5."),
    (
        "Crosses",
        "`CROSSES OwnedCrossSubsetting`, `CROSSES = '=>' | 'crosses'`. `SysML` 8.2.2.6.5.",
    ),
    ("OwnedCrossSubsetting", "`QualifiedName | OwnedFeatureChain`. `SysML` 8.2.2.6.5."),
    ("TypedBy", "`( ':' | 'defined' 'by' ) FeatureTyping`. `SysML` 8.2.2.6.5."),
    ("FeatureTyping", "`OwnedFeatureTyping | ConjugatedPortTyping`. `SysML` 8.2.2.6.5."),
    ("OwnedFeatureTyping", "`QualifiedName | OwnedFeatureChain`. `SysML` 8.2.2.6.5."),
    # The multiplicity a FeatureSpecializationPart may carry. `SysML` 8.2.2.6.6.
    # MultiplicityExpressionMember reaches only LiteralExpression and
    # FeatureReferenceExpression, not OwnedExpression, so a bound is a literal or a
    # name and never an operator expression.
    (
        "MultiplicityPart",
        (
            "`OwnedMultiplicity | OwnedMultiplicity? ( 'ordered' 'nonunique'? "
            "| 'nonunique' 'ordered'? )`. `SysML` 8.2.2.6.6."
        ),
    ),
    ("OwnedMultiplicity", "`ownedRelatedElement += MultiplicityRange`. `SysML` 8.2.2.6.6."),
    (
        "MultiplicityRange",
        (
            "`'[' ( MultiplicityExpressionMember '..' )? MultiplicityExpressionMember ']'`. "
            "`SysML` 8.2.2.6.6."
        ),
    ),
    (
        "MultiplicityExpressionMember",
        "`LiteralExpression | FeatureReferenceExpression`. `SysML` 8.2.2.6.6.",
    ),
    # The value a usage may carry. `SysML` 8.2.2.6.2.
    ("ValuePart", "`ownedRelationship += FeatureValue`. `SysML` 8.2.2.6.2."),
    (
        "FeatureValue",
        ("`( '=' | ':=' | 'default' ( '=' | ':=' )? ) OwnedExpression`. `SysML` 8.2.2.6.2."),
    ),
    # The fourth PackageBodyElement. `SysML` 8.2.2.5.1.
    ("ElementFilterMember", "`MemberPrefix 'filter' OwnedExpression ';'`. `SysML` 8.2.2.5.1."),
    # -- the expression layer, KerML 8.2.5.8 --
    #
    # OwnedExpression's eight alternatives carry no precedence and cannot: KerML
    # 8.2.5.8.1 note 2 states the grouping of nested OperatorExpressions is not
    # expressed in the productions. The tiers are data in docs/operator-precedence.toml.
    (
        "ConditionalExpression",
        (
            "`'if' ArgumentMember '?' ArgumentExpressionMember 'else' "
            "ArgumentExpressionMember EmptyResultMember`. `KerML` 8.2.5.8.1."
        ),
    ),
    (
        "ConditionalBinaryOperatorExpression",
        (
            "`ArgumentMember ConditionalBinaryOperator ArgumentExpressionMember "
            "EmptyResultMember`. `KerML` 8.2.5.8.1."
        ),
    ),
    (
        "BinaryOperatorExpression",
        "`ArgumentMember BinaryOperator ArgumentMember EmptyResultMember`. `KerML` 8.2.5.8.1.",
    ),
    (
        "UnaryOperatorExpression",
        "`UnaryOperator ArgumentMember EmptyResultMember`. `KerML` 8.2.5.8.1.",
    ),
    (
        "ClassificationExpression",
        (
            "`ArgumentMember? ( ClassificationTestOperator TypeReferenceMember "
            "| CastOperator TypeResultMember ) EmptyResultMember`. `KerML` 8.2.5.8.1."
        ),
    ),
    (
        "MetaclassificationExpression",
        (
            "`MetadataArgumentMember ( MetaclassificationTestOperator TypeReferenceMember "
            "| MetaCastOperator TypeResultMember ) EmptyResultMember`. `KerML` 8.2.5.8.1."
        ),
    ),
    ("ExtentExpression", "`'all' TypeReferenceMember`. `KerML` 8.2.5.8.1."),
    # The operand memberships. A membership per operand, as the clause reifies them.
    ("ArgumentMember", "`ownedMemberParameter = Argument`. `KerML` 8.2.5.8.1."),
    ("Argument", "`ownedRelationship += ArgumentValue`. `KerML` 8.2.5.8.1."),
    ("ArgumentValue", "`value = OwnedExpression`. `KerML` 8.2.5.8.1."),
    (
        "ArgumentExpressionMember",
        "`ownedRelatedElement += ArgumentExpression`. `KerML` 8.2.5.8.1.",
    ),
    ("ArgumentExpression", "`ownedRelationship += ArgumentExpressionValue`. `KerML` 8.2.5.8.1."),
    (
        "ArgumentExpressionValue",
        "`value = OwnedExpressionReference`. `KerML` 8.2.5.8.1.",
    ),
    (
        "OwnedExpressionReference",
        "`ownedRelationship += OwnedExpressionMember`. `KerML` 8.2.5.8.1.",
    ),
    ("OwnedExpressionMember", "`ownedFeatureMember = OwnedExpression`. `KerML` 8.2.5.8.1."),
    ("MetadataArgumentMember", "`ownedRelatedElement += MetadataArgument`. `KerML` 8.2.5.8.1."),
    ("MetadataArgument", "`ownedRelationship += MetadataValue`. `KerML` 8.2.5.8.1."),
    ("MetadataValue", "`value = MetadataReference`. `KerML` 8.2.5.8.1."),
    ("MetadataReference", "`ownedRelationship += ElementReferenceMember`. `KerML` 8.2.5.8.1."),
    ("ElementReferenceMember", "`memberElement = [QualifiedName]`. `KerML` 8.2.5.8.3."),
    # The result parameter every OperatorExpression owns, and which consumes no tokens.
    ("EmptyResultMember", "`ownedRelatedElement += EmptyFeature`. `KerML` 8.2.5.8.1."),
    ("EmptyFeature", "`{ }`, a Feature that consumes no tokens. `KerML` 8.2.5.8.1."),
    # The type operand of a classification, a cast or an extent.
    ("TypeReferenceMember", "`ownedMemberFeature = TypeReference`. `KerML` 8.2.5.8.1."),
    ("TypeResultMember", "`ownedMemberFeature = TypeReference`. `KerML` 8.2.5.8.1."),
    ("TypeReference", "`ownedRelationship += ReferenceTyping`. `KerML` 8.2.5.8.1."),
    ("ReferenceTyping", "`type = [QualifiedName]`. `KerML` 8.2.5.8.1."),
    # Primary expressions, KerML 8.2.5.8.2.
    ("SequenceExpression", "`'(' SequenceExpressionList ')'`. `KerML` 8.2.5.8.2."),
    (
        "SequenceExpressionList",
        "`OwnedExpression ','? | SequenceOperatorExpression`. `KerML` 8.2.5.8.2.",
    ),
    (
        "SequenceOperatorExpression",
        ("`OwnedExpressionMember ',' SequenceExpressionListMember`. `KerML` 8.2.5.8.2."),
    ),
    (
        "SequenceExpressionListMember",
        "`ownedRelatedElement += SequenceExpressionList`. `KerML` 8.2.5.8.2.",
    ),
    # Base expressions, KerML 8.2.5.8.3.
    ("NullExpression", "`'null' | '(' ')'`. `KerML` 8.2.5.8.3."),
    (
        "FeatureReferenceExpression",
        "`FeatureReferenceMember EmptyResultMember`. `KerML` 8.2.5.8.3.",
    ),
    ("FeatureReferenceMember", "`memberElement = FeatureReference`. `KerML` 8.2.5.8.3."),
    ("FeatureReference", "`[QualifiedName]`. `KerML` 8.2.5.8.3."),
    # Literal expressions, KerML 8.2.5.8.4.
    ("LiteralBoolean", "`'true' | 'false'`. `KerML` 8.2.5.8.4."),
    ("LiteralString", "`STRING_VALUE`. `KerML` 8.2.5.8.4."),
    ("LiteralInteger", "`DECIMAL_VALUE`. `KerML` 8.2.5.8.4."),
    (
        "LiteralReal",
        (
            "`DECIMAL_VALUE? '.' ( DECIMAL_VALUE | EXPONENTIAL_VALUE ) "
            "| EXPONENTIAL_VALUE`. `KerML` 8.2.5.8.4."
        ),
    ),
    ("LiteralInfinity", "`'*'`. `KerML` 8.2.5.8.4."),
    ("Error", "recovered-over text; carries its bytes so the tree stays lossless"),
]

WORD = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")

# The fixed Rust text, kept as templates so it reads as the Rust it becomes. Each
# one is a run of lines; render() puts the blank lines between the blocks.
ENUM_HEADER = f"""\
// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Token and node kinds. GENERATED — do not edit.
//!
//! Written by `{GENERATED_BY}` from `.claude/state/grammar/keywords.json`,
//! the token set extracted from the pinned Pilot Xtext. Keyword and operator
//! variants track that file; lexical and node variants are authored in the
//! generator. Change the generator or re-pin the grammar, then regenerate.

/// Every kind of token and node in the `SysML` v2 / `KerML` syntax tree.
///
/// `repr(u16)` because rowan stores a kind as a `u16`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
#[non_exhaustive]
pub enum SyntaxKind {{"""

ENUM_FOOTER = """\
    /// Sentinel for rowan's `from_raw`; never produced by the lexer.
    Tombstone,
}"""

ALL_HEADER = """\
/// Every kind, in declaration order, so `ALL[n as usize]` is the kind whose
/// discriminant is `n`.
///
/// rowan stores a kind as a `u16` and hands it back as one. Recovering the
/// variant by index is how that round-trip stays safe: the workspace forbids
/// `unsafe_code`, so a transmute is not available and would be wrong anyway —
/// an out-of-range `u16` has to be representable as `None`, not as a variant
/// that was never constructed.
pub const ALL: &[SyntaxKind] = &["""

KEYWORDS_HEADER = """\
/// Every keyword of the language, paired with its kind, sorted by text.
pub const KEYWORDS: &[(&str, SyntaxKind)] = &["""

OPERATORS_HEADER = """\
/// Every operator, paired with its kind, sorted longest first.
///
/// Longest first is what makes maximal munch correct: `::>` must be tried
/// before `::`, and `::` before `:`, or the lexer splits a token.
pub const OPERATORS: &[(&str, SyntaxKind)] = &["""


def variant(keyword: str) -> str:
    """`part def` -> `KwPartDef`. Keyword variants are prefixed so `Kw` groups them."""
    return "Kw" + "".join(part.capitalize() for part in keyword.replace("_", " ").split())


def _variants(named: list[tuple[str, str]]) -> list[str]:
    """Enum body lines: a doc comment and a variant name for each `(name, doc)`."""
    lines = []
    for name, doc in named:
        lines += [f"    /// {doc}", f"    {name},"]
    return lines


def _lexical_variants() -> list[str]:
    """The lexical token variants, each citing the clause it is implemented from."""
    named = [
        (name, f"`{production}`, `KerML` {clause} ({role}).")
        for name, production, clause, role in LEXICAL
    ]
    return [
        "    // -- lexical tokens, named by the specification at KerML 8.2.2 --",
        *_variants(named),
    ]


def _keyword_variants(keywords: list[str]) -> list[str]:
    """The keyword variants, from the pinned token set."""
    return [
        f"    // -- {len(keywords)} keywords, from the pinned token set --",
        *_variants([(variant(k), f"`{k}`") for k in keywords]),
    ]


def _operator_variants(operators: list[str]) -> list[str]:
    """The operator variants, from the pinned token set."""
    return [
        f"    // -- {len(operators)} operators, from the pinned token set --",
        *_variants([(SYMBOLS[o], f"`{o}`") for o in operators]),
    ]


def _node_variants() -> list[str]:
    """The node variants, authored here because no pinned input implies a node."""
    return ["    // -- nodes, authored --", *_variants(NODES)]


def _all_entries(keywords: list[str], operators: list[str]) -> list[str]:
    """Every variant name in declaration order, so index equals discriminant."""
    names = [name for name, _, _, _ in LEXICAL]
    names += [variant(k) for k in keywords]
    names += [SYMBOLS[o] for o in operators]
    names += [name for name, _ in NODES]
    names.append("Tombstone")
    return [f"    SyntaxKind::{name}," for name in names]


def _keyword_entries(keywords: list[str]) -> list[str]:
    """The `KEYWORDS` table rows, in the sorted order the keywords arrive in."""
    return [f'    ("{k}", SyntaxKind::{variant(k)}),' for k in keywords]


def _operator_entries(operators: list[str]) -> list[str]:
    """The `OPERATORS` table rows, longest first so maximal munch stays correct."""
    rows = []
    for operator in sorted(operators, key=lambda op: (-len(op), op)):
        escaped = operator.replace("\\", "\\\\").replace('"', '\\"')
        rows.append(f'    ("{escaped}", SyntaxKind::{SYMBOLS[operator]}),')
    return rows


def render(keywords: list[str], operators: list[str]) -> str:
    """The generated Rust module, as text."""
    lines = ENUM_HEADER.splitlines()
    lines += _lexical_variants()
    lines += ["", *_keyword_variants(keywords)]
    lines += ["", *_operator_variants(operators)]
    lines += ["", *_node_variants()]
    lines += ["", *ENUM_FOOTER.splitlines()]
    lines += ["", *ALL_HEADER.splitlines(), *_all_entries(keywords, operators), "];"]
    lines += ["", *KEYWORDS_HEADER.splitlines(), *_keyword_entries(keywords), "];"]
    lines += ["", *OPERATORS_HEADER.splitlines(), *_operator_entries(operators), "];"]
    lines.append("")
    return "\n".join(lines)


def build() -> str:
    """Render the module from the pinned token set."""
    tokens = json.loads(TOKENS.read_text())
    keywords = sorted(k for k in tokens["keywords"] if WORD.match(k))
    operators = sorted(tokens["operators"])
    unmapped = [o for o in operators if o not in SYMBOLS]
    if unmapped:
        message = (
            f"operators with no variant name in SYMBOLS: {unmapped}. "
            "The token set changed; name them in the generator rather than letting one "
            "be transliterated into an identifier nobody chose."
        )
        raise SystemExit(message)
    return render(keywords, operators)


def main(argv: list[str] | None = None) -> int:
    """Write the generated kinds module, or with --check fail if it is stale."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if the module is stale")
    args = parser.parse_args(argv)
    os.chdir(ROOT)

    if not TOKENS.is_file():
        print("no token set — run python3.11 scripts/extract_productions.py")
        return 0

    text = build()
    if args.check:
        if not OUT.is_file() or OUT.read_text() != text:
            print(f"{OUT} is stale — run python3.11 {GENERATED_BY}")
            return 1
        print(f"syntax kinds current ({OUT})")
        return 0

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    print(f"wrote {OUT}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
