## Context

`clipd`'s `commands.rs` handles `Command::UpdateSettings { settings }` by delegating straight to the
store's settings persistence (`settings::set_value` per key), with no validation step. `clip-platform`
already exports `hotkeys::parse_binding(spec: &str) -> Result<HotkeyBinding, HotkeyError>`, used internally
by `GlobalHotkeyBackend`/`FakeHotkeyBackend` to turn a binding string like `"Ctrl+Shift+V"` into a
`HotkeyBinding` before registering it. `clipd` does not yet depend on `clip-platform`'s `hotkeys` module in
`commands.rs` (only `app.rs` pulls in `clip-platform` for the backend itself), so this change adds that one
call.

## Goals / Non-Goals

**Goals:**
- Reject an `UpdateSettings` call whose `hotkey_binding` fails `parse_binding`, returning a descriptive
  error string (e.g. surfacing `HotkeyError`'s message) instead of persisting it.
- Leave every other `AppSettings` field's update path (retention window, default paste mode, capture
  paused) untouched - validation is additive and scoped to `hotkey_binding` only.

**Non-Goals:**
- No change to how `clipd` currently (does not) re-register the live hotkey when the binding changes - that
  is the subject of `clipd-hotkey-popup-activation`. This change only stops bad strings from being saved;
  it does not make a valid new binding take effect without a daemon restart.
- No change to the wire protocol or `AppSettings`'s shape.

## Decisions

- **Validate only when `hotkey_binding` is present in the update payload**: `UpdateSettings` updates are
  partial (only the settings included in the request change), so validation only runs against the
  submitted `hotkey_binding` string, not against the full persisted `AppSettings` on every call.
- **Reuse `clip_platform::hotkeys::parse_binding` as-is**: no new parsing logic in `clipd`; add
  `clip-platform` as read-only dependency surface for this one function (already a workspace dependency of
  `clipd` via `app.rs`'s backend wiring, so no new `Cargo.toml` edit is needed).
- **Error surfacing**: return `Err(HotkeyError)`'s `Display` output as the command's error string, matching
  the existing `Result<Value, String>` handler return convention in `commands.rs`, so `clip-ui-tauri`'s
  `update_settings` Tauri command (already implemented) propagates it unchanged.

### Test strategy

- Unit test in `crates/clipd/src/commands.rs`'s existing `#[cfg(test)]` module:
  1. **Red**: write a test calling `CommandHandler::handle` with
     `Command::UpdateSettings { settings: AppSettings { hotkey_binding: "NotAKey+++".into(), .. } }`
     against a `FakeStore`, asserting the result is `Err(_)` and that the fake store's settings were NOT
     updated.
  2. **Confirm failure**: run `cargo test -p clipd commands::` and confirm it fails because today's handler
     returns `Ok(_)` and persists the bad string.
  3. **Green**: add the `parse_binding` call and early-return on error - minimum code to pass.
  4. **Confirm suite green**: run `cargo test -p clipd` (all 36+ existing tests plus the new one).
  5. **Refactor**: none anticipated; keep the validation as a single guard clause at the top of the
     `UpdateSettings` arm.
- A second test asserts a *valid* binding (e.g. `"Ctrl+Shift+V"`) still updates and round-trips through
  `GetSettings`, to guard against the validation guard accidentally rejecting well-formed input.
