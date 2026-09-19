#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
# Every deterministic check, cheapest first. Exit 0 only when all pass.
# One definition of "done" — the Stop hook and CI both call exactly this.
#
#   scripts/gate.sh
#
# This is the one script in scripts/ that calls into .claude/scripts/: the gate
# is the union of every check, including the frozen-grammar, decisions-index,
# and state checks that live beside the Claude Code tooling they serve.
#
# Network: none. vendor_sync.py is the only script that fetches, and it is never
# called from a gate or a build.
#
# Two interpreters. Every check script runs under the system ${PY}, standard library
# only, so the gate runs where nothing is installed (STD-001-PY §3.1). The TEST
# TOOLING — pytest, and jsonschema for the state schema's full check — comes from
# the project .venv that `uv sync` builds from pyproject's dev group, when it exists;
# without one, the tests are skipped and the schema check takes its structural
# fallback, exactly as before.
set -euo pipefail

readonly PY=python3.12
readonly VENV_PY=.venv/bin/python
failed=0

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

# run_optional <name> <tool> [args...]: run_check when the tool is installed.
run_optional() {
  local name=$1 tool=$2
  if command -v "${tool}" >/dev/null 2>&1; then
    run_check "$@"
  else
    printf '  skip  %s (%s not on PATH)\n' "${name}" "${tool}"
  fi
}

main() {
  cd "$(dirname "$0")/.."
  if ! command -v "${PY}" >/dev/null 2>&1; then
    printf 'gate: %s not on PATH\n' "${PY}" >&2
    exit 1
  fi
  # The interpreter for the test tooling: the project .venv when `uv sync` has built
  # one, else the system interpreter, which may simply not have pytest.
  local tools=${PY}
  if [[ -x ${VENV_PY} ]]; then
    tools=${VENV_PY}
  fi

  echo "gate: running deterministic checks"

  # --- the checkout itself ---
  run_check "path case" "${PY}" scripts/check_path_case.py

  # --- pinned inputs ---
  run_check "vendor hashes" "${PY}" scripts/vendor_verify.py
  run_check "namespace link" "${PY}" scripts/check_namespace_link.py
  run_check "wiki receipts" "${PY}" .claude/scripts/build_wiki_receipts.py --check
  run_check "citations" "${PY}" scripts/check_citations.py

  # --- derived artifacts ---
  run_check "derived artifacts" "${PY}" scripts/extract_productions.py --check
  run_check "derived BNF" "${PY}" scripts/extract_bnf.py --check
  run_check "syntax kinds" "${PY}" scripts/gen_syntax_kinds.py --check
  run_check "metaclass map" "${PY}" scripts/check_metaclass_map.py
  run_check "grammar diff" "${PY}" scripts/grammar_diff.py --check
  run_check "decisions index" "${PY}" .claude/scripts/index_decisions.py --check
  run_check "bnf coverage" "${PY}" scripts/bnf_coverage.py --check
  run_check "frozen grammar" "${PY}" .claude/scripts/grammar_freeze.py --check
  # Inert without the local clause export, which is not in the repository.
  run_check "derivation prose" "${PY}" .claude/scripts/check_derivation_text.py

  # --- code ---
  if command -v cargo >/dev/null 2>&1; then
    run_check "rust workspace" "${PY}" scripts/check_rust_workspace.py
    run_check "cargo fmt" cargo fmt --all --check
    run_check "cargo clippy" cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
    run_check "cargo test" cargo test --workspace --all-features --locked
    run_check "cargo doc" env RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
    # Offline subset: bans, licenses, and sources read only the lockfile and
    # deny.toml. `cargo deny check advisories` needs the advisory database, which
    # is a network fetch, so it runs in CI against the mirror (STD-002-RS §12
    # rule 4) rather than here.
    run_optional "cargo deny" cargo-deny --offline check bans licenses sources
  else
    printf '  skip  cargo checks (cargo not on PATH)\n'
  fi

  # --- scripts: STD-001-PY and STD-003-SH ---
  run_check "shell standard" "${PY}" scripts/check_shell_standard.py
  run_check "program headers" "${PY}" scripts/check_headers.py
  run_check "standards config" "${PY}" scripts/check_standards_config.py
  run_check "rust patterns" "${PY}" scripts/check_rust_patterns.py
  run_optional "shellcheck" shellcheck scripts/*.sh .claude/scripts/*.sh
  run_optional "shfmt" shfmt -d scripts .claude/scripts
  run_optional "ruff" ruff check
  run_optional "ruff format" ruff format --check
  run_optional "mypy" mypy
  if "${tools}" -m pytest --version >/dev/null 2>&1; then
    run_check "script tests" "${tools}" -m pytest -q
  else
    printf '  skip  script tests (pytest not installed for %s; run uv sync)\n' "${tools}"
  fi

  # --- corpus and state ---
  run_check "corpus sweep" scripts/corpus-sweep.sh
  # Under ${tools} so jsonschema is there for the full check when .venv is; the
  # script falls back to a structural check when it is not.
  run_check "state schema" "${tools}" .claude/scripts/validate_state.py
  run_check "state file" "${PY}" .claude/scripts/regen_state.py --check

  if ((failed == 0)); then
    echo "gate: green"
  else
    echo "gate: RED"
  fi
  exit "${failed}"
}

main "$@"
