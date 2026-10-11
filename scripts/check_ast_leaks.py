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

STRUCT = re.compile(r"pub struct\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{(.*?)\n\}", re.DOTALL)
#: The idiomatic rowan shape, `pub struct Part(SyntaxNode);`. Without this the checker
#: would read no accessor in the crate and pass every one of them.
TUPLE_STRUCT = re.compile(r"pub struct\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)\s*;")
FIELD = re.compile(r"^\s*(?:pub\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*:\s*([^,\n]+)", re.MULTILINE)
PUB_FN_STRING = re.compile(r"pub fn\s+[A-Za-z_][A-Za-z0-9_]*\s*\([^)]*\)\s*->\s*String")
PUB_ITEM = re.compile(r"pub (?:fn|struct|enum|trait|const|static)\s+([A-Za-z_][A-Za-z0-9_]*)")
CFG_TEST_PUB = re.compile(r"#\[cfg\(test\)\]\s*\n\s*pub\s")
MARKER = re.compile(r"//\s*node:\s*([A-Za-z_][A-Za-z0-9_]*)")
SCAFFOLD = ("test_", "dump_", "debug_", "fixture_")
OWNED_TEXT = ("String", "Box<str>", "Vec<String>", "Cow<")


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
        parts = [p.strip() for p in match.group(2).split(",") if p.strip()]
        fields = [(str(index), part) for index, part in enumerate(parts)]
        found.append((match.group(1), match.group(2), fields, match.start()))
    return found


def _struct_findings(text: str, path: str) -> list[Finding]:
    found: list[Finding] = []
    claimed = set(MARKER.findall(text))
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
        if name not in claimed:
            found.append(
                Finding(
                    path,
                    _line(text, start),
                    f"`{name}` has no `// node:` marker, so coverage cannot count it",
                )
            )
    return found


def findings(text: str, path: str) -> list[Finding]:
    """Every rule this file breaks, in line order."""
    found = _struct_findings(text, path)
    found += [
        Finding(path, _line(text, m.start()), "a public fn returns String; return a token instead")
        for m in PUB_FN_STRING.finditer(text)
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
