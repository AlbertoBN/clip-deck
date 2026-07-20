//! `ClipboardBackend` trait implemented separately by the x11 and wayland adapters.

use clip_core::models::{AppContext, ClipRepresentation, PasteMode};

/// Error surface shared by every `ClipboardBackend` implementation.
#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("no previously focused window is available to target")]
    NoFocusedWindow,
    #[error("backend error: {0}")]
    Backend(String),
}

/// A snapshot of the clipboard's content at read time. May contain zero or
/// more representations; zero representations means "empty", not an error.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ClipboardSnapshot {
    pub representations: Vec<ClipRepresentation>,
}

impl ClipboardSnapshot {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.representations.is_empty()
    }
}

/// What a given backend actually supports; defaults to nothing so a new or
/// partially-implemented adapter never silently claims support it lacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct BackendCapabilities {
    pub capture: bool,
    pub paste_simulation: bool,
    pub hotkeys: bool,
    pub focus_detection: bool,
}

/// Uniform capture/paste/diagnostics surface implemented by each display-server adapter.
pub trait ClipboardBackend: Send + Sync {
    /// Starts the change-watch loop, invoking `on_capture` once per distinct
    /// clipboard content change.
    fn start(
        &self,
        on_capture: Box<dyn Fn(ClipboardSnapshot) + Send + Sync>,
    ) -> Result<(), PlatformError>;

    /// Reads the current clipboard content.
    fn read_current(&self) -> Result<ClipboardSnapshot, PlatformError>;

    /// Writes plain-text content onto the clipboard.
    fn set_current(&self, content: &str) -> Result<(), PlatformError>;

    /// The currently (or, per `focused-window-detection`, previously) focused
    /// application, or `None` if there isn't one.
    fn focused_app(&self) -> Option<AppContext>;

    /// Pastes a clip's representations into the previously focused window.
    /// `mode = PlainText` uses only the plain-text-rendered form, discarding
    /// any richer representation.
    fn simulate_paste(&self, representations: &[ClipRepresentation], mode: PasteMode) -> Result<(), PlatformError>;

    /// What this backend actually supports.
    fn capabilities(&self) -> BackendCapabilities;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeBackend {
        capabilities: BackendCapabilities,
    }

    impl ClipboardBackend for FakeBackend {
        fn start(
            &self,
            _on_capture: Box<dyn Fn(ClipboardSnapshot) + Send + Sync>,
        ) -> Result<(), PlatformError> {
            Ok(())
        }

        fn read_current(&self) -> Result<ClipboardSnapshot, PlatformError> {
            Ok(ClipboardSnapshot::empty())
        }

        fn set_current(&self, _content: &str) -> Result<(), PlatformError> {
            Ok(())
        }

        fn focused_app(&self) -> Option<AppContext> {
            Some(AppContext::new("fake-app"))
        }

        fn simulate_paste(&self, _representations: &[ClipRepresentation], _mode: PasteMode) -> Result<(), PlatformError> {
            Ok(())
        }

        fn capabilities(&self) -> BackendCapabilities {
            self.capabilities
        }
    }

    #[test]
    fn a_minimal_fake_backend_satisfies_the_clipboard_backend_trait() {
        let backend: Box<dyn ClipboardBackend> = Box::new(FakeBackend {
            capabilities: BackendCapabilities { paste_simulation: true, ..Default::default() },
        });

        assert!(backend.read_current().unwrap().is_empty());
        assert_eq!(backend.focused_app(), Some(AppContext::new("fake-app")));
        assert!(backend.capabilities().paste_simulation);
        assert!(!backend.capabilities().hotkeys);
    }

    #[test]
    fn default_capabilities_report_nothing_supported() {
        let caps = BackendCapabilities::default();
        assert!(!caps.capture);
        assert!(!caps.paste_simulation);
        assert!(!caps.hotkeys);
        assert!(!caps.focus_detection);
    }

    #[test]
    fn capability_flags_are_independently_settable() {
        let caps = BackendCapabilities { paste_simulation: true, ..Default::default() };
        assert!(caps.paste_simulation);
        assert!(!caps.capture);
        assert!(!caps.hotkeys);
        assert!(!caps.focus_detection);
    }

    #[test]
    fn empty_clipboard_produces_an_empty_snapshot() {
        let snapshot = ClipboardSnapshot::empty();
        assert!(snapshot.is_empty());
        assert!(snapshot.representations.is_empty());
    }

    #[test]
    fn populated_clipboard_produces_a_snapshot_with_representations() {
        let snapshot = ClipboardSnapshot {
            representations: vec![ClipRepresentation::new("text/plain", 0).with_text_value("hi")],
        };
        assert!(!snapshot.is_empty());
        assert_eq!(snapshot.representations.len(), 1);
    }
}
