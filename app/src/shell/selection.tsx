// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The selected element, and the one way to read or change it (IX-04).
 *
 * THE SEAM FOR THE SECOND WINDOW. Selecting in the diagram or the Elements
 * tree updates the sidebar, the status bar and — once IX-07 exists — the
 * editor in another OS window. If every component kept or passed its own
 * copy, making selection cross windows would mean finding all of them. Here
 * there is one: `useSelection`. Phase 6 changes this module to sync through
 * `ipc/`, and no component changes.
 *
 * Selection is a handle, not a name, so it survives a rename (ADR-0016), and
 * selecting what is already selected is a no-op by value, not by object
 * identity, so a re-fetched handle does not churn every subscriber.
 */

import { createContext, type ReactNode, useContext, useState } from "react";

import type { ElementHandle } from "@/contract/element-id";
import { sameHandle } from "@/model/element-handle";

/** The selection and how to change it. */
export type Selection = Readonly<{
  selected: ElementHandle | null;
  select: (handle: ElementHandle | null) => void;
}>;

const SelectionContext = createContext<Selection | null>(null);

/** Holds the selection for everything inside it. One per window. */
export function SelectionProvider({
  children,
}: Readonly<{ children: ReactNode }>): React.JSX.Element {
  const [selected, setSelected] = useState<ElementHandle | null>(null);
  const select = (handle: ElementHandle | null): void => {
    setSelected((current) => {
      const unchanged =
        current === handle || (current !== null && handle !== null && sameHandle(current, handle));
      return unchanged ? current : handle;
    });
  };
  return <SelectionContext value={{ selected, select }}>{children}</SelectionContext>;
}

/**
 * The current selection. Throws outside a `SelectionProvider`: that is a
 * wiring defect (§7.2), and a silent default would hide it.
 */
export function useSelection(): Selection {
  const selection = useContext(SelectionContext);
  if (selection === null) {
    throw new Error("useSelection called outside a SelectionProvider");
  }
  return selection;
}
