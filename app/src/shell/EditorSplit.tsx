// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The editor in its split placement, beside the views (the plan's
 * `EditorPlacement` `split`).
 *
 * It asks for the file's text, and once it has it, hands it to `editor/`,
 * which owns it from then on (§8.4). The shell never sees the text again.
 *
 * EDITS ARE NOT SAVED, AND IT SAYS SO. ADR-0013 requires the editor to accept
 * input before the parser exists, so it does; but no command writes a file
 * yet, and an editor that silently drops edits would be the worst kind of
 * lie. The header states it in plain words.
 */

import type { WorkspacePath } from "@/contract/file";
import { FileEditor } from "@/editor/FileEditor";

import { AnswerView } from "./AnswerView";
import { IslandBoundary } from "./IslandBoundary";
import { Icon } from "./primitives/Icon";
import { ENABLED, IconButton } from "./primitives/IconButton";
import { useAnswer, useServices } from "./services";

/** Props for `EditorSplit`. */
export type EditorSplitProps = Readonly<{ path: WorkspacePath; onClose: () => void }>;

/** One open file, editable, beside the views. */
export function EditorSplit({ path, onClose }: EditorSplitProps): React.JSX.Element {
  const { report } = useServices();
  const file = useAnswer((queries) => queries.fileText(path), `file_text:${path}`);
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
          label="Close editor"
          icon={<Icon name="close" />}
          availability={ENABLED}
          onPress={onClose}
        />
      </header>
      <div className="min-h-0 flex-1">
        <IslandBoundary island="editor" report={report}>
          <AnswerView query={file} what="The file">
            {(data) => <FileEditor key={data.path} text={data.text} label={data.path} />}
          </AnswerView>
        </IslandBoundary>
      </div>
    </section>
  );
}
