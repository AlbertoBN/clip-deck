## Why

The PRD deliberately defers Wayland to its own milestone because compositor support for clipboard
automation, global hotkeys, and focused-window discovery is inconsistent - but Ubuntu's default session on
recent releases is Wayland, so ClipDeck needs a Wayland adapter with honest capability reporting rather
than assuming X11-level feature parity.

## What Changes

- Implement a Wayland `ClipboardBackend` adapter (`clip-platform::wayland`) supporting clipboard read/
  write and change-watching where the compositor allows it (via `wlr-data-control` on compositors that
  support it, e.g. wlroots-based ones).
- Extend `platform-diagnostics` so its report distinguishes the Wayland backend and surfaces exactly which
  capabilities are unavailable in the current session, rather than only ever reporting X11's full support.
- Extend `global-hotkey-registration` and `focused-window-detection` to degrade gracefully (report
  unsupported via `capabilities()`, don't panic or silently no-op) when the running compositor doesn't
  support the underlying mechanism, instead of assuming X11-level support everywhere.

## Capabilities

### New Capabilities
- `wayland-clipboard-capture`: Wayland-specific implementation of `ClipboardBackend` for reading/writing
  clipboard content and watching for changes on compositors that support the data-control protocol.

### Modified Capabilities
- `platform-diagnostics`: The report now identifies "wayland" as a possible backend value (in addition to
  "x11") and, on Wayland, surfaces per-capability unsupported flags reflecting real compositor limitations
  instead of assuming full support.
- `global-hotkey-registration`: Registration on Wayland reports an explicit unsupported/degraded result
  (via `capabilities()`) rather than erroring unpredictably or hanging, when the compositor provides no
  global-shortcut mechanism the registration layer can use.
- `focused-window-detection`: On Wayland, `focused_app` reports unsupported (rather than guessing or
  crashing) on compositors that don't expose focused-window information to clients, matching Wayland's
  security model.

## Impact

- Affected code: `crates/clip-platform/src/wayland/mod.rs`, `crates/clip-platform/src/diagnostics.rs`,
  `crates/clip-platform/src/hotkeys.rs`, `crates/clip-platform/src/focus.rs`,
  `crates/clip-platform/Cargo.toml` (adds Wayland client bindings, e.g. `wayland-client` +
  `wayland-protocols-wlr`).
- Depends on: `clip-platform-x11-adapter` (shares the `ClipboardBackend` trait and
  `BackendCapabilities`/diagnostics shape) and `clip-platform-rich-content` (Wayland capture should offer
  the same representation richness as X11 where the protocol allows, per the same
  `ClipboardSnapshot`/`ClipRepresentation` model).
- Downstream: `clipd-daemon-core`'s backend-selection logic gains a real second option (this change does
  not modify `clipd`'s selection logic itself, only provides the backend it can select); `clip-ui-tauri-
  shell`'s diagnostics screen (already spec'd to show unsupported capabilities explicitly) starts showing
  genuinely-partial Wayland reports instead of only ever seeing X11's full-support report.
