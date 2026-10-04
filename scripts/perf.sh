#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
# Build the performance probe, then measure, ratchet, or step a series with it.
#
#   scripts/perf.sh check                         the gate's ratchet
#   scripts/perf.sh measure                       counters and timing for this tree
#   scripts/perf.sh wasm                          timing under WebAssembly engines (FIT-4)
#   scripts/perf.sh record --reason "..."         re-baseline, deliberately
#   scripts/perf.sh series open|step|close|abandon ...
#
# STD-003-SH §2.2: this side builds the tool and hands over its exit code. The logic
# is scripts/perf.py (ADR-0024).
#
# Two builds of one probe. Counters come from a dev build with the `counters` feature,
# because counts do not depend on the profile and the gate already has dev artifacts.
# Timing comes from a release build without the feature, because counting would be timed.
# `check` needs only the first, so the gate never pays for a release build.
#
# `wasm` adds a third build, the probe for wasm32-wasip1 (rust-toolchain.toml installs
# the target), and looks for the engines that run it: wasmtime and bun. A missing engine
# is reported as skipped, not as a failure.
#
# Environment: CARGO_TARGET_DIR, when set, is where cargo puts the binaries.
#   WASMTIME_HOME, when set, is where wasmtime's installer put it.
# Network: none.
set -euo pipefail

readonly PY=python3.12
readonly PERF=scripts/perf.py
readonly PROBE=perf_probe
readonly WASI_TARGET=wasm32-wasip1

# engine <name> <home-relative fallback>: the engine's path, or nothing when absent.
engine() {
  local name=$1 fallback=$2
  if command -v "${name}" >/dev/null 2>&1; then
    command -v "${name}"
  elif [[ -x ${fallback} ]]; then
    printf '%s\n' "${fallback}"
  fi
}

main() {
  cd "$(dirname "$0")/.."
  local target=${CARGO_TARGET_DIR:-target}

  # Rebuilt every run, never merely checked for: a ratchet measured with the previous
  # parser's probe would report the previous parser's numbers, and green.
  cargo build --quiet --locked -p sv2-syntax --example "${PROBE}" --features counters
  if [[ ${1:-} != check ]]; then
    cargo build --quiet --locked --release -p sv2-syntax --example "${PROBE}"
  fi

  local -a wasm_args=()
  if [[ ${1:-} == wasm ]]; then
    cargo build --quiet --locked --release --target "${WASI_TARGET}" -p sv2-syntax \
      --example "${PROBE}"
    local wasmtime bun
    wasmtime=$(engine wasmtime "${WASMTIME_HOME:-${HOME}/.wasmtime}/bin/wasmtime")
    bun=$(engine bun "${HOME}/.bun/bin/bun")
    wasm_args=(--wasm-probe "${target}/${WASI_TARGET}/release/examples/${PROBE}.wasm")
    if [[ -n ${wasmtime} ]]; then
      wasm_args+=(--wasmtime "${wasmtime}")
    fi
    if [[ -n ${bun} ]]; then
      wasm_args+=(--bun "${bun}")
    fi
  fi

  exec "${PY}" "${PERF}" \
    --counters-probe "${target}/debug/examples/${PROBE}" \
    --timing-probe "${target}/release/examples/${PROBE}" \
    "${wasm_args[@]}" \
    "$@"
}

main "$@"
