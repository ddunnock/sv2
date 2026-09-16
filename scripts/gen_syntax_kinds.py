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
