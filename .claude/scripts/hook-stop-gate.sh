#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
# Stop hook entry point: keeps the turn open while the gate is red.
# The logic is in hook_stop_gate.py.
#
# Failure policy: OPEN. If this hook cannot run it releases with exit 0, because
# blocking here would trap the session rather than protect anything
# (STD-003-SH §9.2).
set -euo pipefail

if ! command -v python3.11 >/dev/null 2>&1; then
  printf 'hook-stop-gate: python3.11 not on PATH; skipping.\n' >&2
  exit 0
fi
here=$(dirname "${BASH_SOURCE[0]}")
exec python3.11 "${here}/hook_stop_gate.py"
