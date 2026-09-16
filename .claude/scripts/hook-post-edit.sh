#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
# PostToolUse hook entry point: formats and lints an edited Rust file.
# The logic is in hook_post_edit.py.
#
# Failure policy: OPEN. If this hook cannot run it releases with exit 0, because
# blocking here would trap the session rather than protect anything
# (STD-003-SH §9.2).
set -euo pipefail

if ! command -v python3.11 >/dev/null 2>&1; then
  printf 'hook-post-edit: python3.11 not on PATH; skipping.\n' >&2
  exit 0
fi
here=$(dirname "${BASH_SOURCE[0]}")
exec python3.11 "${here}/hook_post_edit.py"
