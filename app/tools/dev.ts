// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The development server: `Bun.serve` with `index.html` as a route.
 *
 * The other thing Tauri needs from the frontend — the URL named by `devUrl`
 * (STD-004-TS §3.4). `tools/build.ts` is the static folder for release.
 *
 * BINDS 127.0.0.1 ONLY, on a fixed port (§3.4, rule 3). It serves a webview that
 * has IPC access to the file system, and nothing off this machine has a reason
 * to reach it. The port is fixed rather than chosen so that `devUrl` in
 * `tauri.conf.json` can name it; a port that moved would make that file wrong
 * every other run.
 *
 * Hot reload and React Fast Refresh are the server's, not the application's:
 * `import.meta.hot` does not appear under `src/` (§3.4, rule 4). If Fast Refresh
 * loses component state or recreates the CodeMirror `EditorView` on an unrelated
 * edit, that is VF-1 in §3.5 and the condition under which Vite comes back.
 */

import index from "../src/index.html";

/** The port `devUrl` in `tauri.conf.json` names. Fixed on purpose; see above. */
export const DEV_PORT = 1420;

const server = Bun.serve({
  port: DEV_PORT,
  hostname: "127.0.0.1",
  routes: {
    "/*": index,
  },
  development: {
    hmr: true,
    // Echo the webview's console to this terminal. In a Tauri window the
    // devtools are a keystroke away, but the server's terminal is where a
    // failure during startup is actually seen.
    console: true,
  },
});

console.log(`sv2-studio dev server: ${server.url}`);
