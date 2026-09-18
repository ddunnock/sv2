# Building the sv2-studio UI shell

Working plan and handoff for the front-end workstream. Read this first when picking
the UI work back up; it is written to be enough on its own.

**Why this file is here.** `.claude/state/` is machine-readable JSON by its own
README, and `state.json`'s `authored` block is a closed schema describing the grammar
workstream — neither is the right home for a second workstream's prose. `.claude/` is
committed on purpose, so this travels with the repository.

**It is not auto-loaded.** `.claude/scripts/hook_session_start.py` prints
`state.json` and `grammar-diff.json` and nothing else, so a new session will not see
this unless it is opened. Say "read `.claude/plans/ui-shell.md`" and it is all here.

---

## Where the work is

| | |
|---|---|
| Branch | `ui/shell` |
| Worktree | `../sysmlv2-editor-ui`, created with `git worktree add` |
| Based on | `main` at `081d85a` |
| Commits | 9, all green |

The worktree exists so the UI work does not collide with grammar work in the main
checkout. To recreate it elsewhere:

```bash
git worktree add ../sysmlv2-editor-ui ui/shell
cd ../sysmlv2-editor-ui/app && bun install --frozen-lockfile
```

**One hazard.** Both checkouts share `CARGO_TARGET_DIR=~/.cargo-target`. A `cargo
test` in one while the other builds produced a transient failure that vanished on
re-run. If a red Rust gate cannot be reproduced, suspect this before believing it.
Fix, if it becomes annoying: give the worktree its own target dir, at the cost of one
cold rebuild.

---

## Context

Three facts set the shape of this work.

1. **The mockup is unusually well specified.** `docs/design/sysml-workbench-mockup/`.
   Its README names regions `UI-01..UI-24`, screens `SCR-01..05`, interactions
   `IX-01..IX-10`, open decisions `OD-01..04`. Those IDs are the acceptance
   vocabulary. `assets/workbench.css` is 92 lines holding the whole visual system.

2. **The backend cannot feed this UI.** `sv2-ast` is 8 lines, `sv2-hir` 9,
   `sv2-resolve` 10 — empty stubs with empty `[dependencies]`. `sv2-syntax` is real
   (lossless CST, typed `Diagnostic`, ~158/558 productions) but everything the
   mockup's sidebar, trees and diagrams show comes from layers that do not exist.
   `state.json`'s roadmap is entirely grammar coverage; no UI work is on it.

3. **`app/` was entirely unenforced.** No gate step, no header check, no dependency
   check. Closed in Phase 1.

## Decisions taken

- **Fidelity:** faithful to the mockup's named regions and interactions, free on
  pixels. Responsive rather than fixed at 1440×900.
- **Data:** contract-first. Zod schemas now, shell driven by fixtures parsed through
  those same schemas, real IPC swapped in later without touching a component.
- **First view:** honest placeholder only. A view-kind dispatcher renders an explicit
  "not implemented" panel per kind. No simulated diagrams.
- **Editor:** built early, plain-text. Must work split-screen, in the right sidebar,
  and undocked into its own window. Full LSP waits on the grammar.
- **ADR-0020 (accepted):** identity is allocated eagerly when a workspace opens. This
  makes `unidentified` a *transient* arm of `ElementHandle`, not a permanent one.
- **STD-004-TS §4.3 rule 1 was rewritten.** See "The one standards change" below.

## Two things everything rests on

### `ElementId` is a brand; `ElementHandle` is a union

ADR-0016's identity-scope table has four categories and only one is a petname.
Memberships are derived `<owned-ID>/m`, implied relationships are hashes in a separate
namespace, standard library elements carry normative KerML UUIDs and never get a
petname. A diagram edge needs a handle for a derived relationship; an inherited
feature needs one for a library element. `ElementId` stays the brand §4.4 names;
`ElementHandle` is what a component receives, with a fifth transient `unidentified`
arm per ADR-0020.

