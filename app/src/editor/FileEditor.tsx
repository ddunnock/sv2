// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * A file open in the editor: its one shared document, and a view onto it.
 *
 * `FileEditor` owns the document for one file — created once from the text
 * the core sent, never replaced (§8.4 rule 3) — and `EditorPane` is one view
 * over it. More placements of the same file (the sidebar, a second window)
 * become more panes over the same document, not more documents.
 *
 * The caller keys this by the file's path, so opening another file makes a
 * new document rather than reusing one that holds the wrong text.
 *
 * A document can also start from a snapshot, when the file has moved here from
 * another window (IX-07), and `onDocument` hands the document to the caller so
 * it can snapshot it to move it on. The caller holds the document, never its
 * text: the text stays in CodeMirror until the snapshot goes to the wire.
 */

import { useEffect, useRef, useState } from "react";

import {
  createSharedDocument,
  type DocumentSnapshot,
  type SharedDocument,
} from "./shared-document";

/** Props for `FileEditor`. */
export type FileEditorProps = Readonly<{
  /**
   * The file's text as the core sent it, or a snapshot from another window.
   * Read once, when the document is made.
   */
  seed: string | DocumentSnapshot;
  /** The editor's accessible name: the file's path. */
  label: string;
  /** Receives the document once made, and `null` when it goes. */
  onDocument?: (document: SharedDocument | null) => void;
}>;

/** One file's document, shown in one pane. */
export function FileEditor({ seed, label, onDocument }: FileEditorProps): React.JSX.Element {
  // Lazy initial state: the document is made once and the text is never state
  // again. Later text belongs to CodeMirror (§8.4 rule 1).
  const [document] = useState(() => createSharedDocument(seed));
  useEffect(() => {
    onDocument?.(document);
    return () => {
      onDocument?.(null);
    };
  }, [document, onDocument]);
  return <EditorPane document={document} label={label} />;
}

/** Props for `EditorPane`. */
export type EditorPaneProps = Readonly<{ document: SharedDocument; label: string }>;

/** One `EditorView` over `document`, created and destroyed with the pane (§8.4 rule 2). */
export function EditorPane({ document, label }: EditorPaneProps): React.JSX.Element {
  const host = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const parent = host.current;
    if (parent === null) {
      return;
    }
    const view = document.attach({ parent, label });
    return () => {
      document.detach(view);
    };
  }, [document, label]);
  return <div ref={host} className="h-full min-h-0 overflow-hidden" />;
}
