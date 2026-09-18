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
 * ADR-0001 is why the layout is the shape it is: text is authoritative and the
 * diagram is a projection of it, so the editor is the primary pane and the
 * diagram is a view beside it, never the other way round.
 */

/** The root component. Takes no props: the composition root supplies services. */
export function Shell(): React.JSX.Element {
  return (
    <main className="shell">
      <h1>sv2 Studio</h1>
      <p>
        A SysML v2 and KerML editor. Text is authoritative; diagrams are
        projections of it (ADR-0001).
      </p>
      <section>
        <h2>Not built yet</h2>
        <ul>
          <li>
            <code>editor/</code> — the CodeMirror host. Owns the text (§8.4).
          </li>
          <li>
            <code>diagram/</code> — the SVG island, projected from the tree.
          </li>
          <li>
            <code>ipc/</code> — the Tauri boundary. The only caller of{" "}
            <code>invoke</code>.
          </li>
          <li>
            <code>wasm/</code> — the <code>sv2-wasm</code> loader and the flat
            node-buffer adapter (ADR-0013).
          </li>
          <li>
            <code>contract/</code> — the Zod half of the IPC contract (§4.2).
          </li>
        </ul>
      </section>
    </main>
  );
}
