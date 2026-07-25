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

/// Simulates a paste into the previously focused window: places the
/// resolved content on the clipboard, then synthesizes the paste key
/// combination targeted at that window.
pub struct PasteSimulator<C: X11Connection> {
    conn: C,
    focus_detection_supported: bool,
}

impl<C: X11Connection> PasteSimulator<C> {
    pub fn new(conn: C) -> Self {
        Self { conn, focus_detection_supported: true }
    }

    /// Like `new`, but for a backend that reports focus-detection as
    /// unsupported (e.g. a Wayland compositor with no focused-window
    /// information available to clients): a missing captured window is not
    /// treated as an error, since there was never any possibility of
    /// capturing one - see `paste-simulation`'s modified spec.
    pub fn without_focus_detection(conn: C) -> Self {
        Self { conn, focus_detection_supported: false }
    }

    pub fn simulate_paste(
        &self,
        target: Option<WindowId>,
        representations: &[ClipRepresentation],
        mode: PasteMode,
    ) -> Result<(), PlatformError> {
        let text = resolve_paste_text(representations, mode).unwrap_or_default();

        let window = match target {
            Some(window) => window,
            None if self.focus_detection_supported => return Err(PlatformError::NoFocusedWindow),
            None => {
                // No window to target and none was ever expected on this
                // backend: place the content on the clipboard and stop -
                // clipboard-only fallback, the user completes the paste
                // manually.
                self.conn.write_selection(&text);
                return Ok(());
            }
        };

        self.conn.write_selection(&text);
        self.conn
            .synthesize_key(window, PASTE_KEY_BINDING)
            .map_err(PlatformError::Backend)?;
        Ok(())
    }

    /// Places `text` on the clipboard only - no focused-window lookup, no
    /// key synthesis. For callers that want to let the user paste manually
    /// wherever they choose, rather than targeting whatever happens to be
    /// focused right now (see `paste-simulation`'s copy-only mode).
    pub fn copy_to_clipboard(&self, text: &str) {
        self.conn.write_selection(text);
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

        simulator.copy_to_clipboard("hello");

        let ops = simulator.conn.ops_log();
        assert_eq!(ops, vec![RecordedOp::WriteSelection("hello".to_string())]);
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
