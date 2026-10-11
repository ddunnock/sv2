---
paths:
  - "crates/sv2-ast/**"
---

# sv2-ast — typed accessors over the CST

This crate is a view. It owns no data, and every rule below follows from that.

## Owns no data

An accessor type wraps exactly one `SyntaxNode` and nothing else. It returns nodes and
tokens; the caller asks those for text. No `String`, `Box<str>`, `Vec<String>` field, and
no public function returning `String`. `scripts/check_ast_leaks.py` fails on each.

ADR-0004 justifies the lossless tree partly because this crate makes it affordable.
Copying text out is how that stops being true.

## The marker, and the three states

Every accessor carries `// node: Kind`. `scripts/ast_coverage.py` counts markers against
`.claude/state/node-kinds.json`:

- **implemented** — a marker claims a kind. Counted.
- **unimplemented** — a kind no marker claims. Tracked, never a failure.
- **absent** — a marker claims a kind that does not exist. A defect: the kind is added to
  the parser's `NODES` and regenerated, or the marker is wrong.

A large unimplemented count is the honest state of this crate, not a problem to fix.

## A malformed case per accessor, in the same change

Three things, because absence is not the only way to be wrong:

1. the parse still yields a tree — it always does, and the accessor sees a partial one;
2. the accessor reports **absence** rather than guessing — and that something *reported*
   it: read `Parse::errors()`, because absence nothing reported is absence guessed;
3. the neighbouring accessors still answer. With one accessor in the crate there is no
   neighbour, so this part is **vacant, not discharged** — say so in the test rather
   than letting a `kind()` check stand in for it.

Where the grammar admits one child and the tree carries two, the case pins **which** is
chosen. `.claude/rules/tests.md` governs where expectations may come from; read it.

A recovery outcome is not a spec rule, and it is usually not an ADR either. The
specification says a package body closes with `}`; it says nothing about what this parser
does when one does not. For a recovery outcome the source is **the parser** — name the
function and the file. ADR-0004 supports only the shape (an error node is just another
node, so a partial tree survives), never a particular outcome.

Do not reach for ADR-0022. It is about deviation diagnostics and strict conformance and
says nothing about recovery; `crates/sv2-syntax/src/parser/recovery.rs` mentions both in
one sentence, which is where that mistake comes from. It has been made twice here.

### The case is a directory named for its kind

`crates/sv2-ast/tests/malformed/<kind-in-kebab>/<condition>.sysml` (or `.kerml`).
`RootNamespace` is proved by `root-namespace/unclosed-body.sysml`.

The kind is the directory, not a filename prefix, because a filename cannot carry both
unambiguously: `-` separates the words inside a kebab kind *and* would separate kind from
condition, and 91 kinds have a kebab form that prefixes another's. `package-body-missing`
reads equally as `Package` + "body missing" and as `PackageBody` + "missing", and no rule
can tell them apart. A directory keeps the two in separate namespaces.

`scripts/ast_coverage.py` fails on an implemented kind with no non-empty case (`unproved`)
and on a case that names no kind (`stray`) — a misspelled directory, or a file left at the
top level, which would otherwise stop counting in silence.

The gate checks the case *exists* and is not empty. It cannot check that it asserts the
right things, so review does.

## The sweep is where an accessor is actually exercised

`tests/sweep.rs` walks every tree in the repository — 311 corpus files and 299 rejection
files. An accessor that is not called from there is not swept at all, and the suite stays
green because it has nothing to check, which is the failure ADR-0025 exists to prevent.
Adding the accessor without adding its sweep assertions is the easiest way to get a green
gate that proves nothing, and **no check catches it**: `sweep()` names each accessor's
assertion function explicitly.

Each kind gets an `assert_<kind>_accessor` asserting:

- the cast contract — `cast` succeeds exactly when `kind()` matches, and `syntax()` hands
  back the node it was cast from;
- the differential — re-derive the same fact by walking the tree directly, then assert
  **equality**, not containment.

Equality, because a subset check reads like a differential and is not one. Measured on the
first accessor against six plausible `members()` defects, `children.contains(member)`
caught one and equality caught all six: containment passes an accessor that returns
nothing, one that reverses the order, and one that silently filters a kind out.

An accessor that legitimately filters — most will, being a view of one child — filters in
the re-derivation too, with a comment saying why. That is the point: the equality forces
the decision to be written down instead of made by accident. What is never acceptable is
relaxing equality to containment to make a filtering accessor pass.

Then mutate the accessor and watch the sweep go red before you trust it. An assertion
nobody has seen fail is a guess.

## What this crate does not do

It raises `ID-MALFORMED` and `ID-ORPHAN`, because attaching an `@id` note to its
declaration needs nothing beyond the current file. It does **not** allocate an identifier,
detect a duplicate, resolve a name, or construct a metaclass. Those need more than one
file, or need the metamodel, and writing one here is how the layering quietly dies.

## Test and development code

Unit tests at the bottom of the file they test, in `#[cfg(test)] mod tests` — the only
place a test reaches a private item, and compiled out of the shipped crate. Everything
else lives outside `src/`: shared support in `tests/trees/`, data in `tests/malformed/`,
tooling in `examples/`. `utils`, `helpers`, `common`, `misc`, `manager` and `handler` are
banned names anywhere under `crates/`, test files included.
