// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The window: its regions, their layout, and the shortcuts that change it.
 *
 * THE MOCKUP'S REGIONS, AS LANDMARKS. The top bar (UI-01) is the `header`,
 * the navigator (UI-03) a `nav`, the view area (UI-05) the `main`, the
 * Specification sidebar (UI-08, UI-09) an `aside`, and the status bar (UI-10)
 * the `footer`, so assistive technology can jump between them. Each region is
 * built from the five primitives in `primitives/`.
 *
 * ONE LAYOUT STATE. What is shown is `layout-state.ts`'s reducer, driven by
 * the buttons and by IX-09's shortcuts, which a window listener turns into the
 * same actions. Focus mode is one of its values, not a separate screen.
 *
 * NOTHING HERE PRETENDS. Panels whose data is not wired yet say so through
 * `<Unavailable>`, naming what sv2 cannot yet do; deferred controls are
 * disabled with the reason in their tooltip. ADR-0001 is why the layout is
 * this shape: the text is the model, so the diagram and the sidebar are views
 * beside it.
 */

import { type Dispatch, useEffect, useReducer, useState } from "react";

import type { SidebarState } from "@/contract/preferences";

import { IslandBoundary, type Report } from "./IslandBoundary";
import {
  INITIAL_LAYOUT,
  type LayoutAction,
  type LayoutState,
  layoutReducer,
  shortcutAction,
} from "./layout-state";
import { Icon } from "./primitives/Icon";
import { type Availability, ENABLED, IconButton } from "./primitives/IconButton";
import { Splitter } from "./primitives/Splitter";
import { Tabs } from "./primitives/Tabs";
import { Toolbar, type ToolbarItem } from "./primitives/Toolbar";
import { SelectionProvider, useSelection } from "./selection";
import { Unavailable } from "./Unavailable";

/** Props for `Shell`. Services come from the composition root, never from an import. */
export type ShellProps = Readonly<{ report: Report }>;

type Theme = "light" | "dark";

/** Why a control is disabled, stated where the user can reach it. */
const NOT_YET = (what: string): Availability => ({
  kind: "disabled",
  reason: `${what} is not implemented yet`,
});

const SIDEBAR_TABS = [
  { id: "specification", label: "Specification" },
  { id: "source", label: "Element Source" },
] as const;

const NAVIGATOR_MODES = [
  { id: "files", label: "Files" },
  { id: "elements", label: "Elements" },
] as const;

const NAVIGATOR_ID = "navigator-panel";
const SIDEBAR_ID = "sidebar-panel";

