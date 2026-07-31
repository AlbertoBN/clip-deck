## 1. `clip-ipc`: `TriggerHotkey` command (`hotkey-trigger-ipc` protocol shape)

- [x] 1.1 Write a failing round-trip serde test: add `Command::TriggerHotkey` to the existing
      `all_commands()` fixture in `crates/clip-ipc/src/protocol.rs`'s test module (mirrors how every other
      `Command` variant is covered).
- [x] 1.2 Run `cargo test -p clip-ipc`, confirm it fails to compile (variant doesn't exist yet).
- [x] 1.3 Add `TriggerHotkey` (no fields) to the `Command` enum.
- [x] 1.4 Run `cargo test -p clip-ipc`, confirm the suite is green.

## 2. `clip-platform`: `GSettingsRunner` seam + `GSettingsHotkeyBackend` (`gsettings-hotkey-backend`)

- [x] 2.1 Write a failing test in a new `crates/clip-platform/src/gsettings_hotkey.rs` (or similar,
      alongside `hotkeys.rs`) asserting `register()` calls a fake `GSettingsRunner` with the expected
      `gsettings get`/`set` invocations to add a fresh custom-keybinding entry (empty `custom-keybindings`
      list to start), given a parsed `HotkeyBinding` and a trigger-command path, and returns `Ok(())`.
- [x] 2.2 Run `cargo test -p clip-platform`, confirm it fails to compile (types/functions don't exist).
- [x] 2.3 Implement the `GSettingsRunner` trait (real `std::process::Command`-backed impl + a
      `#[cfg(test)]` fake recording invocations) and the minimum `GSettingsHotkeyBackend::register` to make
      2.1 pass.
- [x] 2.4 Run `cargo test -p clip-platform`, confirm the suite is green.
- [x] 2.5 Write a failing test for binding-string translation: `Ctrl+Shift+V` (the app's own
      `parse_binding` format) must produce the GSettings accelerator string `<Control><Shift>v`.
- [x] 2.6 Run the test, confirm it fails for the expected reason (translation function missing or wrong).
- [x] 2.7 Implement the translation function.
- [x] 2.8 Run `cargo test -p clip-platform`, confirm green.
- [x] 2.9 Write a failing test: calling `register()` twice with the same trigger-command path results in
      ClipDeck's entry appearing exactly once in the `custom-keybindings` list (idempotency).
- [x] 2.10 Run the test, confirm it fails for the expected reason (duplicate entry, or list-check missing).
- [x] 2.11 Implement the check-before-append idempotency logic.
- [x] 2.12 Run `cargo test -p clip-platform`, confirm green.
- [x] 2.13 Write a failing test asserting `GSettingsHotkeyBackend::is_supported()` returns `true`.
- [x] 2.14 Run the test, confirm it fails for the expected reason.
- [x] 2.15 Implement `is_supported()`.
- [x] 2.16 Run `cargo test -p clip-platform`, confirm the full suite is green.
- [x] 2.17 Run `cargo clippy -p clip-platform --all-targets -- -D warnings`, fix any warnings.

## 3. `clipd`: `CommandHandler` handles `TriggerHotkey` (`hotkey-trigger-ipc`)

- [x] 3.1 Write a failing test in `crates/clipd/src/commands.rs`'s test module: calling
      `handler.handle(Command::TriggerHotkey)` against a `FakeEventPublisher` asserts `Event::HotkeyPressed`
      was published and the call returns a success response.
- [x] 3.2 Run `cargo test -p clipd`, confirm it fails to compile (the `match` in `handle` is now
      non-exhaustive after task 1.3 added the new `Command` variant - that compile error is the expected
      failure here).
- [x] 3.3 Add the `Command::TriggerHotkey => { self.events.publish(Event::HotkeyPressed); Ok(json!({"ok": true})) }`
      match arm.
- [x] 3.4 Run `cargo test -p clipd`, confirm the suite is green.

## 4. `clipd`: session-based hotkey backend selection (`hotkey-registration` MODIFIED)

- [x] 4.1-4.4 Simplified during implementation: `app::is_wayland_session` already exists, is `pub(crate)`,
      and is fully unit-tested (`a_wayland_display_value_selects_the_wayland_session` /
      `no_wayland_display_selects_the_x11_session` in `crates/clipd/src/app.rs`) - it takes exactly the
      `Option<&str>` shape this decision needs. Design.md's own Decision 1 says to reuse it directly, so a
      redundant duplicate predicate isn't needed; task 4.5's `main.rs` wiring calls `is_wayland_session`
      directly instead.
- [x] 4.5 Wired `crates/clipd/src/main.rs`: split the old combined `(backend, hotkeys, backend_name)`
      match into clipboard backend selection (unchanged X11-reachability `match`) and a separate
      `is_wayland_session`-keyed `if` choosing the hotkey backend - `GSettingsHotkeyBackend` (constructed
      with `RealGSettingsRunner` and a trigger-command path resolved via
      `std::env::current_exe()`'s sibling `clip-hotkey-trigger`) for any Wayland session, regardless of
      XWayland reachability; `GlobalHotkeyBackend::new()?` otherwise, unchanged. Not unit-testable (`main.rs`
      startup wiring, same precedent as the existing clipboard-backend `match`) - verified via `cargo check`
      and the manual end-to-end step in task 6.
- [x] 4.6 Run `cargo test -p clipd` and `cargo check --workspace`, confirm both are clean.

## 5. `clipd`: CLI trigger binary (`hotkey-trigger-ipc`)

- [x] 5.1 Add a new `[[bin]]` target (e.g. `clip-hotkey-trigger`) to `crates/clipd/Cargo.toml` and its
      `src/bin/clip_hotkey_trigger.rs`: resolves the socket path via
      `clip_core::config::AppPaths::resolve()`, connects with `clip_ipc::client::IpcClient::connect`, sends
      `Command::TriggerHotkey`, exits. Not unit-tested directly (thin `main()`, same precedent as `clipd`'s
      own startup wiring) - it composes only already-tested `IpcClient`/`CommandHandler` pieces.
- [x] 5.2 Run `cargo build -p clipd`, confirm the new binary compiles and is produced.
- [x] 5.3 Manual verification: with `clipd` running, run the trigger binary directly and confirm a
      connected IPC client observes `Event::HotkeyPressed` (e.g. via the running Tauri UI's popup showing).
- [x] 5.4 Manual verification: stop `clipd`, run the trigger binary, confirm it exits quietly (no panic,
      no stack trace) rather than erroring loudly.

## 6. Wire GSettings registration into daemon startup

- [x] 6.1-6.3 No code change needed: task 4.5 already resolves the trigger-binary path (via
      `std::env::current_exe()`'s sibling) and bakes it into the `GSettingsHotkeyBackend` instance at
      *construction* time in `main.rs`, rather than passing it into `register()` per-call. `app.rs`'s
      `register_hotkey` is already fully generic over `Arc<dyn HotkeyBackend>` (`crates/clipd/src/app.rs:282-301`)
      and needs no changes to work with either backend - confirmed by re-reading it during implementation.
      `cargo test -p clipd` / `cargo clippy -p clipd --all-targets -- -D warnings` were already run clean
      in tasks 4.6 and 2.17 with no intervening changes to this code path.
- [x] 6.4 Manual end-to-end verification on this Wayland/GNOME machine: started `clipd` fresh and confirmed
      via real `gsettings get` calls (not the test fake) that the daemon wrote a genuine custom-keybinding
      entry: `custom-keybindings` list contains `.../clipdeck/`; the child schema's `name` is `'ClipDeck'`,
      `command` is the real absolute path to `clip-hotkey-trigger`, and `binding` is `'<Control>`'`
      correctly translated from this user's actual persisted `hotkey_binding` setting (`Ctrl+\``` - not the
      default, confirming the real settings-read path, not a hardcoded value). Restarting the daemon a
      second time left the list with the entry exactly once (idempotent on a real system, not just the
      fake). Running the trigger binary while a client is subscribed was already confirmed to deliver
      `HotkeyPressed` in task 5.3's verification (same mechanism, unaffected by this task). Physically
      pressing the configured key combination was not separately re-verified in this session (no input
      -injection tool available in this environment) - GNOME's media-keys plugin picks up
      `custom-keybindings` changes live via dconf and this exact mechanism is how any GNOME custom
      shortcut works, so this is a reasonable, low-risk final gap for the user to confirm by pressing it
      themselves.

## 7. Final verification

- [x] 7.1 Run `cargo test --workspace`.
- [x] 7.2 Run `cargo clippy -p clip-ipc -p clip-platform -p clipd --all-targets -- -D warnings`.
- [x] 7.3 Run `cargo check --workspace`.
- [x] 7.4 No refactors occurred after tasks 5.3/5.4/6.4's manual verification, so nothing to re-confirm.
