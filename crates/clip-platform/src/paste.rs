//! Synthetic paste and plain-text paste.

use crate::clipboard::PlatformError;
use crate::x11::{WindowId, X11Connection};
use clip_core::models::{ClipRepresentation, PasteMode};

/// The synthesized key combination for a paste. Not yet configurable (see
/// `global-hotkey-registration` for the popup-activation binding, which is);
/// paste always uses the platform-conventional `Ctrl+V`.
const PASTE_KEY_BINDING: &str = "ctrl+v";

/// Resolves which text to place on the clipboard before pasting.
///
/// `PlainText` looks for the clip's own `text/plain` representation first,
/// discarding any richer one; `Auto`/`Rich` prefer the richest representation
/// available (`text/html` over `text/plain`). Both fall back to the first
/// representation's plain-text-rendered form (`preview_text`, then
/// `text_value`) when nothing matches the preferred mime type, matching
/// Milestone-1's single-representation behavior.
pub fn resolve_paste_text(representations: &[ClipRepresentation], mode: PasteMode) -> Option<String> {
    let first = representations.first()?;
    match mode {
        PasteMode::PlainText => representations
            .iter()
            .find(|r| r.mime_type == "text/plain")
            .and_then(|r| r.text_value.clone())
            .or_else(|| first.preview_text.clone().or_else(|| first.text_value.clone())),
        PasteMode::Auto | PasteMode::Rich => representations
            .iter()
            .find(|r| r.mime_type == "text/html")
            .and_then(|r| r.text_value.clone())
            .or_else(|| first.text_value.clone()),
    }
}

/// The resolved content a paste/copy actually places on the clipboard.
enum PasteContent {
    Text(String),
    Image { mime: String, bytes: Vec<u8> },
}

/// Like `resolve_paste_text`, but falls back to the clip's image bytes (read
/// from its `blob_path`) when there is no text representation to paste at
/// all - a clip captured as an image only has no `text_value` anywhere, so
/// `resolve_paste_text` alone would silently resolve to an empty string,
/// clobbering the clipboard with nothing instead of the image. Text is
/// always preferred when present, matching `resolve_paste_text`'s existing
/// behavior for mixed representations.
fn resolve_paste_content(representations: &[ClipRepresentation], mode: PasteMode) -> PasteContent {
    if let Some(text) = resolve_paste_text(representations, mode) {
        return PasteContent::Text(text);
    }
    let image = representations
        .iter()
        .find(|r| !r.is_preview && r.mime_type.starts_with("image/"))
        .and_then(|r| Some((r, r.blob_path.as_ref()?)))
        .and_then(|(r, blob_path)| std::fs::read(blob_path).ok().map(|bytes| (r.mime_type.clone(), bytes)));
    match image {
        Some((mime, bytes)) => PasteContent::Image { mime, bytes },
        None => PasteContent::Text(String::new()),
    }
}

/// Simulates a paste into the previously focused window: places the
/// resolved content on the clipboard, then synthesizes the paste key
/// combination targeted at that window.
pub struct PasteSimulator<C: X11Connection> {
    conn: C,
    focus_detection_supported: bool,
    key_synthesis_supported: bool,
}

impl<C: X11Connection> PasteSimulator<C> {
    pub fn new(conn: C) -> Self {
        Self { conn, focus_detection_supported: true, key_synthesis_supported: true }
    }

    /// Like `new`, but for a backend that reports focus-detection as
    /// unsupported (e.g. a Wayland compositor with no focused-window
    /// information available to clients): a missing captured window is not
    /// treated as an error, since there was never any possibility of
    /// capturing one - see `paste-simulation`'s modified spec.
    pub fn without_focus_detection(conn: C) -> Self {
        Self { conn, focus_detection_supported: false, key_synthesis_supported: true }
    }

    /// Like `new`, but for a session where the synthetic keystroke itself
    /// (`XTestFakeInput`) is gated behind a compositor permission prompt -
    /// notably GNOME/Mutter, which treats XTest key injection from an
    /// XWayland client as a security-sensitive Remote Desktop portal
    /// operation and pops up a "Share"/"Allow Remote Interaction" consent
    /// dialog for it. Focus detection still works normally (X11 selection
    /// ownership and focus queries aren't gated), but every paste places
    /// content on the clipboard only, for the user to complete with their
    /// own Ctrl+V, rather than ever attempting the synthetic keystroke that
    /// would otherwise interrupt the flow with that dialog.
    pub fn clipboard_only(conn: C) -> Self {
        Self { conn, focus_detection_supported: true, key_synthesis_supported: false }
    }

