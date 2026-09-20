// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The editor's own window (IX-07, SCR-05): the file that moved here, and the
 * way back.
 *
 * THE FILE ARRIVES ONCE. It is taken from `editor_handoff` when the page
 * loads, and a taken handoff is gone, so the request is made once per page and
 * shared by every run of the effect — React's development double run included,
 * which would otherwise take it once and then find nothing.
 *
 * IT LEAVES BY DOCKING, HOWEVER IT LEAVES. The Dock button, the main window's
 * Dock, and this window's own close button all end here: the document is
 * snapshotted and handed back, and Rust closes the window. Nothing is saved,
 * so there is no other way out that keeps the edits.
 *
 * This window reads nothing from the workspace and asks no model question.
 * Its capability grants only the handoff, the dock, and the event.
 */

import { useCallback, useEffect, useRef, useState } from "react";

import type { EditorHandoff } from "@/contract/editor-window";
import { FileEditor } from "@/editor/FileEditor";
import type { SharedDocument } from "@/editor/shared-document";
import type { IpcError } from "@/ipc/ipc-error";
import type { Reply } from "@/ipc/model-queries";
import { firstLoad, type Query } from "@/model/query";

import { AnswerView } from "./AnswerView";
import { IslandBoundary } from "./IslandBoundary";
import { Icon } from "./primitives/Icon";
import { ENABLED, IconButton } from "./primitives/IconButton";
import { type Services, ServicesProvider, useServices } from "./services";

/** Props for `EditorWindowShell`. Services come from the composition root (§2.2). */
export type EditorWindowShellProps = Readonly<{ services: Services }>;

/** The editor window, inside the services it runs on. */
export function EditorWindowShell({ services }: EditorWindowShellProps): React.JSX.Element {
  return (
    <ServicesProvider services={services}>
      <EditorWindowPage />
    </ServicesProvider>
  );
}

function EditorWindowPage(): React.JSX.Element {
  const { editorWindow, report } = useServices();
  const [arrived, setArrived] = useState<Query<EditorHandoff, IpcError>>(firstLoad);
  const taken = useRef<Reply<EditorHandoff> | null>(null);

  useEffect(() => {
    let current = true;
    taken.current ??= editorWindow.handoff();
    taken.current.then(
      (result) => {
        if (!current) {
          return;
        }
        if (result.ok) {
          setArrived({ status: "answered", answer: result.value });
        } else {
          report("taking the file for the editor window failed", result.error);
          setArrived({ status: "failed", error: result.error });
        }
      },
      (defect: unknown) => {
        report("taking the file for the editor window failed", defect);
      },
    );
    return () => {
      current = false;
    };
  }, [editorWindow, report]);

  return (
    <div className="t-light flex h-screen flex-col bg-bg font-sans text-fg text-ui">
      <AnswerView query={arrived} what="The file for this window">
        {(handoff) => <EditorWindowBody handoff={handoff} />}
      </AnswerView>
    </div>
  );
}

/** The file, editable, with the way back to the main window. */
function EditorWindowBody({ handoff }: Readonly<{ handoff: EditorHandoff }>): React.JSX.Element {
  const { editorWindow, report } = useServices();
  const document = useRef<SharedDocument | null>(null);
  const onDocument = useCallback((made: SharedDocument | null) => {
    document.current = made;
  }, []);

  const dock = useCallback((): void => {
    const made = document.current;
    if (made === null) {
      return;
    }
    editorWindow.dock({ path: handoff.path, ...made.snapshot() }).then(
      (result) => {
        // On success Rust closes this window, and nothing here runs again.
        if (!(result.ok && result.value.kind === "ready")) {
          report("docking the editor failed", result.ok ? result.value : result.error);
        }
      },
      (defect: unknown) => {
        report("docking the editor failed", defect);
      },
    );
  }, [editorWindow, report, handoff.path]);

  useEffect(
    () =>
      editorWindow.onDockRequested(dock, (error) => {
        report("listening for a dock request failed", error);
      }),
    [editorWindow, report, dock],
  );

  return (
    <section aria-label="Editor" className="flex min-h-0 flex-1 flex-col bg-panel">
      <header className="flex h-view-tabs shrink-0 items-center gap-2 border-line border-b bg-panel-2 px-3">
        <span className="truncate font-mono text-code text-fg">{handoff.path}</span>
        <span className="text-meta text-warn">
          Edits are not saved: sv2 does not yet support writing files.
        </span>
        <span className="ml-auto" />
        <IconButton
          label="Dock to main window"
          icon={<Icon name="window" />}
          availability={ENABLED}
          onPress={dock}
        />
      </header>
      <div className="min-h-0 flex-1">
        <IslandBoundary island="editor" report={report}>
          <FileEditor
            seed={{ text: handoff.text, state: handoff.state }}
            label={handoff.path}
            onDocument={onDocument}
          />
        </IslandBoundary>
      </div>
    </section>
  );
}
