## Context

`clip_platform::hotkeys::HotkeyBackend` (`crates/clip-platform/src/hotkeys.rs`) currently has two
implementations: `GlobalHotkeyBackend` (real, `XGrabKey`-based via the `global-hotkey` crate - a
background thread blocks on `GlobalHotKeyEvent::receiver().recv()` and invokes the registered callback
synchronously when a grabbed key fires) and `UnsupportedHotkeyBackend` (always returns
`Err(HotkeyError::Unsupported)`, `is_supported() == false`).

`clipd::main` (`crates/clipd/src/main.rs:36-52`) picks between backends using the same "is an X11
connection reachable?" branch it uses for the clipboard `Backend`:

```rust
match app::X11DaemonBackend::connect() {
    Ok(x11_backend) => (Arc::new(x11_backend), Arc::new(GlobalHotkeyBackend::new()?), "x11"),
    Err(_) if app::is_wayland_session(..) => (Arc::new(WaylandDaemonBackend::connect()?), Arc::new(UnsupportedHotkeyBackend::new()), "wayland"),
    Err(x11_error) => return Err(x11_error),
}
```

On GNOME/Mutter with XWayland present (this project's actual dev machine, and the common Ubuntu desktop
case), `X11DaemonBackend::connect()` succeeds, so `GlobalHotkeyBackend` is selected. Its `register()` call
succeeds (the `XGrabKey` request is accepted by the X server), but Mutter does not forward global key
events to XWayland clients' passive grabs, so the callback never fires. This is silent: no error, no log,
just a hotkey that appears registered but never triggers.

`clipd::app::register_hotkey` (`crates/clipd/src/app.rs:282-301`) is the caller of `HotkeyBackend::register`:

```rust
fn register_hotkey(store: &dyn Store, hotkeys: &Arc<dyn HotkeyBackend>, events: Arc<dyn EventPublisher>) {
    // ...loads settings, parses binding...
    if let Err(error) = hotkeys.register(binding, Box::new(move || events.publish(Event::HotkeyPressed))) {
        tracing::warn!(%error, "failed to register global hotkey; ...");
    }
}
```

`CommandHandler` (`crates/clipd/src/commands.rs`) already holds `events: Arc<dyn EventPublisher>` directly
- it does not currently hold a reference to the `HotkeyBackend` at all.

## Goals / Non-Goals

**Goals:**
- Make the global hotkey actually work on GNOME/Mutter Wayland sessions (with or without XWayland
  reachable), using GNOME's own custom-keybinding mechanism so the compositor delivers the keypress.
- Keep `GlobalHotkeyBackend`'s behavior and tests on true native X11 sessions completely unchanged.
- Keep the existing `HotkeyBackend` trait signature unchanged, so `GlobalHotkeyBackend`,
  `UnsupportedHotkeyBackend`, and their existing tests need no changes.

**Non-Goals:**
- Live re-registration when `UpdateSettings` changes the binding mid-run (existing precedent: takes effect
  on next restart, for both backends).
- The `org.freedesktop.portal.GlobalShortcuts` XDG portal, or non-GNOME desktop environments.
- Detecting "XGrabKey silently doesn't deliver" at runtime and auto-falling-back; the fix is to stop
  choosing `GlobalHotkeyBackend` for Wayland sessions in the first place, not to retroactively detect the
  failure.

## Decisions

### 1. Backend selection keys off session type, not X11 reachability, for hotkeys specifically
`main.rs`'s hotkey backend choice changes from "X11 reachable?" to "is this a Wayland session
(`app::is_wayland_session`)?": Wayland session → new `GSettingsHotkeyBackend`; otherwise (true X11,
`WAYLAND_DISPLAY` unset) → `GlobalHotkeyBackend`, unchanged. The clipboard `Backend` selection (`X11DaemonBackend`
vs `WaylandDaemonBackend`) is untouched - it's a separate `match` arm producing a separate value, and its
X11-first reasoning remains correct for clipboard selections.

