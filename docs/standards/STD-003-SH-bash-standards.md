---
title: Bash Standards
document_id: STD-003-SH
status: draft
version: 0.3.0
date: 2026-09-16
review_date: 2027-03-16
owner: David — CSE
applies_to: all shell scripts in this repository, including Claude Code hook entry points
supersedes: null
superseded_by: null
related: [STD-001-PY, STD-002-RS]
bash: "5.1"
---

# Bash Standards

Rules for every shell script in this repository. Where a rule can be enforced by a
tool it is stated as a threshold and mapped to the check that enforces it,
because a convention with no check behind it is a preference that decays within
two contributors.

The enforcement configuration in [§12](#12-enforcement-configuration) is the
normative form of most of this document. Where the prose and the configuration
disagree, the configuration is authoritative and the prose is a defect.

This document is the shell counterpart to STD-001-PY and STD-002-RS and follows
their structure where the languages allow. [Appendix A](#appendix-a-correspondence-with-std-001-py)
maps each Python rule to its shell form.

**Assumptions.**

- GNU bash 5.1 and GNU coreutils, as shipped by RHEL 9. Nothing here targets
  POSIX `sh`, macOS bash 3.2, or BusyBox.
- ShellCheck for linting, shfmt for formatting, `scripts/check_shell_standard.py`
  for the rules neither tool expresses.
- The delivery environment is air-gapped. Every script must work without network
  access unless its header declares otherwise.

---

## 1. What this covers and how to use it

| You are                           | Read                               |
| --------------------------------- | ---------------------------------- |
| Deciding whether to write a script | §2 first — it may not be bash      |
| Writing a script                  | All of it                          |
| Writing a Claude Code hook        | §2, §4, §9, and the checklist in §13 |
| Reviewing a contribution          | §13, then the section it points at |

Rules are `must`, `should`, or `may`. A `must` that is not machine-checkable is
a candidate defect in this document; see whether it can be moved into
[§12](#12-enforcement-configuration) before accepting it as prose.

---

## 2. When bash, and when Python

This is the most important section in the document, because the most common
defect in shell code is that it should not have been shell code.

Bash is excellent at one thing: **running other programs** — sequencing them,
wiring their exit codes together, and passing arguments and environment through.
It is poor at everything else. It has no data structures beyond flat arrays and a
string-keyed map, no exceptions, no types, no test framework that anyone
reaches for, and a quoting model in which the correct form is the verbose one.

### 2.1 The rule

A script **may** be bash when its job is orchestration:

- running a sequence of commands and reporting which failed;
- checking that tools exist and that preconditions hold before running them;
- adapting an interface — a hook entry point, a CI step, a `make` target — to a
  program that does the real work.

A script **must** be Python (under STD-001-PY) when any of the following holds:

| Condition                                                                   | Why                                                                                     |
| --------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| It parses or produces JSON, TOML, YAML, or XML beyond one field lookup      | `jq -r .a.b` is orchestration; building a document with string concatenation is a bug   |
| It embeds a program in another language (`python3 -`, `python3 -c`, a multi-line `awk` or `perl` program) | The embedded program cannot be linted, typed, imported, or tested. It is Python wearing a bash file extension |
| It needs a map, a nested structure, or a list of records                    | Associative arrays exist; code that depends on them is past the point bash is for        |
| It applies regular expressions to file contents to extract meaning         | Extraction logic has edge cases, and edge cases need tests                               |
| It contains logic that deserves a unit test                                 | See §11: bash logic is effectively untestable here                                        |
| It exceeds 150 lines                                                        | A soft proxy for all of the above                                                         |

The embedded-program case deserves emphasis. A bash script whose body is a
heredoc piped into `python3` gains nothing from being bash — its three lines of
shell are a `cd` and an interpreter call — and loses everything STD-001-PY
provides. Write the Python file and call it.

### 2.2 The seam

When a task has both an orchestration part and a logic part, split it at the
seam: bash runs the tools and handles exit codes; Python holds the logic and is
invoked as a program with arguments. The bash side never parses the Python
side's output beyond its exit code and, at most, a single line.

---

## 3. File layout, naming, and header

### 3.1 Two kinds of file

| Kind       | Name                 | Shebang                  | Executable bit | Strict-mode line |
| ---------- | -------------------- | ------------------------ | -------------- | ---------------- |
| Executable | `kebab-case.sh`      | `#!/usr/bin/env bash`    | required       | required         |
| Library    | `_kebab-case.sh`     | prohibited               | prohibited     | prohibited       |

**Executables** are run directly. Unlike Python under STD-001-PY §2.3, a shell
script's shebang is load-bearing: it is how the script is invoked, and hooks,
CI, and humans all call it by path. `#!/usr/bin/env bash`, never `#!/bin/sh` —
these scripts use bash features, and a `sh` shebang is a false claim that fails
on the first `[[`.

**Libraries** are sourced, never run. A shebang or an executable bit on one
invites running it, and a sourced file setting `set -e` changes the caller's
error handling without the caller asking. Libraries define functions and
constants and nothing else — the shell counterpart of STD-001-PY §2.1.

The `.sh` extension is required on both. It is what editors, ShellCheck, shfmt,
and the enforcement check use to find shell code.

### 3.2 File order

```text
#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
<purpose and usage comment block>
set -euo pipefail
<readonly constants>
<functions>
main "$@"
```

### 3.3 Purpose and usage block

Every executable opens with a comment that says what the script is for, why it
exists in the form it does, and how to call it. This is the shell equivalent of a
module docstring and it is the only documentation most scripts will ever have.

```bash
#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
# Positive and negative acceptance over a pinned corpus.
#
# Positive-only sweeps are a known trap: a parser that accepts everything passes
# them. A missing negative set is therefore a failure, not a skip.
#
#   scripts/corpus-sweep.sh
#
# Environment: SV2_CORPUS_DIR overrides the corpus root.
# Network: none.
set -euo pipefail
```

**Rules.**

1. The first paragraph states the purpose in one or two sentences.
2. Every flag and positional argument is listed with a usage line.
3. Every environment variable the script reads is named.
4. A script that touches the network **must** say so in its header. One that does
   not say so **must not** touch the network. See §7.3.
5. A hook entry point **must** state its failure policy. See §9.2.

### 3.4 Program header and history

The program header rules of STD-001-PY §2.3 apply unchanged, with the one
placement difference the shebang forces: the header is exactly two `#` comment
lines, immediately **after** the shebang and before the purpose block.

```bash
#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
```

Both lines, verbatim, on every shell file. A library has no shebang (§3.1), so
there they are lines 1 and 2. The point of an SPDX identifier and a copyright line
is that they travel with a file copied out of the repository, which is the whole
of what MIT asks for.

**No historical change comments**, for the same reason as there: version control
answers "what changed" accurately and a comment answers it inaccurately within two
commits.

**Enforcement.** `scripts/check_headers.py`, which reads the required strings from
`[tool.sv2.headers]` in `pyproject.toml` and walks every `.sh` file under
`scripts/` and `.claude/scripts/` alongside the Python and Rust sources. A shell
file's opening comment block starts at the shebang, so the two required lines sit
inside it. They do not disturb the `set -euo pipefail` rule of §4.1 either:
`check_shell_standard.py` ignores comment lines when it looks for the first
command.

---

## 4. Strict mode and errors

### 4.1 The strict-mode line

**Every executable must set `set -euo pipefail` before its first command.**

| Option     | Effect                                                    | Without it                                                     |
| ---------- | --------------------------------------------------------- | -------------------------------------------------------------- |
| `-e`       | Exit when a command fails outside a condition             | A failed `cd` is followed by the rest of the script, in the wrong directory |
| `-u`       | Referencing an unset variable is an error                 | `rm -rf "$BUILD_DIR/"` with `BUILD_DIR` unset                  |
| `-o pipefail` | A pipeline fails if any stage fails                   | `broken-tool \| sort` succeeds because `sort` did              |

A script that deliberately continues past failures — a gate that runs every check
and reports all of them — still sets `-e`. It expresses the continuation
explicitly, with the failure inside a condition, where `-e` does not apply:

```bash
run_check() {
  local name=$1
  shift
  local out
  if out=$("$@" 2>&1); then
    printf '  pass  %s\n' "${name}"
  else
    printf '  FAIL  %s\n' "${name}"
    printf '%s\n' "${out}" | sed -n '1,30s/^/        /p'
    failed=1
  fi
}
```

Omitting `-e` because some commands are expected to fail disables it for all the
commands that are not.

### 4.2 Where `-e` does not protect you

`-e` has well-known holes. Each has a rule, and ShellCheck enforces most of them.

1. **`local var=$(cmd)` masks the failure of `cmd`** — the exit status is that of
   `local`. Declare and assign separately (SC2155).
2. **Functions called in a condition run with `-e` suspended**, including every
   command inside them. `if my_function; then` does not stop on a failure inside
   `my_function`. Keep functions used as predicates small and explicit about their
   own return codes (`check-set-e-suppressed`).
3. **`((count++))` returns 1 when `count` was 0**, which exits the script.
   Use `count=$((count + 1))`.
4. **`cmd | head -n N` under `pipefail`** can fail with SIGPIPE (141) when the
   producer writes more than `head` reads. Use `sed -n '1,Np'`, which consumes its
   whole input, when the producer's output is unbounded.
5. **Command substitution inside another command** (`echo "$(cmd)"`) discards
   `cmd`'s status. Assign first, then use (`check-extra-masked-returns`).

### 4.3 Exit codes

| Code | Meaning                                                       |
| ---- | ------------------------------------------------------------- |
| 0    | Success, or the check passed, or the check is inert because its inputs are absent and the header says so |
| 1    | The check failed, or the operation failed                     |
| 2    | Usage error — or, in a Claude Code hook, a **blocking** result. See §9.3 |
| other | Never returned deliberately                                  |

A check that cannot run because its inputs do not exist yet **should** exit 0 and
say why on stdout ("no Xtext vendored — check inert"). A check that cannot run
because a required tool is missing **must** exit non-zero unless its header
declares it optional. The difference is that the first is a state of the
repository and the second is a broken environment.

### 4.4 Temporary files and cleanup

Temporary files come from `mktemp` and are removed by a trap registered
immediately after creation:

```bash
tmp=$(mktemp)
trap 'rm -f -- "${tmp}"' EXIT
```

Never write to a fixed path in `/tmp`. Never leave cleanup to the success path.

### 4.5 Error messages

Diagnostics go to stderr, name the thing that failed, and — where one exists —
state the command that fixes it:

```bash
printf 'no production inventory — run scripts/extract_productions.py\n' >&2
```

---

## 5. Quoting, expansions, and builtins

These are the rules that prevent the defects shell code is known for. ShellCheck
enforces nearly all of them; §12 makes its findings build failures.

1. **Quote every expansion**: `"${var}"`, `"$(cmd)"`, `"$@"`. The exceptions are
   the right-hand side of an assignment and inside `[[ ]]`, and even there quoting
   is harmless. Unquoted expansion splits on whitespace and expands globs, and a
   filename with a space is not an edge case.
2. **`[[ ]]`, never `[ ]` or `test`** (`require-double-brackets`). `[[` does not
   word-split, supports `&&`, `||`, and `=~`, and fails loudly on a syntax error.
3. **`$(...)`, never backticks** (SC2006). They nest and they read.
4. **Arrays for argument lists.** A command built in a string is a command that
   breaks on the first argument containing a space.

   ```bash
   args=(--workspace --all-targets)
   [[ -n "${crate}" ]] && args=(-p "${crate}" --all-targets)
   cargo clippy "${args[@]}"
   ```

5. **`printf`, not `echo`, for anything that is not a fixed string.** `echo`
   interprets a leading `-n` or `-e` in the data.
6. **`local` for every function variable, `readonly` for every constant.**
7. **No `eval`.** There is no quoting of its argument that is both correct and
   readable, and there is always an array-based alternative.
8. **Never parse `ls`.** Use a glob, or `find ... -print0` with
   `while IFS= read -r -d '' f`. Globs that may match nothing need `shopt -s
   nullglob` or an existence test.
9. **`command -v`, not `which`** (`deprecate-which`). `which` is not a builtin,
   is not guaranteed present, and its output format varies.
10. **`read -r`, always** (SC2162). Without `-r`, backslashes are eaten.
11. **`cd` in a subshell or with an explicit return path.** A script that `cd`s
    and then sources or calls a function that assumes the original directory is a
    script with an ordering bug.

---

## 6. Structure and shape

### 6.1 `main`

Every executable longer than about twenty lines defines its logic in functions and
ends with a single call:

```bash
main() {
  cd "$(dirname "$0")/.."
  ...
}

main "$@"
```

This makes the entry point obvious, keeps variables local by default, and ensures
a truncated download of the script executes nothing — bash parses the whole
`main` definition before running the final line.

### 6.2 Thresholds

| Property                   | Limit | Check                        | Why                                                        |
| -------------------------- | ----- | ---------------------------- | ---------------------------------------------------------- |
| Script length              | 150 lines | `check_shell_standard.py`  | Past this, §2 almost certainly applies                     |
| Nested blocks              | 3     | review                       | Same limit, same reason as STD-001-PY §5.1                 |
| Function length            | 40 lines | review                    | A function that does not fit on a screen in bash is two functions |
| Positional parameters used by a function | 3 | review          | Beyond three, name them with `local` on the first line     |

The script-length limit is checked; the others are review rules because no
available tool measures them reliably for bash. If one becomes checkable, it moves
into §12.

---

## 7. Input, output, and side effects

### 7.1 stdout is the product

stdout carries what the script produces — a report, a path, a status line — and
nothing else. Progress chatter, warnings, and errors go to stderr. A script whose
stdout mixes both cannot be composed, and in a hook (§9.3) stdout can be injected
directly into a model's context.

### 7.2 What must never be printed

STD-001-PY §8.6 applies unchanged: no credentials, no environment variable values,
no file contents, no vendored source text. `set -x` **must not** be left on
in a committed script, because it prints every expanded argument, including
secrets passed as arguments.

### 7.3 Network

The network is off by default. A script that fetches anything **must** declare
`Network: required` in its header, **must** be a deliberate, separately invoked
act, and **must never** be called from a gate, a build, a test, or a hook. A gate
that fetches passes on a developer workstation and fails in the enclave, and the
enclave is the target environment.

### 7.4 Working directory

A script that operates on repository paths establishes its working directory
explicitly, from its own location, on its first line of `main`:

```bash
cd "$(dirname "$0")/.."
```

It never assumes the caller's directory. A hook uses `"${CLAUDE_PROJECT_DIR}"`
instead, because Claude Code guarantees it and the hook's own location is not the
project in every configuration.

---

## 8. Portability and dependencies

1. **Target bash 5.1 and GNU coreutils** on RHEL 9. GNU options (`sed -i`,
   `find -print0`, `sort -z`, `readarray`) are permitted. BSD compatibility is not
   a goal, and pretending otherwise produces scripts that are portable to nothing.
2. **Check required tools before using them**, once, near the top of `main`, with
   a message naming the tool and what it is needed for:

   ```bash
   command -v cargo >/dev/null 2>&1 || { printf 'cargo not on PATH\n' >&2; exit 1; }
   ```

3. **Optional tools** are checked the same way and skipped with a visible notice,
   never silently: `printf '  skip  cargo checks (cargo not on PATH)\n'`.
4. **No new tool dependency without a reason in the header.** `jq`, `yq`, and
   friends are orchestration aids, not a data layer; where they would be needed
   for more than one field, §2 applies and the script is Python.
5. **Locale-sensitive operations set it.** Anything whose output is hashed,
   compared, or committed runs with `LC_ALL=C` — `sort` order otherwise varies
   by environment.

---

## 9. Claude Code hooks

Hooks are the place in the repository where a defect in a shell script silently
disables an enforcement layer. They get their own rules.

### 9.1 The shim pattern

**A hook entry point is a shim.** It checks that its interpreter exists, applies
its failure policy, and `exec`s a Python module that holds the logic.

```bash
#!/usr/bin/env bash
# PreToolUse: blocks edits to generated, vendored, and pinned paths.
# Failure policy: CLOSED — a guard that cannot run must block, not pass.
set -euo pipefail
if ! command -v python3.11 >/dev/null 2>&1; then
  printf 'BLOCKED: this hook needs python3.11 and it is not on PATH.\n' >&2
  exit 2
fi
exec python3.11 "$(dirname "$0")/hook_protect_paths.py"
```

The logic is Python because every hook parses JSON from stdin (§2.1), and because
the logic of a guard is exactly the code that most needs tests.

The shim is bash rather than a direct interpreter call in `settings.json` because of
§9.2: a missing interpreter exits 127, and Claude Code treats 127 as a
non-blocking error. A guard invoked directly through a missing interpreter
**fails open** while appearing configured.

**Name the interpreter version explicitly.** On RHEL 9, `/usr/bin/python3` is
the platform Python (3.9) and newer interpreters are separate binaries such as
`python3.11`. A shim that calls bare `python3` runs whichever one is first on the
hook's `PATH`, which differs between a workstation and the enclave.

### 9.2 Failure policy

**Every hook must state its failure policy in its header**, and the shim must
implement it.

| Policy  | Meaning                                         | Use for                                                 |
| ------- | ----------------------------------------------- | ------------------------------------------------------- |
| CLOSED  | Cannot run → exit 2, block the action           | PreToolUse guards. A guard that passes when broken is worse than none, because it is trusted |
| OPEN    | Cannot run → exit 0, allow the action           | Stop, PostToolUse, SessionStart. Blocking here traps the session instead of protecting anything |

### 9.3 Decisions: JSON on stdout, exit 2 only as a fallback

A hook reports a decision by printing JSON on stdout and exiting 0. Exit 2 with
stderr also blocks, but Claude receives it framed as a hook error prefixed with
the hook's command line; the JSON form delivers only the reason. Exit 2 is kept
for the one case JSON cannot cover: a CLOSED hook that failed before it could
decide.

Observed on Claude Code, 2026-09-15, by probing each channel with a unique token
(the published reference was ambiguous on several rows, so this table records
behavior, not documentation):

| Event        | Output                                                   | Effect                                                  | What Claude receives                          |
| ------------ | -------------------------------------------------------- | ------------------------------------------------------- | --------------------------------------------- |
| PreToolUse   | `hookSpecificOutput.permissionDecision: "deny"` + reason | Tool call cancelled                                     | The reason only                               |
| PreToolUse   | exit 2 + stderr                                          | Tool call cancelled                                     | "hook error", command line, stderr            |
| PostToolUse  | `hookSpecificOutput.additionalContext`                   | Nothing blocked (the tool already ran)                  | The text, as additional context               |
| PostToolUse  | `decision: "block"` + `reason`                           | Nothing undone                                          | "blocking error" + reason                     |
| PostToolUse  | exit 2 + stderr                                          | Nothing undone                                          | "blocking error", command line, stderr        |
| Stop         | `decision: "block"` + `reason`                           | Turn continues                                          | The reason, as Stop hook feedback             |
| any          | `systemMessage`                                          | None                                                    | **Nothing** — it is for the user              |
| SessionStart | plain stdout, exit 0                                     | —                                                       | **stdout is injected into context**           |

Rules that follow:

1. **Decisions are JSON.** Deny with `permissionDecision`, feed findings back with
   `additionalContext`, hold a turn open with `decision: "block"`.
2. **Every non-zero code other than 2 is a pass.** A hook that crashes with an
   unhandled error allows the action it was meant to evaluate. Hook logic catches
   its failures in a top-level handler and maps them deliberately: exit 2 for
   CLOSED, exit 0 for OPEN.
3. **`systemMessage` is not a channel to Claude.** Use it only for text meant for
   the person at the terminal.
4. **SessionStart stdout is context.** Emit data, not prose, and never emit
   anything §7.2 prohibits.
5. **A PreToolUse guard for file tools does not guard Bash.** A shell command can
   write any path. A guard that protects paths must also match `Bash`, tokenize
   the command, and treat an unclassifiable mention of a protected path as a
   denial. It remains a tripwire rather than a boundary — a script file or
   variable indirection can still reach the path — so pair it with a check that
   detects the change after the fact.

### 9.4 Loop safety

A Stop hook that blocks **must** honor `stop_hook_active`: when Claude Code reports
the hook has already blocked once this turn, release with exit 0. Without it, a
persistently failing gate loops forever.

### 9.5 Configuration

- Hook commands in `.claude/settings.json` reference the shim through
  `"$CLAUDE_PROJECT_DIR"`, quoted, never a relative path.
- Every hook has an explicit `timeout` sized to its worst case, not its typical
  case.
- Hook shims live in `.claude/scripts/`, with the Python they call. Project
  tooling that runs without Claude Code lives in `scripts/`.

---

## 10. Naming

| Kind                  | Convention             | Note                                                         |
| --------------------- | ---------------------- | ------------------------------------------------------------ |
| Executable script     | `kebab-case.sh`        | Verb or verb phrase: `corpus-sweep.sh`, `grammar-preflight.sh` |
| Library               | `_kebab-case.sh`       | Sourced only                                                 |
| Hook shim             | `hook-<purpose>.sh`    | Paired with `hook_<purpose>.py`                              |
| Function              | `snake_case`           | Verb phrase: `run_check`, `require_tool`                     |
| Predicate function    | `is_`, `has_` prefix   | Returns 0 or 1, no output, no side effects                   |
| Local variable        | `snake_case`           |                                                              |
| Constant, exported or environment variable | `UPPER_SNAKE` | Prefix project variables: `SV2_CORPUS_DIR`          |

The banned module suffixes of STD-001-PY §9 apply to libraries: no
`_utils.sh`, `_helpers.sh`, `_common.sh`, `_misc.sh`.

---

## 11. Tests

Bash has no test framework in use here, and this document does not introduce one.
That is a deliberate consequence of §2 rather than a gap: **logic that deserves a
test belongs in Python, where STD-001-PY §10 governs its tests.** What remains in
bash is orchestration, and orchestration is verified by running it.

1. Every executable **must** pass `bash -n` and ShellCheck. Both run in the gate.
2. Every executable **should** have a smoke invocation exercised by the gate or by
   CI — the gate itself runs most checks this way.
3. A hook shim **must** be exercised against both branches of its failure policy
   when it is introduced or changed: once with its interpreter present, once with
   `PATH` stripped of it.
4. bats **may** be used for a script whose orchestration is intricate enough to
   need cases. If one needs many cases, re-read §2.

---

## 12. Enforcement configuration

### 12.1 `.shellcheckrc`

At the repository root. Individual scripts **must not** carry file-wide
`# shellcheck disable=` directives; a single-line suppression is permitted with a
reason on the same line.

```ini
shell=bash
external-sources=true
enable=avoid-nullary-conditions
enable=check-extra-masked-returns
enable=check-set-e-suppressed
enable=deprecate-which
enable=quote-safe-variables
enable=require-double-brackets
```

```bash
# shellcheck disable=SC2034  # read by the sourcing script
```

### 12.2 `.editorconfig` (shfmt)

shfmt reads formatting options from `.editorconfig` when invoked without flags.

```ini
[*.sh]
indent_style = space
indent_size = 2
shell_variant = bash
binary_next_line = true
switch_case_indent = true
space_redirects = false
function_next_line = false
```

### 12.3 `scripts/check_shell_standard.py`

The rules neither tool expresses, each mapped to its section:

| Rule                                                            | Section |
| --------------------------------------------------------------- | ------- |
| File name matches `_?kebab-case.sh`                             | §3.1, §10 |
| Executable: line 1 is exactly `#!/usr/bin/env bash`             | §3.1    |
| Executable: has the executable bit                              | §3.1    |
| Executable: `set -euo pipefail` precedes the first command      | §4.1    |
| Library: no shebang, no executable bit, no `set` line           | §3.1    |
| No embedded interpreter program (`python3 -`, `python3 -c`, `perl -e`, `node -e`, multi-line `awk`) | §2.1 |
| No `eval`                                                       | §5      |
| No file-wide `shellcheck disable`                               | §12.1   |
| At most 150 lines                                               | §6.2    |

### 12.4 CI command set

```bash
python3.11 scripts/check_shell_standard.py
python3.11 scripts/check_headers.py
shellcheck scripts/*.sh .claude/scripts/*.sh
shfmt -d scripts .claude/scripts
```

All four run in the gate. `check_headers.py` covers Python and Rust as well
(§3.4). ShellCheck and shfmt are skipped with a visible notice
when not installed, so a workstation without them is not blocked; CI and the
enclave image **must** provide them. See §14.

---

## 13. Reviewing a shell contribution

Checks in the order that fails fastest.

1. §2: should this be bash at all? No embedded programs, no structured-data
   parsing, under 150 lines.
2. Correct kind: executable or library, with matching shebang and executable bit.
3. `set -euo pipefail` present, and no failure-tolerant section relying on its
   absence.
4. Purpose and usage block present; environment variables and network use
   declared.
5. Every expansion quoted; `[[ ]]`; arrays for argument lists; no `eval`; no
   parsed `ls`.
6. None of the §4.2 holes: no `local x=$(...)`, no `((n++))`, no unbounded
   producer piped into `head`.
7. stdout carries only the product; diagnostics on stderr; nothing from §7.2.
8. No network unless declared, and never from a gate, build, test, or hook.
9. For a hook: shim pattern, failure policy stated and implemented, exercised with
   and without its interpreter, `stop_hook_active` honored for Stop, decisions
   returned as JSON rather than exit 2 (§9.3), and a path guard that also matches
   `Bash`.
10. `check_shell_standard.py`, ShellCheck, and `shfmt -d` all pass with no
    file-wide suppressions.

Checks 1 and 9 are the ones a reviewer fluent in bash but new to this codebase
will not think to make. Check 1 is the cheapest defect to fix at review and the
most expensive later; check 9 is the one whose violation is invisible, because a
hook that fails open reports nothing.

---

## 14. Open items

1. **Tool availability in the enclave.** ShellCheck and shfmt are static binaries
   and must be added to the air-gapped toolchain image. Until they are, the gate
   skips them there with a notice, and §12 is enforced on workstations and CI only.
2. **Complexity measurement.** Nesting depth and function length are review rules
   (§6.2). If a maintained tool measures them for bash, move them into §12.

---

## Appendix A. Correspondence with STD-001-PY

| STD-001-PY                         | STD-003-SH                                    | Note                                                                  |
| ---------------------------------- | --------------------------------------------- | --------------------------------------------------------------------- |
| §2 Script layout                   | §3.1 Two kinds of file                        | `scripts/` and `.claude/scripts/` hold both languages, split the same way |
| §2.1 A private module defines and does not execute | §3.1 Libraries define functions and constants only | Neither may execute anything on load          |
| §2.2 The file ends with a call to `main` | §6.1 `main "$@"`                        | `main` carries the exit code on both sides                            |
| §2.3 No shebang                    | §3.1 Shebang required                         | Deliberately opposite: shell scripts are invoked by path, Python modules are not |
| §2.3 Program header, no history    | §3.4                                          | The same two lines; after the shebang rather than above the docstring |
| §4 Modeling data                   | §2.1                                          | Data modeling is a reason the code is Python                           |
| §5.1 Thresholds                    | §6.2                                          | Fewer are machine-checkable in bash                                    |
| §6 Typing                          | —                                             | No equivalent; another reason for §2                                   |
| §7 Errors                          | §4                                            | `set -euo pipefail` and exit codes replace exceptions                  |
| §8.2 stdout is the product         | §7.1, §9.3                                    | A hook's stdout is a protocol channel on both sides                    |
| §8.6 What must never be printed    | §7.2                                          | Unchanged, plus `set -x`                                               |
| §9 Naming, banned suffixes         | §10                                           | Banned suffixes apply to libraries                                     |
| §10 Tests                          | §11                                           | Logic that needs tests moves to Python                                 |
| §12 Enforcement configuration      | §12                                           | ShellCheck, shfmt, and `check_shell_standard.py`                       |
| §13 Review checklist               | §13                                           |                                                                       |
