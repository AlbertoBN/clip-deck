//! X11 clipboard and automation adapter. First fully-supported backend;
//! baseline for end-to-end tests.

mod real;
pub use real::RealX11Connection;

/// X11 window identifier (an `xproto::Window`, kept as a plain `u32` here so
/// this module doesn't leak `x11rb` types into callers that only need to
/// compare/store an id).
pub type WindowId = u32;

/// Narrow internal seam over the raw X11 protocol: selection read/write,
/// selection-change notification, window property lookup, and synthetic key
/// delivery. Implemented once for real via `x11rb` and once as an in-memory
/// fake for unit tests (see `fake` below).
pub trait X11Connection: Send + Sync {
    /// The current selection's text content, or `None` if unowned/empty.
    fn read_selection(&self) -> Option<String>;

    /// Takes ownership of the selection and sets its text content.
    fn write_selection(&self, content: &str);

    /// Drains one pending selection-change notification, if any are queued.
    /// Callers re-read via `read_selection` to see the resulting content.
    fn poll_selection_change(&self) -> Option<()>;

    /// Reads a window property (e.g. `WM_CLASS`, `_NET_WM_NAME`) as a string.
    fn window_property(&self, window: WindowId, name: &str) -> Option<String>;

    /// The currently focused window, or `None` if focus is on the desktop/root.
    fn focused_window(&self) -> Option<WindowId>;

    /// Synthesizes a key combination (e.g. `"ctrl+v"`) targeted at `window`.
    fn synthesize_key(&self, window: WindowId, binding: &str) -> Result<(), String>;
}

/// In-memory fake `X11Connection` for unit tests (sections 3-5). Not backed
/// by any real X server.
#[cfg(test)]
pub(crate) mod fake {
    use super::{WindowId, X11Connection};
    use std::collections::{HashMap, VecDeque};
    use std::sync::Mutex;

    #[derive(Debug, Clone, PartialEq)]
    pub(crate) enum RecordedOp {
        WriteSelection(String),
        SynthesizeKey(WindowId, String),
    }

    #[derive(Default)]
    struct State {
        selection: Option<String>,
        pending_changes: VecDeque<()>,
        window_properties: HashMap<(WindowId, String), String>,
        focused_window: Option<WindowId>,
        ops_log: Vec<RecordedOp>,
    }

    #[derive(Default)]
    pub(crate) struct FakeX11Connection {
        state: Mutex<State>,
    }

    impl FakeX11Connection {
        pub(crate) fn new() -> Self {
            Self::default()
        }

        /// Test helper: queue a "selection changed" notification without
        /// necessarily changing the underlying content (simulates an
        /// owner-churn event with unchanged content).
        pub(crate) fn queue_selection_change(&self) {
            self.state.lock().unwrap().pending_changes.push_back(());
        }

        pub(crate) fn set_window_property(&self, window: WindowId, name: &str, value: &str) {
            self.state
                .lock()
                .unwrap()
                .window_properties
                .insert((window, name.to_string()), value.to_string());
        }

        pub(crate) fn set_focused_window(&self, window: Option<WindowId>) {
            self.state.lock().unwrap().focused_window = window;
        }

        pub(crate) fn ops_log(&self) -> Vec<RecordedOp> {
            self.state.lock().unwrap().ops_log.clone()
        }
    }

    impl X11Connection for FakeX11Connection {
        fn read_selection(&self) -> Option<String> {
            self.state.lock().unwrap().selection.clone()
        }

        fn write_selection(&self, content: &str) {
            let mut state = self.state.lock().unwrap();
            state.selection = Some(content.to_string());
            state.ops_log.push(RecordedOp::WriteSelection(content.to_string()));
        }

        fn poll_selection_change(&self) -> Option<()> {
            self.state.lock().unwrap().pending_changes.pop_front()
        }

        fn window_property(&self, window: WindowId, name: &str) -> Option<String> {
            self.state.lock().unwrap().window_properties.get(&(window, name.to_string())).cloned()
        }

        fn focused_window(&self) -> Option<WindowId> {
            self.state.lock().unwrap().focused_window
        }

        fn synthesize_key(&self, window: WindowId, binding: &str) -> Result<(), String> {
            self.state
                .lock()
                .unwrap()
                .ops_log
                .push(RecordedOp::SynthesizeKey(window, binding.to_string()));
            Ok(())
        }
    }
}

use crate::clipboard::{BackendCapabilities, ClipboardSnapshot, PlatformError};
use clip_core::models::ClipRepresentation;
use std::sync::Mutex;

/// X11 implementation of the clipboard read/write/watch surface, generic over
/// any `X11Connection` (the real `x11rb`-backed one or, in tests, the fake).
pub struct X11Backend<C: X11Connection> {
    conn: C,
    last_hash: Mutex<Option<String>>,
}

