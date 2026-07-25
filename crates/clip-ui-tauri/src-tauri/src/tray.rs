//! Tray icon, menu, and its event handling.

use std::sync::atomic::{AtomicBool, Ordering};

use clip_ipc::protocol::{ClearScope, Command};

use crate::client::Client;

/// Tracks whether capture is currently paused, as last reported by a
/// `CapturePaused` event - drives the tray's displayed state and menu label.
#[derive(Default)]
pub struct TrayState {
    paused: AtomicBool,
}

impl TrayState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::SeqCst);
    }
}

/// The tray menu label for the pause/resume action, reflecting current state.
pub fn pause_menu_label(tray_state: &TrayState) -> &'static str {
    if tray_state.is_paused() {
        "Resume capture"
    } else {
        "Pause capture"
    }
}

/// Toggles capture pause state: issues `PauseCapture` with the opposite of
/// the tray's currently-known paused state.
pub async fn handle_pause_toggle(client: &dyn Client, tray_state: &TrayState) -> Result<(), String> {
    let new_paused = !tray_state.is_paused();
    match client.call(Command::PauseCapture { paused: new_paused }).await {
        clip_ipc::protocol::Response::Ok { .. } => Ok(()),
        clip_ipc::protocol::Response::Err { error, .. } => Err(error),
    }
}

/// Clears clip history, preserving pinned clips - the same default scope the
/// Manager's own "Clear history" button uses.
pub async fn handle_clear_history(client: &dyn Client) -> Result<(), String> {
    match client.call(Command::ClearHistory { scope: ClearScope::ExcludingPinned }).await {
        clip_ipc::protocol::Response::Ok { .. } => Ok(()),
        clip_ipc::protocol::Response::Err { error, .. } => Err(error),
    }
}

/// Issues the quit action via an injectable `exit` function, so tests can
/// assert it was invoked without actually terminating the test process.
pub fn handle_quit(exit: &dyn Fn()) {
    exit();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::fakes::FakeClient;
    use clip_ipc::protocol::Response;
    use std::sync::atomic::AtomicUsize;

    #[tokio::test]
    async fn selecting_pause_capture_issues_pause_capture_true() {
        let client = FakeClient::new();
        client.push_response(Response::ok("test", serde_json::json!({"ok": true})));
        let tray_state = TrayState::new();

        handle_pause_toggle(&client, &tray_state).await.unwrap();

        assert_eq!(client.calls(), vec![Command::PauseCapture { paused: true }]);
    }

    #[tokio::test]
    async fn selecting_resume_capture_issues_pause_capture_false() {
        let client = FakeClient::new();
        client.push_response(Response::ok("test", serde_json::json!({"ok": true})));
        let tray_state = TrayState::new();
        tray_state.set_paused(true);

        handle_pause_toggle(&client, &tray_state).await.unwrap();

        assert_eq!(client.calls(), vec![Command::PauseCapture { paused: false }]);
    }

    #[tokio::test]
    async fn selecting_clear_history_issues_clear_history_excluding_pinned() {
        let client = FakeClient::new();
        client.push_response(Response::ok("test", serde_json::json!({"ok": true})));

        handle_clear_history(&client).await.unwrap();

        assert_eq!(client.calls(), vec![Command::ClearHistory { scope: ClearScope::ExcludingPinned }]);
    }

    #[test]
    fn selecting_quit_invokes_the_exit_function() {
        let called = AtomicUsize::new(0);
        handle_quit(&|| {
            called.fetch_add(1, Ordering::SeqCst);
        });

        assert_eq!(called.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn capture_paused_event_updates_the_trays_displayed_state() {
        let tray_state = TrayState::new();
        assert!(!tray_state.is_paused());

        tray_state.set_paused(true);

        assert!(tray_state.is_paused());
    }

    #[test]
    fn menu_label_toggles_between_pause_and_resume() {
        let tray_state = TrayState::new();
        assert_eq!(pause_menu_label(&tray_state), "Pause capture");

        tray_state.set_paused(true);

        assert_eq!(pause_menu_label(&tray_state), "Resume capture");
    }
}
