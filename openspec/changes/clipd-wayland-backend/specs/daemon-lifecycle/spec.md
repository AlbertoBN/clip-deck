## ADDED Requirements

### Requirement: Startup selects the clipboard backend by reachability, preferring X11
Startup SHALL attempt to connect the X11-backed `ClipboardBackend` first, unconditionally (this succeeds
both for native X11 sessions and for Wayland sessions where an XWayland X11 display is reachable). Only
when that connection attempt fails SHALL startup check whether the process is running under a Wayland
session (via the `WAYLAND_DISPLAY` environment variable) and, if so, fall back to the Wayland-backed
`ClipboardBackend` and an unsupported hotkey backend. If the X11 connection attempt fails and no Wayland
session is detected either, startup SHALL fail with the underlying X11 connection error. The daemon SHALL
never mix pieces from both backends, and the diagnostics backend identifier SHALL reflect whichever
backend was actually selected.

#### Scenario: A reachable X11 display selects the X11 backend
- **WHEN** the daemon starts and connecting the X11-backed `ClipboardBackend` succeeds (whether under a
  native X11 session or a Wayland session with XWayland available)
- **THEN** it selects the X11-backed `ClipboardBackend` and the real global-hotkey backend, and a
  subsequent `GetDiagnostics` reports the backend identifier as `"x11"`

#### Scenario: A Wayland-only session (X11 unreachable) falls back to the Wayland backend
- **WHEN** the daemon starts, connecting the X11-backed `ClipboardBackend` fails, and `WAYLAND_DISPLAY` is
  set
- **THEN** it constructs the Wayland-backed `ClipboardBackend` and an unsupported hotkey backend, and a
  subsequent `GetDiagnostics` reports the backend identifier as `"wayland"`

#### Scenario: No reachable display of either kind fails startup
- **WHEN** the daemon starts, connecting the X11-backed `ClipboardBackend` fails, and `WAYLAND_DISPLAY` is
  unset
- **THEN** startup fails with the underlying X11 connection error, matching pre-existing behavior for an
  environment with no display available at all