**Alternative considered:** keep `GlobalHotkeyBackend` for the XWayland-reachable case and have it detect
delivery failure at runtime (e.g. a timeout with no events ever received) and fall back. Rejected: there's
no reliable positive signal that a grab "isn't delivering" versus "the user just hasn't pressed it yet" -
this would require guessing a timeout, which is fragile and untestable in a meaningful way. Session-type
detection is the same signal already trusted for the clipboard-backend decision, deterministic, and
already covered by an existing helper function and its tests.

### 2. `GSettingsHotkeyBackend::register()`'s callback parameter is intentionally unused
Resolving the trait-shape mismatch flagged in the proposal: `register(&self, binding, callback)`'s
`callback` is accepted (so `register_hotkey`'s call site doesn't need per-backend special-casing) but is
**not stored or invoked** by this backend. Actual hotkey delivery for this backend is fully decoupled from
`register()` and instead flows: GNOME runs the trigger CLI on keypress → CLI sends `Command::TriggerHotkey`
over the existing Unix socket → `CommandHandler` (which already owns `events: Arc<dyn EventPublisher>`)
publishes `Event::HotkeyPressed` directly - no `HotkeyBackend` involved at all on that path.
`register()`'s only job for this backend is writing the GSettings custom-keybinding entry; its `Result`
reflects whether that GSettings write succeeded, not whether a future keypress will fire.

**Alternative considered:** extend `HotkeyBackend` with a `fn trigger(&self)` method, store the callback
in `GSettingsHotkeyBackend`, and have `CommandHandler` hold an `Arc<dyn HotkeyBackend>` to call `.trigger()`
on `TriggerHotkey`, so the callback is genuinely invoked end-to-end. Rejected for this change: it adds a
trait method every current and future `HotkeyBackend` impl must carry (dead weight for
`GlobalHotkeyBackend`/`UnsupportedHotkeyBackend`), and threads a new dependency (`Arc<dyn HotkeyBackend>`)
into `CommandHandler` purely to re-derive something `CommandHandler` can already do directly with the
`EventPublisher` it holds. Documented as a deliberate, explicit trade-off (see Risks) rather than left
implicit.

### 3. GSettings access shells out to the `gsettings` CLI behind a small seam trait, not a `gio`/`glib` binding
A `GSettingsRunner` trait (real impl: `std::process::Command::new("gsettings")...`; fake impl: records
invocations) lets tests assert exactly what `gsettings get`/`set` calls a registration would make, without
touching real dconf state in `cargo test`, per this repo's "prefer fakes over real OS state" testing rule.

**Alternative considered:** the `gio` crate's native `gio::Settings` bindings. Rejected: pulls in
glib/gobject FFI as a new dependency category none of this workspace's crates currently use, for a
one-time, infrequent (startup-only) read-modify-write of a handful of string values - shelling out to a
CLI that's guaranteed present on any GNOME session is simpler and keeps the same "prefer fakes" testability
via the seam trait either way.

### 4. Registration is idempotent - safe to call on every daemon startup
`register()` reads the current `custom-keybindings` list, only appends ClipDeck's path if not already
present, and always overwrites `name`/`command`/`binding` at that path to the current values. No
unregistration happens on daemon shutdown (mirrors `GlobalHotkeyBackend`, which also relies on process
death releasing its `XGrabKey` grab rather than an explicit unregister call). A stale entry pointing at a
stopped daemon just means the trigger CLI's `IpcClient::connect` fails with `DaemonNotRunning` and exits
silently - harmless, and consistent with "hotkey registration failure degrades gracefully."

