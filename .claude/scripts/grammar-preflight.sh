#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
# Phase 1. Deterministic. Refuses to start derivation on unsound inputs.
#
# Derivation is the expensive phase. Starting it against a drifted pin or a
# stale inventory means throwing the work away, so this gate is deliberately
# strict and runs first.
#
#   .claude/scripts/grammar-preflight.sh
#
# Environment: SV2_WIKI_CLAUSES overrides the specification clause export.
# Network: none.
set -euo pipefail

readonly PY=python3.11
failed=0

# run_check <label> <command...>: run the command, report pass or FAIL with its output.
run_check() {
  local label=$1
  shift
  local out
  if out=$("$@" 2>&1); then
    printf '  pass  %s\n' "${label}"
  else
    printf '  FAIL  %s\n' "${label}"
    printf '%s\n' "${out}" | sed 's/^/        /'
    failed=1
  fi
}

main() {
  cd "$(dirname "$0")/../.."
  local clauses=${SV2_WIKI_CLAUSES:-vendor/wiki/bnf-clauses.json} f

  echo "phase 1 — preflight"
  run_check scripts/vendor_verify.py "${PY}" scripts/vendor_verify.py
  run_check scripts/check_namespace_link.py "${PY}" scripts/check_namespace_link.py
  run_check scripts/extract_productions.py "${PY}" scripts/extract_productions.py --check
  run_check scripts/check_metaclass_map.py "${PY}" scripts/check_metaclass_map.py

  for f in .claude/state/grammar/productions.json .claude/state/grammar/keywords.json; do
    if [[ ! -f ${f} ]]; then
      printf '  FAIL  missing %s\n' "${f}"
      failed=1
    fi
  done

  if [[ ! -f ${clauses} ]]; then
    printf '  FAIL  no specification BNF clauses at %s\n' "${clauses}"
    echo "        Export from the KerML / SysML v2 wiki as {production: {ref, text}}."
    echo "        Deriving from the Xtext alone is not acceptable — see docs/DERIVATION.md."
    failed=1
  fi

  if ((failed == 0)); then
    echo "preflight green — derivation may start"
  else
    echo "preflight RED — do not derive"
  fi
  exit "${failed}"
}

main "$@"
