## Why

`clip-ui-tauri-shell`'s settings screen has a passing test for "invalid binding shows an error and is not
saved," but it only exercises a mocked `update_settings` rejection - the real `clipd` handler accepts any
string as `hotkey_binding` and persists it unchecked. `clip-platform` (`clip-platform-x11-adapter`) already
has a binding parser (`hotkeys::parse_binding`) used to validate bindings before registering them with
`HotkeyBackend`; `clipd`'s `UpdateSettings` handler just never calls it. This was flagged as a known gap in
`clip-ui-tauri-shell`'s proposal.

## What Changes

- `UpdateSettings`'s handler in `clipd` (crate: `clipd`, module `commands.rs`) validates a submitted
  `hotkey_binding` via `clip_platform::hotkeys::parse_binding` before persisting it, rejecting the command
  with a descriptive error (not silently accepting or silently ignoring) when parsing fails.
- No change to the wire protocol: `UpdateSettings` already returns `Result<Value, String>` per
  `clip-ipc-transport`; this only tightens what counts as success inside the existing handler.

## Capabilities

### Modified Capabilities
- `ipc-command-handlers` (owned by `clipd-daemon-core`): `UpdateSettings`'s existing "settings round-trip"
  requirement gains a validation precondition on `hotkey_binding`.

### New Capabilities
(none)

## Impact

- Affected code: `crates/clipd/src/commands.rs` (the `UpdateSettings` match arm), which already depends on
  `crates/clip-platform/src/hotkeys.rs`'s `parse_binding` (public since `clip-platform-x11-adapter`).
- Build order: sits entirely within the already-completed `clipd` crate; no new cross-crate dependency
  beyond the existing `clip-platform` one `clipd-daemon-core` already declared.
- Depends on: `clipd-daemon-core` (defines `UpdateSettings`'s current handler and the `Store`/`AppError`
  plumbing it uses), `clip-platform-x11-adapter` (defines `parse_binding`).
- No frontend change required: `clip-ui-tauri-shell`'s settings component already surfaces whatever error
  string `update_settings` rejects with via `role="alert"`; it will now receive a real validation error
  instead of only a mocked one.
