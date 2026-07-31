## Why

The global hotkey to show ClipDeck's popup doesn't actually work on GNOME/Mutter Wayland sessions with
XWayland present (the common Ubuntu desktop case, and the machine this was found on). `clipd`'s
backend-selection logic (`crates/clipd/src/main.rs`) picks the hotkey backend using the same "is an X11
connection reachable?" check it uses for the clipboard backend: if XWayland is reachable, it uses
`clip_platform::hotkeys::GlobalHotkeyBackend` (`XGrabKey`-based, via the `global-hotkey` crate), only
falling back to `UnsupportedHotkeyBackend` when no X11 connection can be made at all.

That reasoning is correct for clipboard capture (XWayland transparently supports clipboard-selection
ownership) but wrong for hotkeys: Mutter's Wayland compositor does not forward global key events to
XWayland clients' `XGrabKey` grabs - GNOME reserves system-wide keyboard-shortcut interception for the
compositor itself, for the same reason arbitrary apps can't keylog through Wayland. `register()` reports
success (the grab call itself succeeds against the X server) while the physical key press is never
delivered, so the failure is silent - `clipd-hotkey-popup-activation` (a prior change) explicitly scoped
Wayland hotkey support out as a non-goal, and `clip-platform-wayland-adapter` only ever covered the "no X11
reachable at all" case via `UnsupportedHotkeyBackend`. Neither covers "X11 is reachable via XWayland, but
grabs still won't deliver," which is the actual state of this machine.

## What Changes

- `clipd`'s backend-selection (`main.rs`) stops using X11-reachability alone to choose the hotkey backend.
  When the session is a Wayland session (`app::is_wayland_session`, already used for the clipboard-backend
  decision), it selects a new GSettings-based hotkey backend instead of `GlobalHotkeyBackend`, even though
  XWayland is reachable. True native X11 sessions (no `WAYLAND_DISPLAY`) are unaffected.
- A new `clip-platform` hotkey backend registers a GNOME custom keybinding (via the
  `org.gnome.settings-daemon.plugins.media-keys.custom-keybindings` GSettings schema) that runs an external
  trigger command when pressed, instead of holding a live in-process key grab. This means the existing
  `HotkeyBackend` trait's `register(binding, callback)` shape - built around a callback fired synchronously
  from inside the registering process - needs a bridging mechanism, since the callback now needs to fire
  in response to an out-of-process trigger arriving later. Resolving that shape mismatch is a design
  decision, not just an implementation detail.
- A new `clip-ipc` `Command::TriggerHotkey`, handled by `clipd`'s `CommandHandler` by publishing the
  existing `Event::HotkeyPressed` (no changes needed on the UI side, which already reacts to that event).
- A new small CLI binary (new `[[bin]]` target) that connects to `clipd`'s existing Unix socket via
  `clip-ipc`'s client and sends `Command::TriggerHotkey`. This is the command GNOME's custom keybinding
  actually runs on keypress.
- Translation from this app's `Ctrl+Shift+V`-style binding-string format (`clip_platform::hotkeys::parse_binding`)
  to GSettings' binding syntax (e.g. `<Ctrl><Shift>v`).

## Capabilities

### New Capabilities
- `gsettings-hotkey-backend` (owned by `clip-platform`): registers/unregisters a GNOME custom-keybinding
  global shortcut via GSettings/DConf, pointed at an external trigger command, as a `HotkeyBackend`
  alternative to `XGrabKey` for sessions where key grabs can't actually deliver events.
- `hotkey-trigger-ipc` (owned by `clip-ipc` + `clipd`): a `TriggerHotkey` command that an external process
  (the CLI trigger binary) sends to make the daemon publish `Event::HotkeyPressed`, plus that CLI binary
  itself.

### Modified Capabilities
- `hotkey-registration` (owned by `clipd`, from `clipd-hotkey-popup-activation`): backend selection at
  startup now depends on session type (X11 vs. Wayland), not just X11 reachability, when choosing between
  `GlobalHotkeyBackend` and the new GSettings-based backend.

## Impact

- Affected crates: `clip-platform` (new backend module alongside `hotkeys.rs`'s existing
  `GlobalHotkeyBackend`/`UnsupportedHotkeyBackend`), `clip-ipc` (new `Command` variant), `clipd`
  (`main.rs` backend selection, `commands.rs` new match arm, new `[[bin]]` trigger binary, `Cargo.toml`).
- Depends on already-implemented work: `clip-platform-x11-adapter` (`HotkeyBackend` trait, `parse_binding`,
  `GlobalHotkeyBackend`), `clip-platform-wayland-adapter` (`UnsupportedHotkeyBackend`, `is_wayland_session`
  reasoning precedent), `clipd-wayland-backend` (the existing X11-first-with-Wayland-fallback backend
  selection this change is narrowing), `clipd-hotkey-popup-activation` (`register_hotkey`, the
  `HotkeyPressed` event plumbing this change reuses unchanged on the publish side).
- This sits after the already-completed X11 hotkey milestone and the Wayland clipboard-backend milestone
  in the PRD's build order - it closes a gap those milestones left (Wayland hotkey support was an explicit
  non-goal of `clipd-hotkey-popup-activation`).
- Non-goal: live re-registration when `UpdateSettings` changes the binding mid-run. Per
  `clipd-hotkey-validation`'s existing precedent, a changed binding takes effect on the next daemon
  restart for both backends - this change does not add live re-registration for either.
- Non-goal: the `org.freedesktop.portal.GlobalShortcuts` XDG portal path, or desktops other than GNOME.
  GSettings custom keybindings are GNOME-specific; broader desktop-environment support is a future change
  if it turns out to matter.
