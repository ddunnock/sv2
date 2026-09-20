// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import { afterEach, describe, expect, test } from "bun:test";
import { Transaction } from "@codemirror/state";
import { type EditorView, runScopeHandlers } from "@codemirror/view";

import { EditorHandoffSchema } from "@/contract/editor-window";

import { createSharedDocument, type SharedDocument } from "./shared-document";

const TEXT = "part def Heater {\n}\n";

let open: Array<[SharedDocument, EditorView]> = [];

afterEach(() => {
  for (const [document, view] of open) {
    document.detach(view);
  }
  open = [];
});

/** Attaches a view to `document` in a fresh container, remembered for cleanup. */
function attach(document: SharedDocument, label: string): EditorView {
  const parent = window.document.createElement("div");
  window.document.body.append(parent);
  const view = document.attach({ parent, label });
  open.push([document, view]);
  return view;
}

/** Types `text` at the end of `view`, as a person would. */
function type(view: EditorView, text: string): void {
  view.dispatch({
    changes: { from: view.state.doc.length, insert: text },
    annotations: Transaction.userEvent.of("input.type"),
  });
}

/** Presses a Ctrl chord in `view`, through CodeMirror's own keymap handling. */
function press(view: EditorView, chord: "Ctrl+Z" | "Ctrl+Shift+Z"): void {
  const shiftKey = chord === "Ctrl+Shift+Z";
  runScopeHandlers(
    view,
    new KeyboardEvent("keydown", { key: "z", ctrlKey: true, shiftKey }),
    "editor",
  );
}

describe("createSharedDocument", () => {
  test("every view shows the same text", () => {
    const document = createSharedDocument(TEXT);
    const a = attach(document, "split");
    const b = attach(document, "sidebar");
    expect(a.state.doc.toString()).toBe(TEXT);
    expect(b.state.doc.toString()).toBe(TEXT);
  });

  test("an edit in one view appears in the others", () => {
    const document = createSharedDocument(TEXT);
    const a = attach(document, "split");
    const b = attach(document, "sidebar");
    type(a, "// edited\n");
    expect(b.state.doc.toString()).toBe(`${TEXT}// edited\n`);
  });

  test("there is one undo scope: undo in one view undoes an edit made in another", () => {
    const document = createSharedDocument(TEXT);
    const a = attach(document, "split");
    const b = attach(document, "sidebar");
    type(a, "// edited\n");
    press(b, "Ctrl+Z");
    expect(a.state.doc.toString()).toBe(TEXT);
    expect(b.state.doc.toString()).toBe(TEXT);
  });

  test("redo restores it everywhere", () => {
    const document = createSharedDocument(TEXT);
    const a = attach(document, "split");
    const b = attach(document, "sidebar");
    type(b, "// edited\n");
    press(a, "Ctrl+Z");
    press(a, "Ctrl+Shift+Z");
    expect(a.state.doc.toString()).toBe(`${TEXT}// edited\n`);
    expect(b.state.doc.toString()).toBe(`${TEXT}// edited\n`);
  });

  test("a view attached later opens on the current text, not the original", () => {
    const document = createSharedDocument(TEXT);
    const a = attach(document, "split");
    type(a, "// edited\n");
    expect(attach(document, "window").state.doc.toString()).toBe(`${TEXT}// edited\n`);
  });

  test("a detached view is no longer updated", () => {
    const document = createSharedDocument(TEXT);
    const a = attach(document, "split");
    const parent = window.document.createElement("div");
    const b = document.attach({ parent, label: "sidebar" });
    document.detach(b);
    type(a, "// edited\n");
    expect(b.state.doc.toString()).toBe(TEXT);
  });

  test("each view carries its accessible name", () => {
    const document = createSharedDocument(TEXT);
    const view = attach(document, "model/ThermalControl.sysml");
    expect(view.contentDOM.getAttribute("aria-label")).toBe("model/ThermalControl.sysml");
  });
});

describe("moving a document to another window", () => {
  test("a restored snapshot has the text as last edited", () => {
    const original = createSharedDocument(TEXT);
    type(attach(original, "split"), "// edited\n");
    const moved = createSharedDocument(original.snapshot());
    expect(attach(moved, "window").state.doc.toString()).toBe(`${TEXT}// edited\n`);
  });

  test("the undo history moves with it: undo in the new window undoes the old window's edit", () => {
    const original = createSharedDocument(TEXT);
    type(attach(original, "split"), "// edited\n");
    const view = attach(createSharedDocument(original.snapshot()), "window");
    press(view, "Ctrl+Z");
    expect(view.state.doc.toString()).toBe(TEXT);
  });

  test("the snapshot survives the wire: through JSON and the contract, it still restores", () => {
    const original = createSharedDocument(TEXT);
    type(attach(original, "split"), "// edited\n");
    const sent = { path: "model/Heater.sysml", ...original.snapshot() };
    const received = EditorHandoffSchema.safeParse(JSON.parse(JSON.stringify(sent)));
    if (!received.success) {
      throw new Error("a snapshot does not satisfy EditorHandoffSchema");
    }
    const view = attach(createSharedDocument(received.data), "window");
    press(view, "Ctrl+Z");
    expect(view.state.doc.toString()).toBe(TEXT);
  });

  test("a state that will not restore falls back to the text, losing only undo", () => {
    const view = attach(createSharedDocument({ text: TEXT, state: { nonsense: true } }), "window");
    expect(view.state.doc.toString()).toBe(TEXT);
    press(view, "Ctrl+Z");
    expect(view.state.doc.toString()).toBe(TEXT);
  });

  test("a state whose text disagrees with the snapshot's text is not trusted", () => {
    const other = createSharedDocument("something else\n").snapshot();
    const view = attach(createSharedDocument({ text: TEXT, state: other.state }), "window");
    expect(view.state.doc.toString()).toBe(TEXT);
  });
});
