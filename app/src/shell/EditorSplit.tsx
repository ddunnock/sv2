// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The editor in its split placement, beside the views (the plan's
 * `EditorPlacement` `split`).
 *
 * It asks for the file's text, and once it has it, hands it to `editor/`,
 * which owns it from then on (§8.4). The shell never sees the text again. A
 * file that has just docked back from the editor window arrives as a snapshot
 * instead, with its unsaved edits and undo history, and is not read again.
 *
 * EDITS ARE NOT SAVED, AND IT SAYS SO. ADR-0013 requires the editor to accept
 * input before the parser exists, so it does; but no command writes a file
 * yet, and an editor that silently drops edits would be the worst kind of
 * lie. The header states it in plain words.
 */

import type { WorkspacePath } from "@/contract/file";
import { FileEditor } from "@/editor/FileEditor";
import type { DocumentSnapshot, SharedDocument } from "@/editor/shared-document";

import { AnswerView } from "./AnswerView";
import { IslandBoundary } from "./IslandBoundary";
import { Icon } from "./primitives/Icon";
import { type Availability, ENABLED, IconButton } from "./primitives/IconButton";
import { useAnswer, useServices } from "./services";

/** Props for `EditorSplit`. */
export type EditorSplitProps = Readonly<{
  path: WorkspacePath;
  /** A file docked back from the editor window, or `null` to read it from disk. */
  seed: DocumentSnapshot | null;
  /** Whether it can move to its own window, and if not, why. */
  undock: Availability;
  onUndock: () => void;
  onClose: () => void;
  /** Receives the document once made, so the shell can move it. */
  onDocument: (document: SharedDocument | null) => void;
}>;

/** One open file, editable, beside the views. */
export function EditorSplit(props: EditorSplitProps): React.JSX.Element {
  const { report } = useServices();
  const { path, seed, onDocument } = props;
  return (
    <section
      aria-label="Editor"
      className="flex min-w-0 flex-1 flex-col border-line border-l bg-panel"
    >
      <header className="flex h-view-tabs shrink-0 items-center gap-2 border-line border-b bg-panel-2 px-3">
        <span className="truncate font-mono text-code text-fg">{path}</span>
        <span className="text-meta text-warn">
          Edits are not saved: sv2 does not yet support writing files.
        </span>
        <span className="ml-auto" />
        <IconButton
          label="Move to its own window"
          icon={<Icon name="window" />}
          availability={props.undock}
          onPress={props.onUndock}
        />
        <IconButton
          label="Close editor"
          icon={<Icon name="close" />}
          availability={ENABLED}
          onPress={props.onClose}
        />
      </header>
      <div className="min-h-0 flex-1">
        <IslandBoundary island="editor" report={report}>
          {seed === null ? (
            <FromDisk path={path} onDocument={onDocument} />
          ) : (
            <FileEditor seed={seed} label={path} onDocument={onDocument} />
          )}
        </IslandBoundary>
      </div>
    </section>
  );
}

/** The file read from the workspace, then edited. */
function FromDisk(
  props: Readonly<{
    path: WorkspacePath;
    onDocument: (document: SharedDocument | null) => void;
  }>,
): React.JSX.Element {
  const { path, onDocument } = props;
  const file = useAnswer((queries) => queries.fileText(path), `file_text:${path}`);
  return (
    <AnswerView query={file} what="The file">
      {(data) => (
        <FileEditor key={data.path} seed={data.text} label={data.path} onDocument={onDocument} />
      )}
    </AnswerView>
  );
}
