# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""sv2-ast owns no data, and ships no test scaffolding (ADR-0025).

    python3.12 scripts/check_ast_leaks.py           every crates/sv2-ast/src/**/*.rs

Three rules:

1. A public accessor type wraps exactly one `SyntaxNode` and nothing else. Copying text
   out of the tree means the tree has stopped being the single source of truth, which is
   half of what ADR-0004 bought with a lossless tree.
2. Nothing public is test scaffolding: no `test_`, `dump_`, `debug_` or `fixture_` name,
   and no public item gated on `#[cfg(test)]`.
3. An accessor type carries its `// node:` marker, so coverage cannot undercount.

Network: none.
"""

from __future__ import annotations

import argparse
import os
import re
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = Path("crates/sv2-ast/src")

#: A braced struct, generics and a `where` clause included. `[^{;(]*` swallows `<'a, T>`
#: and `where T: Clone` and stops at the body.
STRUCT = re.compile(r"pub struct\s+([A-Za-z_][A-Za-z0-9_]*)[^{;(]*\{(.*?)\n\}", re.DOTALL)
#: The idiomatic rowan shape, `pub struct Part(SyntaxNode);`. Without this the checker
#: would read no accessor in the crate and pass every one of them. The field list is
#: matched greedily to the last `)` so a nested paren -- `(pub Vec<(String, u32)>)` --
#: does not end it early.
TUPLE_STRUCT = re.compile(
    r"pub struct\s+([A-Za-z_][A-Za-z0-9_]*)[^;({]*\((.*)\)\s*(?:where[^;]*)?;"
)
#: Any public struct at all. A header neither pattern above can read is exempt from every
#: rule in this file, not merely unchecked for owned text -- a generic struct used to pass
#: with three fields, an owned `String` and no marker, in silence. This is the backstop:
#: a shape the checker cannot parse is reported rather than waved through.
ANY_STRUCT = re.compile(r"pub struct\s+([A-Za-z_][A-Za-z0-9_]*)")
FIELD = re.compile(r"^\s*(?:pub\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*:\s*([^,\n]+)", re.MULTILINE)
PUB_FN_STRING = re.compile(r"pub fn\s+[A-Za-z_][A-Za-z0-9_]*\s*\([^)]*\)\s*->\s*String")
PUB_ITEM = re.compile(r"pub (?:fn|struct|enum|trait|const|static)\s+([A-Za-z_][A-Za-z0-9_]*)")
CFG_TEST_PUB = re.compile(r"#\[cfg\((?:any\([^)]*)?[^)]*\btest\b[^)]*\)*\)\]\s*\n?\s*pub\s")
MARKER = re.compile(r"//\s*node:\s*([A-Za-z_][A-Za-z0-9_]*)")
#: The reasoned exemption for a public type that is not a node view -- a named iterator,
#: say. Without it such a type deadlocks: the marker rule demands `// node: Members`, and
#: that marker then makes `Members` an `absent` kind in ast_coverage.py. Red either way.
NOT_A_NODE = re.compile(r"//\s*not-a-node:\s*\S")
SCAFFOLD = ("test_", "dump_", "debug_", "fixture_")
OWNED_TEXT = ("String", "Box<str>", "Vec<String>", "Cow<", "Arc<str>", "Rc<str>")
#: How far above a declaration its marker may sit: doc comments and attributes intervene.
MARKER_LOOKBEHIND = 12


@dataclass(frozen=True, slots=True)
class Finding:
    """One rule broken, at one place."""

    path: str
    line: int
    message: str

    def render(self) -> str:
        """The finding as one `path:line: message` line."""
        return f"{self.path}:{self.line}: {self.message}"


def _line(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1


def _declared_types(text: str) -> list[tuple[str, str, list[tuple[str, str]], int]]:
    """(name, body, fields, offset) for every public struct, braced or tuple.

    A tuple struct's fields are positional, so they are named by index for the message.
    """
    found = [
        (match.group(1), match.group(2), FIELD.findall(match.group(2)), match.start())
        for match in STRUCT.finditer(text)
    ]
    for match in TUPLE_STRUCT.finditer(text):
        parts = _split_fields(match.group(2))
        fields = [(str(index), part) for index, part in enumerate(parts)]
        found.append((match.group(1), match.group(2), fields, match.start()))
    return found


def _split_fields(body: str) -> list[str]:
    """`body` split on top-level commas only.

    A plain `split(",")` reads `Vec<(String, u32)>` as two fields and reports a one-field
    tuple struct as holding two, which sends the reader looking for a field that is not
    there.
    """
    parts: list[str] = []
    depth = 0
    current = ""
    for char in body:
        if char in "<([{":
            depth += 1
        elif char in ">)]}":
            depth -= 1
        if char == "," and depth == 0:
            parts.append(current)
            current = ""
            continue
        current += char
    parts.append(current)
    return [part.strip() for part in parts if part.strip()]


def _in_comment(text: str, offset: int) -> bool:
    """Whether `offset` sits on a line whose code is commented out.

    The owned-text rules describe themselves in prose, and a doc comment saying "never
    `pub fn name(&self) -> String`" is documentation, not a leak.
    """
    line_start = text.rfind("\n", 0, offset) + 1
    return text[line_start:offset].lstrip().startswith("//")


def _preamble(text: str, start: int) -> str:
    """The few lines above `start`, where a declaration's markers and attributes sit."""
    lines = text[:start].splitlines()
    return "\n".join(lines[-MARKER_LOOKBEHIND:])


