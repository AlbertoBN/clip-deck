//! Focused app/window discovery where available.

use crate::x11::X11Connection;
use clip_core::models::AppContext;
use std::sync::Mutex;

/// Extracts the application name from a `WM_CLASS` property value, which per
/// ICCCM is one or more NUL-separated components (`"instance\0class\0"`);
/// the first non-empty component (the instance name) is used as the app
/// name.
fn app_name_from_wm_class(raw: &str) -> String {
    raw.split('\0').find(|s| !s.is_empty()).unwrap_or(raw).to_string()
}

/// Discovers the focused window's `AppContext` via an `X11Connection`, and
/// retains a snapshot captured at popup-open time so paste can still target
/// the originally focused window after the popup itself takes focus.
pub struct FocusTracker<C: X11Connection> {
    conn: C,
    retained: Mutex<Option<AppContext>>,
}

impl<C: X11Connection> FocusTracker<C> {
    pub fn new(conn: C) -> Self {
        Self { conn, retained: Mutex::new(None) }
    }

    /// The currently focused window's `AppContext`, or `None` if there is no
    /// focused application window (e.g. focus is on the desktop/root).
    pub fn focused_app(&self) -> Option<AppContext> {
        let window = self.conn.focused_window()?;
        let wm_class = self.conn.window_property(window, "WM_CLASS")?;
        let app = app_name_from_wm_class(&wm_class);
        match self.conn.window_property(window, "_NET_WM_NAME") {
            Some(title) => Some(AppContext::with_window(app, title)),
            None => Some(AppContext::new(app)),
        }
    }

    /// Captures the currently focused app and retains it for later
    /// `retained()` calls, so it stays available after focus subsequently
    /// moves to the popup itself.
    pub fn capture(&self) -> Option<AppContext> {
        let ctx = self.focused_app();
        *self.retained.lock().unwrap() = ctx.clone();
        ctx
    }

    /// The `AppContext` captured by the most recent `capture()` call.
    pub fn retained(&self) -> Option<AppContext> {
        self.retained.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::x11::fake::FakeX11Connection;

    #[test]
    fn a_focused_terminal_window_is_reported_with_its_app_name() {
        let conn = FakeX11Connection::new();
        conn.set_focused_window(Some(1));
        conn.set_window_property(1, "WM_CLASS", "gnome-terminal\0Gnome-terminal\0");
        conn.set_window_property(1, "_NET_WM_NAME", "user@host: ~");
        let tracker = FocusTracker::new(conn);

        let ctx = tracker.focused_app().unwrap();
        assert_eq!(ctx.app, "gnome-terminal");
        assert_eq!(ctx.window.as_deref(), Some("user@host: ~"));
    }

    #[test]
    fn desktop_focus_reports_no_application_context() {
        let conn = FakeX11Connection::new();
        conn.set_focused_window(None);
        let tracker = FocusTracker::new(conn);

        assert_eq!(tracker.focused_app(), None);
    }

    #[test]
    fn paste_targets_the_window_focused_before_the_popup_opened() {
        let conn = FakeX11Connection::new();
        conn.set_focused_window(Some(1));
        conn.set_window_property(1, "WM_CLASS", "editor");
        let tracker = FocusTracker::new(conn);

        let captured = tracker.capture().unwrap();
        assert_eq!(captured.app, "editor");

        // The popup itself subsequently takes input focus.
        tracker.conn.set_focused_window(Some(2));

        let retained = tracker.retained().unwrap();
        assert_eq!(retained.app, "editor");
    }
}
