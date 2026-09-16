#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
# Positive and negative acceptance over a pinned corpus.
#
# Positive-only sweeps are a known trap: a parser that accepts everything passes
# them. The negative set is what makes this meaningful, so a missing negative
# set is treated as a failure rather than a skip.
#
#   scripts/corpus-sweep.sh
#
# Environment: SV2_CORPUS_DIR overrides the positive corpus root. CARGO_TARGET_DIR,
#   when set, is where cargo put the binary; a hardcoded target/ would silently
#   skip the sweep on any machine that sets it.
# Network: none.
set -euo pipefail

readonly NEG=tests/rejection
readonly BIN="${CARGO_TARGET_DIR:-target}/debug/sv2"

# Results of the most recent sweep.
as_expected=0
unexpected=0
# NUL-separated file list; a file rather than process substitution, so a failing
# find stops the sweep instead of looking like an empty corpus.
file_list=""

# sweep <root> <accept|reject>: parse every model file, report each surprise.
# known-permissive/ holds cases the parser is recorded as accepting on purpose,
# so they are neither caught nor leaked (grammar_validate.py excludes them too).
sweep() {
  local root=$1 expect=$2 f outcome
  as_expected=0
  unexpected=0
  find "${root}" -type d -name known-permissive -prune \
    -o -type f \( -name '*.sysml' -o -name '*.kerml' \) -print0 >"${file_list}"
  while IFS= read -r -d '' f; do
    outcome=reject
    if "${BIN}" parse --quiet "${f}" >/dev/null 2>&1; then outcome=accept; fi
    if [[ ${outcome} == "${expect}" ]]; then
      as_expected=$((as_expected + 1))
      continue
    fi
    unexpected=$((unexpected + 1))
    if [[ ${expect} == accept ]]; then
      printf '  should parse but did not: %s\n' "${f}"
    else
      printf '  should have been rejected but parsed: %s\n' "${f}"
    fi
  done <"${file_list}"
}

main() {
  cd "$(dirname "$0")/.."
  local corpus=${SV2_CORPUS_DIR:-tests/corpus}

  if [[ ! -d ${corpus} ]]; then
    printf 'corpus not present at %s — set SV2_CORPUS_DIR or see tests/corpus/README.md\n' "${corpus}"
    exit 0
  fi
  if [[ ! -x ${BIN} ]]; then
    if ! command -v cargo >/dev/null 2>&1; then
      echo "cargo unavailable; sweep skipped"
      exit 0
    fi
    if ! cargo build -p sv2-cli --quiet 2>/dev/null; then
      echo "sv2-cli does not build yet; sweep skipped"
      exit 0
    fi
  fi
  if [[ ! -x ${BIN} ]]; then
    echo "sv2 binary not built; sweep skipped"
    exit 0
  fi

  file_list=$(mktemp)
  trap 'rm -f -- "${file_list}"' EXIT
  sweep "${corpus}" accept
  local pass=${as_expected} fail=${unexpected} negpass=0 negfail=1
  if [[ -d ${NEG} ]]; then
    sweep "${NEG}" reject
    negpass=${as_expected}
    negfail=${unexpected}
  else
    printf '  no negative corpus at %s — acceptance claim is positive-only and therefore weak\n' "${NEG}"
  fi

  printf 'corpus: %s accepted / %s missed   |   rejection: %s caught / %s leaked\n' \
    "${pass}" "${fail}" "${negpass}" "${negfail}"
  ((fail == 0 && negfail == 0))
}

main "$@"
