// SPDX-License-Identifier: MIT
// Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
//! The editor's own window (IX-07): moving an open file into it and back.
//!
//! THE EDITOR IS IN ONE PLACE AT A TIME. Two windows are two JavaScript
//! contexts, so a document cannot be shared between them the way two views in
//! one window share it. Instead the file *moves*: the window giving it up sends
//! an `EditorHandoff` — text, and the editor's own state with its undo history —
//! and this module keeps it in a slot addressed to the window that will take
//! it. There is never a second live copy, so there is nothing to reconcile.
//!
//! CLOSING THE EDITOR WINDOW DOCKS IT. Nothing can be saved yet, so a close
//! that dropped the file would lose edits without asking. The close is held
//! back and the editor window is asked to dock; it sends its handoff, and the
//! window goes. A second close while that is pending is let through, so a
//! window whose page has failed can still be closed.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use tauri::{
    AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder, Window,
    WindowEvent,
};

use crate::wire::{Answer, EditorHandoff};

/// The main window's label, as `tauri.conf.json` names it.
pub(crate) const MAIN: &str = "main";
/// The editor window's label. There is one editor window at most.
pub(crate) const EDITOR: &str = "editor";
/// Sent to the main window when a handoff is waiting for it.
const DOCKED: &str = "editor-docked";
/// Sent to the editor window to ask it to hand its file back.
const DOCK_REQUESTED: &str = "editor-dock-requested";

/// One waiting handoff per destination window, keyed by its label.
#[derive(Debug, Default)]
pub(crate) struct Handoffs(Mutex<HashMap<String, EditorHandoff>>);

impl Handoffs {
    /// Leave `handoff` for the window labelled `to`, replacing any not yet taken.
    fn put(&self, to: &str, handoff: EditorHandoff) {
        self.slots().insert(to.to_owned(), handoff);
    }

    /// Take what is waiting for the window labelled `to`, if anything.
    fn take(&self, to: &str) -> Option<EditorHandoff> {
        self.slots().remove(to)
    }

    fn slots(&self) -> MutexGuard<'_, HashMap<String, EditorHandoff>> {
        // A panic while the lock was held cannot leave the map half-written:
        // every operation on it is a single insert or remove. So a poisoned
        // lock is still a consistent map, and is used rather than propagated.
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Whether the editor window has been asked to dock and has not yet answered.
#[derive(Debug, Default)]
pub(crate) struct DockPending(AtomicBool);

/// `editor_undock`: from the main window. Opens the editor window on `handoff`.
///
/// Async because creating a window from a synchronous command deadlocks on
/// Windows. Calling it while the editor window is open is a defect in the
/// webview, which offers undocking only when docked, so it is refused.
#[tauri::command]
pub(crate) async fn editor_undock(
    app: AppHandle,
    handoffs: State<'_, Handoffs>,
    pending: State<'_, DockPending>,
    handoff: EditorHandoff,
) -> Result<Answer<()>, String> {
    if app.get_webview_window(EDITOR).is_some() {
        return Err("the editor window is already open".to_owned());
    }
    let title = format!("{} — sv2 Studio", handoff.path);
    handoffs.put(EDITOR, handoff);
    pending.0.store(false, Ordering::SeqCst);
    WebviewWindowBuilder::new(&app, EDITOR, WebviewUrl::default())
        .title(title)
        .inner_size(900.0, 800.0)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(Answer::Ready { data: () })
}

/// `editor_dock`: from the editor window. Hands its file to the main window and closes.
#[tauri::command]
pub(crate) async fn editor_dock(
    app: AppHandle,
    window: WebviewWindow,
    handoffs: State<'_, Handoffs>,
    handoff: EditorHandoff,
) -> Result<Answer<()>, String> {
    if window.label() != EDITOR {
        return Err("only the editor window can dock".to_owned());
    }
    handoffs.put(MAIN, handoff);
    app.emit_to(MAIN, DOCKED, ()).map_err(|e| e.to_string())?;
    // `destroy`, not `close`: a close would be held back by `on_window_event`
    // and ask this window to dock again.
    window.destroy().map_err(|e| e.to_string())?;
    Ok(Answer::Ready { data: () })
}

/// `editor_handoff`: the file waiting for the calling window, taken once.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its arguments by value"
)]
#[tauri::command]
pub(crate) fn editor_handoff(
    window: WebviewWindow,
    handoffs: State<'_, Handoffs>,
) -> Answer<EditorHandoff> {
    match handoffs.take(window.label()) {
        Some(handoff) => Answer::Ready { data: handoff },
        None => Answer::not_found(),
    }
}

