// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//
// Run a WASI preview-1 module under a JavaScript engine's `node:wasi`, for timing the
// parser where the editor will run it (ADR-0024; docs/perf/roadmap.md Phase 1).
//
//   bun scripts/wasi_run.mjs <module.wasm> <dir> [args...]
//
// <dir> is preopened as the guest's working directory, and is the only directory the
// guest can see. scripts/perf.py passes a temporary copy of the files it times, never
// the repository. Under Bun this is JavaScriptCore, the engine of Tauri's webview on
// macOS and Linux.
//
// A launcher, not application code: STD-004-TS governs the UI repository's TypeScript.

import { readFile } from "node:fs/promises";
import process from "node:process";
import { WASI } from "node:wasi";

const [modulePath, dir, ...args] = process.argv.slice(2);
if (!modulePath || !dir) {
  process.stderr.write("usage: wasi_run.mjs <module.wasm> <dir> [args...]\n");
  process.exit(2);
}

const wasi = new WASI({
  version: "preview1",
  args: [modulePath, ...args],
  preopens: { ".": dir },
  returnOnExit: true,
});
const module = await WebAssembly.compile(await readFile(modulePath));
// `wasiImport` rather than `getImportObject()`, which Bun's node:wasi does not provide.
const instance = await WebAssembly.instantiate(module, {
  wasi_snapshot_preview1: wasi.wasiImport,
});
process.exitCode = wasi.start(instance);
