// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The composition root, and the only module that does work at load time
 * (STD-004-TS §2.2).
 *
 * It does three things in this order and nothing else: configure Zod for the
 * Tauri content-security policy, install the global failure handlers, then
 * create the React root and render the shell.
 *
 * Every other module defines and does not execute (§3.3). A module that needs a
 * service receives it here, through a parameter or React context, and never
 * constructs one at import time — which is why the reporter is passed into
 * `installGlobalHandlers` rather than imported by it.
 */

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { z } from "zod";
import { installGlobalHandlers, reportToConsole } from "@/diagnostics/handlers";
import { fixtureTransport } from "@/ipc/fixture-client";
import { THERMAL_CONTROL } from "@/ipc/generated/thermal-control";
import { createModelQueries, type ModelQueries } from "@/ipc/model-queries";
import { insideTauri, tauriTransport } from "@/ipc/tauri-client";
import { Shell } from "@/shell/Shell";

// 1. Configure Zod for the Tauri CSP, before anything can parse (§4.3 rule 6).
//    Zod's compiled fast path builds functions with `new Function`, and the
//    content-security policy allows no `unsafe-eval`.
z.config({ jitless: true });

// 2. Install the global failure handlers, before the first render can fail.
installGlobalHandlers(reportToConsole);

// 3. Create the React root and render the shell, handing it its services
//    (§2.2). Inside a Tauri window the Rust core answers. In a plain browser
//    (`bun run dev` opened directly) there is no core, so the mockup's sample
//    model answers instead, and `provenance` makes the status bar say so.
const queries: ModelQueries = insideTauri()
  ? createModelQueries(tauriTransport(), "backend")
  : createModelQueries(fixtureTransport(THERMAL_CONTROL), "fixture");

const services = { queries, report: reportToConsole } as const;

const container = document.getElementById("root");
if (container === null) {
  throw new Error("index.html has no #root element to mount into");
}

createRoot(container).render(
  <StrictMode>
    <Shell services={services} />
  </StrictMode>,
);