### 5. New `Command::TriggerHotkey` and CLI trigger binary
`clip-ipc::protocol::Command` gains `TriggerHotkey` (no fields - mirrors `Command::PauseCapture`-style
simple commands, minus the payload). `CommandHandler::handle` gains a match arm publishing
`Event::HotkeyPressed` and returning `Ok(json!({"ok": true}))`, following the exact existing pattern of
every other simple command. `clipd`'s `Cargo.toml` gains a new `[[bin]]` (e.g. `clip-hotkey-trigger`) whose
`main()` resolves the socket path via `clip_core::config::AppPaths::resolve()` (same helper `clipd`/
`clip-ui-tauri` already use), connects via `clip_ipc::client::IpcClient::connect`, sends `TriggerHotkey`,
and exits - GNOME's custom keybinding's `command` field points at this binary's installed path.

No new authorization boundary: any local process able to reach the Unix socket can already issue any
`Command` (existing "local-only auth" model, gated by socket file permissions) - `TriggerHotkey` adds
nothing more sensitive than `PasteClip` or `ClearHistory` already expose.

### Test strategy per component
- **`clip-platform` (`GSettingsHotkeyBackend`)**: red - failing unit test asserting `register()` calls the
  `GSettingsRunner` fake with the expected `get`/`set` arguments (list-append logic, binding-string
  translation from `HotkeyBinding` to `<Control><Shift>v`-style syntax, idempotency when the path is
  already present in the list) and returns `Ok(())`; a separate failing test for `is_supported() == true`.
  Green - minimum `GSettingsRunner`-backed implementation. No test touches real dconf.
- **`clip-ipc` (`TriggerHotkey`)**: red - failing round-trip serde test added to the existing
  `all_commands()` fixture (mirrors how every other `Command` variant is covered). Green - add the variant.
- **`clipd` (`CommandHandler`)**: red - failing test calling `handle(Command::TriggerHotkey)` against a
  `FakeEventPublisher` asserting `Event::HotkeyPressed` was published. Green - add the match arm.
- **`clipd` (backend selection)**: red - failing unit test for whatever pure function encapsulates "which
  hotkey backend for this session" (extracted similarly to the existing `is_wayland_session` helper, so
  it's unit-testable without a real display server), covering: Wayland session → GSettings backend
  selected; native X11 (no `WAYLAND_DISPLAY`) → `GlobalHotkeyBackend` selected. Green - implement the
  selection function and wire it into `main.rs`.
- **CLI trigger binary**: thin enough (resolve path, connect, send one command, exit) that it is not
  unit-tested directly, consistent with this repo's precedent for other thin binary `main()`s (e.g.
  `clipd`'s own `main.rs` startup wiring) - covered instead by the already-unit-tested
  `IpcClient`/`CommandHandler` pieces it composes, plus manual verification (register the real GSettings
  keybinding, press it, confirm the popup shows) since it's the one piece that genuinely can't be faked.

## Risks / Trade-offs

- **The unused `callback` parameter in `GSettingsHotkeyBackend::register()` is a real trait-fit wart** →
  Mitigated by an explicit doc comment on the impl explaining why, plus this design doc recording the
  alternative that was rejected and why, so it reads as a deliberate choice instead of an oversight if
  revisited later.
- **GSettings schema (`org.gnome.settings-daemon.plugins.media-keys.custom-keybindings`) is GNOME-specific**
  → Explicit non-goal; `is_supported()` and registration only ever get reached via the Wayland-session
  branch, which this project already treats as GNOME/Mutter-specific elsewhere (see
  `clipd-wayland-backend`'s own scoping).
- **Stale GSettings entry if ClipDeck is uninstalled without cleanup** → Accepted trade-off, same class of
  leftover state as any app that registers a GNOME custom keybinding; out of scope for this change (no
  uninstall/packaging story exists yet for this project regardless).
- **`gsettings` CLI must be on `PATH`** → It ships with `gnome-settings-daemon`, present on any GNOME
  session by definition; if missing, `register()` returns an `Err` that degrades gracefully exactly like
  any other registration failure (existing `hotkey-registration` requirement already covers this).