/// `editor_focus`: bring the editor window forward (IX-07's Focus).
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its arguments by value"
)]
#[tauri::command]
pub(crate) fn editor_focus(app: AppHandle) -> Result<Answer<()>, String> {
    let Some(editor) = app.get_webview_window(EDITOR) else {
        return Ok(Answer::not_found());
    };
    editor.set_focus().map_err(|e| e.to_string())?;
    Ok(Answer::Ready { data: () })
}

/// `editor_request_dock`: from the main window (IX-07's Dock). Asks the editor to hand back.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri hands a command its arguments by value"
)]
#[tauri::command]
pub(crate) fn editor_request_dock(app: AppHandle) -> Result<Answer<()>, String> {
    if app.get_webview_window(EDITOR).is_none() {
        return Ok(Answer::not_found());
    }
    app.emit_to(EDITOR, DOCK_REQUESTED, ())
        .map_err(|e| e.to_string())?;
    Ok(Answer::Ready { data: () })
}

/// Window events: a close of the editor docks it; a close of the main window ends the app.
pub(crate) fn on_window_event(window: &Window, event: &WindowEvent) {
    let WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };
    match window.label() {
        EDITOR => {
            let pending = window.state::<DockPending>();
            if !pending.0.swap(true, Ordering::SeqCst)
                && window.emit_to(EDITOR, DOCK_REQUESTED, ()).is_ok()
            {
                api.prevent_close();
            }
        }
        // The editor window is the main window's; it does not outlive it.
        MAIN => {
            if let Some(editor) = window.app_handle().get_webview_window(EDITOR) {
                let _closing = editor.destroy();
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handoff(path: &str) -> EditorHandoff {
        EditorHandoff {
            path: path.to_owned(),
            text: "package P;\n".to_owned(),
            state: serde_json::json!({ "doc": "package P;\n" }),
        }
    }

    #[test]
    fn a_handoff_is_taken_by_the_window_it_was_left_for_and_only_once() {
        let handoffs = Handoffs::default();
        handoffs.put(EDITOR, handoff("a.sysml"));
        assert_eq!(handoffs.take(MAIN), None);
        assert_eq!(handoffs.take(EDITOR), Some(handoff("a.sysml")));
        assert_eq!(handoffs.take(EDITOR), None);
    }

    #[test]
    fn a_newer_handoff_to_the_same_window_replaces_the_older() {
        let handoffs = Handoffs::default();
        handoffs.put(MAIN, handoff("a.sysml"));
        handoffs.put(MAIN, handoff("b.sysml"));
        assert_eq!(
            handoffs.take(MAIN).map(|h| h.path),
            Some("b.sysml".to_owned())
        );
    }

    #[test]
    fn the_wire_shape_is_the_contracts_and_refuses_unknown_fields() {
        let json = r#"{"path":"a.sysml","text":"x","state":{"doc":"x"}}"#;
        let parsed: EditorHandoff = serde_json::from_str(json).unwrap();
        assert_eq!(serde_json::to_string(&parsed).unwrap(), json);
        let extra = r#"{"path":"a.sysml","text":"x","state":null,"saved":true}"#;
        assert!(serde_json::from_str::<EditorHandoff>(extra).is_err());
    }
}
