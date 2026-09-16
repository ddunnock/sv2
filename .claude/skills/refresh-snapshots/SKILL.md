---
name: refresh-snapshots
description: Review and accept insta snapshot changes one at a time, with an explanation for each. Use when snapshots are pending, when tests fail on snapshot mismatch, or when the user says "review snapshots", "accept snapshots", or "the snapshots changed".
---

# Refresh snapshots

Snapshots are the drift detector for tree shape. Accepting them in bulk destroys that,
so this workflow is deliberately slower than `cargo insta accept`.

**Never run `cargo insta accept`.** If you are tempted to, that is the signal that too
much changed in one step.

## 1. Inventory

```bash
cargo insta pending-snapshots
```

More than about five pending? The change was too large. Say so, and propose splitting it
rather than reviewing thirty diffs.

## 2. For each snapshot, in order

Look at the diff and answer, out loud, before accepting:

1. **Which code change caused this?** Name the function or the commit hunk. If you cannot,
   stop — something changed that you did not intend.
2. **Is the new shape right?** Not "is it plausible" — check it against the retrieved
   metaclass shape. A snapshot that is merely different from before is not evidence that
   it is correct now.
3. **Did anything disappear?** Lost trivia is a losslessness violation and outranks
   whatever you were working on.

Then accept that one:

```bash
cargo insta accept --snapshot <path>
```

## 3. Unexplained diffs

A snapshot that moved for no reason you can identify means a shared code path changed.
Find it before accepting anything else. Reject the snapshot, re-run, and investigate.

## 4. Commit together

Snapshots go in the same commit as the code that changed them. A separate
"update snapshots" commit hides drift in a diff nobody reads.

## 5. Close out

```bash
./scripts/gate.sh
```