impl<C: X11Connection> X11Backend<C> {
    pub fn new(conn: C) -> Self {
        Self { conn, last_hash: Mutex::new(None) }
    }

    pub fn read_current(&self) -> Result<ClipboardSnapshot, PlatformError> {
        match self.conn.read_selection() {
            Some(text) if !text.is_empty() => Ok(ClipboardSnapshot {
                representations: vec![ClipRepresentation::new("text/plain", 0)
                    .with_text_value(text.clone())
                    .with_byte_size(text.len() as u64)],
            }),
            _ => Ok(ClipboardSnapshot::empty()),
        }
    }

    pub fn set_current(&self, content: &str) -> Result<(), PlatformError> {
        self.conn.write_selection(content);
        Ok(())
    }

    /// Drains pending selection-change notifications, emitting `on_capture`
    /// only when the content hash actually changed since the last observed
    /// value. Returns once the connection has no more pending notifications
    /// (the fake's queue is finite; a real, blocking `X11Connection` makes
    /// this loop indefinitely, which is the intended "start a watch loop"
    /// behavior for the daemon).
    pub fn start(
        &self,
        on_capture: Box<dyn Fn(ClipboardSnapshot) + Send + Sync>,
    ) -> Result<(), PlatformError> {
        while self.conn.poll_selection_change().is_some() {
            let snapshot = self.read_current()?;
            let hash = snapshot
                .representations
                .first()
                .and_then(|r| r.text_value.as_deref())
                .map(|text| clip_core::hashing::hash_content(text.as_bytes()));

            let mut last = self.last_hash.lock().unwrap();
            if *last != hash {
                *last = hash;
                drop(last);
                on_capture(snapshot);
            }
        }
        Ok(())
    }

    pub fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities { capture: true, paste_simulation: true, hotkeys: true, focus_detection: true }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::FakeX11Connection;
    use super::{X11Backend, X11Connection};

    #[test]
    fn fake_connection_satisfies_the_x11_connection_trait() {
        let conn: Box<dyn X11Connection> = Box::new(FakeX11Connection::new());
        assert_eq!(conn.read_selection(), None);
        assert_eq!(conn.focused_window(), None);
    }

    #[test]
    fn reading_a_populated_clipboard_returns_its_text() {
        let conn = FakeX11Connection::new();
        conn.write_selection("ssh user@host");
        let backend = X11Backend::new(conn);
        let snapshot = backend.read_current().unwrap();
        assert_eq!(snapshot.representations.len(), 1);
        assert_eq!(snapshot.representations[0].text_value.as_deref(), Some("ssh user@host"));
    }

    #[test]
    fn reading_an_empty_clipboard_returns_an_empty_snapshot() {
        let conn = FakeX11Connection::new();
        let backend = X11Backend::new(conn);
        let snapshot = backend.read_current().unwrap();
        assert!(snapshot.is_empty());
    }

    #[test]
    fn written_content_is_observable_on_next_read() {
        let conn = FakeX11Connection::new();
        let backend = X11Backend::new(conn);
        backend.set_current("paste me").unwrap();
        let snapshot = backend.read_current().unwrap();
        assert_eq!(snapshot.representations[0].text_value.as_deref(), Some("paste me"));
    }

    #[test]
    fn copying_new_content_emits_one_event() {
        let conn = FakeX11Connection::new();
        conn.write_selection("first copy");
        conn.queue_selection_change();
        let backend = X11Backend::new(conn);

        let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured_clone = captured.clone();
        backend
            .start(Box::new(move |snapshot| captured_clone.lock().unwrap().push(snapshot)))
            .unwrap();

        let captured = captured.lock().unwrap();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].representations[0].text_value.as_deref(), Some("first copy"));
    }

    #[test]
    fn an_unchanged_content_notification_does_not_emit_a_duplicate_event() {
        let conn = FakeX11Connection::new();
        conn.write_selection("same text");
        conn.queue_selection_change();
        conn.queue_selection_change();
        let backend = X11Backend::new(conn);

        let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured_clone = captured.clone();
        backend
            .start(Box::new(move |snapshot| captured_clone.lock().unwrap().push(snapshot)))
            .unwrap();

        assert_eq!(captured.lock().unwrap().len(), 1);
    }

    #[test]
    fn x11_capabilities_report_every_baseline_flag_supported() {
        let backend = X11Backend::new(FakeX11Connection::new());
        let caps = backend.capabilities();
        assert!(caps.capture);
        assert!(caps.paste_simulation);
        assert!(caps.hotkeys);
        assert!(caps.focus_detection);
    }
}
