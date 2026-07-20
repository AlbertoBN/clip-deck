//! Clipboard event loop: forwards backend capture events to ingest.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::app::{Backend, EventPublisher, Store};
use clip_platform::clipboard::{ClipboardSnapshot, PlatformError};

/// Consumes `Backend` capture events and forwards them to `ingest`, one at a
/// time, in the order received. Tracks a paused flag consumed by
/// `PauseCapture`.
#[derive(Default)]
pub struct WatchLoop {
    paused: Arc<AtomicBool>,
}

impl WatchLoop {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::SeqCst);
    }

    #[cfg(test)]
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    /// Registers the capture callback with `backend`. Each capture event is
    /// forwarded to `ingest` unless capture is currently paused; an ingest
    /// failure is logged and does not stop the loop.
    pub fn start(
        &self,
        backend: Arc<dyn Backend>,
        store: Arc<dyn Store>,
        events: Arc<dyn EventPublisher>,
    ) -> Result<(), PlatformError> {
        let paused = self.paused.clone();
        let backend_for_focus = backend.clone();
        backend.start(Box::new(move |snapshot: ClipboardSnapshot| {
            if paused.load(Ordering::SeqCst) {
                return;
            }
            let source = backend_for_focus.focused_app();
            if let Err(err) = crate::ingest::ingest(store.as_ref(), events.as_ref(), snapshot, source) {
                tracing::warn!(error = %err, "ingest failed for a captured clipboard change");
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::WatchLoop;
    use crate::app::fakes::{FakeBackend, FakeEventPublisher, FakeStore};
    use crate::app::Store;
    use clip_core::models::ClipRepresentation;
    use clip_platform::clipboard::ClipboardSnapshot;
    use std::sync::Arc;

    fn snapshot_with(mime: &str, text: &str) -> ClipboardSnapshot {
        ClipboardSnapshot {
            representations: vec![ClipRepresentation::new(mime, 0).with_text_value(text).with_byte_size(text.len() as u64)],
        }
    }

    #[test]
    fn two_sequential_capture_events_are_both_forwarded_in_order() {
        let backend = Arc::new(FakeBackend::new());
        let store = Arc::new(FakeStore::new());
        let events = Arc::new(FakeEventPublisher::new());
        let watch_loop = WatchLoop::new();
        watch_loop.start(backend.clone(), store.clone(), events.clone()).unwrap();

        backend.emit_capture(snapshot_with("text/plain", "first"));
        backend.emit_capture(snapshot_with("text/plain", "second"));

        let clips = store.search("", &Default::default()).unwrap();
        assert_eq!(clips.len(), 2);
        let mut texts: Vec<_> = clips.iter().map(|c| c.display_text.clone().unwrap()).collect();
        texts.sort();
        assert_eq!(texts, vec!["first".to_string(), "second".to_string()]);
    }

    #[test]
    fn watch_loop_survives_one_ingest_failure() {
        let backend = Arc::new(FakeBackend::new());
        let store = Arc::new(FakeStore::new());
        let events = Arc::new(FakeEventPublisher::new());
        let watch_loop = WatchLoop::new();
        watch_loop.start(backend.clone(), store.clone(), events.clone()).unwrap();

        // An invalid MIME type makes the first ingest fail.
        backend.emit_capture(snapshot_with("not-a-mime-type", "bad"));
        backend.emit_capture(snapshot_with("text/plain", "good"));

        let clips = store.search("", &Default::default()).unwrap();
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].display_text.as_deref(), Some("good"));
    }

    #[test]
    fn capture_events_are_ignored_while_paused() {
        let backend = Arc::new(FakeBackend::new());
        let store = Arc::new(FakeStore::new());
        let events = Arc::new(FakeEventPublisher::new());
        let watch_loop = WatchLoop::new();
        watch_loop.start(backend.clone(), store.clone(), events.clone()).unwrap();

        watch_loop.set_paused(true);
        backend.emit_capture(snapshot_with("text/plain", "while paused"));

        assert!(store.search("", &Default::default()).unwrap().is_empty());
    }

    #[test]
    fn capture_resumes_forwarding_once_unpaused() {
        let backend = Arc::new(FakeBackend::new());
        let store = Arc::new(FakeStore::new());
        let events = Arc::new(FakeEventPublisher::new());
        let watch_loop = WatchLoop::new();
        watch_loop.start(backend.clone(), store.clone(), events.clone()).unwrap();

        watch_loop.set_paused(true);
        backend.emit_capture(snapshot_with("text/plain", "while paused"));
        watch_loop.set_paused(false);
        backend.emit_capture(snapshot_with("text/plain", "after unpause"));

        let clips = store.search("", &Default::default()).unwrap();
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].display_text.as_deref(), Some("after unpause"));
    }
}
