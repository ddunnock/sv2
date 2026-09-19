// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! Tauri's build step: reads `tauri.conf.json` and `capabilities/`.

fn main() {
    tauri_build::build();
}
