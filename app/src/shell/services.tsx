// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The services the composition root supplies, and the hook that asks them.
 *
 * SERVICES COME FROM `main.tsx` (§2.2). The model queries and the failure
 * reporter are constructed there and handed down through this context, which
 * is what §8.3 rule 5 keeps context for: services and rarely changing values.
 * No shell module constructs a client, so choosing fixtures or the Rust core
 * is one line in the composition root.
 *
 * `useAnswer` IS THE ONE WAY A PANEL ASKS. It turns a query into a `Query`
 * state, keeps the previous answer on screen while asking again (ADR-0002, no
 * flicker), ignores a reply that arrives after a newer question, and reports a
 * failed read — a defect — while leaving the panel to render it (§9.2).
 */

import { createContext, type ReactNode, useContext, useEffect, useRef, useState } from "react";

import type { EditorWindow } from "@/ipc/editor-window";
import type { IpcError } from "@/ipc/ipc-error";
import type { ModelQueries, Reply } from "@/ipc/model-queries";
import { firstLoad, type Query, reload } from "@/model/query";

import type { Report } from "./IslandBoundary";

/**
 * What the composition root supplies. `editorWindow` is `NO_EDITOR_WINDOW`
 * outside the desktop app, so a test or a plain browser says so explicitly.
 */
export type Services = Readonly<{
  queries: ModelQueries;
  report: Report;
  editorWindow: EditorWindow;
}>;

const ServicesContext = createContext<Services | null>(null);

/** Supplies the services to everything inside it. */
export function ServicesProvider(
  props: Readonly<{ services: Services; children: ReactNode }>,
): React.JSX.Element {
  return <ServicesContext value={props.services}>{props.children}</ServicesContext>;
}

/** The services. Throws outside a `ServicesProvider`, which is a wiring defect (§7.2). */
export function useServices(): Services {
  const services = useContext(ServicesContext);
  if (services === null) {
    throw new Error("useServices called outside a ServicesProvider");
  }
  return services;
}

/**
 * Asks `ask` whenever `key` changes, and holds the state of the latest answer.
 *
 * `key` names the question, log-safely — `workspace`, or `element:<petname>` —
 * because it is also what a failure report says it was about (§9.4). `ask` may
 * be a new function every render; only a new `key` asks again.
 */
export function useAnswer<T>(
  ask: (queries: ModelQueries) => Reply<T>,
  key: string,
): Query<T, IpcError> {
  const { queries, report } = useServices();
  const [query, setQuery] = useState<Query<T, IpcError>>(firstLoad);
  const latest = useRef(ask);
  latest.current = ask;

  useEffect(() => {
    let current = true;
    setQuery(reload);
    latest.current(queries).then(
      (result) => {
        if (!current) {
          return;
        }
        if (result.ok) {
          setQuery({ status: "answered", answer: result.value });
        } else {
          report(`query ${key} failed`, result.error);
          setQuery({ status: "failed", error: result.error });
        }
      },
      (defect: unknown) => {
        // A transport must not reject (ipc/model-queries.ts). One that did is a
        // defect in the transport, reported as one.
        report(`query ${key} rejected`, defect);
      },
    );
    return () => {
      current = false;
    };
  }, [key, queries, report]);

  return query;
}
