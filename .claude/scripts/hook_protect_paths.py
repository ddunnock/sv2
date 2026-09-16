# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""PreToolUse logic: block writes to paths owned by scripts or pinned by the conformance target.

Reads the tool call as JSON on stdin. A denial is a JSON ``permissionDecision`` on
stdout with exit 0: Claude receives only the reason, without the "hook error"
framing an exit 2 carries. Exit 2 remains the fallback when this hook cannot
evaluate a call at all. Invoked through hook-protect-paths.sh, whose failure
policy is CLOSED.

File tools (Edit, Write, NotebookEdit) name their path. Bash does not, so a
command is tokenized and every path it mentions is resolved against the call's
working directory. The Bash check is a tripwire, not a boundary: a script file,
variable indirection, or ``xargs`` can still reach a protected path. The gate's
``--check`` modes are the backstop that catches those after the fact.
"""

from __future__ import annotations

import json
import os
import re
import shlex
import sys
from fnmatch import fnmatchcase
from itertools import pairwise, takewhile
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Callable

    from _state import Json

REGEN_VENDOR = "python3.11 scripts/vendor_sync.py"

# First match wins, as in a shell `case`. A None message means "writable".
RULES: list[tuple[tuple[str, ...], str | None]] = [
    (
        ("*/src/generated/*", "crates/*/generated/*"),
        "Generated code. Change the generator or the grammar, then regenerate.",
    ),
    (
        ("vendor/omg/*", "vendor/pilot/*", "vendor/corpus/*", "vendor/spec-bnf/*"),
        (
            "Vendored upstream artifact, pinned by sha256 in vendor/sources.lock.toml.\n"
            "Editing it breaks the hash and severs traceability to the OMG file ID.\n"
            f"To move to a new upstream version: {REGEN_VENDOR} --accept-new"
        ),
    ),
    (("vendor/sources.lock.toml",), f"Vendor lockfile. Hashes are written by {REGEN_VENDOR}."),
    # Derived by the workflow — writable, schema-checked.
    ((".claude/state/grammar/units/*",), None),
    (
        (
            ".claude/state/grammar/reference.json",
            ".claude/state/grammar/reference.ebnf",
            ".claude/state/grammar/ledger.json",
            ".claude/state/grammar/validation.json",
            ".claude/state/grammar/rebase.json",
        ),
        (
            "Script-owned. reference/ledger come from"
            " python3.11 .claude/scripts/grammar_freeze.py,\n"
            "validation from grammar_validate.py, rebase from grammar_rebase.py."
        ),
    ),
    (
        (".claude/state/grammar/*",),
        (
            "Derived from the pinned grammars. Regenerate with"
            " python3.11 scripts/extract_productions.py\n"
            "(from the Xtext) or python3.11 scripts/extract_bnf.py (from the specification BNF).\n"
            "If the content is wrong, the extractor or the pinned grammar is wrong — not this file."
        ),
    ),
    (
        (
            ".claude/state/coverage.json",
            ".claude/state/grammar-diff.json",
            ".claude/state/decisions.json",
        ),
        "Script-owned. Regenerate with bnf_coverage.py, grammar_diff.py, or index_decisions.py.",
    ),
    (
        (".claude/state/wiki-receipts.json",),
        (
            "Wiki receipts. Every clause citation in deviations.json resolves against this\n"
            "file, so hand-editing it would make an unpinned citation look pinned.\n"
            "Regenerate with python3.11 .claude/scripts/build_wiki_receipts.py."
        ),
    ),
    (
        (".claude/state/schema/*",),
        (
            "State schema. Changing it changes what a valid handoff is — edit deliberately,\n"
            "outside an agent turn, and bump schema_version in the documents it governs."
        ),
    ),
    (
        ("docs/conformance-target.toml",),
        (
            "Conformance pin. Tier A namespace and Tier B revision are reviewed changes;\n"
            f"edit deliberately and re-run {REGEN_VENDOR},"
            " then record why in .claude/state/deviations.json."
        ),
    ),
]

GENERATED_STATE = (
    'The "generated" object of state.json is written by'
    " python3.11 .claude/scripts/regen_state.py.\n"
    'Edit "authored" freely — objective, next_step, pending_decisions, log.'
)


def repo_relative(path: str, root: str) -> str:
    """Normalize to a repo-relative path before matching.

    Tool calls arrive with absolute paths, relative paths, and ./-prefixed paths,
    and a glob written for one form silently fails open on the others.
    """
    rel = path[len(root) + 1 :] if path.startswith(root + "/") else path
    return rel.removeprefix("./")


def _first_rule(rel: str) -> tuple[bool, str | None]:
    """(whether any rule matched, its message). A matched None means explicitly writable."""
    for patterns, message in RULES:
        if any(fnmatchcase(rel, p) for p in patterns):
            return True, message
    return False, None


def path_rule(rel: str) -> str | None:
    """The denial message for a repo-relative path, or None if it may be edited."""
    return _first_rule(rel)[1]


_GLOB = frozenset("*?[")


def _contains_protected(
    directory: str, pattern: str, exists: Callable[[str], bool] | None = None
) -> bool:
    """Whether some path strictly under ``directory`` could match ``pattern``.

    Matched segment by segment: a ``*`` segment stands for one directory level,
    except a leading ``*``, which may stand for several (``*/src/generated/*``
    covers ``crates/sv2-syntax/src/generated/x``). Whole-string fnmatch was wrong
    here, because ``*`` also crosses ``/``: ``crates/*/generated/*`` then matched
    ``crates/sv2-cli/src/lib.rs/generated/x`` and every new file under crates/
    looked like a parent of generated code.

    The pattern segments the directory stands in for must include a literal one,
    so a leading ``*`` does not make a one-word argument (``2``, ``print(1)``)
    look like a parent of protected files.

    A leading ``*`` matches any prefix, so on its own it would make every ``src``
    directory in or out of a crate a parent of ``*/src/generated/*``. For such a
    pattern, a directory short of the literal path counts only when the rest of
    that literal path exists: ``crates/x/src`` is protected when
    ``crates/x/src/generated`` is on disk, and ``packages/y/src`` is not.
    ``exists`` answers for a repo-relative path; without it nothing exists.
    """
    parts = [] if directory in ("", ".") else directory.rstrip("/").split("/")
    segments = pattern.split("/")
    if not parts:
        return True
    leading = segments[0] == "*"
    for absorbed in range(1, len(parts) + 1) if leading else (0,):
        rest_parts = parts[absorbed:]
        rest_segments = segments[1:] if leading else segments
        if len(rest_parts) >= len(rest_segments):
            continue
        consumed = rest_segments[: len(rest_parts)]
        if not any(not _GLOB & set(seg) for seg in consumed):
            continue
        if not all(fnmatchcase(part, seg) for part, seg in zip(rest_parts, consumed, strict=True)):
            continue
        if not leading or _remainder_exists(parts, rest_segments[len(rest_parts) :], exists):
            return True
    return False


def _remainder_exists(
    parts: list[str], remainder: list[str], exists: Callable[[str], bool] | None
) -> bool:
    """Whether ``parts`` plus the literal head of ``remainder`` names something on disk."""
    literal = list(takewhile(lambda seg: not _GLOB & set(seg), remainder))
    if not literal:
        return True  # the directory already covers every literal segment
    return exists is not None and exists("/".join([*parts, *literal]))


def protection(rel: str, exists: Callable[[str], bool] | None = None) -> str | None:
    """The denial message if ``rel`` is protected or is a directory containing protected paths.

    ``rm -rf vendor`` names no protected file, but it deletes all of them.
    """
    matched, message = _first_rule(rel)
    if matched:
        return message
    for patterns, rule_message in RULES:
        if rule_message is not None and any(_contains_protected(rel, p, exists) for p in patterns):
            return rule_message
    return None


# ---- Bash ---------------------------------------------------------------------

# Commands that only read files or print. Naming a protected path in their
# arguments is not a write; redirections are checked separately.
READERS = frozenset(
    [
        "cat",
        "less",
        "more",
        "head",
        "tail",
        "wc",
        "grep",
        "egrep",
        "fgrep",
        "rg",
        "ls",
        "stat",
        "file",
        "diff",
        "cmp",
        "comm",
        "sort",
        "uniq",
        "cut",
        "tr",
        "jq",
        "sha256sum",
        "sha1sum",
        "md5sum",
        "realpath",
        "readlink",
        "basename",
        "dirname",
        "test",
        "[",
        "echo",
        "printf",
        "true",
        "false",
        "tree",
        "du",
        "hexdump",
        "xxd",
        "od",
        "strings",
        "column",
        "nl",
        "fold",
        "mypy",
        "shellcheck",
        "pytest",
    ]
)
# git subcommands that do not rewrite working-tree files.
GIT_READERS = frozenset(
    [
        "status",
        "diff",
        "log",
        "show",
        "blame",
        "grep",
        "ls-files",
        "ls-tree",
        "rev-parse",
        "cat-file",
        "add",
        "commit",
        "describe",
        "shortlog",
    ]
)
# Commands that write only their final argument (or their -t directory).
COPIERS = frozenset(("cp", "install", "rsync", "scp", "ln"))
# Commands that act on the path they are given and nothing beneath it, so a
# directory that merely contains protected paths is not at risk.
PATH_ONLY = frozenset(("mkdir", "touch", "tee", "ln", "install", "truncate"))
SHELLS = frozenset(("bash", "sh", "zsh"))
WRAPPERS = frozenset(("sudo", "env", "command", "exec", "nice", "nohup", "time", "timeout"))
LAUNCHER_VALUE_OPTIONS = frozenset(("--from", "--with", "--python", "-p", "--with-requirements"))
RUFF_WRITES = frozenset(("--fix", "--unsafe-fixes", "--add-noqa"))
INTERPRETER = re.compile(r"python[\d.]*|perl|ruby|node|deno|bun|php|awk|gawk|mawk")
ASSIGNMENT = re.compile(r"\w+=.*")
OPTION_VALUE = re.compile(r"^[\w-]+=")
REDIRECTS = frozenset((">", ">>", ">|", "&>", "&>>", "<>"))
OPERATORS = frozenset((";", "&&", "||", "|", "|&", "&", "(", ")"))
FIND_WRITES = frozenset(("-delete", "-exec", "-execdir", "-ok", "-okdir", "-fprint"))
HEREDOC = re.compile(r"<<-?\s*(['\"]?)([A-Za-z_][A-Za-z0-9_]*)\1")
PATH_IN_TEXT = re.compile(r"[\w.~-]*/[\w./-]+")


def split_heredocs(command: str) -> list[tuple[str, list[str]]]:
    """Each line of shell with the heredoc bodies it opens. Line continuations are joined.

    A body belongs to the line that opens it only: text fed to ``cat >> notes``
    must not count as the program of a ``python3`` run later in the same call.
    """
    lines = command.replace("\\\n", "").split("\n")
    shell: list[tuple[str, list[str]]] = []
    i = 0
    while i < len(lines):
        line, bodies = lines[i], []
        i += 1
        for match in HEREDOC.finditer(line):
            body: list[str] = []
            while i < len(lines) and lines[i].strip() != match.group(2):
                body.append(lines[i])
                i += 1
            i += 1
            bodies.append("\n".join(body))
        shell.append((line, bodies))
    return shell


def simple_commands(line: str) -> list[list[str]]:
    """The words of each simple command on one shell line, split at control operators."""
    lexer = shlex.shlex(line, posix=True, punctuation_chars=True)
    lexer.whitespace_split = True
    lexer.commenters = "#"
    commands: list[list[str]] = [[]]
    for token in lexer:
        if token in OPERATORS:
            commands.append([])
        else:
            commands[-1].append(token)
    return [c for c in commands if c]


def _launcher_end(words: list[str], i: int) -> int:
    """Index past a ``uvx [opts]`` or ``uv run [opts]`` launcher starting at ``i``, else ``i``."""
    if words[i] == "uvx":
        i += 1
    elif words[i] == "uv" and words[i + 1 : i + 2] == ["run"]:
        i += 2
    else:
        return i
    while i < len(words) and words[i].startswith("-"):
        i += 2 if words[i] in LAUNCHER_VALUE_OPTIONS else 1
    return i


def _strip_prefix(words: list[str]) -> list[str]:
    """Drop leading assignments, wrappers (``sudo``, ``env X=1``) and launchers (``uvx``)."""
    i = 0
    while i < len(words):
        if words[i] in WRAPPERS or ASSIGNMENT.fullmatch(words[i]):
            i += 1
            while i < len(words) and words[i].startswith("-") and words[i - 1] in WRAPPERS:
                i += 1
            continue
        end = _launcher_end(words, i)
        if end == i:
            break
        i = end
    return words[i:]


class Scope:
    """Resolves the paths one Bash call names, tracking ``cd`` as it goes."""

    def __init__(self, root: str, cwd: str) -> None:
        self.root = root
        self.cwd = cwd

    def exists(self, rel: str) -> bool:
        """Whether ``rel``, repository-relative, is present on disk."""
        return (Path(self.root) / rel).exists()

    def resolve(self, word: str) -> str:
        """``word`` as an absolute path, against the call's current directory."""
        return os.path.normpath(Path(self.cwd) / Path(word).expanduser())

    def hit(self, word: str, *, recursive: bool = True) -> str | None:
        r"""The path and the rule that protects it, as "<rel>\n<message>", or None.

        ``recursive`` also counts a directory that contains protected paths, for
        commands that act on everything beneath what they are given.
        """
        if OPTION_VALUE.match(word):
            word = word.split("=", 1)[1]
        if not word or "://" in word or "$" in word or word.startswith("-"):
            return None
        path = self.resolve(word)
        if path == self.root:
            rel = "."
        elif path.startswith(self.root + "/"):
            rel = repo_relative(path, self.root)
        else:
            return None
        message = protection(rel, self.exists) if recursive else path_rule(rel)
        return f"{rel}\n{message}" if message is not None else None

    def first_hit(self, words: list[str], *, recursive: bool = True) -> str | None:
        """The first word that names a protected path, or None if none does."""
        return next((h for w in words if (h := self.hit(w, recursive=recursive))), None)


def _operands(words: list[str]) -> list[str]:
    """Arguments that are not redirection operators or their targets."""
    return [
        w
        for prev, w in zip(["", *words], words, strict=False)
        if w not in REDIRECTS and prev not in REDIRECTS
    ]


def _subcommand(args: list[str]) -> str:
    return next((a for a in args if not a.startswith("-")), "")


# Commands that are readers only when invoked a particular way.
READ_ONLY_WHEN: dict[str, Callable[[list[str]], bool]] = {
    "git": lambda args: _subcommand(args) in GIT_READERS,
    "sed": lambda args: not any(a.startswith(("-i", "--in-place")) for a in args),
    "find": lambda args: not FIND_WRITES & set(args),
    "ruff": lambda args: (
        (_subcommand(args) == "check" and not RUFF_WRITES & set(args))
        or (_subcommand(args) == "format" and bool({"--check", "--diff"} & set(args)))
    ),
    "shfmt": lambda args: not {"-w", "--write"} & set(args),
}


def _is_reader(name: str, args: list[str]) -> bool:
    condition = READ_ONLY_WHEN.get(name)
    return condition(args) if condition else name in READERS


def _copy_targets(args: list[str]) -> list[str]:
    targets = [b for a, b in pairwise(args) if a in ("-t", "--target-directory")]
    operands = [a for a in args if not a.startswith("-")]
    return targets + operands[-1:]


def _program_hit(args: list[str], bodies: list[str], scope: Scope) -> str | None:
    mentions = [m for text in [*args, *bodies] for m in PATH_IN_TEXT.findall(text)]
    # A program naming the repository root is almost always locating files to read,
    # and would otherwise count as a parent of every protected path. Protected
    # paths it names below the root still deny.
    words = [w for w in [*args, *mentions] if scope.resolve(w) != scope.root]
    return scope.first_hit(words)


def _argument_violation(words: list[str], scope: Scope, bodies: list[str]) -> str | None:
    name, args = Path(words[0]).name, _operands(words)[1:]
    if name == "cd":
        scope.cwd = scope.resolve(args[0]) if args else scope.root
        return None
    if (name in SHELLS and "-c" in args) or name == "eval":
        return script_violation(args[-1] if args else "", scope)
    if INTERPRETER.fullmatch(name):
        hit = _program_hit(args, bodies, scope)
        return f"{name} program or argument names {hit}" if hit else None
    if _is_reader(name, args):
        return None
    operands = _copy_targets(args) if name in COPIERS else args
    hit = scope.first_hit(operands, recursive=name not in PATH_ONLY)
    return f"{name} is given {hit}" if hit else None


def command_violation(words: list[str], scope: Scope, bodies: list[str]) -> str | None:
    """Why one simple command would write a protected path, or None."""
    words = _strip_prefix(words)
    if not words:
        return None
    # A redirection target is written whatever the command is.
    targets = [b for a, b in pairwise(words) if a in REDIRECTS]
    hit = scope.first_hit(targets, recursive=False)
    if hit:
        return f"redirects into {hit}"
    return _argument_violation(words, scope, bodies)


def script_violation(command: str, scope: Scope) -> str | None:
    """Why a shell script would write a protected path, or None."""
    for line, bodies in split_heredocs(command):
        try:
            commands = simple_commands(line)
        except ValueError:
            # Unbalanced quoting leaves nothing to classify: any protected mention denies.
            hit = scope.first_hit(PATH_IN_TEXT.findall(line))
            return f"unparseable command names {hit}" if hit else None
        for words in commands:
            reason = command_violation(words, scope, bodies)
            if reason:
                return reason
    return None


def bash_violation(command: str, root: str, cwd: str) -> str | None:
    """Why a Bash command would write a protected path, or None if it names none that way."""
    return script_violation(command, Scope(root, cwd or root))


def rewrites_generated_state(tool_input: Json, state_file: Path) -> bool:
    """Whether a whole-file write to state.json changes its script-owned block.

    Compare parsed JSON rather than grepping — a grep for the key name would also
    fire on the word appearing anywhere in an authored string.
    """
    new_text = tool_input.get("new_string") or tool_input.get("content")
    if not isinstance(new_text, str) or not new_text:
        return False
    try:
        new = json.loads(new_text)
        current = json.loads(state_file.read_text())
    except (ValueError, OSError):
        return False  # a partial edit, not a whole-file write; or no state to protect
    return (
        isinstance(new, dict)
        and isinstance(current, dict)
        and "generated" in new
        and new["generated"] != current.get("generated")
    )


def decide(tool_input: Json, root: str) -> tuple[str, str | None]:
    """(path, denial message or None) for one tool call."""
    path = tool_input.get("file_path") or tool_input.get("notebook_path") or ""
    if not isinstance(path, str) or not path:
        return "", None
    message = path_rule(repo_relative(path, root))
    if message is None and Path(path).name == "state.json":
        # state.json is split-owned: "authored" is yours, "generated" is the script's.
        state_file = Path(root) / ".claude/state/state.json"
        if rewrites_generated_state(tool_input, state_file):
            message = GENERATED_STATE
    return path, message


def deny(reason: str) -> int:
    """Deny the tool call with a reason delivered to Claude only."""
    decision = {
        "hookEventName": "PreToolUse",
        "permissionDecision": "deny",
        "permissionDecisionReason": reason,
    }
    print(json.dumps({"hookSpecificOutput": decision}))
    return 0


def main() -> int:
    """Cancel the tool call when it would write a path the repository owns."""
    root = os.environ.get("CLAUDE_PROJECT_DIR") or str(Path(__file__).resolve().parents[2])
    try:
        payload = json.load(sys.stdin)
    except ValueError:
        return 0  # not a tool call this hook can inspect
    tool_input = payload.get("tool_input") if isinstance(payload, dict) else None
    if not isinstance(tool_input, dict):
        return 0
    if payload.get("tool_name") == "Bash":
        command = tool_input.get("command")
        if not isinstance(command, str):
            return 0
        reason = bash_violation(command, root, str(payload.get("cwd") or root))
        return deny(f"BLOCKED (Bash): {reason}") if reason else 0
    path, message = decide(tool_input, root)
    return deny(f"BLOCKED: {path}\n{message}") if message else 0


def run() -> int:
    """``main`` behind the top-level handler that applies the CLOSED failure policy."""
    try:
        return main()
    except Exception as exc:  # noqa: BLE001  # top-level handler; policy CLOSED, STD-003-SH §9.2
        sys.stderr.write(f"BLOCKED: hook-protect-paths could not evaluate this call: {exc}\n")
        return 2


if __name__ == "__main__":
    raise SystemExit(run())
