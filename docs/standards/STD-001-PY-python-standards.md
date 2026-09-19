---
title: Python Standards
document_id: STD-PY-001
status: baselined
version: 2.2.0
date: 2026-09-19
review_date: 2027-03-16
owner: David — CSE
applies_to: the repository's Python tooling — scripts/, .claude/scripts/, and their pytest suites
supersedes: null
superseded_by: null
related: [STD-002-RS, STD-003-SH]
python: "3.12"
---

# Python Standards

Rules for every Python file in this repository. Where a rule can be enforced by a
tool it is stated as a threshold and mapped to the check that enforces it,
because a convention with no check behind it is a preference that decays within
two contributors.

The enforcement configuration in [§12](#12-enforcement-configuration) is the
normative form of most of this document. Where the prose and the configuration
disagree, the configuration is authoritative and the prose is a defect.

**Assumptions.** The Rust workspace is the product; the Python here is the
tooling around it. There is no installable distribution — `pyproject.toml` has no
`[project]` table beyond the three keys uv needs, there is no `src/` layout, no
package, and nothing to build —
so the rules below govern scripts, not artifacts. Python 3.12: scripts run on the
RHEL 9 AppStream `python3.12` package, invoked explicitly as `python3.12`,
because the platform `/usr/bin/python3` is 3.9. The version is pinned in four
places, each of which acts on something different: `.python-version` for anything
uv creates, `requires-python` so uv stops guessing, `target-version` for ruff, and
`python_version` for mypy ([§12](#12-enforcement-configuration)). Standard library only
([§3](#3-dependencies-and-import-cost)). Ruff for linting and formatting, mypy
for types, pytest for tests, and the checks in `scripts/` for the rules none of
them expresses.

---

## 1. What this covers and how to use it

Three audiences, three entry points.

| You are                                            | Read                                                      |
| -------------------------------------------------- | --------------------------------------------------------- |
| Writing project tooling in `scripts/`              | All of it                                                 |
| Writing a Claude Code helper in `.claude/scripts/` | All of it, plus STD-003-SH §9 for the shim that calls it   |
| Reviewing a contribution                           | §13, then the section it points at                        |

Rules are `must`, `should`, or `may`. A `must` that is not machine-checkable is
a candidate defect in this document; see whether it can be moved into
[§12](#12-enforcement-configuration) before accepting it as prose.

---

## 2. Script layout

Two directories hold Python, and which one a script belongs in is decided by a
single question: **does it run without Claude Code?**

```text
scripts/                         project tooling — runs without Claude Code
├── _lock.py                     private module, imported by its siblings
├── gate.sh                      the single definition of "done"
├── bnf_coverage.py
├── check_headers.py
├── <check or generator>.py
└── tests/
    ├── data/                    fixture inputs — data, not code
    └── test_<module>.py

.claude/scripts/                 helpers for Claude Code hooks, skills, and agents
├── _state.py                    private modules
├── _grammar.py
├── _earley.py
├── hook-<purpose>.sh            shim (STD-003-SH §9.1)
├── hook_<purpose>.py            the logic it execs
├── <helper>.py
└── tests/
    └── test_<module>.py
```

**Rules.**

1. A script in `scripts/` **must** work with no Claude Code process anywhere: the
   gate, CI, and a person at a terminal all call it directly. A script in
   `.claude/scripts/` **may** assume a hook payload on stdin,
   `CLAUDE_PROJECT_DIR`, or the state under `.claude/state/`.
2. **No module in `scripts/` imports a module in `.claude/scripts/`.** The
   dependency runs one way, and the reason is the rule above: the moment a gate
   check imports a hook helper, the project tooling stops being usable without
   the agent tooling. `scripts/gate.sh` is the single documented exception to the
   directory split, and even it only *invokes* three of those scripts as
   programs — it imports nothing. The reverse direction is allowed the same way:
   `.claude/scripts/regen_state.py` runs `scripts/` programs as subprocesses.
3. Private modules carry a leading underscore and live beside the scripts that
   import them. `mypy_path` and `pythonpath` in
   [§12](#12-enforcement-configuration) are what make
   `from _state import REPO_ROOT` resolve.
4. There is no `src/`, no distribution name, and no import name to correspond to
   one. A file's path is its identity.

### 2.1 These directories are not packages

There is no `__init__.py` anywhere in this repository, and there **must not** be
one. Each script directory is placed on the import path by `pythonpath` and
`mypy_path` ([§12](#12-enforcement-configuration)), which is what makes
`from _lock import read_lock` work from a file invoked as
`python3.12 scripts/vendor_verify.py`. Adding an `__init__.py` would make the
directory a package, change every import line in it, and break that invocation —
which is the one the hooks, the gate, and the README all use. `INP001` is ignored
for both directories in [§12](#12-enforcement-configuration) for exactly this
reason.

The shared-code seam is therefore a private module, not a package root, and the
rule on it is the one a package root would have carried: **a private module
defines, and it does not execute.** `_lock.py`, `_state.py`, `_grammar.py`, and
`_earley.py` define readers, constants, and types; none of them reads a file,
runs a subprocess, or prints at import time. Module-level work runs on any import
of anything that reaches it, including a `PreToolUse` hook that pays for it
before every tool call ([§3.2](#32-import-cost-at-the-hook-boundary)).

### 2.2 Entry points

Every script is invoked by path — `python3.12 scripts/bnf_coverage.py --check` —
never as `python -m` and never through a console script. Two rules follow.

1. The file ends with a call and nothing else:

   ```python
   if __name__ == "__main__":
       raise SystemExit(main())
   ```

2. **`main(argv: list[str] | None = None) -> int` returns an exit code and never
   calls `sys.exit` internally**, so a test can call it and assert on the result.
   Argument parsing belongs inside `main`, through `argparse` with
   `description=__doc__` ([§11](#11-docstrings)).

A hook's Python logic adds one wrinkle: `main` is wrapped by `run()`, which
applies the shim's failure policy (STD-003-SH §9.2) and is what the module
actually raises `SystemExit` on. The wrapper exists so the policy is one line in
one place rather than a `try` around every statement.

---

### 2.3 File header, module docstring, and file order

Every module opens in the same order. Nothing else may precede these.

```text
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
<module docstring>
from __future__ import annotations
<imports>
<code>
```

**No shebang line, anywhere in this repository's Python.** Nothing here is
invoked as a bare script: the gate, CI, and the hook shims all call
`python3.12 <path>`, and none of them reads a shebang. A shebang on a module that
is never executed directly is a claim the file does not support.

#### Program header

The header is exactly two comment lines, above the module docstring:

```python
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
```

**Rules.**

1. The header is a `#` comment block, **never** a docstring. It must not become
   `__doc__`, because the docstring is the script's `--help` text
   ([§11](#11-docstrings)) and its generated documentation; keeping the two
   separate is what keeps the licence out of both.
2. Both lines, verbatim, on every source file. The point of an SPDX identifier and
   a copyright line is that they travel with a file copied out of the repository,
   which is the whole of what MIT asks for. A file missing them is a file that
   leaves the project unlicensed.
3. **No historical change comments.** Not in the header, not anywhere in a source
   file. Version control answers "what changed" accurately and a comment block
   answers it inaccurately within two commits.
4. Shell carries the same two lines, placed immediately after the shebang
   (STD-003-SH §3.4); Rust carries them as `//` comments, never `//!` or `///`,
   so that legal text stays out of rustdoc.

**Enforcement.** `scripts/check_headers.py` reads the requirement from
`pyproject.toml` and asserts that every module under `scripts/` and
`.claude/scripts/`, and every crate source, opens with a comment block carrying
each required string. Checking the _values_ rather than the presence of a block is
the point — a header with the wrong copyright line passes a presence check and
fails a review.

```toml
[tool.sv2.headers]
required = true
must_contain = [
    "SPDX-License-Identifier: MIT",
    "Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>",
]
```

That table is compared against the repository the same way the tables in
[§12](#12-enforcement-configuration) are. The shebang prohibition is
unconditional and is checked whether or not headers are required.

---

## 3. Dependencies and import cost

### 3.1 The standard library, and one optional import

**Every script here imports the standard library and nothing else.** That is not
austerity, it is what makes the tooling runnable: the hooks run before every tool
call, the gate runs at the end of every turn, and both must work on a machine
where nothing has been installed and — in an air-gapped enclave — nothing can be.
A gate that needs a download is a gate that is green only on the workstation where
it was written.

**Rule: no third-party import at module scope without a runtime fallback.** The
one optional import in the repository is `jsonschema`, in
`.claude/scripts/validate_state.py`:

```python
try:
    import jsonschema

    HAVE_JSONSCHEMA = True
except ImportError:  # optional dependency: the structural fallback below covers it
    HAVE_JSONSCHEMA = False
```

The fallback is a structural check over required keys, enums, and the `minLength`
rules that stop `"TBD"` being accepted as a handoff, and it never passes something
the real validator would reject on those grounds. The script says which one it
used. An optional import with no fallback is a required import with a worse error
message.

mypy needs the `[[tool.mypy.overrides]]` entry in
[§12](#12-enforcement-configuration) to accept the missing stubs. Ruff, mypy, and
pytest themselves are development tools rather than imports: `scripts/gate.sh`
runs each with `run_optional` and skips it with a visible notice when it is not on
`PATH`.

pytest and `jsonschema` are declared in `pyproject.toml`'s `dev` dependency group, not
in `[project]`, and `uv sync` installs them into the project `.venv`. The gate runs
the script tests and `validate_state.py` under `.venv/bin/python` when that exists,
and under the system `python3.12` otherwise — where pytest is usually absent and is
skipped, and `jsonschema` is absent and the structural fallback runs. Every CHECK
script still runs under the system `python3.12`. So a `.venv` makes the gate check
more, and its absence never makes the gate fail: the property this section exists
for holds on a machine where nothing has been installed.

### 3.2 Import cost at the hook boundary

A `PreToolUse` hook's imports are paid before every tool call in a session, and a
`Stop` hook's before every turn ends. Nothing measures this, so the rule is
structural rather than a threshold: **module scope defines; it does not compute,
read files, or run subprocesses**
([§2.1](#21-these-directories-are-not-packages)). A constant built from a literal
is a definition; a constant built by reading `.claude/state/` is not.

Deferring an import _inside a function_ is a last resort, not the pattern. It
hides the dependency from static analysis and moves the cost to an unpredictable
first call. The supported way to avoid a runtime import is [§6](#6-typing)'s
`TYPE_CHECKING` block, which removes the cost of an import that only annotations
needed.

---

## 4. Modeling data: which construct, and where

The choice of construct is not a matter of taste. It follows from one question:
**does this value cross a boundary where it might be wrong?**

### 4.1 The boundary rule

> Validate once at the boundary. Move plain frozen values inside it.

Everything these scripts read comes from outside the process. What that means
concretely:

| Input                                                                                                     | What it is                             | How it is read                                                       |
| ---------------------------------------------------------------------------------------------------------- | -------------------------------------- | --------------------------------------------------------------------- |
| `pyproject.toml`, `vendor/sources.lock.toml`, `docs/conformance-target.toml`, `scripts/rust_binaries.toml` | repository configuration               | `tomllib`, always table-qualified ([§4.4](#44-boundary-validation))  |
| `.claude/state/state.json`, `.claude/state/deviations.json`                                                | authored by a person, read by tooling  | validated against `.claude/state/schema/*.json`                       |
| `.claude/state/**/*.json` derived files                                                                    | written by one script, read by another | the writing script is the schema; a `_generated_by` key names it       |
| `vendor/` Xtext, XMI, corpus                                                                               | pinned upstream material               | hash-verified by `vendor_verify.py` before anything parses it          |
| a hook payload on stdin                                                                                    | Claude Code                            | `json.load`, then narrowed defensively — a missing key is not a crash |
| `cargo metadata`, `git log` output                                                                         | a subprocess                           | parsed by the caller, which owns the failure                          |

| Construct                             | Use for                                                                                                      | Do not use for                                          |
| ------------------------------------- | -------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| `@dataclass(frozen=True, slots=True)` | Internal value types: a parsed setting, a rule, a plan entry — anything whose invariants hold by construction    | Anything filled in over the course of a run             |
| `@dataclass(slots=True)` (mutable)    | Accumulators owned by one call chain: `Verification` in `vendor_verify.py`                                      | Anything shared between callers or mutated after return  |
| `enum.StrEnum`                        | Closed vocabularies that are serialized                                                                         | Open sets, or values that grow with each contribution    |
| `dict` / `list` typed `Any`           | A document just deserialized, before its caller narrows it: `Json` in `_state.py`                               | Anything that has already been checked                   |

### 4.2 Why the split matters here

Two concrete reasons.

**Cost.** `_earley.py` allocates an item per rule, dot position, and origin, at
every position in the input. A recognizer is an inner loop, and `slots=True` is
what keeps it affordable: it removes the per-instance `__dict__`, and it turns a
typo in an attribute assignment into an `AttributeError` rather than a silently
created field.

**There is no schema generator.** This repository has no validating-model library
([§3](#3-dependencies-and-import-cost)), so a Python class cannot produce a JSON
Schema and a JSON Schema cannot produce a class. The schemas under
`.claude/state/schema/` are the only statement of those shapes, and a dataclass
restating one would be a second statement that drifts. The division is therefore
sharp: **the schema is the contract; a dataclass is a convenience for code that
has already passed it.**

### 4.3 Internal value types

```python
from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class Settings:
    """The repository's header requirement, from [tool.sv2.headers]."""

    required: bool
    values: tuple[str, ...]
```

`frozen=True` **must** be used for any value type. Immutability is what makes a
value safe to pass to every check in a run without defensive copying, and it is
what makes `__hash__` meaningful.

`slots=True` **should** be used on every dataclass, for the reason in
[§4.2](#42-why-the-split-matters-here).

`__post_init__` **should not** perform validation in internal value types. If a
`Settings` can be constructed with a value that is not a string, the reader that
produced it is the defect. Validation in the value type is a second check in the
wrong place, and it costs on every construction to catch a bug in code you
control.

Sequence fields **should** be `tuple[...]` rather than `list[...]`, so a frozen
value is actually immutable rather than immutable in its top level only.

### 4.4 Boundary validation

There is no pydantic here and there must not be
([§3](#3-dependencies-and-import-cost)). The discipline it provides is provided
by the JSON Schemas and by four rules.

1. **Validate against the schema, not by hand.** The authored state files each
   have a schema under `.claude/state/schema/`, and
   `.claude/scripts/validate_state.py` is the only thing that decides whether one
   is valid. A script that re-checks a field it could have let the validator check
   has created a second opinion, and two opinions drift.
2. **Refuse unknown shapes.** `"additionalProperties": false` on every authored
   object, for the same reason a forbidding model has it: a key with a typo in it
   validates, loads, and behaves as though the field were absent. Where a map is
   deliberately open — `gates`, whose keys are check names — the schema carries the
   *value* schema instead (`{"enum": ["pass", "fail", "skipped"]}`), because
   skipping it let any string through as a gate result.
3. **Read TOML table-qualified.** `pin_value("tier_b_pilot.revision")` names its
   table because a flat scan once took an earlier table's `revision` for the Pilot
   revision. A key is read from its own table or it is not read.
4. **A pinned input is verified before it is parsed.** `vendor_verify.py` checks
   every vendored file's sha256 against `vendor/sources.lock.toml`, and the gate
   runs it first, ahead of everything that reads those files. Parsing an unverified
   pin produces a correct-looking derivation from the wrong input, which is the
   failure this repository is built to prevent.

Between the boundary and the narrowing, the value is `Any` and says so: `Json` in
`_state.py` carries a comment naming it as a deserialization boundary
([§6](#6-typing)). Everywhere past the narrowing it is a concrete type.

---

## 5. Function shape and complexity

A rule about function length that no tool enforces is a preference. Each
threshold below maps to a check that fails the build.

### 5.1 Thresholds

| Property                           | Limit | Check     | Why this number                                                                                                                                                               |
| ---------------------------------- | ----- | --------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Nested blocks                      | 3     | `PLR1702` | Four levels of nesting is reliably two functions and a predicate. Three accommodates a loop containing a conditional containing a guard, which is a legitimate shape.         |
| Cyclomatic complexity              | 10    | `C901`    | The conventional threshold, and it correlates with the point where a function stops fitting in one reading.                                                                   |
| Branches                           | 12    | `PLR0912` | Above this, a dispatch table or a match statement is clearer than a chain.                                                                                                    |
| Return statements                  | 6     | `PLR0911` | Early returns are good; six exits means the function has more than one job.                                                                                                   |
| Arguments                          | 5     | `PLR0913` | Keyword-only parameters after `*` are exempt from the readability concern but not from the count — a function needing more than five inputs usually wants a parameter object. |
| Statements                         | 40    | `PLR0915` | A soft ceiling. Hitting it is a prompt to look, not automatically a defect.                                                                                                   |
| Boolean expressions in a condition | 5     | `PLR0916` | A five-term condition wants a named predicate.                                                                                                                                |

Module length is deliberately unbounded. A thousand-line module of thirty
independent small functions is fine; a two-hundred-line module with one function
in it is not, and a line count cannot tell the difference.

### 5.2 What to do when you exceed one

Nesting is the one worth a specific technique, because the usual response —
extracting the innermost block into a helper that takes eight parameters — is
worse than the original.

```python
# exceeds nesting: for → if → if → try
def derived_units(root: Path) -> list[str]:
    names = []
    for path in root.glob("*.json"):
        if path.is_file():
            unit = json.loads(path.read_text())
            if unit.get("status") == "derived":
                try:
                    names.append(unit["name"])
                except KeyError:
                    print(f"  {path}: unit has no name")
    return names
```

```python
# guard clauses flatten it; the predicate gets a name
def derived_units(root: Path) -> list[str]:
    units = (json.loads(p.read_text()) for p in root.glob("*.json") if p.is_file())
    return [unit["name"] for unit in units if _is_named_derived(unit)]


def _is_named_derived(unit: Json) -> bool:
    return unit.get("status") == "derived" and "name" in unit
```

The general moves, in order of preference: invert a condition into a guard clause
and return early; name the condition as a predicate function; extract the loop
body when it is cohesive; replace a branch chain with a dispatch mapping or a
`match`.

### 5.3 Signatures

Booleans in a signature are prohibited by `FBT`. `render(document, True)` is
unreadable at the call site and the call site is where it is read.

```python
def render(document: dict[str, Any], *, compact: bool = False) -> str: ...
```

Keyword-only after `*` is **required** for any parameter that is not obviously
positional from the function name. Two positional parameters of the same type
adjacent in a signature is an argument-order bug waiting to be written — which is
why the counts a coverage report prints are passed by name:

```python
def _check(absent: list[str], *, implemented: int, declared: int, unimplemented: int) -> int: ...
```

Mutable default arguments are prohibited by `B006`. Return types are required on
every function including `-> None`, because an unannotated return is invisible to
mypy in strict mode and produces an implicitly `Any`-typed call site.

---

## 6. Typing

Strict mypy over `scripts` and `.claude/scripts`, with no per-directory
relaxation. The suites are excluded (`exclude = ["/tests/"]`) because ruff's
per-file ignores already treat a test as deliberately un-annotated; everything
that runs in the gate is checked.

**`from __future__ import annotations` is required at the top of every module.**
It makes annotations lazy, which is what allows `TC` rules to move type-only
imports into a `TYPE_CHECKING` block — which is in turn what keeps a hook's import
to what it actually executes
([§3.2](#32-import-cost-at-the-hook-boundary)).

```python
from __future__ import annotations

from typing import TYPE_CHECKING

from _state import REPO_ROOT, STATE, load_json

if TYPE_CHECKING:
    from _state import Json          # annotations only; not imported at runtime
```

**`Any` is permitted in exactly two places:** a deserialization boundary before
narrowing, and a third-party interface with no stubs. Both require a comment
naming the reason, and both appear here — `Json = Any` in `_state.py`, commented
as documents "deserialized from disk and narrowed by their callers", and the
`jsonschema` override in [§12](#12-enforcement-configuration). Everywhere else,
`object` plus narrowing is the honest form: it forces the narrowing that `Any`
lets you skip.

**One alias per document shape.** `Json` is defined once and imported, so every
signature carrying a parsed document says so in the same word. An alias costs
nothing at runtime and is the only thing that distinguishes "a dict" from "a
document nobody has checked yet".

There are no abstract base classes in this tooling and there should not be. A seam
that genuinely needs one wants `typing.Protocol`, so the coupling is the shape
rather than the inheritance.

**No `# type: ignore` without a code and a reason**, and the same for `# noqa`:

```python
except Exception:  # noqa: BLE001  # top-level handler; policy OPEN, STD-003-SH §9.2
```

A bare suppression silences everything on the line, including the error introduced
next year.

---

## 7. Errors

### 7.1 Failure is a diagnostic and an exit code

These are command-line tools. There is no error taxonomy, no exception that
crosses a process boundary, and nothing that catches an exception on their behalf.
A script fails by **printing why and returning non-zero from `main`**:

```python
if absent:
    print(f"coverage: {len(absent)} production(s) claimed by the parser but not in the inventory:")
    for name in absent:
        print(f"  {name}")
    return 1
```

`scripts/gate.sh` runs each check with stdout and stderr captured together and
prints the first thirty lines under a `FAIL` line. Two consequences worth stating
as rules: a diagnostic **must** be legible in thirty lines, and it **must** name
what to run next ([§8.4](#84-writing-a-message)).

An unhandled traceback reaching the gate is a defect in the script, not a report.
The condition it died on was a condition it could have detected and described.

### 7.2 Rules

1. **A failure the script anticipates is a message and a return code, not an
   exception.** Where an exception is the natural way to carry a failure out of a
   helper, it is a module-local class that the script's own `main` catches —
   `SourceError` in `export_error_codes.py`, `MetadataError` in
   `check_rust_workspace.py`. It is caught before it becomes a traceback.
2. **No bare `except:` and no bare `except Exception:`** (`E722`, `BLE001`). The
   one place `except Exception` is correct is a hook's top-level handler, which
   exists to apply a declared failure policy (STD-003-SH §9.2) and carries the
   `noqa` and the policy on the same line.
3. **Catch what you can name.** `except (ImportError, AttributeError) as exc`
   states which failures the code anticipated; `except Exception` states only that
   the author stopped thinking about it.
4. **Chain, always.** `raise SourceError(msg) from exc`. Enforced by `B904`.
   Losing the original traceback across a wrap makes the wrap worse than nothing.
5. **Exception messages are assigned, not inlined** (`EM101`, `EM102`). The
   pattern reads oddly at first and it keeps tracebacks readable when the message
   is long.
6. **Never swallow.** An `except` that continues **must** say in a comment why
   continuing is correct. The optional import in
   [§3.1](#31-the-standard-library-and-one-optional-import) says so; so does
   `regen_state.py`, where a command that cannot be started is a gate recorded as
   `skipped` rather than a crash. Almost everywhere else, continuing is a defect.
7. **`assert` is not error handling.** It is stripped under `-O` and it is
   prohibited outside tests (`S101`).
8. **Messages carry identifiers, never content**
   ([§8.6](#86-what-must-never-be-printed)).

---

## 8. Output

### 8.1 These scripts print

There is no logging in this repository. Nothing here is a service, nothing runs
longer than a gate, and every one of these programs exists to produce a report
that a person or the gate reads once. `print` is the product, not a lapse.

`T20` is selected and then ignored for both script directories
([§12](#12-enforcement-configuration)), which between them hold every Python file
in the repository. The restriction on output is therefore this section rather than
a lint rule — which is why the section is specific about what stdout is for. If
something here ever does need `logging`, the rule is one logger per module named
`__name__`, configured once, inside `main`.

### 8.2 stdout is the product

| Process                          | stdout carries                                                          |
| -------------------------------- | ------------------------------------------------------------------------ |
| A check run by `scripts/gate.sh` | its findings and one summary line; the gate shows the first thirty lines |
| A hook's Python logic            | exactly one JSON decision object, or nothing at all                      |
| `state_report.py`                | the report; it writes no files                                           |
| Tests                            | captured by pytest                                                       |

**A hook's stdout is a protocol channel, not a place to write.** Claude Code reads
it: a `permissionDecision` from `PreToolUse`, a `decision: "block"` from `Stop`,
and — for `SessionStart` — plain text that is injected directly into the model's
context (STD-003-SH §9.3). A stray `print` in hook logic either corrupts the
decision or silently becomes context. This is the rule most likely to be violated
by accident and the hardest to diagnose when it is.

Diagnostics that are not the product go to stderr, and are written there
explicitly:

```python
sys.stderr.write("stop-gate: already blocked once this turn; releasing to avoid a loop.\n")
```

### 8.3 Printing belongs to the entry point

**A private module prints nothing.** `_lock.py`, `_state.py`, `_grammar.py`, and
`_earley.py` return values and raise; the script that imported them decides what
the user sees. A helper that prints has taken over the output of every script that
imports it — including a hook whose stdout is a protocol channel
([§8.2](#82-stdout-is-the-product)), where the helper's one helpful line is a
corrupted decision.

The corollary from [§2.1](#21-these-directories-are-not-packages): nothing at
module scope prints either, because that runs on import.

### 8.4 Writing a message

```python
print(f"  {path}: header does not carry {value!r}")                     # a finding
print(f"program headers present and correct on {len(files)} file(s)")   # the summary
```

- **One line per finding**, indented two spaces, opening with the path or
  identifier it is about.
- **One summary line**, stating the counts, whether or not there were findings. A
  check that prints nothing on success is a check nobody can tell ran.
- **Where a fix exists, name the command.**
  `"no production inventory — run python3.12 scripts/vendor_sync.py then python3.12 scripts/extract_productions.py"`.
  A diagnostic that names the failure but not the remedy sends its reader into the
  source of the script that produced it.

The lazy-`%s` rule of a logging framework has no analogue here: `print` formats
eagerly because its output is always emitted.

### 8.5 There are no levels

A script's vocabulary is the gate's, and it has four words.

| Outcome | Exit | What is printed                                                              |
| ------- | ---- | ----------------------------------------------------------------------------- |
| pass    | 0    | the summary line                                                              |
| fail    | 1    | each finding, then the summary                                                |
| inert   | 0    | why the check could not apply — its inputs are not in the repository yet      |
| skipped | —    | nothing; the script never runs and `gate.sh`'s `run_optional` says so         |

The distinction between `inert` and `skipped` is the one that matters, and it is
STD-003-SH §4.3's: the first is a state of the repository, the second is a missing
tool. Neither is a failure and both are visible.

Do not print a warning that changes nothing. If a condition matters it is a
finding; if it does not, it is noise in a report someone reads on every commit,
and a report people skim is a report that has stopped working.

### 8.6 What must never be printed

This section is a hard rule, not guidance. It is the surface where the boundary is
most often crossed by accident, because a printed line feels ephemeral and — once
it is in a gate log, a CI record, or a model's context — is not.

- **Credentials.** Tokens, keys, and the *values* of environment variables. Name
  the variable if you must name anything.
- **File contents.** A check reports the path and what is wrong with it, not the
  text it read. `vendor_verify.py` reports a path and two digests, never the bytes.
- **Vendored source text.** The pinned OMG and Pilot material under `vendor/`
  carries its own licence terms. Quote a rule name, a production, or a line
  number — not the file.
- **Anything a hook hands back to Claude.** A hook's `reason` enters the transcript
  and is read by a model. Keep it to identifiers, counts, and the command that
  fixes the problem.

An exception message counts as a printed line. `raise SourceError(f"failed on
{text}")` puts file content into every traceback and every gate log that catches
it. Error messages carry identifiers, never content.

### 8.7 Say what it is about

A finding that does not name its subject cannot be acted on without re-running the
check.

```python
f"{path}: header does not carry {value!r}"
f"{key}: standard {expected!r}, repository {actual!r}"
```

Every finding names a path, a production, a grammar unit, or a dotted
configuration key. That is what lets a red line in a gate log be traced back to
the file that caused it, by someone who was not there when it ran.

---

## 9. Naming and module organization

| Kind              | Convention                                            | Note                                                                               |
| ----------------- | ----------------------------------------------------- | ---------------------------------------------------------------------------------- |
| Module            | `snake_case.py`, named for what it checks or produces | `bnf_coverage.py`, `check_headers.py`; not `coverage_utils.py`                     |
| Private module    | `_leading_underscore.py`                              | Imported by its siblings only; §2                                                  |
| Hook logic module | `hook_<purpose>.py`                                   | Paired with the `hook-<purpose>.sh` shim that execs it (STD-003-SH §10)            |
| Class             | `PascalCase`, noun                                    |                                                                                    |
| Exception         | `PascalCase` + `Error`                                | Name the condition. Never shadow a builtin: `CallTimeoutError`, not `TimeoutError` |
| Function          | `snake_case`, verb phrase                             | `read_lock`, not `lock_reading`                                                    |
| Predicate         | `is_`, `has_`, `can_` prefix                          | Returns `bool`, no side effects                                                    |
| Constant          | `UPPER_SNAKE` at module scope                         | `ROOT`, `LOCKFILE`, `PYPROJECT`                                                    |
| Type alias        | `PascalCase`                                          | `Json`                                                                             |
| Test              | `test_<subject>_<condition>`                          | `test_rust_doc_comments_are_not_a_header`                                          |

**Banned suffixes: `_utils`, `_helpers`, `_misc`, `_common`, `manager`,
`handler`.** Every one of them names a module by what it is not. `utils.py` becomes
the place code goes when nobody decided where it belongs, and within a year it is
the module with the most imports and the least coherence. If a function has no
home, the missing thing is a concept, not a bucket. The same list is enforced on
the Rust side by `scripts/check_rust_patterns.py`, which cites this section; on
the Python side it is a review rule.

**Relative imports are prohibited** (`TID252`, `ban-relative-imports = "all"`).
Absolute imports are greppable and survive a module move — and here they are
load-bearing as well: `from _state import ...` resolves through `pythonpath`
([§12](#12-enforcement-configuration)), and a relative import would require a
package, which [§2.1](#21-these-directories-are-not-packages) forbids.

Beyond that, the only structural import rule is the direction in
[§2](#2-script-layout): no module in `scripts/` imports one in
`.claude/scripts/`. There is no import-linter in this repository and no packages
for it to layer, so that one is a review rule.

---

## 10. Tests

```text
scripts/tests/
├── data/
│   └── sample.xtext                 fixture input — data, not code
├── test_check_headers.py
└── test_vendor_verify.py

.claude/scripts/tests/
├── test_earley.py
└── test_hook_protect_paths.py
```

**Rules.**

1. Test layout **mirrors** the script directory. `test_check_headers.py` tests
   `check_headers.py`. A reviewer should not have to search for a module's tests.
   Both suites run from one invocation; `testpaths` and `pythonpath` in
   [§12](#12-enforcement-configuration) are what make the imports resolve.
2. **Test the functions, not the process.** A test imports `check`, `settings`,
   and `leading_comment` and calls them; it does not spawn
   `python3.12 scripts/check_headers.py` and grep the output. This is why
   [§2.2](#22-entry-points) keeps `main` thin and makes it return an `int` rather
   than exit.
3. **Fixture inputs are data.** A file a test must read lives in `tests/data/`; a
   file it constructs is built under `tmp_path`. An expected value pasted into an
   assertion cannot say where it came from.
4. **No network.** `scripts/vendor_sync.py` is the only script that fetches, and no
   test runs its fetch. Nothing automated enforces this — it is a review rule, and
   it is the rule STD-003-SH §7.3 states for shell. The target environment is
   air-gapped; a test suite that needs the internet is testing something else.
5. **No wall clock.** Anything time-dependent takes its time as a parameter, or is
   asserted on shape rather than value. A test that passes except at midnight UTC
   is a test that fails at midnight UTC.
6. **Parametrise rather than loop.** A loop inside a test reports one failure for
   twenty cases; `pytest.mark.parametrize` reports twenty.
7. **One behavior per test.** The test name states the condition and the
   expectation, and if the name needs "and" it is two tests.
8. **`conftest.py` at the narrowest scope that works.** There is none today: the
   fixtures in use are `tmp_path` and local helper functions. When one is needed it
   belongs in the suite that needs it, never above both — the two suites cover
   different directories and share no code, which is [§2](#2-script-layout)'s rule
   restated for tests.

---

## 11. Docstrings

Required on: every module, every class, and every function that is not a two-line
private helper. Google-style convention, configured through ruff's pydocstyle
settings. Not required on magic methods or on `__init__` (`D105` and `D107` are
ignored): under the Google convention the class docstring carries construction in
its `Attributes:` section, so a docstring on `__init__` states it twice.

`D` is enforced: it is in `select` and, unlike `T20`, it is **not** ignored for
the script directories. Only the test suites are exempt, where a test's name is
its description. A missing docstring on a public function or method fails
`ruff check` and therefore the gate.

**The rule specific to this codebase: a script's module docstring is its `--help`
text.**

```python
parser = argparse.ArgumentParser(description=__doc__)
```

Every script does this, so the docstring is read by whoever *runs* the script, not
only by whoever opens it. Its shape is therefore a contract: one or two sentences
on what the script asserts and why it exists in the form it does, then every
invocation, indented, one line per mode.

```python
"""Classify every production in productions.json against what the parser claims.

    implemented    parser handles it, snapshot-tested
    unimplemented  in the inventory, deliberately not yet handled  -- legitimate
    absent         parser claims a production the inventory does not contain  -- DEFECT

    python3.12 scripts/bnf_coverage.py           write .claude/state/coverage.json
    python3.12 scripts/bnf_coverage.py --check   fail on any `absent`
"""
```

A mode that is not in the docstring is a mode that is not in `--help`. The two
program header lines sit *above* the docstring and are not part of it
([§2.3](#23-file-header-module-docstring-and-file-order)), which is what keeps the
licence out of that output.

---

## 12. Enforcement configuration

Workspace root `pyproject.toml`. This repository ships no Python distribution, so
its `[project]` table carries three keys and no dependencies — it is tool
configuration and nothing else. The test tooling is a `[dependency-groups]` `dev`
group outside `[project]` ([§3.1](#31-the-standard-library-and-one-optional-import)).
There is no second place to configure these tools from, and there should not be
one.

```toml
[tool.ruff]
# py312 is the interpreter the scripts run on — invoked as `python3.12`, the
# newest one RHEL 9 AppStream provides as a supported package — and ruff and mypy
# both target it, so the lints and the type check assume the interpreter that
# actually runs the gate.
target-version = "py312"
line-length = 100
src = ["scripts", ".claude/scripts"]
# Only the script directories. Without this, ruff also formats Python code blocks
# inside Markdown, including the examples in docs/standards/.
include = ["scripts/**/*.py", ".claude/scripts/**/*.py"]

[tool.ruff.lint]
select = [
    "F",      # pyflakes
    "E", "W", # pycodestyle
    "I",      # import sorting
    "N",      # pep8 naming
    "UP",     # pyupgrade
    "ANN",    # missing annotations
    "S",      # bandit
    "BLE",    # blind except
    "FBT",    # boolean trap in signatures
    "B",      # bugbear
    "A",      # shadowing builtins
    "C4",     # comprehensions
    "DTZ",    # naive datetimes
    "EM",     # exception message literals
    "ISC",    # implicit string concat
    "ICN",    # import conventions
    "LOG", "G",  # logging correctness and format
    "INP",    # implicit namespace packages
    "PIE", "T20", "PT", "Q", "RSE", "RET",
    "SLF",    # private member access
    "SIM",
    "TID",    # tidy imports
    "TC",     # type-checking blocks
    "ARG",    # unused arguments
    "PTH",    # pathlib over os.path
    "ERA",    # commented-out code
    "PL",     # pylint refactor/warning/error
    "TRY",    # exception antipatterns
    "PERF",
    "D",      # docstrings
    "RUF",
]
ignore = [
    "D203", "D213",   # conflict with the chosen convention
    "D105", "D107",   # magic methods and __init__; see §11
    "ANN401",         # Any is governed by §6, with a required comment
    "TRY003",         # long messages are fine; EM101/EM102 govern placement
    "ISC001",         # conflicts with the formatter
]

[tool.ruff.lint.pydocstyle]
convention = "google"

[tool.ruff.lint.mccabe]
max-complexity = 10

[tool.ruff.lint.pylint]
max-nested-blocks = 3
max-branches = 12
max-returns = 6
max-args = 5
max-statements = 40
max-bool-expr = 5

[tool.ruff.lint.flake8-tidy-imports]
ban-relative-imports = "all"

[tool.ruff.lint.flake8-type-checking]
strict = true

[tool.ruff.lint.per-file-ignores]
# Leading **/ is required: the suites live at scripts/tests/ and
# .claude/scripts/tests/, and a root-anchored pattern matches neither.
"**/tests/**" = ["S101", "D", "PLR2004", "ANN", "INP001", "SLF001"]  # asserts, docstrings, magic numbers, no __init__.py, private seams
# The script directories: stdout is their product, and they are not a package.
# S603/S607: these scripts exist to run the repository's own fixed commands (cargo,
# git, sibling scripts) with argument lists, never a shell string.
# D is NOT ignored here: a script's module docstring is its --help text (§11).
"scripts/**" = ["T20", "INP001", "S603", "S607"]
".claude/scripts/**" = ["T20", "INP001", "S603", "S607"]

[tool.ruff.format]
docstring-code-format = true


[tool.mypy]
python_version = "3.12"
strict = true
warn_unreachable = true
warn_no_return = true
disallow_any_explicit = false          # governed by §6 with a required comment
enable_error_code = ["redundant-expr", "truthy-bool", "ignore-without-code"]
# The private modules each script imports live beside it.
mypy_path = ["scripts", ".claude/scripts"]
files = ["scripts", ".claude/scripts"]
# Annotation in tests is governed by ruff, whose per-file ignores already treat
# a test as deliberately un-annotated.
exclude = ["/tests/"]

[[tool.mypy.overrides]]
# Optional at runtime: validate_state.py falls back to a structural check.
module = ["jsonschema", "jsonschema.*"]
ignore_missing_imports = true


[tool.pytest.ini_options]
addopts = "--strict-markers --strict-config -q"
# Tests live beside the scripts they cover.
testpaths = ["scripts/tests", ".claude/scripts/tests"]
pythonpath = ["scripts", ".claude/scripts"]
markers = ["slow: excluded from the default run"]
```

The `[tool.sv2.headers]` table in
[§2.3](#23-file-header-module-docstring-and-file-order) is part of the same
configuration and is compared the same way.

Two entries above carry architecture rather than style: `ban-relative-imports` and
the `pythonpath` / `mypy_path` pair. Together they mean every import line in the
repository names its module absolutely and resolves through a directory that is
deliberately not a package ([§2.1](#21-these-directories-are-not-packages)). There
is no import-linter here and no package layers for it to enforce; the one
architectural rule — `scripts/` does not import `.claude/scripts/` — is stated in
[§2](#2-script-layout) and reviewed by eye.

### CI command set

```bash
ruff check
ruff format --check
mypy
.venv/bin/python -m pytest -q     # python3.12 -m pytest -q when there is no .venv
python3.12 scripts/check_headers.py
python3.12 scripts/check_shell_standard.py
python3.12 scripts/check_standards_config.py
```

All seven run in `scripts/gate.sh`, which is the single definition of "done" — the
`Stop` hook and CI both call exactly it. Ruff, mypy, and pytest are skipped with a
visible notice when they are not installed, so a workstation without them is not
blocked; the three checks in `scripts/` import only the standard library and
therefore always run. `check_shell_standard.py` is STD-003-SH §12.3's check, and
is listed here because the gate runs the script checks as one group.

`scripts/check_standards_config.py` compares the tables above with the tables in
`pyproject.toml`, key by key, and fails the build when they diverge. This document
claims its configuration is normative; the check is what makes that true rather
than aspirational. It compares only top-level `tool` tables, and only keys this
document sets — a key the repository adds on its own is not compared. A difference
made on purpose is recorded in `[tool.sv2.deviations]` of `pyproject.toml` as
`"<file>:<dotted.key>" = "<reason>"`, and a recorded deviation that no longer
differs is itself a finding, so the register cannot go stale.

---

## 13. Reviewing a contribution

Ten checks, in the order that fails fastest.

1. **Right directory.** Does it run without Claude Code? `scripts/`. Does it serve
   a hook, a skill, or an agent? `.claude/scripts/`. No module in `scripts/`
   imports one from `.claude/scripts/` ([§2](#2-script-layout)).
2. Two-line SPDX and copyright header, above the module docstring; no shebang; no
   historical change comments
   ([§2.3](#23-file-header-module-docstring-and-file-order)).
3. Module docstring states what the script asserts and lists every invocation, and
   is passed to argparse as `description=__doc__` ([§11](#11-docstrings)).
4. `main(argv) -> int`, `raise SystemExit(main())` at the foot of the file, and no
   `sys.exit` inside ([§2.2](#22-entry-points)).
5. Standard library only — or an optional import behind a fallback, with a mypy
   override ([§3](#3-dependencies-and-import-cost)).
6. Every document read at a boundary is table-qualified TOML, schema-validated
   JSON, or a hash-verified vendored file. Nothing parses a pin that
   `vendor_verify.py` has not checked ([§4.4](#44-boundary-validation)).
7. Failures are a message and a return code. No bare `except` outside a hook's
   top-level handler; every `except` that continues says why in a comment
   ([§7](#7-errors)).
8. stdout carries the findings and one summary line; a hook's stdout carries a JSON
   decision and nothing else; nothing from
   [§8.6](#86-what-must-never-be-printed) appears anywhere.
9. Tests mirror the module, call its functions rather than the process, keep
   fixture input in `tests/data/` or `tmp_path`, parametrise rather than loop, and
   touch no network ([§10](#10-tests)).
10. `ruff check`, `ruff format --check`, `mypy`, and `pytest` pass, and
    `scripts/check_headers.py` and `scripts/check_standards_config.py` agree
    ([§12](#12-enforcement-configuration)).

Checks 5, 6, and 8 are the ones a reviewer fluent in Python but new to this
codebase will not think to make. An installed-only-here import passes on the
workstation where it was written and fails in the enclave; an unverified pin
produces a correct-looking derivation from the wrong input, which no test will
catch; and a stray line on a hook's stdout either corrupts a decision or becomes
context, and reports itself as neither.
