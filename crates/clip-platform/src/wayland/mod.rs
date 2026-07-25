//! Wayland adapter and capability detection. Deferred to its own milestone
//! due to inconsistent compositor support for automation; supports clipboard
//! text read/write/watch via the `wlr-data-control` protocol where the
//! compositor offers it, and reports hotkeys/focus-detection as unsupported
//! (see `crate::hotkeys::UnsupportedHotkeyBackend`,
//! `crate::focus::UnsupportedFocusTracker`) rather than assuming X11-level
//! parity.

mod real;
pub use real::RealWaylandConnection;

/// Narrow internal seam over the Wayland data-control protocol: selection
/// read/write and change notification, plus a one-time capability check.
/// Implemented once for real via `wayland-client`/`wayland-protocols-wlr`,
/// and once as an in-memory fake for unit tests.
pub trait WaylandConnection: Send + Sync {
    /// Whether this compositor advertises the `zwlr_data_control_manager_v1`
    /// global at all. `false` means construction must fail clearly rather
    /// than produce a backend that will never observe clipboard changes.
    fn supports_data_control(&self) -> bool;

    /// The current selection's text content, or `None` if unowned/empty.
    fn read_selection(&self) -> Option<String>;

    /// Takes ownership of the selection and sets its text content.
    fn write_selection(&self, content: &str);

    /// Drains one pending selection-change notification, if any are queued.
    /// Callers re-read via `read_selection` to see the resulting content.
    fn poll_selection_change(&self) -> Option<()>;
}

/// In-memory fake `WaylandConnection` for unit tests. Not backed by any real
/// compositor.
#[cfg(test)]
pub(crate) mod fake {
    use super::WaylandConnection;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    struct State {
        selection: Option<String>,
        pending_changes: VecDeque<()>,
        data_control_supported: bool,
    }

    impl Default for State {
        fn default() -> Self {
            Self { selection: None, pending_changes: VecDeque::new(), data_control_supported: true }
        }
    }

    #[derive(Default)]
    pub(crate) struct FakeWaylandConnection {
        state: Mutex<State>,
    }

    impl FakeWaylandConnection {
        pub(crate) fn new() -> Self {
            Self::default()
        }

        /// Test helper: queue a "selection changed" notification.
        pub(crate) fn queue_selection_change(&self) {
            self.state.lock().unwrap().pending_changes.push_back(());
        }

        /// Test helper: configure whether this fake reports data-control
        /// support (default `true`).
        pub(crate) fn set_data_control_supported(&self, supported: bool) {
            self.state.lock().unwrap().data_control_supported = supported;
        }
    }

    impl WaylandConnection for FakeWaylandConnection {
        fn supports_data_control(&self) -> bool {
            self.state.lock().unwrap().data_control_supported
        }

        fn read_selection(&self) -> Option<String> {
            self.state.lock().unwrap().selection.clone()
        }

        fn write_selection(&self, content: &str) {
            self.state.lock().unwrap().selection = Some(content.to_string());
        }

        fn poll_selection_change(&self) -> Option<()> {
            self.state.lock().unwrap().pending_changes.pop_front()
        }
    }
}

use crate::clipboard::{BackendCapabilities, ClipboardSnapshot, PlatformError};
use clip_core::models::ClipRepresentation;
use std::sync::Mutex;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum WaylandError {
    #[error("this compositor does not support the wlr-data-control protocol")]
    DataControlUnsupported,
}

/// Wayland implementation of the clipboard read/write/watch surface, generic
/// over any `WaylandConnection` (the real `wayland-client`-backed one or, in
/// tests, the fake). Text-only, matching `wayland-clipboard-capture`'s spec;
/// unlike the X11 adapter this does not yet capture HTML/image
/// representations.
pub struct WaylandBackend<C: WaylandConnection> {
    conn: C,
    last_hash: Mutex<Option<String>>,
}

impl<C: WaylandConnection> WaylandBackend<C> {
    /// Fails clearly (rather than constructing a backend that will never
    /// observe changes) when the compositor doesn't advertise data-control.
    pub fn new(conn: C) -> Result<Self, WaylandError> {
        if !conn.supports_data_control() {
            return Err(WaylandError::DataControlUnsupported);
        }
        Ok(Self { conn, last_hash: Mutex::new(None) })
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
    /// value - matching the X11 adapter's dedup-by-hash approach.
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

    /// Wayland's honest capability set: capture/paste work via data-control,
    /// but hotkeys and focus-detection are categorically unsupported in this
    /// version (no portal-based global-shortcut integration; Wayland's
    /// security model withholds focused-window info from clients) - see
    /// `UnsupportedHotkeyBackend`/`UnsupportedFocusTracker`.
    pub fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities { capture: true, paste_simulation: true, hotkeys: false, focus_detection: false }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::FakeWaylandConnection;
    use super::{WaylandBackend, WaylandConnection, WaylandError};

    #[test]
    fn fake_connection_satisfies_the_wayland_connection_trait() {
        let conn: Box<dyn WaylandConnection> = Box::new(FakeWaylandConnection::new());
        assert_eq!(conn.read_selection(), None);
        assert!(conn.supports_data_control());
    }

    #[test]
    fn reading_a_populated_clipboard_returns_its_text() {
        let conn = FakeWaylandConnection::new();
        conn.write_selection("ssh user@host");
        let backend = WaylandBackend::new(conn).unwrap();

        let snapshot = backend.read_current().unwrap();

        assert_eq!(snapshot.representations[0].text_value.as_deref(), Some("ssh user@host"));
    }

    #[test]
    fn written_content_is_observable_on_next_read() {
        let conn = FakeWaylandConnection::new();
        let backend = WaylandBackend::new(conn).unwrap();

        backend.set_current("paste me").unwrap();

        let snapshot = backend.read_current().unwrap();
        assert_eq!(snapshot.representations[0].text_value.as_deref(), Some("paste me"));
    }

    #[test]
    fn copying_new_content_emits_one_event() {
        let conn = FakeWaylandConnection::new();
        conn.write_selection("first copy");
        conn.queue_selection_change();
        let backend = WaylandBackend::new(conn).unwrap();

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
        let conn = FakeWaylandConnection::new();
        conn.write_selection("same text");
        conn.queue_selection_change();
        conn.queue_selection_change();
        let backend = WaylandBackend::new(conn).unwrap();

        let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured_clone = captured.clone();
        backend
            .start(Box::new(move |snapshot| captured_clone.lock().unwrap().push(snapshot)))
            .unwrap();

        assert_eq!(captured.lock().unwrap().len(), 1);
    }

    #[test]
    fn construction_fails_clearly_when_data_control_is_unsupported() {
        let conn = FakeWaylandConnection::new();
        conn.set_data_control_supported(false);

        let result = WaylandBackend::new(conn);

        assert_eq!(result.err(), Some(WaylandError::DataControlUnsupported));
    }
}