### One document, many editor views

The mockup's Element Source tab (IX-06) stages edits behind Apply/Revert — a second
buffer, a second undo scope, and a window where the sidebar and the file disagree.
That contradicts §8.4 rule 1 and ADR-0001. Because the editor is in scope early, the
answer is one CodeMirror `EditorState` with several `EditorView`s over it:

```ts
type EditorPlacement =
  | { kind: "split"; orientation: "horizontal" | "vertical" }
  | { kind: "sidebar" }                          // the Element Source tab
  | { kind: "window"; windowId: WindowId };
```

Apply and Revert disappear — there is nothing to apply. **OD-01 and OD-02 are both
answered by this.** The panel lives in `src/editor/`, not `src/shell/`: only
`editor/` may touch CodeMirror and `biome.json` enforces it.

## The one standards change

`STD-004-TS §4.3` rule 1 said the type is inferred from the schema. Under §13.3's
`isolatedDeclarations: true` that is impossible — `export const S =
z.discriminatedUnion(...)` is TS9010/TS9013. Measured, not guessed:

- Zod's own types are no escape: `ZodObject<{…}, $strict>` and `$ZodBranded<ZodString,
  "Name", "out">` are unwritable by hand and leak internals into every signature.
- Hiding the schema does not help: an exported `z.infer<typeof S>` drags the
  requirement back onto a module-local const.

§1 says that where prose and §13 disagree, §13 wins and the prose is the defect. So
rule 1 now reads: **declare the type, annotate the schema `z.ZodType<Name, Wire>`**,
and the compiler rejects a schema that does not produce the declared type. Two
consequences are now in the rule:

- Union member schemas stay module-local — that is what keeps `z.discriminatedUnion`
  usable.
- Brands mint with `.transform()`, not `.brand()`, because `$ZodBranded` is not
  assignable to `z.ZodType` under `exactOptionalPropertyTypes`.

`contract/offset.ts` is the reference example of the shape.

---

## Phase status

### Phase 1 — Enforcement floor and design tokens · **DONE**

- `scripts/check_headers.py` extended to `app/**/*.{ts,tsx}` — 126 files now checked.
  The `/** */` form §2.3 rule 1 forbids is proven to fail even with a correct licence
  inside it.
- New `scripts/check_web_dependencies.py` — closed allowlist enforced, 27 tests. It
  found and fixed a real defect: `typescript` was `"peerDependencies": {"^7"}`, a
  range §3.1 rule 2 bans, and Bun *resolves and installs* peer deps so it was live.
- `scripts/gate.sh` runs six front-end checks. `env -C app`, not `cd` (run_check
  captures a subprocess and a `cd` would leak). One `command -v bun` guard, matching
  the cargo block — `run_optional` is wrong here because it tests `$2`, which is `env`.
- IBM Plex vendored: 108 KB latin subset, `app/src/assets/fonts/`, provenance and
  sha256 in its README, OFL notice in `NOTICE.md`.
- `src/index.css` is the mockup's palette as Tailwind v4 tokens: 25 colours × 2
  themes, six named type sizes, two radii. **Zero arbitrary values** — every mockup
  dimension lands on the 4px scale except view-tabs (34px) and controls (26px).

**Two findings recorded in the code so they are not rediscovered:**

- Plain `@theme`, never `@theme inline` — inline substitutes literals and leaves
  `.t-dark` nothing to override.
- Bun **inlines** the fonts as base64: the stylesheet is 145 KB and no `.woff2`
  reaches `dist/`. `loader: {".woff2": "file"}` does not change it (that map governs
  JS imports, not CSS `url()`) and was removed rather than left looking effective.

The VF-3 probe was done by eye and caught a real bug: `body` carried `bg-bg` while the
theme class sits one element inside it, so dark mode left the page margins light.

### Phase 2 — Contract, projection, fixtures · **IN PROGRESS**

Done:

- `contract/offset.ts` — `Utf16Offset` brand, `TextSpan`. One unit, UTF-16, converted
  Rust-side (ADR-0013 RISK-013-2). Type tests + 17 value tests.
- `contract/element-id.ts` — `ElementId`, `ViewId`, `ElementHandle`. Type tests prove
  both substitutions §4.4 names.
- `contract/diagnostic.ts` — mirrors `sv2_syntax::Diagnostic`. **Code open, severity
  closed**: `DiagnosticCode` is `#[non_exhaustive]`, so a closed enum would drop a
  diagnostic from a newer core — filtering on diagnostic state, which ADR-0002
  forbids. Severity is *carried, not re-derived*: re-applying Rust's map in TypeScript
  is a second opinion in a second language (ADR-0013 DD-1).

