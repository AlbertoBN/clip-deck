//! Platform support report.

use crate::clipboard::{BackendCapabilities, ClipboardBackend};

/// What Settings shows about the currently active backend: which one it is,
/// and what it actually supports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticsReport {
    pub backend: String,
    pub capabilities: BackendCapabilities,
}

/// Builds a `DiagnosticsReport` from the active backend's real
/// `capabilities()`, rather than a static hardcoded report.
pub fn generate_report(backend_name: &str, backend: &dyn ClipboardBackend) -> DiagnosticsReport {
    DiagnosticsReport { backend: backend_name.to_string(), capabilities: backend.capabilities() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clip_core::models::{AppContext, ClipRepresentation, PasteMode};
    use crate::clipboard::{ClipboardSnapshot, PlatformError};

    struct FakeBackend {
        capabilities: BackendCapabilities,
    }

    impl ClipboardBackend for FakeBackend {
        fn start(&self, _on_capture: Box<dyn Fn(ClipboardSnapshot) + Send + Sync>) -> Result<(), PlatformError> {
            Ok(())
        }

        fn read_current(&self) -> Result<ClipboardSnapshot, PlatformError> {
            Ok(ClipboardSnapshot::empty())
        }

        fn set_current(&self, _content: &str) -> Result<(), PlatformError> {
            Ok(())
        }

        fn focused_app(&self) -> Option<AppContext> {
            None
        }

        fn simulate_paste(&self, _representations: &[ClipRepresentation], _mode: PasteMode) -> Result<(), PlatformError> {
            Ok(())
        }

        fn capabilities(&self) -> BackendCapabilities {
            self.capabilities
        }
    }

    #[test]
    fn diagnostics_report_matches_the_active_backends_capabilities() {
        let backend =
            FakeBackend { capabilities: BackendCapabilities { paste_simulation: true, hotkeys: false, ..Default::default() } };

        let report = generate_report("fake", &backend);

        assert!(report.capabilities.paste_simulation);
        assert!(!report.capabilities.hotkeys);
    }

    #[test]
    fn x11_backends_report_identifies_itself_as_x11() {
        let backend = FakeBackend { capabilities: BackendCapabilities::default() };

        let report = generate_report("x11", &backend);

        assert_eq!(report.backend, "x11");
    }

    #[test]
    fn wayland_backends_report_identifies_itself_as_wayland() {
        let backend = FakeBackend { capabilities: BackendCapabilities::default() };

        let report = generate_report("wayland", &backend);

        assert_eq!(report.backend, "wayland");
    }

    #[test]
    fn a_mixed_capability_report_lists_each_capability_individually_rather_than_collapsing() {
        let backend = FakeBackend {
            capabilities: BackendCapabilities {
                capture: true,
                paste_simulation: true,
                hotkeys: false,
                focus_detection: false,
            },
        };

        let report = generate_report("wayland", &backend);

        assert!(report.capabilities.capture);
        assert!(report.capabilities.paste_simulation);
        assert!(!report.capabilities.hotkeys);
        assert!(!report.capabilities.focus_detection);
    }
}
