## 1. Scaffolding

- [x] 1.1 Run `cargo tauri init` inside `crates/clip-ui-tauri` to generate `build.rs`, `tauri.conf.json`,
      and the React/TS `src/` frontend (Vite template), replacing the current bare `main.rs` stub.
- [x] 1.2 Add Vitest + React Testing Library to the generated frontend's `package.json` dev dependencies
      and wire an `npm test` script.
- [x] 1.3 Add `ammonia` to `crates/clip-ui-tauri/Cargo.toml`.
- [x] 1.4 Verify `cargo check -p clip-ui-tauri` and `npm run build` both succeed on the freshly scaffolded,
      still-empty app before adding any feature code.

## 2. IPC bridge (`ui-ipc-state`, Rust side)

- [x] 2.1 Write a failing Rust test calling a `search_clips` Tauri-command function directly against a
      fake `clip-ipc::Client` and asserting it returns the fake's canned clip list.
- [x] 2.2 Run `cargo test -p clip-ui-tauri` and confirm failure.
- [x] 2.3 Implement the managed `clip-ipc::Client` state and the `#[tauri::command]` wrappers for the
      commands the UI needs (`search_clips`, `get_clip`, `paste_clip`, `pin_clip`, `assign_group`,
      `delete_clip`, `clear_history`, `list_groups`, `save_rule`, `delete_rule`, `get_settings`,
      `update_settings`, `get_diagnostics`, `pause_capture`) - minimum code to pass.
- [x] 2.4 Write a failing test asserting a fake client's `Err` response is surfaced as an error from the
      command function (not silently swallowed).
- [x] 2.5 Implement error propagation; run `cargo test -p clip-ui-tauri` and confirm green.
- [x] 2.6 Write a failing test asserting daemon events are forwarded to a Tauri event emitter call.
- [x] 2.7 Implement the event-forwarding subscription task.

## 3. Frontend state layer (`ui-ipc-state`, TS side)

- [x] 3.1 Write a failing Vitest test asserting a successful mocked `invoke` resolves the state layer's
      call with the payload, per `specs/ui-ipc-state/spec.md`.
- [x] 3.2 Run `npm test` and confirm failure.
- [x] 3.3 Implement `src/state`'s command-calling wrapper around `invoke` - minimum code to pass.
- [x] 3.4 Write a failing test asserting a mocked `Err` rejects the call with the error message.
- [x] 3.5 Implement rejection handling.
- [x] 3.6 Write failing tests for `ClipCaptured` append, `ClipUpdated` update, and `ClipDeleted` removal
      against a mocked `listen`.
- [x] 3.7 Implement the event-subscription reducers.
- [x] 3.8 Write a failing test asserting a "daemon not running" error surfaces a distinct disconnected
      state without throwing.
- [x] 3.9 Implement the disconnected-state handling; run `npm test` and confirm green.

## 4. Popup picker (`popup-picker-ui`)

- [x] 4.1 Write failing component tests for auto-focus-on-open and unchanged backend ordering, per
      `specs/popup-picker-ui/spec.md`.
- [x] 4.2 Run `npm test` and confirm failure.
- [x] 4.3 Implement `src/views/popup`'s initial render and empty-query search - minimum code to pass.
- [x] 4.4 Write failing tests for arrow-down moving selection and arrow-up not wrapping past the top.
- [x] 4.5 Implement keyboard navigation.
- [x] 4.6 Write a failing test asserting Enter issues `PasteClip` and closes the popup.
- [x] 4.7 Implement Enter-to-paste.
- [x] 4.8 Write a failing test asserting typing issues a debounced `SearchClips` with the typed query.
- [x] 4.9 Implement incremental search; run `npm test` and confirm green.

## 5. Manager window (`manager-window-ui`)

- [x] 5.1 Write failing component tests for filter-change re-query, inline pin/delete/assign-group actions,
      per `specs/manager-window-ui/spec.md`.
- [x] 5.2 Run `npm test` and confirm failure.
- [x] 5.3 Implement `src/views/manager`'s filter bar and list actions - minimum code to pass.
- [x] 5.4 Write a failing test for bulk clear with a selected scope removing only the matching clips from
      the rendered list as `ClipDeleted` events arrive.
- [x] 5.5 Implement bulk clear.
- [x] 5.6 Write a failing test asserting a `ClipCaptured` event live-updates the visible list.
- [x] 5.7 Implement live update via the state layer's event subscription; run `npm test` and confirm green.

## 6. Preview pane (`preview-pane-ui`)

- [x] 6.1 Write a failing Rust test asserting `ammonia` sanitization strips a `<script>` tag but preserves
      benign formatting tags, per `specs/preview-pane-ui/spec.md`.
- [x] 6.2 Run `cargo test -p clip-ui-tauri` and confirm failure.
- [x] 6.3 Implement the sanitizing Tauri command that returns preview-ready HTML - minimum code to pass.
- [x] 6.4 Write failing component tests for full-untruncated-text preview and image-preview-from-blob-path
      rendering.
- [x] 6.5 Implement `src/` preview components; run `npm test` and confirm green.

## 7. Tray integration (`tray-integration`)

- [x] 7.1 Write a failing Rust test calling the tray menu-event handler directly for the "Pause capture"
      item against a fake client, asserting it issues `PauseCapture { paused: true }`, per
      `specs/tray-integration/spec.md`.
- [x] 7.2 Run `cargo test -p clip-ui-tauri` and confirm failure.
- [x] 7.3 Implement the tray menu and its event handler using Tauri's tray APIs - minimum code to pass.
- [x] 7.4 Write a failing test asserting a "Quit" menu selection exits the application.
- [x] 7.5 Implement the quit handler.
- [x] 7.6 Write failing tests asserting a `CapturePaused` event updates the tray's displayed state and
      toggles the menu label between "Pause capture"/"Resume capture".
- [x] 7.7 Implement tray state reflection; run `cargo test -p clip-ui-tauri` and confirm green.

## 8. Settings (`settings-ui`)

- [x] 8.1 Write failing component tests for valid-binding-saves and invalid-binding-shows-error, per
      `specs/settings-ui/spec.md`.
- [x] 8.2 Run `npm test` and confirm failure.
- [x] 8.3 Implement `src/views/settings`'s hotkey editor - minimum code to pass.
- [x] 8.4 Write a failing test asserting an unsupported capability from `GetDiagnostics` renders with an
      explicit unsupported indicator rather than being omitted.
- [x] 8.5 Implement the diagnostics display.
- [x] 8.6 Write failing tests for rule creation issuing `SaveRule` and deletion issuing `DeleteRule` plus
      removing it from the rendered list.
- [x] 8.7 Implement the rules management UI; run `npm test` and confirm green.

## 9. Crate-level and end-to-end verification

- [x] 9.1 Run `cargo test -p clip-ui-tauri` and confirm every Rust-side test from sections 2, 6, and 7
      passes.
- [x] 9.2 Run `npm test` inside `crates/clip-ui-tauri` and confirm every frontend test from sections 3-6
      and 8 passes.
- [x] 9.3 Run `cargo check --workspace` and confirm the rest of the workspace still compiles.
- [x] 9.4 Run `cargo clippy -p clip-ui-tauri -- -D warnings` and fix any lints introduced by this change.
- [ ] 9.5 Manually verify against a real running `clipd`: popup opens focused, search-then-Enter pastes
      into a real previously-focused window, tray pause/resume works, and settings changes persist across
      an app restart. Record the result in the PR description.