/** The whole window. */
export function Shell({ report }: ShellProps): React.JSX.Element {
  const [layout, dispatch] = useReducer(layoutReducer, INITIAL_LAYOUT);
  const [theme, setTheme] = useState<Theme>("light");
  const [navigatorWidth, setNavigatorWidth] = useState(280);
  const [sidebarWidth, setSidebarWidth] = useState(340);

  useEffect(() => {
    // A window listener is outside React, which is what an effect is for (§8.3 rule 2).
    const onKeyDown = (event: KeyboardEvent): void => {
      const action = shortcutAction(event);
      if (action !== null) {
        event.preventDefault();
        dispatch(action);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, []);

  return (
    <SelectionProvider>
      <div
        className={`flex h-screen flex-col bg-bg font-sans text-fg text-ui ${theme === "dark" ? "t-dark" : "t-light"}`}
      >
        <TopBar
          layout={layout}
          dispatch={dispatch}
          theme={theme}
          onTheme={() => {
            setTheme(theme === "dark" ? "light" : "dark");
          }}
        />
        <div className="flex min-h-0 flex-1">
          <ActivityRail layout={layout} dispatch={dispatch} />
          {layout.navigator.kind === "open" ? (
            <>
              <Navigator layout={layout} dispatch={dispatch} width={navigatorWidth} />
              <Splitter
                label="Resize navigator"
                controls={NAVIGATOR_ID}
                side="start"
                value={navigatorWidth}
                min={200}
                max={480}
                onChange={setNavigatorWidth}
              />
            </>
          ) : null}
          <ViewArea />
          {layout.sidebar.kind === "open" ? (
            <Splitter
              label="Resize sidebar"
              controls={SIDEBAR_ID}
              side="end"
              value={sidebarWidth}
              min={260}
              max={560}
              onChange={setSidebarWidth}
            />
          ) : null}
          <Sidebar
            sidebar={layout.sidebar}
            dispatch={dispatch}
            width={sidebarWidth}
            report={report}
          />
        </div>
        <StatusBar layout={layout} />
      </div>
    </SelectionProvider>
  );
}

type RegionProps = Readonly<{ layout: LayoutState; dispatch: Dispatch<LayoutAction> }>;

/** UI-01: the app name, the panel toggles, and the controls not built yet. */
function TopBar(
  props: RegionProps & Readonly<{ theme: Theme; onTheme: () => void }>,
): React.JSX.Element {
  const { layout, dispatch } = props;
  return (
    <header className="flex h-10 shrink-0 items-center gap-2 border-line border-b bg-panel px-3">
      <h1 className="font-semibold text-name">sv2 Studio</h1>
      <div className="ml-auto flex items-center gap-1">
        <IconButton
          label="Search"
          icon={<Icon name="search" />}
          availability={NOT_YET("Search")}
          onPress={() => undefined}
        />
        <IconButton
          label="Navigator"
          icon={<Icon name="navigator" />}
          availability={ENABLED}
          pressed={layout.navigator.kind === "open"}
          onPress={() => {
            dispatch({ kind: "toggle-navigator" });
          }}
        />
        <IconButton
          label="Sidebar"
          icon={<Icon name="sidebar" />}
          availability={ENABLED}
          pressed={layout.sidebar.kind === "open"}
          onPress={() => {
            dispatch({ kind: "toggle-sidebar" });
          }}
        />
        <IconButton
          label="Editor window"
          icon={<Icon name="window" />}
          availability={NOT_YET("The separate editor window")}
          onPress={() => undefined}
        />
        <button
          type="button"
          onClick={props.onTheme}
          className="h-control rounded-wb border border-line bg-panel px-3 text-code text-fg hover:bg-hover"
        >
          {props.theme === "dark" ? "Light theme" : "Dark theme"}
        </button>
      </div>
    </header>
  );
}

/** UI-02: Explorer shows or hides the navigator; the rest are not built yet. */
function ActivityRail({ layout, dispatch }: RegionProps): React.JSX.Element {
  const items: readonly ToolbarItem[] = [
    {
      id: "explorer",
      label: "Explorer",
      icon: "files",
      availability: ENABLED,
      pressed: layout.navigator.kind === "open",
      onPress: () => {
        dispatch({ kind: "toggle-navigator" });
      },
    },
    {
      id: "search",
      label: "Search",
      icon: "search",
      availability: NOT_YET("Search"),
      onPress: () => undefined,
    },
    {
      id: "validation",
      label: "Validation",
      icon: "validate",
      availability: NOT_YET("Validation"),
      onPress: () => undefined,
    },
    {
      id: "vcs",
      label: "Version control",
      icon: "branch",
      availability: NOT_YET("Version control"),
      onPress: () => undefined,
    },
    {
      id: "settings",
      label: "Settings",
      icon: "settings",
      availability: NOT_YET("Settings"),
      onPress: () => undefined,
    },
  ];
  return (
    <div className="flex w-11 shrink-0 flex-col items-center border-line border-r bg-panel-2 py-2">
      <Toolbar label="Activity" orientation="vertical" items={items} />
    </div>
  );
}

/** UI-03: the Files and Elements trees, not wired to data until Phase 4. */
function Navigator({
  layout,
  dispatch,
  width,
}: RegionProps & Readonly<{ width: number }>): React.JSX.Element {
  const mode = layout.navigator.mode;
  return (
    <nav
      id={NAVIGATOR_ID}
      aria-label="Navigator"
      style={{ width }}
      className="flex shrink-0 flex-col border-line border-r bg-panel"
    >
      <Tabs
        label="Navigator mode"
        tabs={NAVIGATOR_MODES}
        selected={mode}
        onSelect={(id) => {
          dispatch({ kind: "show-navigator-mode", mode: id === "elements" ? "elements" : "files" });
        }}
        activation="automatic"
        orientation="horizontal"
        panel={
          mode === "files" ? (
            <Unavailable
              what="The Files tree"
              because={{ kind: "not-implemented", capability: "listing a workspace's files" }}
            />
          ) : (
            <Unavailable
              what="The Elements tree"
              because={{ kind: "not-implemented", capability: "building the element hierarchy" }}
            />
          )
        }
      />
    </nav>
  );
}

/** UI-05: the open views. None can be opened until Phase 4 wires the Views list. */
function ViewArea(): React.JSX.Element {
  return (
    <main aria-label="Views" className="flex min-w-0 flex-1 flex-col bg-canvas">
      <p className="p-4 text-muted">No view is open.</p>
    </main>
  );
}

/** UI-08 open, or UI-09 collapsed to a strip of the same tabs. */
function Sidebar(
  props: Readonly<{
    sidebar: SidebarState;
    dispatch: Dispatch<LayoutAction>;
    width: number;
    report: Report;
  }>,
): React.JSX.Element {
  const { sidebar, dispatch } = props;
  const open = sidebar.kind === "open";
  return (
    <aside
      id={SIDEBAR_ID}
      aria-label="Specification sidebar"
      style={open ? { width: props.width } : undefined}
      className="flex shrink-0 flex-col border-line border-l bg-panel"
    >
      <Tabs
        label="Sidebar"
        tabs={SIDEBAR_TABS}
        selected={sidebar.tab}
        onSelect={(id) => {
          dispatch({ kind: "show-sidebar-tab", tab: id === "source" ? "source" : "specification" });
        }}
        activation="automatic"
        orientation={open ? "horizontal" : "vertical"}
        panel={
          open ? (
            <IslandBoundary island="sidebar" report={props.report}>
              <SidebarPanel tab={sidebar.tab} />
            </IslandBoundary>
          ) : null
        }
      />
    </aside>
  );
}

/** The open sidebar's content for its tab. */
function SidebarPanel({ tab }: Readonly<{ tab: SidebarState["tab"] }>): React.JSX.Element {
  const { selected } = useSelection();
  if (tab === "source") {
    return (
      <Unavailable
        what="Element Source"
        because={{ kind: "not-implemented", capability: "showing an element's source" }}
      />
    );
  }
  return selected === null ? (
    <p className="p-4 text-muted">Nothing is selected.</p>
  ) : (
    <Unavailable
      what="The specification"
      because={{ kind: "not-implemented", capability: "reading an element's specification" }}
    />
  );
}

/** UI-10: the window's state in one line. */
function StatusBar({ layout }: Readonly<{ layout: LayoutState }>): React.JSX.Element {
  return (
    <footer className="flex h-6 shrink-0 items-center gap-3 border-line border-t bg-panel-2 px-3 text-meta text-muted">
      {layout.mode.kind === "focus" ? <span>Focus mode</span> : null}
      <span className="ml-auto">SysML v2 · KerML</span>
    </footer>
  );
}
