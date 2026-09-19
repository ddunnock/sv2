#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
# SessionStart hook entry point: injects the work state into context.
# The logic is in hook_session_start.py.
#
# Failure policy: OPEN. If this hook cannot run it releases with exit 0, because
# blocking here would trap the session rather than protect anything
# (STD-003-SH §9.2).
set -euo pipefail

if ! command -v python3.12 >/dev/null 2>&1; then
  printf 'hook-session-start: python3.12 not on PATH; skipping.\n' >&2
  exit 0
fi
here=$(dirname "${BASH_SOURCE[0]}")
exec python3.12 "${here}/hook_session_start.py"