    /// Writes `content` to the connection via whichever primitive matches
    /// its kind - `write_selection` for text, `write_selection_target` for
    /// an image's raw bytes under its own mime type.
    fn write_content(&self, content: &PasteContent) {
        match content {
            PasteContent::Text(text) => self.conn.write_selection(text),
            PasteContent::Image { mime, bytes } => self.conn.write_selection_target(mime, bytes),
        }
    }

    pub fn simulate_paste(
        &self,
        target: Option<WindowId>,
        representations: &[ClipRepresentation],
        mode: PasteMode,
    ) -> Result<(), PlatformError> {
        let content = resolve_paste_content(representations, mode);

        if !self.key_synthesis_supported {
            self.write_content(&content);
            return Ok(());
        }

        let window = match target {
            Some(window) => window,
            None if self.focus_detection_supported => return Err(PlatformError::NoFocusedWindow),
            None => {
                // No window to target and none was ever expected on this
                // backend: place the content on the clipboard and stop -
                // clipboard-only fallback, the user completes the paste
                // manually.
                self.write_content(&content);
                return Ok(());
            }
        };

        self.write_content(&content);
        self.conn
            .synthesize_key(window, PASTE_KEY_BINDING)
            .map_err(PlatformError::Backend)?;
        Ok(())
    }

    /// Places the clip's content on the clipboard only - no focused-window
    /// lookup, no key synthesis. For callers that want to let the user paste
    /// manually wherever they choose, rather than targeting whatever happens
    /// to be focused right now (see `paste-simulation`'s copy-only mode).
    pub fn copy_to_clipboard(&self, representations: &[ClipRepresentation], mode: PasteMode) {
        self.write_content(&resolve_paste_content(representations, mode));
    }

