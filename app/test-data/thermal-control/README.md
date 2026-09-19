# ThermalControl — sample IPC replies

The mockup's invented sample model (`docs/design/sysml-workbench-mockup`, "Sample
model"), as the raw replies a backend would send. STD-004-TS §11 rule 7: fixture
inputs are data, here.

| File | Reply to |
|---|---|
| `workspace.json` | `workspace` |
| `views.json` | `views` |
| `elements/<petname>.json` | `element_detail` for that petname |
| `files/<workspace path>.json` | `file_text` for that file |

**Everything here is sample data.** Every petname and library UUID is invented; the
offsets are real positions in the one file text below. It reaches the screen only
behind the status bar's "Fixture data" badge.

**The files are raw, and are checked.** They are parsed by the same schemas as a
backend reply (`src/contract/registry.ts`), so a file that drifts from the contract
fails `src/ipc/fixture-client.test.ts`. Plain JSON, rather than a TypeScript module,
so the Rust side can load the same replies to check its own serialization.

**`RES-UNRESOLVED-NAME` is not a real code.** `Heater` refers to `HeaterState`, which
does not exist — the mockup's deliberate diagnostic. No `RES-*` code exists in
`sv2-syntax` yet; the code has the shape `contract/diagnostic.ts` admits, which is all
a fixture can honestly claim.

**The file text is the mockup's own.** `files/model/ThermalControl.sysml.json` is the
text SCR-05's pop-out editor shows, lines 1–36 exactly, including line 27's
`attribute state : HeaterState;`. The mockup's view is scrolled and shows no more, so
the text stops there — unclosed — rather than inventing the rest. The element spans in
`elements/` are real offsets into this text. Only this file has text; asking for any
other is answered `not-implemented`, because the file exists and only the sample
lacks it.

**What is deliberately absent.** No layout: every view's layout is answered
`not-implemented` by `src/ipc/fixture-client.ts`, because the plan forbids simulated
diagrams. An element with no file here is `not-found`.

**After editing,** run `bun run fixtures` to regenerate
`src/ipc/generated/thermal-control.ts`. `src/ipc/fixture-data.test.ts` fails until you
do.
