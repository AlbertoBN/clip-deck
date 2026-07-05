## Why

Per the PRD, X11 is the first fully-supported environment and the baseline for end-to-end acceptance
tests (Milestone 1: text capture on X11). `clipd` cannot have a real watch loop until something actually
implements `ClipboardBackend` for a real display server, and the PRD explicitly requires x11 and wayland to
be separate adapters behind one trait rather than a single implementation assuming feature parity.

## What Changes

- Define the `ClipboardBackend` trait (`start`, `read_current`, `set_current`, `focused_app`,
  `simulate_paste`, `capabilities`) in `clip-platform::clipboard`, shared by every adapter.
- Implement the X11 clipboard adapter (`clip-platform::x11`): reading the current selection/clipboard
  content, writing content back, and starting a change-watch loop that emits capture events.
- Implement global hotkey registration (`clip-platform::hotkeys`) for the popup-activation shortcut.
- Implement focused-window discovery (`clip-platform::focus`) so paste-back can target the previously
  focused window.
- Implement synthetic paste / plain-text paste (`clip-platform::paste`) for pasting a selected clip back
  into the previously focused window.
- Implement the capability diagnostics report (`clip-platform::diagnostics`) so Settings can show what the
  current backend supports.

## Capabilities

### New Capabilities
- `clipboard-backend-trait`: The `ClipboardBackend` trait and its `BackendCapabilities` /
  `ClipboardSnapshot` supporting types, implemented independently by each adapter.
- `x11-clipboard-capture`: X11-specific implementation of `ClipboardBackend` for reading/writing clipboard
  content and watching for changes.
- `global-hotkey-registration`: Registering and listening for a configurable global hotkey that triggers
  popup activation.
- `focused-window-detection`: Discovering the currently (or previously) focused application/window on
  X11.
- `paste-simulation`: Simulating a paste of selected clip content into the previously focused window,
  including a plain-text-only paste mode.
- `platform-diagnostics`: A capability report describing what the active backend supports/does not
  support, for display in Settings.

### Modified Capabilities
(none)

## Impact

- Affected code: `crates/clip-platform/src/{clipboard,x11/mod.rs,hotkeys,focus,paste,diagnostics}.rs`,
  `crates/clip-platform/Cargo.toml` (adds X11-specific bindings and a hotkey crate - see design.md).
- Depends on: `clip-core-foundations` (`AppContext`, `PasteMode`).
- Downstream: unlocks `clipd-daemon-core`'s watch loop, which consumes `ClipboardBackend` events.
- Non-goals for this change: Wayland support (`clip-platform-wayland-adapter`), HTML/image capture
  (`clip-platform-rich-content`) - this change is plain-text-only on X11, matching Milestone 1.
