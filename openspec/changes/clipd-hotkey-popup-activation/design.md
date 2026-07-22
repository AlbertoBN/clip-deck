## Context

Two halves of this workflow already exist in isolation but were never connected:

- **Protocol/forwarding**: `Event::HotkeyPressed` is already a variant in `clip-ipc`'s `Event` enum, and
  `clip-ui-tauri`'s `spawn_event_forwarder` + `lib.rs`'s event-forwarding closure already forward every
  `Event` (including `HotkeyPressed`) from the IPC client to the frontend's `"daemon-event"` channel - this
  is covered by an existing passing test (`client.rs`'s `daemon_events_are_forwarded_to_the_emitter`).
  Nothing publishes `HotkeyPressed` in practice, though: `clipd`'s `app::run` never touches
  `clip_platform::hotkeys` at all.
- **Window/UI**: `App.tsx` already routes to `<Popup />` when the current window's label is `"popup"`, and
  `Popup.tsx` already auto-focuses its input and issues an empty-query search - but only on component
  mount. `tauri.conf.json` only declares one window (`"main"`), so no `"popup"`-labeled window exists to
  route into, and nothing shows/focuses any window in response to `HotkeyPressed`.

This change closes both gaps: `clipd` starts actually registering the hotkey and publishing the event, and
`clip-ui-tauri` gets a real (initially hidden) `"popup"` window that's shown/focused when the event arrives,
with `Popup.tsx` updated to re-run its open-time behavior on every show, not just first mount.

## Goals / Non-Goals

**Goals:**
- `clipd` registers a `clip_platform::hotkeys::HotkeyBackend` binding parsed from the persisted
  `hotkey_binding` setting at startup, and publishes `Event::HotkeyPressed` via the existing
  `EventPublisher` when the hotkey fires.
- Registration failure degrades gracefully (logged, daemon keeps running) rather than crashing startup -
  matching `CLAUDE.md`'s "degrade and surface status in Settings instead of failing silently" rule for
  platform integration.
- A `"popup"` Tauri window exists, initially hidden, and is shown+focused when `HotkeyPressed` arrives.
- The popup's focus-and-empty-search behavior runs every time it's shown, not only once per process
  lifetime.

**Non-Goals:**
- No live re-registration of the hotkey when `UpdateSettings` changes the binding while the daemon is
  running - consistent with `clipd-hotkey-validation`'s design, a new binding takes effect on next daemon
  restart. Live re-registration (unregister old binding, register new one, from within the running
  `CommandHandler`) is a reasonable, separable follow-up if this turns out to matter in practice.
  Interestingly, changing anyway requires unregistering the old hotkey with the same `HotkeyBackend`
  instance that's registered before the borrow into `CommandHandler` is possible today - out of scope here.
- No Escape-to-dismiss keybinding for the popup (not previously specified in `popup-picker-ui`; a
  reasonable future addition but not part of closing this specific gap).
- No Wayland hotkey path - `clip-platform-wayland-adapter` is a separate future milestone.

## Decisions

- **`app::run` takes a new `hotkeys: Arc<dyn clip_platform::hotkeys::HotkeyBackend>` parameter**, mirroring
  how `backend: Arc<dyn Backend>` is already injected, so tests can substitute a fake without touching the
  real `global-hotkey` crate (which needs a live desktop session). `main.rs` constructs
  `clip_platform::hotkeys::GlobalHotkeyBackend::new()` for the real run, matching how `X11DaemonBackend`
  is constructed there today.
- **Registration happens once, after settings are loadable, before `server.run_with_shutdown`**: read
  `store.get_settings()?.hotkey_binding`, parse via `clip_platform::hotkeys::parse_binding`, and call
  `hotkeys.register(binding, callback)` where `callback` clones the already-constructed
  `Arc<dyn EventPublisher>` and calls `events.publish(Event::HotkeyPressed)`. A parse or registration
  error is logged via `tracing::warn!` and startup continues - no `AppError` variant added, since a
  bad/unavailable hotkey must not take down clipboard capture.
- **`clipd` defines its own `FakeHotkeyBackend`** in `app::fakes` (implementing `clip_platform`'s public
  `HotkeyBackend` trait) rather than reusing `clip-platform`'s internal fake, which is
  `#[cfg(test)] pub(crate)`-scoped and not visible outside that crate - consistent with how `clipd`
  already defines its own `FakeStore`/`FakeBackend`/`FakeEventPublisher` rather than importing
  `clip-store`'s or `clip-platform`'s test doubles.
- **A statically-declared, initially-hidden `"popup"` window** in `tauri.conf.json` (`visible: false`,
  `decorations: false`, `alwaysOnTop: true`, `skipTaskbar: true`), rather than a runtime-created
  `WebviewWindowBuilder` window - simpler, and consistent with how the existing single `"main"` window is
  already declared statically. `App.tsx`'s existing label-based routing needs no changes.
- **The `HotkeyPressed` -> show/focus wiring lives inline in `lib.rs`'s existing event-forwarding closure**,
  calling `app_handle.get_webview_window("popup")` + `.show()`/`.set_focus()`, directly parallel to the
  existing (already-untested) `"show"`/`"hide"` tray-menu arms for the `"main"` window in the same file.
  This thin Tauri-window-API glue is not unit-tested for the same reason those arms aren't: it requires a
  live window system Tauri's test harness doesn't substitute for at this level, so it is verified manually
  (see Task list) rather than with an automated test, matching the project's carve-out for "the small
  surface that can't be faked."
- **`Popup.tsx` re-runs its open behavior on every `HotkeyPressed`, not only on mount**: it already can
  listen on the same `"daemon-event"` channel `Manager.tsx` already listens on (via `@tauri-apps/api/event`
  `listen`), so add a `HotkeyPressed` branch there that re-triggers the focus + empty-query search, in
  addition to keeping the existing mount-time effect (covers the very first time the window is created,
  before any hotkey press if the OS ever shows it eagerly).

### Test strategy

- **`clipd`**: red - failing test in `app.rs`'s test module constructing a `FakeHotkeyBackend`, calling
  `app::run` with it against a `FakeStore` pre-seeded with `hotkey_binding: "Ctrl+Shift+V"`, triggering the
  fake's registered callback directly, and asserting a `HotkeyPressed` event was published via
  `FakeEventPublisher`. Green - implement the registration call in `run`. Confirm `cargo test -p clipd`
  green. A second test asserts an invalid persisted binding (shouldn't normally happen post
  `clipd-hotkey-validation`, but defensively) logs and does not panic/fail startup.
- **`clip-ui-tauri` (frontend)**: red - failing Vitest test in `Popup.test.tsx` asserting that, after
  initial mount and a subsequent mocked `"daemon-event"` of `{ type: 'HotkeyPressed' }`, the search input
  is (re-)focused and an empty-query `search_clips` is issued again (call count increases), mirroring the
  existing `listen` mock pattern from `Manager.test.tsx`. Green - add the `HotkeyPressed` listener branch
  to `Popup.tsx`. Confirm `npm test` green.
- **`tauri.conf.json` / `lib.rs` glue**: no automated test (see Decisions); verified manually per the task
  list, alongside the existing manual-verification task left open by `clip-ui-tauri-shell`.
