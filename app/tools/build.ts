// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The production build: `Bun.build` from `index.html` into `dist/`.
 *
 * One of the two things Tauri needs from the frontend — the static folder named
 * by `frontendDist` (STD-004-TS §3.4). `tools/dev.ts` is the other.
 *
 * This is a SCRIPT rather than a command line, and §3.4 rule 1 says why: the
 * options are a typed `Bun.build` call checked by `tsconfig.test.json`, so a
 * misspelled option is a type error. The same options written as flags in a
 * package script are a string nobody checks, and a misspelling is silently
 * ignored.
 *
 * `dist/` is ignored by Git and rebuilt from the lockfile and the source; it is
 * never edited (§3.4, rule 2).
 */

import { rm } from "node:fs/promises";
import path from "node:path";
import tailwind from "bun-plugin-tailwind";

const root = path.resolve(import.meta.dir, "..");
const outdir = path.join(root, "dist");

await rm(outdir, { recursive: true, force: true });

const result = await Bun.build({
  entrypoints: [path.join(root, "src/index.html")],
  outdir,
  plugins: [tailwind],
  target: "browser",
  minify: true,
  // Linked rather than inline: the webview loads the map only when a person
  // opens the devtools, and an inline map is carried on every load.
  sourcemap: "linked",
  define: {
    "process.env.NODE_ENV": JSON.stringify("production"),
  },
});

if (!result.success) {
  for (const log of result.logs) {
    console.error(log);
  }
  // A build that failed must not leave a zero exit behind it, or the gate reads
  // a broken bundle as a good one.
  process.exit(1);
}

for (const output of result.outputs) {
  const size = (output.size / 1024).toFixed(1);
  console.log(`  ${path.relative(root, output.path)}  ${size} KB`);
}
