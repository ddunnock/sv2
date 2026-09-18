// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
/**
 * The composition root, and the only module that does work at load time
 * (STD-004-TS §2.2).
 *
 * It does three things in this order and nothing else: configure Zod for the
 * Tauri content-security policy, install the global failure handlers, then
 * create the React root and render the shell. The first of the three is absent
 * until `zod` is on the allowlist and the `contract` layer exists; the comment
 * below is the placeholder, and it is deliberately not a silent omission.
 *
 * Every other module defines and does not execute (§3.3). A module that needs a
 * service receives it here, through a parameter or React context, and never
 * constructs one at import time — which is why the reporter is passed into
 * `installGlobalHandlers` rather than imported by it.
 */

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { installGlobalHandlers, reportToConsole } from "@/diagnostics/handlers.ts";
import { Shell } from "@/shell/Shell.tsx";

// 1. Configure Zod for the Tauri CSP — not yet: `zod` is not on the allowlist
//    (§3.1) and the `contract` layer does not exist. When it lands it goes here,
//    before anything can parse.

// 2. Install the global failure handlers, before the first render can fail.
installGlobalHandlers(reportToConsole);

// 3. Create the React root and render the shell.
const container = document.getElementById("root");
if (container === null) {
  throw new Error("index.html has no #root element to mount into");
}

createRoot(container).render(
  <StrictMode>
    <Shell />
  </StrictMode>,
);
