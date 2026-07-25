## ADDED Requirements

### Requirement: clipd composes a Wayland-backed ClipboardBackend
`clipd` SHALL provide a `WaylandDaemonBackend` implementing `ClipboardBackend` by composing
`clip-platform`'s Wayland capture backend for `start`/`read_current`/`capabilities`, and an
unsupported-focus tracker for `focused_app`, so the daemon can run on a Wayland session using the same
`Backend` trait `clipd`'s command handlers and watch loop already depend on.

#### Scenario: Capture, capabilities, and focus delegate to the composed Wayland pieces
- **WHEN** `WaylandDaemonBackend`'s `start`, `capabilities`, and `focused_app` are called
- **THEN** they reflect the underlying Wayland capture backend's capture/capabilities behavior and the
  unsupported-focus-tracker's always-`None` result, respectively

### Requirement: Paste on the Wayland daemon backend is clipboard-only
`WaylandDaemonBackend`'s `simulate_paste` SHALL resolve the clip content to paste (per the same
Auto/PlainText representation-selection rules used on X11) and place it on the clipboard, and SHALL NOT
attempt any synthetic key delivery, since no input-synthesis mechanism is available on Wayland in this
version.

#### Scenario: simulate_paste places content on the clipboard without synthesizing a key press
- **WHEN** `WaylandDaemonBackend::simulate_paste` is called with a clip's representations and a paste mode
- **THEN** the resolved content is placed on the clipboard and the call succeeds, without any key-press
  synthesis being attempted

#### Scenario: simulate_paste succeeds even with no previously focused window captured
- **WHEN** `WaylandDaemonBackend::simulate_paste` is called and no previously-focused window was captured
- **THEN** the call still succeeds (clipboard-only), matching `paste-simulation`'s
  focus-detection-unsupported degradation behavior rather than returning an error
