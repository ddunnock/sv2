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
| Worktree | `../sv2-ui`, created with `git worktree add` |
| Based on | `main` at `081d85a`, merged up to `752ff41` for the case-collision fix |
| Commits | 37, all green |

The worktree exists so the UI work does not collide with grammar work in the main
checkout. To recreate it elsewhere:

```bash
git worktree add ../sv2-ui ui/shell
cd ../sv2-ui/app && bun install --frozen-lockfile
```

That needs Bun 1.4.2, the `packageManager` pin. Bun 1.3.x cannot read `bun.lock`'s
lockfile version 2, so the install fails and every web gate goes red with it.

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
- `contract/element.ts` — `ElementRef` (the plan's `TypeRef`; §4.3 already names it
  that), `FeatureRow` with `origin`, `RelationshipRow`, `ElementSummary`,
  `ElementFacet`, `ElementDetail`. Every field is checked against the pinned
  metamodel. Three readings worth keeping: `~TempPort` is the *effective name* of a
  `ConjugatedPortDefinition`, so a reference needs no conjugation flag; multiplicity
  bounds are carried as written, not evaluated; `isReference` is left off because
  the metamodel derives it as `isComposite = false`. **`MetaclassSchema` checks shape
  only**, and closes when the Rust type exists and `check_ipc_contract.py` compares
  the two enums — a closed list typed out by hand would be unsourced (invariant 4).
- `contract/file.ts` — `Workspace`, `WorkspaceFile`, `Language`, `DiagnosticCounts`,
  and a `WorkspacePath` brand that `element.ts`'s `SourceLocation` now uses too.
  **The wire is flat; the tree is derived** — a nested shape would state each folder
  twice, as a node and as a path prefix, so `model/tree.ts` builds folders and sorts.
  Only model files cross (IX-01); sidecars do not. Paths arrive in one form,
  converted Rust-side like offsets, and the schema rejects rather than normalizes.
  `language` is carried, not derived from the extension, for the severity reason.
  No ADR yet names the Rust owner of workspace enumeration.
- `contract/view.ts` — `ViewKind` is all eight standard views of clause 9.2.20.2
  (`StandardViewDefinitions`, subclauses .1–.8, no gaps — the wiki receipts cite
  each), `state-transition` included. `ViewSummary` carries `exposes`, because the
  mockup names every view by kind and exposed element ("GV ThermalControl"), never
  by its own name. `kind: null` is a view whose definition specializes no standard
  view: legal SysML, listed, rendered unavailable. The type test proves a switch
  that forgets a kind does not compile. **Known seam:** a view is an element, but
  `ViewId` and `ElementId` are separate brands, so opening a view in the sidebar
  needs its own query rather than a cast.
