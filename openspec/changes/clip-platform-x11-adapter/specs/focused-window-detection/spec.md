## ADDED Requirements

### Requirement: Focused window is reported as an AppContext when available
`focused_app` SHALL return the currently focused window's application and window information as an
`AppContext`, parsed from the X11 window properties (e.g. `WM_CLASS`/`_NET_WM_NAME`) of the active window.

#### Scenario: A focused terminal window is reported with its app name
- **WHEN** the active X11 window has `WM_CLASS` identifying it as `"gnome-terminal"`
- **THEN** `focused_app` returns `Some(AppContext { app: "gnome-terminal", .. })`

### Requirement: No focused window is reported as absence, not an error
`focused_app` SHALL return `None` rather than an error when there is no focused application window (e.g.
focus is on the desktop/root window, or no window is currently focused).

#### Scenario: Desktop focus reports no application context
- **WHEN** the X11 root/desktop window currently has input focus
- **THEN** `focused_app` returns `None`

### Requirement: Focused window snapshot is captured at popup-open time for paste targeting
The system SHALL capture and retain the focused-window context at the moment the popup is activated, so
`simulate_paste` can target that originally-focused window even after the popup itself has taken input
focus.

#### Scenario: Paste targets the window focused before the popup opened
- **WHEN** window `"editor"` is focused, the popup is activated (capturing `"editor"` as the paste
  target), and the popup itself subsequently holds input focus
- **THEN** invoking paste targets `"editor"`, not the popup window
