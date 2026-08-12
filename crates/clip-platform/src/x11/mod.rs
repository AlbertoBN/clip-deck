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

    /// The current selection's raw content for a non-text target (e.g.
    /// `"text/html"`, `"image/png"`), or `None` if that target isn't
    /// offered/unowned.
    fn read_selection_target(&self, mime: &str) -> Option<Vec<u8>>;

    /// Takes ownership of the selection and sets its text content.
    fn write_selection(&self, content: &str);

    /// Takes ownership of the selection and sets its content for a specific
    /// non-text target (e.g. `"image/png"`), so another client requesting
    /// that exact target via `ConvertSelection` receives `bytes` back.
    fn write_selection_target(&self, mime: &str, bytes: &[u8]);

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
        WriteSelectionTarget(String, Vec<u8>),
        SynthesizeKey(WindowId, String),
    }

    #[derive(Default)]
    struct State {
        selection: Option<String>,
        selection_targets: HashMap<String, Vec<u8>>,
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

        /// Test helper: configure the bytes offered for a non-text selection
        /// target (e.g. `"text/html"`, `"image/png"`) for the next read.
        pub(crate) fn set_selection_target(&self, mime: &str, bytes: Vec<u8>) {
            self.state.lock().unwrap().selection_targets.insert(mime.to_string(), bytes);
        }

        pub(crate) fn ops_log(&self) -> Vec<RecordedOp> {
            self.state.lock().unwrap().ops_log.clone()
        }
    }

    impl X11Connection for FakeX11Connection {
        fn read_selection(&self) -> Option<String> {
            self.state.lock().unwrap().selection.clone()
        }

        fn read_selection_target(&self, mime: &str) -> Option<Vec<u8>> {
            self.state.lock().unwrap().selection_targets.get(mime).cloned()
        }

        fn write_selection(&self, content: &str) {
            let mut state = self.state.lock().unwrap();
            state.selection = Some(content.to_string());
            state.ops_log.push(RecordedOp::WriteSelection(content.to_string()));
        }

        fn write_selection_target(&self, mime: &str, bytes: &[u8]) {
            let mut state = self.state.lock().unwrap();
            state.selection_targets.insert(mime.to_string(), bytes.to_vec());
            state.ops_log.push(RecordedOp::WriteSelectionTarget(mime.to_string(), bytes.to_vec()));
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
use std::path::PathBuf;
use std::sync::Mutex;

/// Bound (on the long edge) for generated image thumbnails, per
/// `thumbnail-generation`'s spec.
const MAX_THUMBNAIL_DIM: u32 = 256;

/// X11 implementation of the clipboard read/write/watch surface, generic over
/// any `X11Connection` (the real `x11rb`-backed one or, in tests, the fake).
pub struct X11Backend<C: X11Connection> {
    conn: C,
    /// Directory image blobs (and a `thumbnails/` subdirectory) are written
    /// to; created on demand.
    blob_dir: PathBuf,
    last_hash: Mutex<Option<String>>,
}

impl<C: X11Connection> X11Backend<C> {
    pub fn new(conn: C, blob_dir: impl Into<PathBuf>) -> Self {
        Self { conn, blob_dir: blob_dir.into(), last_hash: Mutex::new(None) }
    }

    pub fn read_current(&self) -> Result<ClipboardSnapshot, PlatformError> {
        let mut representations = Vec::new();
        let mut ordinal = 0i64;

        if let Some(text) = self.conn.read_selection() {
            if !text.is_empty() {
                representations.push(
                    ClipRepresentation::new("text/plain", ordinal)
                        .with_text_value(text.clone())
                        .with_byte_size(text.len() as u64),
                );
                ordinal += 1;
            }
        }

        if let Some(html_bytes) = self.conn.read_selection_target("text/html") {
            if !html_bytes.is_empty() {
                let html = String::from_utf8_lossy(&html_bytes).into_owned();
                representations.push(
                    ClipRepresentation::new("text/html", ordinal)
                        .with_byte_size(html.len() as u64)
                        .with_text_value(html),
                );
                ordinal += 1;
            }
        }

        if let Some(image_bytes) = self.conn.read_selection_target("image/png") {
            if let Ok(decoded) = image::load_from_memory(&image_bytes) {
                let hash = clip_core::hashing::hash_content(&image_bytes);
                if let Some(blob_path) = self.write_blob(&hash, &image_bytes) {
                    let mut repr = ClipRepresentation::new("image/png", ordinal)
                        .with_byte_size(image_bytes.len() as u64);
                    repr.blob_path = Some(blob_path);
                    repr.width = Some(decoded.width());
                    repr.height = Some(decoded.height());
                    representations.push(repr);
                    ordinal += 1;

                    if let Some((thumb_path, thumb_width, thumb_height, thumb_size)) =
                        self.write_thumbnail(&hash, &decoded)
                    {
                        let mut thumb =
                            ClipRepresentation::new("image/png", ordinal).with_byte_size(thumb_size);
                        thumb.blob_path = Some(thumb_path);
                        thumb.width = Some(thumb_width);
                        thumb.height = Some(thumb_height);
                        thumb.is_preview = true;
                        representations.push(thumb);
                    }
                }
            }
            // An undecodable target (or a blob-write failure) is skipped
            // silently; other representations already collected are kept.
        }

        if representations.is_empty() {
            Ok(ClipboardSnapshot::empty())
        } else {
            Ok(ClipboardSnapshot { representations })
        }
    }

    /// Writes `bytes` (already-encoded PNG data) to `<blob_dir>/<hash>.png`,
    /// creating `blob_dir` if needed. Returns the path as a string, or `None`
    /// if either step fails.
    fn write_blob(&self, hash: &str, bytes: &[u8]) -> Option<String> {
        std::fs::create_dir_all(&self.blob_dir).ok()?;
        let path = self.blob_dir.join(format!("{hash}.png"));
        std::fs::write(&path, bytes).ok()?;
        Some(path.to_string_lossy().into_owned())
    }

    /// Generates a bounded-size thumbnail (preserving aspect ratio, per
    /// `thumbnail-generation`'s spec) and writes it to
    /// `<blob_dir>/thumbnails/<hash>.png`. Returns its path, dimensions, and
    /// byte size, or `None` if directory creation, encoding, or the write
    /// fails - a failure here must not block persisting the full image.
    fn write_thumbnail(&self, hash: &str, image: &image::DynamicImage) -> Option<(String, u32, u32, u64)> {
        // `imageops::thumbnail` resamples to the *exact* box given, so the
        // target box itself must already respect the source's aspect ratio
        // (bounded to `MAX_THUMBNAIL_DIM` on the long edge) rather than
        // passing a fixed square and relying on the function to preserve it.
        let (width, height) = (image.width().max(1), image.height().max(1));
        let scale = (MAX_THUMBNAIL_DIM as f64 / width.max(height) as f64).min(1.0);
        let target_width = ((width as f64 * scale).round() as u32).max(1);
        let target_height = ((height as f64 * scale).round() as u32).max(1);
        let thumb = image::imageops::thumbnail(image, target_width, target_height);
        let thumb_dir = self.blob_dir.join("thumbnails");
        std::fs::create_dir_all(&thumb_dir).ok()?;
        let path = thumb_dir.join(format!("{hash}.png"));
        thumb.save(&path).ok()?;
        let byte_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        Some((path.to_string_lossy().into_owned(), thumb.width(), thumb.height(), byte_size))
    }

    pub fn set_current(&self, content: &str) -> Result<(), PlatformError> {
        self.conn.write_selection(content);
        Ok(())
    }

    /// Computes a stable identity hash for a snapshot's non-preview
    /// representations, used by `start` to decide whether content actually
    /// changed. Prefers the first representation's `text_value`; falls back
    /// to its (already content-addressed) `blob_path` for representations
    /// without text (e.g. images), so an image-only copy is still detected
    /// as new content rather than comparing `None == None`.
    fn identity_hash(snapshot: &ClipboardSnapshot) -> Option<String> {
        let first = snapshot.representations.iter().find(|r| !r.is_preview)?;
        let identity = first.text_value.as_deref().or(first.blob_path.as_deref())?;
        Some(clip_core::hashing::hash_content(identity.as_bytes()))
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
            let hash = Self::identity_hash(&snapshot);

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

    /// A fresh temp dir alongside a backend pointed at it, so image blob
    /// writes have somewhere to go without polluting a shared location. The
    /// `TempDir` guard must stay alive for the test's duration (it deletes
    /// the directory on drop).
    fn test_backend(conn: FakeX11Connection) -> (tempfile::TempDir, X11Backend<FakeX11Connection>) {
        let dir = tempfile::tempdir().unwrap();
        let backend = X11Backend::new(conn, dir.path());
        (dir, backend)
    }

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
        let (_tmp, backend) = test_backend(conn);
        let snapshot = backend.read_current().unwrap();
        assert_eq!(snapshot.representations.len(), 1);
        assert_eq!(snapshot.representations[0].text_value.as_deref(), Some("ssh user@host"));
    }

    #[test]
    fn reading_rich_text_returns_both_plain_text_and_html_representations() {
        let conn = FakeX11Connection::new();
        conn.write_selection("ssh user@host");
        conn.set_selection_target("text/html", b"<b>ssh user@host</b>".to_vec());
        let (_tmp, backend) = test_backend(conn);

        let snapshot = backend.read_current().unwrap();

        assert_eq!(snapshot.representations.len(), 2);
        assert_eq!(snapshot.representations[0].mime_type, "text/plain");
        assert_eq!(snapshot.representations[0].text_value.as_deref(), Some("ssh user@host"));
        assert_eq!(snapshot.representations[1].mime_type, "text/html");
        assert_eq!(snapshot.representations[1].text_value.as_deref(), Some("<b>ssh user@host</b>"));
    }

    fn encode_png(width: u32, height: u32) -> Vec<u8> {
        let image = image::DynamicImage::ImageRgb8(image::RgbImage::new(width, height));
        let mut bytes = Vec::new();
        image.write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png).unwrap();
        bytes
    }

    #[test]
    fn reading_an_image_target_produces_an_image_representation() {
        let conn = FakeX11Connection::new();
        conn.set_selection_target("image/png", encode_png(200, 100));
        let (_tmp, backend) = test_backend(conn);

        let snapshot = backend.read_current().unwrap();

        let full = snapshot.representations.iter().find(|r| !r.is_preview).unwrap();
        assert_eq!(full.mime_type, "image/png");
    }

    #[test]
    fn a_captured_image_representation_reports_its_dimensions() {
        let conn = FakeX11Connection::new();
        conn.set_selection_target("image/png", encode_png(200, 100));
        let (_tmp, backend) = test_backend(conn);

        let snapshot = backend.read_current().unwrap();

        let full = snapshot.representations.iter().find(|r| !r.is_preview).unwrap();
        assert_eq!(full.width, Some(200));
        assert_eq!(full.height, Some(100));
    }

    #[test]
    fn capturing_a_large_image_produces_a_bounded_thumbnail() {
        let conn = FakeX11Connection::new();
        conn.set_selection_target("image/png", encode_png(4000, 3000));
        let (_tmp, backend) = test_backend(conn);

        let snapshot = backend.read_current().unwrap();

        let thumb = snapshot.representations.iter().find(|r| r.is_preview).unwrap();
        assert!(thumb.width.unwrap() <= 256);
        assert!(thumb.height.unwrap() <= 256);
    }

    #[test]
    fn a_non_square_thumbnail_preserves_the_original_aspect_ratio() {
        let conn = FakeX11Connection::new();
        conn.set_selection_target("image/png", encode_png(4000, 2000));
        let (_tmp, backend) = test_backend(conn);

        let snapshot = backend.read_current().unwrap();

        let thumb = snapshot.representations.iter().find(|r| r.is_preview).unwrap();
        let ratio = thumb.width.unwrap() as f64 / thumb.height.unwrap() as f64;
        assert!((ratio - 2.0).abs() < 0.05, "expected ~2:1 ratio, got {ratio}");
    }

    #[test]
    fn a_thumbnail_write_failure_still_persists_the_full_image_representation() {
        let conn = FakeX11Connection::new();
        conn.set_selection_target("image/png", encode_png(200, 100));
        let dir = tempfile::tempdir().unwrap();
        // Pre-create a regular file where the `thumbnails/` subdirectory
        // needs to go, so `create_dir_all` fails - a realistic, uncontrived
        // way to force the thumbnailing step to fail without needing a
        // separate injectable thumbnailer seam.
        std::fs::write(dir.path().join("thumbnails"), b"not a directory").unwrap();
        let backend = X11Backend::new(conn, dir.path());

        let snapshot = backend.read_current().unwrap();

        assert!(snapshot.representations.iter().any(|r| !r.is_preview && r.mime_type == "image/png"));
        assert!(!snapshot.representations.iter().any(|r| r.is_preview));
    }

    #[test]
    fn copying_a_new_image_emits_an_event_even_with_no_text_representation() {
        let conn = FakeX11Connection::new();
        conn.set_selection_target("image/png", encode_png(10, 10));
        conn.queue_selection_change();
        let (_tmp, backend) = test_backend(conn);

        let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured_clone = captured.clone();
        backend
            .start(Box::new(move |snapshot| captured_clone.lock().unwrap().push(snapshot)))
            .unwrap();

        assert_eq!(captured.lock().unwrap().len(), 1);
    }

    #[test]
    fn an_undecodable_image_target_is_skipped_but_plain_text_is_still_returned() {
        let conn = FakeX11Connection::new();
        conn.write_selection("ssh user@host");
        conn.set_selection_target("image/png", b"not a real png".to_vec());
        let (_tmp, backend) = test_backend(conn);

        let snapshot = backend.read_current().unwrap();

        assert_eq!(snapshot.representations.len(), 1);
        assert_eq!(snapshot.representations[0].mime_type, "text/plain");
        assert_eq!(snapshot.representations[0].text_value.as_deref(), Some("ssh user@host"));
    }

    #[test]
    fn reading_an_empty_clipboard_returns_an_empty_snapshot() {
        let conn = FakeX11Connection::new();
        let (_tmp, backend) = test_backend(conn);
        let snapshot = backend.read_current().unwrap();
        assert!(snapshot.is_empty());
    }

    #[test]
    fn written_content_is_observable_on_next_read() {
        let conn = FakeX11Connection::new();
        let (_tmp, backend) = test_backend(conn);
        backend.set_current("paste me").unwrap();
        let snapshot = backend.read_current().unwrap();
        assert_eq!(snapshot.representations[0].text_value.as_deref(), Some("paste me"));
    }

    #[test]
    fn copying_new_content_emits_one_event() {
        let conn = FakeX11Connection::new();
        conn.write_selection("first copy");
        conn.queue_selection_change();
        let (_tmp, backend) = test_backend(conn);

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
        let (_tmp, backend) = test_backend(conn);

        let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured_clone = captured.clone();
        backend
            .start(Box::new(move |snapshot| captured_clone.lock().unwrap().push(snapshot)))
            .unwrap();

        assert_eq!(captured.lock().unwrap().len(), 1);
    }

    #[test]
    fn x11_capabilities_report_every_baseline_flag_supported() {
        let (_tmp, backend) = test_backend(FakeX11Connection::new());
        let caps = backend.capabilities();
        assert!(caps.capture);
        assert!(caps.paste_simulation);
        assert!(caps.hotkeys);
        assert!(caps.focus_detection);
    }
}
