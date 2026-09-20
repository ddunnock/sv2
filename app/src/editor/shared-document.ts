// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * One document, any number of editor views over it.
 *
 * THE PLAN'S SECOND LOAD-BEARING DECISION. The mockup's Element Source tab
 * staged edits behind Apply and Revert: a second buffer, a second undo scope,
 * and a window in which the sidebar and the file disagree — which ADR-0001 and
 * §8.4 both forbid. Instead, every place the text is shown — a split beside
 * the views, the sidebar, a second OS window — is a view over this one
 * document, and there is nothing to apply.
 *
 * HOW. A headless `EditorState` is the authority: it holds the text and the
 * only undo history. Each view has its own state for its own selection and
 * scroll, and when a person edits in one view, the change is applied to the
 * authority and relayed to every other view, tagged so it is not relayed back.
 * Undo and redo, pressed in any view, run against the authority and are
 * relayed to all of them — one undo scope, whichever view the edit was made
 * in. This is CodeMirror's split-view pattern, generalized from two views to
 * any number.
 *
 * THE TEXT NEVER LEAVES CODEMIRROR (§8.4 rule 1). Nothing here hands the text
 * to React; a view shows it, and the authority keeps it.
 *
 * MOVING TO ANOTHER WINDOW. A second OS window is a second JavaScript context,
 * so it cannot hold a view over this document. The document moves instead
 * (IX-07): `snapshot` serializes the authority, undo history included, and
 * `createSharedDocument` restores one from it in the other window. The
 * snapshot is the only time the text leaves CodeMirror, and it goes straight
 * to the wire.
 */

import { defaultKeymap, history, historyField, redo, undo } from "@codemirror/commands";
import {
  Annotation,
  EditorState,
  type Extension,
  type StateCommand,
  Transaction,
} from "@codemirror/state";
import {
  drawSelection,
  EditorView,
  highlightActiveLine,
  keymap,
  lineNumbers,
} from "@codemirror/view";

/** Marks a change as relayed from another view, so it is applied and not relayed again. */
const relayed = Annotation.define<boolean>();

/** How one view is attached. */
export type AttachOptions = Readonly<{
  parent: HTMLElement;
  /** The editor's accessible name: "model/ThermalControl.sysml". */
  label: string;
}>;

/**
 * A document serialized to move between windows: the text, and the state it
 * restores from. `state` is this module's to write and read, and opaque to
 * everything the snapshot passes through.
 */
export type DocumentSnapshot = Readonly<{ text: string; state: unknown }>;

/** A document that views attach to and detach from. */
export type SharedDocument = Readonly<{
  attach: (options: AttachOptions) => EditorView;
  detach: (view: EditorView) => void;
  /** The document now, with its undo history, to move it to another window. */
  snapshot: () => DocumentSnapshot;
}>;

/** The state fields a snapshot carries beyond the text and selection. */
const SNAPSHOT_FIELDS = { history: historyField } as const;

/** Plain-text editing: line numbers, a drawn selection, the default keys (ADR-0013: no parser yet). */
const PLAIN_TEXT: Extension = [
  lineNumbers(),
  drawSelection(),
  highlightActiveLine(),
  EditorView.theme({
    "&": { height: "100%", backgroundColor: "var(--color-panel)", color: "var(--color-fg)" },
    ".cm-scroller": { fontFamily: "var(--font-mono)", fontSize: "var(--text-code)" },
    ".cm-gutters": {
      backgroundColor: "var(--color-panel-2)",
      color: "var(--color-faint)",
      borderRight: "1px solid var(--color-line)",
    },
    ".cm-activeLine": { backgroundColor: "var(--color-highlight)" },
  }),
];

/**
 * The authority to start from: fresh from `text`, or restored from a snapshot.
 *
 * A snapshot that will not restore, or restores to text other than it claims,
 * falls back to its `text`: the words survive and only the undo history is
 * lost, which is what `contract/editor-window.ts` promises.
 */
function authorityFrom(seed: string | DocumentSnapshot): EditorState {
  const text = typeof seed === "string" ? seed : seed.text;
  if (typeof seed !== "string") {
    try {
      const restored = EditorState.fromJSON(seed.state, { extensions: history() }, SNAPSHOT_FIELDS);
      if (restored.doc.toString() === text) {
        return restored;
      }
    } catch {
      // Not a state this module wrote. Start from the text, below.
    }
  }
  return EditorState.create({ doc: text, extensions: history() });
}

/** A document holding `seed`'s text, with no views yet. */
export function createSharedDocument(seed: string | DocumentSnapshot): SharedDocument {
  let authority = authorityFrom(seed);
  const views = new Set<EditorView>();

  /** Sends `changes` to every view but `except`, marked as relayed. */
  const relay = (tr: Transaction, except: EditorView | null): void => {
    for (const view of views) {
      if (view !== except) {
        view.dispatch({ changes: tr.changes, annotations: relayed.of(true) });
      }
    }
  };

  /** Runs undo or redo against the authority and shows the result everywhere. */
  const onAuthority = (command: StateCommand) => (): boolean =>
    command({
      state: authority,
      dispatch: (tr) => {
        authority = tr.state;
        relay(tr, null);
      },
    });

  const attach = ({ parent, label }: AttachOptions): EditorView => {
    const view: EditorView = new EditorView({
      parent,
      state: EditorState.create({
        doc: authority.doc,
        extensions: [
          PLAIN_TEXT,
          EditorView.contentAttributes.of({ "aria-label": label }),
          keymap.of([
            { key: "Mod-z", run: onAuthority(undo), preventDefault: true },
            { key: "Mod-y", run: onAuthority(redo), preventDefault: true },
            { key: "Mod-Shift-z", run: onAuthority(redo), preventDefault: true },
            ...defaultKeymap,
          ]),
        ],
      }),
      dispatchTransactions: (transactions) => {
        view.update(transactions);
        for (const tr of transactions) {
          if (tr.changes.empty || tr.annotation(relayed) === true) {
            continue;
          }
          // A person's edit in this view: record it once, in the one history,
          // keeping its user event so history groups it as it would locally.
          const userEvent = tr.annotation(Transaction.userEvent);
          authority = authority.update({
            changes: tr.changes,
            ...(userEvent === undefined
              ? {}
              : { annotations: Transaction.userEvent.of(userEvent) }),
          }).state;
          relay(tr, view);
        }
      },
    });
    views.add(view);
    return view;
  };

  const detach = (view: EditorView): void => {
    views.delete(view);
    view.destroy();
  };

  const snapshot = (): DocumentSnapshot => ({
    text: authority.doc.toString(),
    state: authority.toJSON(SNAPSHOT_FIELDS),
  });

  return { attach, detach, snapshot };
}
