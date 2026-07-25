## 1. `clipd`: register hotkey and publish HotkeyPressed (`hotkey-registration`, `daemon-lifecycle`)

- [x] 1.1 Write a failing test in `crates/clipd/src/app.rs`'s test module defining a local
      `FakeHotkeyBackend` (implementing `clip_platform::hotkeys::HotkeyBackend`) in `app::fakes`, calling
      `app::run(...)` with a `FakeStore` pre-seeded with `hotkey_binding: "Ctrl+Shift+V"` and the fake
      hotkey backend, invoking the fake's stored callback directly, and asserting a `HotkeyPressed` event
      was published via `FakeEventPublisher`, per `specs/hotkey-registration/spec.md`.
- [x] 1.2 Run `cargo test -p clipd app::` and confirm it fails (no `hotkeys` param, no registration call
      exists yet).
- [x] 1.3 Add a `hotkeys: Arc<dyn clip_platform::hotkeys::HotkeyBackend>` parameter to `app::run`, and
      implement the registration step (parse `store.get_settings()?.hotkey_binding`, call
      `hotkeys.register(binding, callback)` where the callback publishes `Event::HotkeyPressed`) - minimum
      code to pass. Update `main.rs` to construct `clip_platform::hotkeys::GlobalHotkeyBackend::new()` and
      pass it through.
- [x] 1.4 Write a failing test asserting an invalid/unparseable persisted `hotkey_binding` (or a fake that
      returns a registration error) logs via `tracing` and does NOT prevent `app::run` from binding its IPC
      socket and serving commands.
- [x] 1.5 Implement the graceful-degradation path (log and continue past registration failure) - minimum
      code to pass.
- [x] 1.6 Run `cargo test -p clipd` and confirm the full suite is green.

## 2. `clip-ui-tauri`: popup window definition and show/focus on HotkeyPressed

- [x] 2.1 Add a second, initially-hidden `"popup"` window entry to
      `crates/clip-ui-tauri/src-tauri/tauri.conf.json` (`visible: false`, `decorations: false`,
      `alwaysOnTop: true`, `skipTaskbar: true`, a smaller default size than `"main"`).
- [x] 2.2 In `crates/clip-ui-tauri/src-tauri/src/lib.rs`'s existing event-forwarding closure, add a branch
      for `Event::HotkeyPressed` that fetches the `"popup"` window via `get_webview_window` and calls
      `.show()` + `.set_focus()`, directly parallel to the existing `"show"`/`"hide"` tray-menu arms. This
      glue is Tauri-window-API-only and is not unit tested (see `design.md`'s Test strategy) - it is
      exercised by the manual verification task below instead.
- [x] 2.3 Run `cargo check -p clip-ui-tauri` and `cargo clippy -p clip-ui-tauri --all-targets -- -D
      warnings` to confirm the addition compiles cleanly.

## 3. `clip-ui-tauri` (frontend): popup re-runs open behavior on every HotkeyPressed

- [x] 3.1 Write a failing Vitest test in `Popup.test.tsx` asserting that, after initial mount and a
      subsequently-mocked `"daemon-event"` payload `{ type: 'HotkeyPressed' }` (mirroring the `listen` mock
      pattern already used in `Manager.test.tsx`), the search input is re-focused and a fresh empty-query
      `search_clips` call is issued (call count increases beyond the mount-time call), per
      `specs/popup-picker-ui/spec.md`.
- [x] 3.2 Run `npm test` (inside `crates/clip-ui-tauri`) and confirm it fails (no `daemon-event` listener
      exists in `Popup.tsx` yet).
- [x] 3.3 Implement a `HotkeyPressed` listener in `src/views/popup/Popup.tsx` that re-runs the existing
      focus-and-empty-query-search logic (extract it to a shared function if needed to avoid duplicating
      the mount-effect body) - minimum code to pass.
- [x] 3.4 Run `npm test` and confirm the full frontend suite (including the existing Popup tests) is green.

## 4. Crate-level and manual verification

- [x] 4.1 Run `cargo test -p clipd` and `cargo test -p clip-ui-tauri` and confirm every test from sections
      1 and 3 passes.
- [x] 4.2 Run `cargo check --workspace` and confirm the rest of the workspace still compiles.
- [x] 4.3 Run `cargo clippy -p clipd -p clip-ui-tauri --all-targets -- -D warnings` and fix any lints
      introduced by this change.
- [x] 4.4 Run `npm run build` inside `crates/clip-ui-tauri` and confirm the frontend still type-checks and
      builds with the new window entry in `tauri.conf.json`.
- [ ] 4.5 Manually verify against a real running `clipd` + built app on a live X11 session: pressing the
      configured hotkey shows and focuses the popup with the search field focused and current results,
      pasting hides it, and pressing the hotkey again re-shows it with fresh results. Record the result in
      the PR description (this extends, rather than duplicates, `clip-ui-tauri-shell`'s existing open
      manual-verification task).
