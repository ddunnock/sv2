// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The application shell: panels, tabs, and the island boundaries.
 *
 * A STUB. The shell's job is to place the two islands the editor and the
 * diagram live in and to own anything that crosses between them (STD-004-TS
 * §2.1). Neither island exists yet, so this renders what is true today and says
 * what it is waiting for.
 *
 * ADR-0001 is why the layout will be the shape it is: text is authoritative and
 * the diagram is a projection of it, so the editor is a primary surface and the
 * diagram is a view beside it, never the other way round.
 *
 * What is real here is the theme. The root element carries `t-light` or
 * `t-dark`, which is the mechanism the whole token system in `index.css`
 * depends on, and the toggle exists so that mechanism is verified by eye in a
 * rendered window rather than assumed. §3.5 VF-3 makes a `bun-plugin-tailwind`
 * failure a reason to bring Vite back, so it has to be something someone looked
 * at.
 */

import { useState } from "react";

/** Which palette the shell renders in. A union, not a boolean (§4.5). */
type Theme = "light" | "dark";

/** The token groups, so the probe shows every one rather than a plausible few. */
const SWATCHES: readonly (readonly [string, string])[] = [
  ["bg", "bg-bg"],
  ["panel", "bg-panel"],
  ["panel-2", "bg-panel-2"],
  ["hover", "bg-hover"],
  ["highlight", "bg-highlight"],
  ["accent", "bg-accent"],
  ["accent-bg", "bg-accent-bg"],
  ["line", "bg-line"],
  ["edge", "bg-edge"],
  ["ok", "bg-ok"],
  ["warn", "bg-warn"],
  ["err", "bg-err"],
];

/** The five syntax colours, which the editor and the sidebar both read. */
const SYNTAX: readonly (readonly [string, string])[] = [
  ["keyword", "text-syntax-keyword"],
  ["type", "text-syntax-type"],
  ["string", "text-syntax-string"],
  ["comment", "text-syntax-comment"],
  ["number", "text-syntax-number"],
];

/** The layers that do not exist yet, and what each is waiting on. */
const MISSING: readonly (readonly [string, string])[] = [
  ["contract/", "the Zod half of the IPC contract (§4.2)"],
  ["model/", "the render shapes projected from it"],
  ["ipc/", "the Tauri boundary; the only caller of invoke"],
  ["wasm/", "the sv2-wasm loader and flat node-buffer adapter (ADR-0013)"],
  ["editor/", "the CodeMirror host. Owns the text (§8.4)"],
  ["diagram/", "the SVG island, projected from the resolved model"],
];

/** The root component. Takes no props: the composition root supplies services. */
export function Shell(): React.JSX.Element {
  const [theme, setTheme] = useState<Theme>("light");

  return (
    <div className={`min-h-screen bg-bg text-fg ${theme === "dark" ? "t-dark" : "t-light"}`}>
      <main className="mx-auto max-w-3xl p-8">
        <header className="mb-6 flex items-center justify-between gap-4">
          <h1 className="text-name font-semibold">sv2 Studio</h1>
          <button
            type="button"
            onClick={() => {
              setTheme(theme === "dark" ? "light" : "dark");
            }}
            className="h-control rounded-wb border border-line bg-panel px-3 text-code text-fg hover:bg-hover"
          >
            {theme === "dark" ? "Light theme" : "Dark theme"}
          </button>
        </header>

        <p className="text-muted">
          A SysML v2 and KerML editor. Text is authoritative; diagrams are projections of it
          (ADR-0001).
        </p>

        <h2 className="mt-8 mb-2 text-meta font-semibold tracking-wide text-muted uppercase">
          Design tokens
        </h2>
        <div className="flex flex-wrap gap-2">
          {SWATCHES.map(([name, className]) => (
            <div key={name} className="w-28 rounded-wb border border-line bg-panel p-2">
              <div className={`mb-1 h-6 rounded-wb border border-line ${className}`} />
              <span className="font-mono text-nano text-faint">{name}</span>
            </div>
          ))}
        </div>

        <h2 className="mt-8 mb-2 text-meta font-semibold tracking-wide text-muted uppercase">
          Syntax
        </h2>
        <div className="rounded-wb border border-line bg-panel p-3 font-mono text-code">
          {SYNTAX.map(([name, className]) => (
            <div key={name} className={className}>
              {name}
            </div>
          ))}
        </div>

        <h2 className="mt-8 mb-2 text-meta font-semibold tracking-wide text-muted uppercase">
          Not built yet
        </h2>
        <ul className="space-y-1">
          {MISSING.map(([layer, why]) => (
            <li key={layer} className="text-code">
              <code className="rounded-wb bg-panel-2 px-1.5 py-0.5 font-mono text-fg">{layer}</code>{" "}
              <span className="text-muted">{why}</span>
            </li>
          ))}
        </ul>
      </main>
    </div>
  );
}