    /// Like `simulate_paste`, but targets whichever window is currently
    /// focused at call time, rather than a caller-supplied target. Useful
    /// when there is no separate "popup activation" moment to capture a
    /// retained focus snapshot from (e.g. a raw IPC client issuing
    /// `PasteClip` directly).
    pub fn paste_to_focused_window(
        &self,
        representations: &[ClipRepresentation],
        mode: PasteMode,
    ) -> Result<(), PlatformError> {
        self.simulate_paste(self.conn.focused_window(), representations, mode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::x11::fake::{FakeX11Connection, RecordedOp};

    #[test]
    fn copy_to_clipboard_writes_the_given_text_without_synthesizing_any_key() {
        let conn = FakeX11Connection::new();
        let simulator = PasteSimulator::new(conn);
        let representations = vec![ClipRepresentation::new("text/plain", 0).with_text_value("hello")];

        simulator.copy_to_clipboard(&representations, PasteMode::PlainText);

        let ops = simulator.conn.ops_log();
        assert_eq!(ops, vec![RecordedOp::WriteSelection("hello".to_string())]);
    }

    fn image_representation(dir: &std::path::Path, bytes: &[u8]) -> ClipRepresentation {
        let blob_path = dir.join("clip.png");
        std::fs::write(&blob_path, bytes).unwrap();
        let mut repr = ClipRepresentation::new("image/png", 0);
        repr.blob_path = Some(blob_path.to_string_lossy().into_owned());
        repr.byte_size = bytes.len() as u64;
        repr
    }

    #[test]
    fn pasting_an_image_only_clip_writes_its_bytes_to_the_image_target_before_synthesizing_the_key() {
        let dir = tempfile::tempdir().unwrap();
        let image = image_representation(dir.path(), b"fake png bytes");
        let conn = FakeX11Connection::new();
        let simulator = PasteSimulator::new(conn);

        simulator.simulate_paste(Some(1), &[image], PasteMode::Auto).unwrap();

        let ops = simulator.conn.ops_log();
        assert_eq!(
            ops,
            vec![
                RecordedOp::WriteSelectionTarget("image/png".to_string(), b"fake png bytes".to_vec()),
                RecordedOp::SynthesizeKey(1, "ctrl+v".to_string()),
            ]
        );
    }

    #[test]
    fn clipboard_only_mode_writes_image_bytes_for_an_image_only_clip_instead_of_empty_text() {
        let dir = tempfile::tempdir().unwrap();
        let image = image_representation(dir.path(), b"fake png bytes");
        let conn = FakeX11Connection::new();
        let simulator = PasteSimulator::clipboard_only(conn);

        simulator.simulate_paste(Some(1), &[image], PasteMode::Auto).unwrap();

        let ops = simulator.conn.ops_log();
        assert_eq!(ops, vec![RecordedOp::WriteSelectionTarget("image/png".to_string(), b"fake png bytes".to_vec())]);
    }

    #[test]
    fn copy_to_clipboard_with_an_image_only_clip_writes_its_bytes_to_the_image_target() {
        let dir = tempfile::tempdir().unwrap();
        let image = image_representation(dir.path(), b"fake png bytes");
        let conn = FakeX11Connection::new();
        let simulator = PasteSimulator::new(conn);

        simulator.copy_to_clipboard(&[image], PasteMode::Auto);

        let ops = simulator.conn.ops_log();
        assert_eq!(ops, vec![RecordedOp::WriteSelectionTarget("image/png".to_string(), b"fake png bytes".to_vec())]);
    }

    #[test]
    fn a_clip_with_both_text_and_image_representations_still_prefers_pasting_the_text() {
        // Matches `resolve_paste_text`'s existing preference order - an
        // image alongside real text content (e.g. alt text) shouldn't
        // suddenly switch what gets pasted.
        let dir = tempfile::tempdir().unwrap();
        let mut image = image_representation(dir.path(), b"fake png bytes");
        image.ordinal = 1;
        let text = ClipRepresentation::new("text/plain", 0).with_text_value("hello");
        let conn = FakeX11Connection::new();
        let simulator = PasteSimulator::new(conn);

        simulator.simulate_paste(Some(1), &[text, image], PasteMode::Auto).unwrap();

        let ops = simulator.conn.ops_log();
        assert_eq!(ops[0], RecordedOp::WriteSelection("hello".to_string()));
    }

    #[test]
    fn content_is_placed_on_the_clipboard_before_the_key_combination_is_synthesized() {
        let conn = FakeX11Connection::new();
        let simulator = PasteSimulator::new(conn);
        let representations = vec![ClipRepresentation::new("text/plain", 0).with_text_value("hello")];

        simulator.simulate_paste(Some(1), &representations, PasteMode::Auto).unwrap();

        let ops = simulator.conn.ops_log();
        assert_eq!(
            ops,
            vec![
                RecordedOp::WriteSelection("hello".to_string()),
                RecordedOp::SynthesizeKey(1, "ctrl+v".to_string()),
            ]
        );
    }

    #[test]
    fn auto_mode_pastes_html_when_both_representations_are_present() {
        let conn = FakeX11Connection::new();
        let simulator = PasteSimulator::new(conn);
        let text = ClipRepresentation::new("text/plain", 0).with_text_value("hi");
        let html = ClipRepresentation::new("text/html", 1).with_text_value("<b>hi</b>");

        simulator.simulate_paste(Some(1), &[text, html], PasteMode::Auto).unwrap();

        let ops = simulator.conn.ops_log();
        assert_eq!(ops[0], RecordedOp::WriteSelection("<b>hi</b>".to_string()));
    }

    #[test]
    fn plain_text_mode_ignores_a_non_plain_text_representation() {
        let conn = FakeX11Connection::new();
        let simulator = PasteSimulator::new(conn);
        let mut html = ClipRepresentation::new("text/html", 0).with_text_value("<b>hi</b>");
        html.preview_text = Some("hi".to_string());

        simulator.simulate_paste(Some(1), &[html], PasteMode::PlainText).unwrap();

        let ops = simulator.conn.ops_log();
        assert_eq!(ops[0], RecordedOp::WriteSelection("hi".to_string()));
    }

    #[test]
    fn plain_text_mode_strips_html_down_to_its_plain_text_rendering_when_both_are_present() {
        let conn = FakeX11Connection::new();
        let simulator = PasteSimulator::new(conn);
        let text = ClipRepresentation::new("text/plain", 0).with_text_value("hi");
        let html = ClipRepresentation::new("text/html", 1).with_text_value("<b>hi</b>");

        simulator.simulate_paste(Some(1), &[text, html], PasteMode::PlainText).unwrap();

        let ops = simulator.conn.ops_log();
        assert_eq!(ops[0], RecordedOp::WriteSelection("hi".to_string()));
    }

    #[test]
    fn no_previously_focused_window_yields_an_error() {
        let conn = FakeX11Connection::new();
        let simulator = PasteSimulator::new(conn);
        let representations = vec![ClipRepresentation::new("text/plain", 0).with_text_value("hello")];

        let result = simulator.simulate_paste(None, &representations, PasteMode::Auto);

        assert!(matches!(result, Err(PlatformError::NoFocusedWindow)));
    }

    #[test]
    fn missing_focus_capture_still_errors_when_focus_detection_is_supported() {
        // Regression guard: a backend that reports focus-detection as
        // supported (the default) must keep erroring on a missing captured
        // window, not silently degrade to clipboard-only.
        let conn = FakeX11Connection::new();
        let simulator = PasteSimulator::new(conn);
        let representations = vec![ClipRepresentation::new("text/plain", 0).with_text_value("hello")];

        let result = simulator.simulate_paste(None, &representations, PasteMode::Auto);

        assert!(matches!(result, Err(PlatformError::NoFocusedWindow)));
    }

    #[test]
    fn missing_focus_capture_degrades_to_clipboard_only_when_focus_detection_is_unsupported() {
        let conn = FakeX11Connection::new();
        let simulator = PasteSimulator::without_focus_detection(conn);
        let representations = vec![ClipRepresentation::new("text/plain", 0).with_text_value("hello")];

        simulator.simulate_paste(None, &representations, PasteMode::Auto).unwrap();

        let ops = simulator.conn.ops_log();
        assert_eq!(ops, vec![RecordedOp::WriteSelection("hello".to_string())]);
    }

    #[test]
    fn paste_to_focused_window_targets_whatever_is_currently_focused() {
        let conn = FakeX11Connection::new();
        conn.set_focused_window(Some(7));
        let simulator = PasteSimulator::new(conn);
        let representations = vec![ClipRepresentation::new("text/plain", 0).with_text_value("hello")];

        simulator.paste_to_focused_window(&representations, PasteMode::Auto).unwrap();

        let ops = simulator.conn.ops_log();
        assert_eq!(ops[1], RecordedOp::SynthesizeKey(7, "ctrl+v".to_string()));
    }

    #[test]
    fn clipboard_only_mode_never_synthesizes_a_key_even_with_a_focused_window() {
        // GNOME/Mutter treats XTestFakeInput from an XWayland client as a
        // security-sensitive operation gated behind its Remote Desktop
        // portal consent dialog - popping that dialog up on every paste is
        // effectively a broken auto-paste experience on that platform.
        // `clipboard_only` mode is the opt-out: focus detection still works
        // normally, but the synthetic keystroke is never attempted, so the
        // user pastes manually via Ctrl+V themselves.
        let conn = FakeX11Connection::new();
        conn.set_focused_window(Some(7));
        let simulator = PasteSimulator::clipboard_only(conn);
        let representations = vec![ClipRepresentation::new("text/plain", 0).with_text_value("hello")];

        simulator.simulate_paste(Some(7), &representations, PasteMode::Auto).unwrap();

        let ops = simulator.conn.ops_log();
        assert_eq!(ops, vec![RecordedOp::WriteSelection("hello".to_string())]);
    }

    #[test]
    fn clipboard_only_mode_still_succeeds_with_no_focused_window() {
        let conn = FakeX11Connection::new();
        let simulator = PasteSimulator::clipboard_only(conn);
        let representations = vec![ClipRepresentation::new("text/plain", 0).with_text_value("hello")];

        let result = simulator.simulate_paste(None, &representations, PasteMode::Auto);

        assert!(result.is_ok());
    }

    #[test]
    fn paste_to_focused_window_errors_when_nothing_is_focused() {
        let conn = FakeX11Connection::new();
        let simulator = PasteSimulator::new(conn);
        let representations = vec![ClipRepresentation::new("text/plain", 0).with_text_value("hello")];

        let result = simulator.paste_to_focused_window(&representations, PasteMode::Auto);

        assert!(matches!(result, Err(PlatformError::NoFocusedWindow)));
    }

    /// Focuses a target app manually, then simulates a paste of `"clipdeck
    /// paste test"` into it via the real XTest-based key delivery. Needs a
    /// running desktop session: focus a text field in another window, then
    /// run this with `cargo test -p clip-platform -- --ignored
    /// paste::real_paste`, and confirm the text landed there within the
    /// 5-second window.
    #[test]
    #[ignore = "requires a live X11 session and a manually focused target window"]
    fn real_paste_delivers_content_into_the_focused_window() {
        use crate::x11::RealX11Connection;

        let conn = RealX11Connection::connect(None).expect("connect to X server");
        std::thread::sleep(std::time::Duration::from_secs(5));
        let target = conn.focused_window().expect("a window must be focused to target this test");

        let simulator = PasteSimulator::new(conn);
        let representations = vec![ClipRepresentation::new("text/plain", 0).with_text_value("clipdeck paste test")];
        simulator.simulate_paste(Some(target), &representations, PasteMode::Auto).unwrap();
    }
}