Remaining, in order:

1. `contract/element.ts` — the big one. `ElementSummary`, `OwnedFeature`,
   `Relationship`, `ElementDetail`, and `TypeRef` as ADR-0002's resolved/unresolved
   union. **Add `origin: {kind:"owned"} | {kind:"inherited"; from: ElementHandle}` to
   every feature row even though the UI ships owned-only** — OD-04 is a resolver
   question with a wire consequence, and one field now beats a wire change and a full
   fixture regeneration later.
2. `contract/file.ts` — the Files tree.
3. `contract/view.ts` — `ViewKind` as the **complete** union including
   `state-transition` even though OD-03 leaves it undesigned. A deliberately
   incomplete union gets widened under pressure.
4. `contract/layout.ts` — ADR-0017, `GridUnit` brand, `z.looseObject` per R-6.
5. `contract/availability.ts` — see below. High value.
6. `contract/preferences.ts`, `contract/registry.ts`.
7. `model/` — `assert-never.ts`, `Query<T>`, `element-handle.ts` (`isLogSafe`),
   `tree.ts`, `grid.ts`. Pure, no DOM.
8. `ipc/model-queries.ts` + `ipc/fixture-client.ts`, `app/test-data/`,
   `tools/embed-fixtures.ts`, `tools/emit-contract.ts`.

**Make "unavailable" a contract citizen.** The highest-value remaining move:

```ts
UnavailableReason = "not-implemented" | "parse-failed" | "unsupported-language" | "no-model-loaded"
answered(T) = { status: "ready"; data: T } | { status: "unavailable"; reason; subject }
```

Not because it unblocks the UI, but because **`unavailable` never goes away**. When
`sv2-resolve` lands, `"not-implemented"` retires and `"parse-failed"` becomes the
permanent, correct answer for a file ADR-0002 admits partially. The placeholder path
and the production path become one code path.

**Standing rule: a fixture field with no named Rust owner is a defect.** Every
`contract/` module's TSDoc names the Rust type it mirrors, even before that type
exists. Without this the shell can be built to fixtures for months and be wrong.

### Phase 3 — Primitives, accessibility, layout skeleton

Installs `@happy-dom/global-registrator`, `@testing-library/react`,
`@testing-library/user-event`; wires `test-setup.ts` to §13.5's three duties.

**Build five primitives before any panel:** `Tabs`, `Tree`, `Toolbar`, `IconButton`,
`Splitter`. Every one of the ten UI regions is an instance of one of these. The mockup
has roles half-applied, which is worse than none — `role="tab"` with no `tabpanel` or
roving `tabindex`, a tree that is a flat list of buttons with padding indents, 128
buttons with no `type`, ~25 bare `<svg>` that each trip `a11y/noSvgWithoutTitle`. The
whole a11y recommended group is blocking via `preset: "recommended"` plus
`--error-on-warnings`.

**One window, not five screens.** `workbench.js` proves SCR-01..04 are one window with
six flags varied. Its state unions, per §4.5:

```ts
type NavigatorState = { kind: "hidden" } | { kind: "open"; mode: "files" | "elements" };
type SidebarState   = { kind: "collapsed" | "open"; tab: "specification" | "source" };
type LayoutMode     = { kind: "normal" }
                    | { kind: "focus"; restore: { navigator: NavigatorState; sidebar: SidebarState } };
```

`collapsed` still carries `tab` because IX-03 expands *to that tab*. `focus` carries
its restore payload because F11 must be exactly reversible (IX-09).

Also: `shell/IslandBoundary.tsx` (§7.4), `shell/Unavailable.tsx`, `useSelection()` —
the seam that stops the second window moving selection later.

### Phase 4 — Wire the shell

Navigator, spec sidebar, status bar with a **persistent "fixture data" indicator**
driven by `ModelQueries.provenance`. `ViewArea` dispatches on `ViewKind` with an
exhaustive switch ending in `assertNever`, **before any view renders** — ~40 lines
that make ADR-0008's "interconnection second" a one-file change.

### Phase 5 — The editor

CodeMirror, plain text, no WASM. Not scaffolding: ADR-0013 requires the editor to
accept input unhighlighted before the parser is available, so this is production
behaviour built first. Split and sidebar placements work in-window.

### Phase 6 — Tauri, real IPC, the second window

`@tauri-apps/api` arrives with `ipc/tauri-client.ts`. The pop-out window is last: it
is the only item touching the composition root. Note `src/shell/**` is banned from
importing `@tauri-apps/**`, so "open the editor window" is an `ipc/` function the
shell calls.

---

## Deferred, and what shows instead

`shell/Unavailable.tsx` takes `what`, `because`, `blockedBy`. It never renders sample
data, never a spinner, never a greyed-out mock. A spinner is a lie about a thing that
is not loading.

| Deferred | Shows instead |
|---|---|
| `diagram/` | `<Unavailable>`. **Not even an empty dotted canvas** — that reads as "the diagram works and this model is empty", which is false. |
| WASM highlighting | The plain-text editor, which ADR-0013 requires anyway |
| Grid View | `<Unavailable>` until its `model/` functions exist. Last view built. |
| Authoring, search, validation, VCS, settings | Disabled controls with a `title` naming the reason |

## Verification

Per commit, from the repo root:

```bash
python3.11 scripts/check_headers.py
python3.11 scripts/check_web_dependencies.py
env -C app bun run typecheck && env -C app bun run lint && env -C app bun test
./scripts/gate.sh            # the only definition of done
```

## Open questions

- **How the user is told** that opening a workspace writes IDs (ADR-0020 DD-4 requires
  it be visible). A UI decision, belongs with the shell.
- **What read-only mode can do** when a workspace cannot be written (ADR-0020 FIT-5
  fixes only that it writes nothing).
- **`crates/sv2-app` vs `crates/sv2-studio`.** STD-004-TS §2 says `sv2-app` in seven
  places; the repo, `rust_binaries.toml` and ADR-0018 all say `sv2-studio`. Blocks
  `scripts/check_ipc_contract.py`, which cannot know which crate emits
  `rust.schema.json`. Recommend fixing the standard.
- **Grid virtualization.** Nothing on the allowlist; adding one needs a §3.1 rule 4
  record. Worth recording a §3.5-style trigger so it is a condition met, not a
  preference.

## State of `main` when this was written

The gate was **red on main**, for grammar reasons unrelated to this branch:

```
newly accepted, not in the ledger: MassRollup_1.kerml, MassRollup_2.kerml, MassRollup1.sysml
the parser improved; record it with `scripts/corpus_sweep.py --record`
should have been rejected but parsed: tests/rejection/invocation-expression-is-not-implemented.sysml
```

That is InvocationExpression landing, and the second line is exactly the stale-rejection
hazard the `rejection-provenance` pending decision predicts: the file is still
rejected, but no longer for the reason its own comment gives.
