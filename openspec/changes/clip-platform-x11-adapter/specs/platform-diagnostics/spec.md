## ADDED Requirements

### Requirement: Diagnostics report reflects the active backend's real capabilities
The system SHALL produce a diagnostics report derived from the active `ClipboardBackend`'s
`capabilities()` output, rather than a static hardcoded report, so Settings always shows what the running
backend actually supports.

#### Scenario: Diagnostics report matches the active backend's capabilities
- **WHEN** the active backend reports paste-simulation as supported and hotkeys as unsupported
- **THEN** the diagnostics report shows paste-simulation as supported and hotkeys as unsupported

### Requirement: Diagnostics report identifies which backend is active
The diagnostics report SHALL identify which backend produced it (e.g. `"x11"`), so the UI and support
requests can distinguish an X11 session's report from a Wayland session's report.

#### Scenario: X11 backend's report identifies itself as x11
- **WHEN** the diagnostics report is generated while the X11 backend is active
- **THEN** the report's backend identifier is `"x11"`
