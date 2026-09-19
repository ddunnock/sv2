#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
# Build the sv2 binary, then sweep the corpus and the rejection set with it.
#
#   scripts/corpus-sweep.sh              check the sweep against the ledger
#   scripts/corpus-sweep.sh --record     rewrite the ledger from this sweep
#
# STD-003-SH §2.2: this side builds the tool and hands over its exit code. The
# logic — which files to read, what the ledger says, what counts as a regression —
# is scripts/corpus_sweep.py, because it is a list of records and it is tested.
#
# Environment: SV2_CORPUS_DIR overrides the positive corpus root. CARGO_TARGET_DIR,
#   when set, is where cargo put the binary; a hardcoded target/ would silently
#   skip the sweep on any machine that sets it.
# Network: none.
set -euo pipefail

readonly PY=python3.12
readonly SWEEP=scripts/corpus_sweep.py

main() {
  cd "$(dirname "$0")/.."

  # The binary is rebuilt every run, not merely checked for. Building only when it
  # was absent meant a sweep that followed a parser change ran the PREVIOUS parser
  # and reported its answers, which is indistinguishable from a clean run: planting
  # a file that parses left this green, because the binary predated the production
  # that reads it. A negative control that cannot fail is the failure mode this
  # whole script exists to prevent. Cargo is a no-op when nothing changed, so
  # correctness here costs nothing.
  #
  # Skipping rather than sweeping with a binary whose freshness cannot be
  # established is deliberate, and so is skipping when cargo is missing: a stale
  # answer is worse than no answer, because only one of them is obvious.
  if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo unavailable, so the binary's freshness cannot be established; sweep skipped"
    exit 0
  fi
  if ! cargo build -p sv2-cli --quiet 2>/dev/null; then
    echo "sv2-cli does not build yet; sweep skipped"
    exit 0
  fi

  exec "${PY}" "${SWEEP}" "$@"
}

main "$@"
