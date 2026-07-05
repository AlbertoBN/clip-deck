## MODIFIED Requirements

### Requirement: Diagnostics report identifies which backend is active
The diagnostics report SHALL identify which backend produced it (`"x11"` or `"wayland"`), so the UI and
support requests can distinguish an X11 session's report from a Wayland session's report.

#### Scenario: X11 backend's report identifies itself as x11
- **WHEN** the diagnostics report is generated while the X11 backend is active
- **THEN** the report's backend identifier is `"x11"`

#### Scenario: Wayland backend's report identifies itself as wayland
- **WHEN** the diagnostics report is generated while the Wayland backend is active
- **THEN** the report's backend identifier is `"wayland"`

## ADDED Requirements

### Requirement: Diagnostics report surfaces per-capability gaps on a partially-supported backend
The diagnostics report SHALL list each unsupported capability individually rather than collapsing the
whole report to a single generic "partial support" flag, when the active backend reports one or more
capabilities as unsupported (e.g. Wayland without hotkey or focus-detection support on the running
compositor), so Settings can explain exactly what doesn't work.

#### Scenario: Wayland report lists each unsupported capability individually
- **WHEN** the active Wayland backend reports hotkeys and focus-detection as unsupported but capture and
  paste as supported
- **THEN** the diagnostics report lists hotkeys and focus-detection separately as unsupported, and capture
  and paste separately as supported
