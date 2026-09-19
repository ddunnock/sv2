# sv2-studio front end — working agreement

The Tauri v2 webview: a React shell, a CodeMirror editor, and an SVG diagram
island. `crates/sv2-studio` is the Rust host; this package is what it displays.

**`docs/standards/STD-004-TS-typescript-react-standards.md` governs every file
here.** This page is orientation, not a second copy of it. Where the two
disagree, the standard is right — and where the standard's prose and its §13
configuration disagree, §13 is right and the prose is a defect.

## Bun is the whole toolchain

Package manager, script runner, bundler, dev server, test runner. **There is no
Node and no Vite** (assumption A-003).

```bash
bun install            # never npm/yarn/pnpm
bun run typecheck      # tsc --noEmit over both tsconfigs; the type authority
bun run lint           # biome ci --error-on-warnings
bun test               # bun:test, preloaded by src/test-setup.ts
bun run dev            # tools/dev.ts  — 127.0.0.1:1420, the devUrl Tauri names
bun run build          # tools/build.ts — dist/, the frontendDist Tauri names
```

Vite is not banned on taste; it is not needed yet. **§3.5 lists the six
conditions (VF-1 to VF-6) that bring it back**, each with the reproduction that
would establish it. If you hit one, that is a decision record citing the
condition — not a dependency bump. Build speed is not a trigger in either
direction.

## The layout is the architecture

Every module belongs to exactly one layer and its directory is its layer. A file
directly under `src/` other than `main.tsx` and `test-setup.ts` is a defect
(§2, rule 1).

```
src/main.tsx       the composition root; the ONLY module that runs at load time
src/contract/      Zod schemas          — imports nothing but zod
src/model/         pure TypeScript      — imports contract
src/ipc/           the ONLY caller of Tauri's invoke
src/wasm/          the sv2-wasm loader and flat-buffer adapter
src/diagnostics/   the ONLY module that reports to a log channel
src/editor/        the ONLY module that touches CodeMirror
src/diagram/       the SVG island
src/shell/         panels, tabs, island boundaries
tools/             build-time scripts; nothing in src/ imports from here
```

`editor/` and `diagram/` do not import each other. Anything they share goes
through `model` or the shell. The import matrix in §2.1 is the rule.

Tests sit beside what they test: `result.ts` and `result.test.ts` in one
directory.

## Four things that catch people first

- **The webview is not Bun.** `tsconfig.json` loads `"types": []`, so a Bun API
  in application code is a type error. Bun's APIs belong in tests,
  `test-setup.ts`, and `tools/` — which `tsconfig.test.json` covers (§2, rule 5).
- **Asset imports are declared in `src/ambient.d.ts`,** because no types package
  declares them for us. That file is the whole statement of what the bundler may
  import besides TypeScript.
- **Every dependency is a recorded decision.** The §3.1 allowlist is closed;
  nothing arrives transitively as a direct import.
- **Two `//` lines of SPDX header open every file,** never a `/** */` block —
  TSDoc would read the licence as documentation (§2.3).

## The plan, and where the work is

**`.claude/plans/ui-shell.md` is the working plan and the handoff.** Phases, what
each one decided and why, what is deferred and what it shows instead, and the
open questions. It is not loaded automatically — the SessionStart hook prints
`state.json` and nothing else — so open it before starting.

The UI work happens on the `ui/shell` branch, in a worktree, so it does not
collide with grammar work in the main checkout.

## Current state

Phases 1–4 of `.claude/plans/ui-shell.md` are done; read its phase notes rather
than trusting a summary here.

- `contract/` is complete for Phase 4: offsets, identities, diagnostics, elements,
  files, views, layout, availability, preferences, and the command registry.
- `model/`: `result`, `assert-never`, `element-handle`, `query`, `tree`.
- `ipc/`: one parse path under a `Transport`; the fixture transport answers from
  the mockup's sample model. There is no Tauri client yet (Phase 6).
- `shell/`: five accessible primitives, the island boundary, `Unavailable`,
  `useSelection`, the layout reducer, and the window, wired to the queries:
  Files tree, Views list, view tabs with the `ViewKind` dispatcher, Specification.
- `wasm/`, `editor/` and `diagram/` do not exist.

**A schema is declared, not inferred.** `isolatedDeclarations` cannot state the
type of an exported Zod schema, so the type is written and the schema is
annotated `z.ZodType<Name, Wire>`. §4.3 rule 1 carries the whole argument, and
`contract/offset.ts` is the reference example. Brands mint through
`.transform()`, never `.brand()`.

`@tauri-apps/api`, `@codemirror/*` and `sv2-wasm` remain **allowed and not
installed**. Each arrives in the change that adds the first module importing it,
because a package nothing imports is load cost and supply-chain surface for
nothing.

**Test keys by code when the shortcut reads code.** `user-event` gives `{F11}` the
code `Unknown`; write `[F11]`.
