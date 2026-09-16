#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
# PreToolUse hook entry point: blocks edits to generated, vendored, and pinned
# paths. The logic is in hook_protect_paths.py.
#
# Failure policy: CLOSED. Exit 2 cancels the tool call, and a guard that cannot
# run must block rather than pass (STD-003-SH §9.2).
set -euo pipefail

if ! command -v python3.11 >/dev/null 2>&1; then
  printf 'BLOCKED: this hook needs python3.11 and it is not on PATH.\n' >&2
  printf 'Install it, or remove the hook from .claude/settings.json deliberately.\n' >&2
  exit 2
fi
here=$(dirname "${BASH_SOURCE[0]}")
exec python3.11 "${here}/hook_protect_paths.py"