- `contract/layout.ts` — ADR-0017's layout sidecar, one view at a time. **The wire is
  not the file:** camelCase per §4.2 (so the file's `engine_version` crosses as
  `engineVersion`, converted by serde), handles instead of bare strings, records
  grouped by kind. Grouping is lossless (R-2's order is a sort) and avoids a
  string-kinded catch-all arm that would defeat narrowing. R-6 is tested as
  *preservation*, not acceptance: unknown fields and unknown record kinds come back
  out of the parse intact. `schema` is `z.literal(1)` — R-7 bumps it only on a
  breaking change, and DD-4 makes a failed read safe. `GridUnit` is signed `i32`.
  Records are keyed by a new `DurableHandle` in `element-id.ts`: every handle arm
  except `unidentified`. The style sidecar is not here — ADR-0017 gives it no record
  shape.
- `contract/availability.ts` — `Answer<T>` and `UnavailableReason`, below. Differs
  from the sketch that follows in four sourced ways: tagged `kind` not `status`
  (§4.2); no `parse-failed` (invariant 3 and ADR-0002: a broken file is answered
  `ready`, decorated, from its last good subtree); no `unsupported-language`
  (ADR-0014, and `WorkspaceFile.language` cannot name a third); and `opening`,
  `read-only` (both ADR-0020, quoted) and `not-found` (ADR-0002's rename driver)
  added. The subject is not carried — the caller knows what it asked, and matching
  a superseded reply is `ipc/`'s job for `ready` answers too. `Answer` is not
  `Result`: a `Result` error is a defect and is reported; `unavailable` is a
  correct answer and is rendered.
- `contract/preferences.ts` — theme, navigator, sidebar, two panel widths. **One
  storage key per preference**, each versioned (`sv2.theme.v1`) and parsed alone: §4.3
  rule 3 forbids a `.catch()` default, so one record would lose everything to one bad
  field. A mapped type ties the table to `Preferences`. Defaults live in the shell;
  per-workspace state (open tabs) waits for a workspace key. The Phase 3
  `NavigatorState` and `SidebarState` unions are defined here, since they persist.
- `contract/registry.ts` — `COMMANDS`: `workspace`, `views`, `element_detail`,
  `view_layout`, each with its Rust command name, args, `Answer`-wrapped answer and
  owner. No Rust command exists yet, so this *states* the vocabulary Rust must
  implement. `workspace` and `view_layout` are owned by `unassigned`, pinned by a test
  so assigning one is a visible change. The Elements tree, file text and Problems
  panel are not in it yet.

**The contract layer is complete for Phase 4's needs.**

- `model/` — pure, no DOM:
  - `assert-never.ts`. **Its message carries only the discriminant** (`kind=…` or
    `status=…`), not the value: §4.5's own example stringifies the value, which §9.3
    forbids, since an unhandled `ElementRef` can carry what the author wrote.
  - `element-handle.ts`. `logLabel` in place of the planned `isLogSafe` predicate:
    no arm carries model meaning, so every handle has a safe label and the function
    is total (membership is ADR-0016's `<owned-ID>/m`). `sameHandle` for equality.
  - `query.ts`. `Query<T, E>` = loading | failed | answered. **Loading holds the
    previous answer**, so a reload after an edit never blanks a panel (ADR-0002, no
    flicker); only a first load has nothing to show. `E` is a parameter because
    `IpcError` lives in `ipc/`.
  - `tree.ts`. Builds the Files tree from the flat list: folders first, natural
    case-blind order with a code-point tiebreak (locale-independent), folder `key` =
    its path, diagnostic counts summed upward (IX-10).
  - `grid.ts` **deferred** with the Grid View, which is built last.

- `ipc/`:
  - `ipc-error.ts` — `IpcError` = unreachable | rejected | contract. A contract error
    keeps each issue's path and code only; a test proves the rejected value never
    appears in it. Zod's error is taken structurally, because §2.1 lets `ipc/` import
    only `@tauri-apps/api` from outside.
  - `model-queries.ts` — a `Transport` (command, args → raw reply, never throws) under
    **one parse path** shared by the fixture and, later, the Tauri client. Every reply
    is parsed with its registry schema; `provenance` drives the "fixture data" badge.
  - `fixture-client.ts` + the sample data (now `test-data/thermal-control/`) — the mockup's sample model as
    raw replies, parsed like any backend reply. Layout answers `not-implemented` for
    every view (no simulated diagrams); an undescribed element is `not-found`.
    *(Superseded: the fixture is now JSON in `app/test-data/`, embedded by
    `tools/embed-fixtures.ts` — see "Resolved, from §11" under Phase 3.)*
  - `main.tsx` now runs `z.config({ jitless: true })` (§4.3 rule 6).
- **Deferred:** `tools/emit-contract.ts`, until `rust.schema.json` exists for
  `check_ipc_contract.py` to compare it with.

Remaining, in order: Phase 3.

**Make "unavailable" a contract citizen.** Done as `contract/availability.ts`; the
sketch below is the original, and the entry above says where the module differs.

```ts
UnavailableReason = "not-implemented" | "parse-failed" | "unsupported-language" | "no-model-loaded"
answered(T) = { status: "ready"; data: T } | { status: "unavailable"; reason; subject }
```

Not because it unblocks the UI, but because **`unavailable` never goes away**. When
`sv2-resolve` lands, `"not-implemented"` retires query by query, and the other reasons
stay the permanent, correct answer for their cases. The placeholder path and the
production path become one code path. (This paragraph first named `"parse-failed"` as
a permanent reason; ADR-0002 admits a partially parsed file as `ready`, so it is not.)

**Standing rule: a fixture field with no named Rust owner is a defect.** Every
`contract/` module's TSDoc names the Rust type it mirrors, even before that type
exists. Without this the shell can be built to fixtures for months and be wrong.

### Phase 3 — Primitives, accessibility, layout skeleton · **DONE**

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
type NavigatorState = { kind: "hidden" | "open"; mode: "files" | "elements" };
type SidebarState   = { kind: "collapsed" | "open"; tab: "specification" | "source" };
type LayoutMode     = { kind: "normal" }
                    | { kind: "focus"; restore: { navigator: NavigatorState; sidebar: SidebarState } };
```

`collapsed` still carries `tab` because IX-03 expands *to that tab*, and `hidden`
carries `mode` for the same reason — the first sketch dropped it, so Ctrl+B twice lost
the mode. `focus` carries its restore payload because F11 must be exactly reversible
(IX-09).

Also: `shell/IslandBoundary.tsx` (§7.4), `shell/Unavailable.tsx`, `useSelection()` —
the seam that stops the second window moving selection later.

**What was built:**

- Test deps installed (already allowlisted); `test-setup.ts` registers happy-dom, makes
  `fetch` throw, and restores mocks and the clock after each test.
- `shell/primitives/`: `IconButton` (a disabled button *must* carry its reason, and
  stays focusable via `aria-disabled`), `Icon` (decorative, `aria-hidden`), `Toolbar`
  and `Tabs` (one tab stop, shared `roving.ts` keyboard; `Tabs` links tab↔panel both
  ways, automatic or manual activation, no panel for the collapsed strip, vertical
  text when vertical), `Tree` (APG tree view, flat rows with level/setsize/posinset;
  pure keyboard in `tree-rows.ts`; focus ≠ selection), `Splitter` (APG window
  splitter, keyboard and pointer, clamped).
- `IslandBoundary`, `Unavailable` (plain statement from the reason; `opening` alone is
  a live status), `selection.tsx` (`useSelection`, throws outside its provider).
- `layout-state.ts`: one reducer; an explicit toggle while focused leaves focus mode
  and discards the record, so F11 never undoes a choice just made. Shortcuts match on
  `code` (Option+B types "∫"); Cmd counts as Ctrl.
- `Shell.tsx`: the regions as landmarks. Unwired panels say what sv2 cannot do yet;
  deferred controls are disabled with reasons. `main.tsx` passes the reporter in.
- Checked by eye in Chrome, light and dark; that caught the collapsed strip rendering
  as a wide column instead of UI-09's narrow vertical one.

**Known:** in a plain browser F11 belongs to the browser's fullscreen and cannot be
taken; in the Tauri window it is ours. Test with `[F11]`, not `{F11}` — user-event
gives the latter the code `Unknown`.

**Resolved, from §11:**

- Rule 7 — **the code now follows it.** The replies are JSON in
  `app/test-data/thermal-control/` (one file per reply, so the Rust side can load
  them); `tools/embed-fixtures.ts` (`bun run fixtures`) generates
  `src/ipc/generated/thermal-control.ts` for the bundle, with no tsconfig change; and
  `ipc/fixture-data.test.ts` fails on a stale copy (mutation-checked). The rule gained
  one sentence covering a fixture the app also ships.
- Rule 6 — **the standard was amended** (0.6.0). Doubles are `Transport`s in `ipc/`;
  `mockIPC` is confined to the Tauri transport's own tests. Wrapping `mockIPC` for every
  island would mock Tauri underneath the parse path and couple every test to it.

### Phase 4 — Wire the shell · **DONE**

Navigator, spec sidebar, status bar with a **persistent "fixture data" indicator**
driven by `ModelQueries.provenance`. `ViewArea` dispatches on `ViewKind` with an
exhaustive switch ending in `assertNever`, **before any view renders** — ~40 lines
that make ADR-0008's "interconnection second" a one-file change.

**What was built:**

- `main.tsx` builds the services (fixture queries + reporter) and passes them in;
  `shell/services.tsx` supplies them by context. `useAnswer` is the one way a panel
  asks: keeps the previous answer while reloading, drops superseded replies (its test
  was mutation-checked — the first version passed with the guard removed), reports
  failed reads. `AnswerView` renders every query state.
- Files tree from `workspace`, all folders open by default (closed set is the state),
  error counts in accessible names. The workspace is asked once; the status bar reuses
  it for problem counts and shows a persistent **Fixture data** badge from `provenance`.
- Views list from `views`; views open in manual-activation tabs. `ViewArea.renderView`
  is the exhaustive dispatcher; every kind is `<Unavailable>`, and a test asserts no
  view panel contains an `svg` or `canvas`.
- `Specification` from `element_detail`: header, problems banner, General, Owned
  features (owned rows only, OD-04), Relationships, Documentation. Unresolved types are
  marked, not hidden. Labels in `model/element-label.ts` fall back to the metaclass.
- Zero arbitrary Tailwind values, still.

**Known gap:** nothing in the UI selects an *element* yet — the Elements tree needs a
query that does not exist, and diagrams are unavailable. The Specification is tested
directly with a handle; it becomes reachable when an element-selecting surface lands.

### Phase 5 — The editor · **DONE (split placement)**

CodeMirror, plain text, no WASM. Not scaffolding: ADR-0013 requires the editor to
accept input unhighlighted before the parser is available, so this is production
behaviour built first. Split and sidebar placements work in-window.

**What was built:**

- `@codemirror/state`, `view`, `commands` installed (allowlisted). No language package:
  plain text until the WASM parser (ADR-0013).
- `editor/shared-document.ts`: **one document, any number of views**. A headless
  `EditorState` is the authority and holds the only history; views relay changes
  through it; undo/redo in any view run against it. Tested with two real views: undo
  in one undoes an edit made in the other. The text is never React state (§8.4).
- `editor/FileEditor.tsx`: the document made once per file (keyed by path), and
  `EditorPane` creating/destroying its view in an effect.
- `file_text` command (owner `unassigned`) and `FileText` contract. The sample text is
  SCR-05's lines 1–36 exactly; element spans in the fixture are now real offsets, and a
  test checks Heater's diagnostic span covers `HeaterState`.
- `shell/EditorSplit.tsx`: choosing a file in the Files tree opens it beside the views,
  with **"Edits are not saved: sv2 does not yet support writing files."** in the header.
- Checked in Chrome: typing, and Cmd+Z undoing it through the shared history.

**Deferred: the sidebar placement (Element Source).** It is a view over an element's
span, and nothing selects an element yet, so it could not be exercised. It is one more
`EditorPane` over the same document, restricted to the span, when selection exists.
The §8.4 rule 4 update listener (derived facts such as the element under the cursor)
arrives with its first consumer.

### Phase 6 — Tauri, real IPC, the second window · **IN PROGRESS (only the pop-out window left)**

`@tauri-apps/api` arrives with `ipc/tauri-client.ts`. The pop-out window is last: it
is the only item touching the composition root. Note `src/shell/**` is banned from
importing `@tauri-apps/**`, so "open the editor window" is an `ipc/` function the
shell calls.

**Scope and approvals (from the user, 2026-09-19).**

- Rust edits allowed in `crates/sv2-studio`, `crates/sv2-wasm`, and — for dependency
  entries only — root `Cargo.toml`, `Cargo.lock`, and `deny.toml`.
- **Approved:** Tauri 2, `serde` (derive), `serde_json`; the Zlib, Apache-2.0 WITH
  LLVM-exception and MPL-2.0 licences; `anyhow` wrappers for Tauri's crates; editing
  STD-002-RS §13.5 to match.
- **Approved: advisories ignored by ID, dated.** All seven (RUSTSEC-2024-0429 unsound
  `glib`, six unmaintained) are in `deny.toml` and §13.5 with reasons and a
  2027-03-19 review. `cargo deny check --disable-fetch advisories` passes.

**What was built (steps 1–3).**

- The WIP archive restored and deleted. `wire.rs`, `workspace.rs`, `commands.rs` as
  described in ADR-0021; `element_detail` and `view_layout` take `IgnoredAny`
  (`InvokeBody` is not a command argument). The seven workspace tests pass.
- `shell.rs`: `sv2-studio [WORKSPACE]`, default the current directory, canonicalized,
  must be a directory; errors to stderr, exit 1; then Tauri. Six tests of the
  argument handling, two of them through `run` failing before any window.
  `generate_context!` sits in its own `context()` under `#[expect(clippy::exit)]`.
- STD-002-RS 0.3.0 (§13.5 and `applies_to`), **ADR-0021** (accepted). Gate green.
- Confirmed: a debug build does not need `app/dist`.

**Steps 4–5, done.**

- `ipc/tauri-client.ts`: `isTauri()` false → `unreachable` without calling `invoke`
  (mutation-checked); `invoke` rejects → `rejected`, text dropped. Tested with
  `mockIPC`, which does **not** set `isTauri` — the tests set it via `Reflect`.
  Rust-shaped replies pass the real parse path. `main.tsx`: Tauri transport and
  `"backend"` inside a window, fixtures otherwise.
- Capabilities grant exactly the five `allow-*` permissions `build.rs` generates
  from Tauri's app manifest; no `core:default`. A grant for a nonexistent command
  fails the build (probed). `permissions/autogenerated/` is ignored. The mirror
  build.rs ↔ `generate_handler!` ↔ capabilities ↔ registry is still by hand until
  `check_ipc_contract.py` exists.
- `RustOwner` gains `sv2-studio` (workspace, file_text), sourced in ADR-0021.
- **Looked at it** on a copy of the SysML examples (96 files; the vendor path is
  hook-blocked in Bash, so copy it to the scratchpad): real tree, real counts, no
  Fixture badge. It exposed two layout bugs fixtures never could — the tree spilled
  over VIEWS, and counts wrapped. Fixed in `Tabs` (fills its column and scrolls when
  it has a panel) and `Tree` (count never shrinks). `screencapture` has no
  permission in-session; ask the user for a screenshot.

**Remaining.**

1. The pop-out editor window (IX-07). Will need `core:window`/`core:webview`
   permissions granted one at a time, and a second capability for the new window.

**Hazards still true.** The PreToolUse hook blocks any Bash command whose text names
the studio crate's path; use the Write tool. A manifest change makes `state.json`
stale (`regen_state.py`); a new ADR makes `decisions.json` stale
(`index_decisions.py`).

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
python3.12 scripts/check_headers.py
python3.12 scripts/check_web_dependencies.py
env -C app bun run typecheck && env -C app bun run lint && env -C app bun test
./scripts/gate.sh            # the only definition of done
```

## Open questions

- ~~§2.1's row for `main.tsx` omits `zod` and `ipc`~~ — **resolved by amending the
  standard** (STD-004-TS 0.6.0): §2.2 and §4.3 rule 6 require both imports.
- **The mockup's Files tree shows `workspace.json` and an empty `verification/`,**
  contradicting its own IX-01 ("a folder tree of `.sysml`/`.kerml` files"). The
  contract follows IX-01.

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
