## Why

ClipDeck's whole popup workflow is described as "opened via the global hotkey" (see `popup-picker-ui`'s
existing requirement text), but nothing actually wires that up: `clipd` never registers a hotkey with
`clip-platform`'s `HotkeyBackend` (the `Event::HotkeyPressed` variant exists in the protocol and is
forwarded end-to-end by `clip-ui-tauri`'s event forwarder - confirmed by an existing passing test - but is
never published in practice), and `clip-ui-tauri` only has one `"main"` window with no listener that shows
or focuses a popup in response to that event. This was flagged as a known gap in both `clipd-daemon-core`'s
and `clip-ui-tauri-shell`'s proposals. Without it, the popup is only reachable by manually navigating to a
window labeled `"popup"`, which doesn't exist yet either.

## What Changes

- `clipd` (crate `clipd`, `main.rs`/`app.rs`) registers a global hotkey at startup using the persisted
  `hotkey_binding` setting (already validated at save-time by `clipd-hotkey-validation`) via
  `clip_platform::hotkeys::HotkeyBackend`, and publishes `Event::HotkeyPressed` when it fires. Registration
  failure (e.g. the binding is already grabbed by another application) is logged and degrades gracefully -
  it does not fail daemon startup, consistent with the project's "degrade and surface status, don't fail
  silently" rule for platform integration.
- `clip-ui-tauri`'s `tauri.conf.json` gains a second, initially-hidden window definition labeled `"popup"`
  (undecorated, always-on-top, not in the taskbar), so `App.tsx`'s existing label-based routing to
  `<Popup />` has a real window to render into.
- `clip-ui-tauri`'s host (`src-tauri/src/lib.rs`) shows and focuses the `"popup"` window when it observes
  `Event::HotkeyPressed`, mirroring the existing tray "show"/"hide" menu-event handling for `"main"`.
- `clip-ui-tauri`'s `Popup` component re-runs its existing focus-and-empty-search behavior every time the
  window is shown (not only on first mount), since the window is shown/hidden rather than
  created/destroyed on each hotkey press.

## Capabilities

### New Capabilities
- `hotkey-registration` (owned by `clipd`): the daemon registers a global hotkey from the persisted binding
  at startup and publishes `HotkeyPressed` when it fires.

### Modified Capabilities
- `daemon-lifecycle` (owned by `clipd-daemon-core`): startup gains a non-fatal hotkey-registration step.
- `popup-picker-ui` (owned by `clip-ui-tauri-shell`): "opens via the global hotkey" becomes real - the
  popup window is shown/focused on `HotkeyPressed`, and its focus/empty-search behavior is scoped to "every
  time the popup becomes visible," not just component mount.

## Impact

- Affected code: `crates/clipd/src/main.rs`, `crates/clipd/src/app.rs` (new `hotkeys` param to `run`),
  `crates/clip-ui-tauri/src-tauri/tauri.conf.json` (new window entry), `crates/clip-ui-tauri/src-tauri/
  src/lib.rs` (new `HotkeyPressed` branch in the event forwarder), `crates/clip-ui-tauri/src/views/popup/
  Popup.tsx` (re-run-on-show behavior).
- Depends on: `clipd-hotkey-validation` (a validated, persisted `hotkey_binding` to register),
  `clip-platform-x11-adapter` (owns `HotkeyBackend`/`GlobalHotkeyBackend`/`parse_binding`),
  `clipd-daemon-core` (owns `app::run`'s startup sequence and the `Event::HotkeyPressed` forwarding path,
  already implemented and tested), `clip-ui-tauri-shell` (owns `tauri.conf.json`, `lib.rs`, `Popup.tsx`).
- Non-goal: live re-registration when `UpdateSettings` changes the binding mid-run - per
  `clipd-hotkey-validation`'s existing precedent, a changed binding takes effect on the next daemon
  restart, not immediately. A future change can add live re-registration if that turns out to matter.
- Non-goal: Wayland hotkey support - `clip-platform-wayland-adapter` is a separate, not-yet-started
  milestone; this change only wires the already-implemented X11/`global-hotkey` path.
