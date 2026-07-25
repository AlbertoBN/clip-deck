## ADDED Requirements

### Requirement: Startup selects the clipboard backend based on session type
Startup SHALL detect whether the process is running under a Wayland session (via the `WAYLAND_DISPLAY`
environment variable) and select the Wayland-backed `ClipboardBackend` and an unsupported hotkey backend
in that case, or the X11-backed `ClipboardBackend` and the real global-hotkey backend otherwise - never
mixing pieces from both. The diagnostics backend identifier SHALL reflect whichever backend was selected.

#### Scenario: A Wayland session selects the Wayland backend
- **WHEN** the daemon starts with `WAYLAND_DISPLAY` set
- **THEN** it constructs the Wayland-backed `ClipboardBackend` and an unsupported hotkey backend, and a
  subsequent `GetDiagnostics` reports the backend identifier as `"wayland"`

#### Scenario: No Wayland display selects the X11 backend
- **WHEN** the daemon starts with `WAYLAND_DISPLAY` unset
- **THEN** it constructs the X11-backed `ClipboardBackend` and the real global-hotkey backend, and a
  subsequent `GetDiagnostics` reports the backend identifier as `"x11"`