def _unreadable_structs(text: str, path: str) -> list[Finding]:
    """Every `pub struct` whose header neither STRUCT nor TUPLE_STRUCT could read.

    Reported rather than skipped. A shape the checker cannot see is exempt from all three
    rules, which is worse than no rule at all: it looks checked.
    """
    readable = {match.start() for match in STRUCT.finditer(text)}
    readable |= {match.start() for match in TUPLE_STRUCT.finditer(text)}
    return [
        Finding(
            path,
            _line(text, match.start()),
            f"`{match.group(1)}` has a shape this checker cannot read, so none of its "
            f"rules applied; simplify the declaration or teach check_ast_leaks.py",
        )
        for match in ANY_STRUCT.finditer(text)
        if match.start() not in readable
    ]


def _struct_findings(text: str, path: str) -> list[Finding]:
    found: list[Finding] = _unreadable_structs(text, path)
    for name, body, fields, start in _declared_types(text):
        body_at = text.index(body, start) if body else start
        for field_name, field_type in fields:
            if any(owned in field_type for owned in OWNED_TEXT):
                at = body_at + (body.index(field_type) if field_type in body else 0)
                found.append(
                    Finding(
                        path,
                        _line(text, at),
                        f"`{name}.{field_name}` owns text; an accessor returns a node or a "
                        f"token and the caller asks it for text",
                    )
                )
        if len(fields) > 1:
            found.append(
                Finding(
                    path,
                    _line(text, start),
                    f"`{name}` holds {len(fields)} fields; an accessor type wraps one "
                    f"SyntaxNode and nothing else",
                )
            )
        preamble = _preamble(text, start)
        if not MARKER.search(preamble) and not NOT_A_NODE.search(preamble):
            found.append(
                Finding(
                    path,
                    _line(text, start),
                    f"`{name}` has no `// node:` marker above it, so coverage cannot count "
                    f"it; a public type that is not a node view carries "
                    f"`// not-a-node: <reason>` instead",
                )
            )
    return found


def findings(text: str, path: str) -> list[Finding]:
    """Every rule this file breaks, in line order."""
    found = _struct_findings(text, path)
    found += [
        Finding(path, _line(text, m.start()), "a public fn returns String; return a token instead")
        for m in PUB_FN_STRING.finditer(text)
        if not _in_comment(text, m.start())
    ]
    found += [
        Finding(
            path,
            _line(text, m.start()),
            f"`{m.group(1)}` is test scaffolding in the public API",
        )
        for m in PUB_ITEM.finditer(text)
        if m.group(1).startswith(SCAFFOLD)
    ]
    found += [
        Finding(path, _line(text, m.start()), "a public item is gated on cfg(test)")
        for m in CFG_TEST_PUB.finditer(text)
    ]
    return sorted(found, key=lambda f: (f.line, f.message))


def main(argv: list[str] | None = None) -> int:
    """Report every leak; 1 if there is one."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="*", help="files or directories; default: the crate's src")
    args = parser.parse_args(argv)
    os.chdir(ROOT)

    roots = [Path(p) for p in args.paths] or [SOURCE]
    files = [
        f for root in roots for f in ([root] if root.is_file() else sorted(root.rglob("*.rs")))
    ]
    found = [
        finding
        for file in files
        for finding in findings(file.read_text(errors="replace"), file.as_posix())
    ]
    for finding in found:
        print(f"  {finding.render()}")
    if found:
        print(f"{len(found)} leak(s). sv2-ast owns no data and ships no scaffolding (ADR-0025).")
        return 1
    print(f"sv2-ast owns no data ({len(files)} file(s) checked)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
