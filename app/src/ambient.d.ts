// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * What the bundler may import besides TypeScript.
 *
 * The webview `tsconfig.json` loads NO types package (`"types": []`, STD-004-TS
 * §13.3), so nothing declares these for us and this file is the whole statement
 * of them (§3.4, rule 6). That is deliberate: without it, a Bun API used in
 * application code would type-check because Bun's types had been loaded for an
 * unrelated reason, and §2 rule 5 is that the webview is not Bun.
 *
 * Kept minimal on purpose. An asset kind that nothing imports does not belong
 * here, because a declaration is a promise that the bundler handles it.
 */

/** A stylesheet imported for its side effect. `Bun.build` inlines it. */
declare module "*.css" {
  const href: string;
  export default href;
}

/** A CSS module, imported for its generated class names. */
declare module "*.module.css" {
  const classes: Readonly<Record<string, string>>;
  export default classes;
}

/**
 * A WebAssembly file imported as an asset.
 *
 * `wasm/loader.ts` imports it with `with { type: "file" }` so the bundler copies
 * it into the build and returns its URL, which is then passed to the
 * wasm-bindgen initializer as `module_or_path` (§3.4, rule 5). The generated
 * glue code's own `new URL(..., import.meta.url)` lookup is NOT relied on: it is
 * the step most likely to differ between bundlers, and VF-2 in §3.5 is the
 * condition that would send this back to Vite.
 */
declare module "*.wasm" {
  const url: string;
  export default url;
}
