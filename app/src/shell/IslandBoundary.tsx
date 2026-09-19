// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The error boundary around each island: the editor, the diagram, and the
 * Specification sidebar (§7.4).
 *
 * A render defect in one island shows a recoverable panel in that island's
 * place and leaves the others working — the diagram failing must not take the
 * text editor with it, because the text is the model (ADR-0001).
 *
 * THE ONE CLASS COMPONENT (§8.1 rule 1). React has no hook that catches a
 * render error, so this is a class, and nothing else in the codebase is.
 *
 * WHAT IS REPORTED. The island's name and the error, through the `report`
 * function the composition root chose. The error's message is written by a
 * throw site that §9.3 already forbids from quoting model text; the panel the
 * user sees shows neither, only which island failed and a way to retry.
 */

import { Component, type ErrorInfo, type ReactNode } from "react";

/** The three islands §7.4 names. */
export type Island = "editor" | "diagram" | "sidebar";

/** How a caught failure is reported: `diagnostics/`'s reporter, passed in. */
export type Report = (what: string, detail: unknown) => void;

/** Props for `IslandBoundary`. */
export type IslandBoundaryProps = Readonly<{
  island: Island;
  report: Report;
  children: ReactNode;
}>;

type State = Readonly<{ failed: boolean }>;

const NAMES: Readonly<Record<Island, string>> = {
  editor: "The editor",
  diagram: "The diagram",
  sidebar: "The specification sidebar",
};

/** Catches a render failure in one island and offers to retry it. */
export class IslandBoundary extends Component<IslandBoundaryProps, State> {
  override state: State = { failed: false };

  /** Switches to the failure panel. React calls this during render. */
  static getDerivedStateFromError(): State {
    return { failed: true };
  }

  /** Reports the failure. React calls this after the failure panel has rendered. */
  override componentDidCatch(error: Error, _info: ErrorInfo): void {
    this.props.report(`render failure in the ${this.props.island} island`, error);
  }

  override render(): ReactNode {
    if (!this.state.failed) {
      return this.props.children;
    }
    return (
      <div role="alert" className="flex flex-col items-start gap-2 p-4 text-ui">
        <p className="text-fg">{NAMES[this.props.island]} stopped because of a defect.</p>
        <p className="text-muted">The rest of the window still works, and it has been reported.</p>
        <button
          type="button"
          onClick={() => {
            this.setState({ failed: false });
          }}
          className="h-control rounded-wb border border-line bg-panel px-3 text-fg hover:bg-hover"
        >
          Try again
        </button>
      </div>
    );
  }
}
