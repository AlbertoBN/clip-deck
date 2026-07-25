## 1. `clip-platform`: expose `resolve_paste_text` for reuse

- [ ] 1.1 Change `resolve_paste_text` in `crates/clip-platform/src/paste.rs` from private to `pub fn` -
      visibility-only change, no behavior change. Run `cargo test -p clip-platform` and confirm the
      existing Auto/PlainText selection tests still pass unmodified (they already characterize its
      behavior; no new test needed for this step).

## 2. `clipd`: session-detection predicate (`daemon-lifecycle`)

- [ ] 2.1 Write a failing test in `crates/clipd/src/app.rs` asserting `is_wayland_session(Some("wayland-0"))`
      is `true` and `is_wayland_session(None)` is `false`, per `specs/daemon-lifecycle/spec.md`.
- [ ] 2.2 Run `cargo test -p clipd app::` and confirm it fails (function does not exist).
- [ ] 2.3 Implement `pub(crate) fn is_wayland_session(wayland_display: Option<&str>) -> bool` in
      `crates/clipd/src/app.rs` - minimum code to pass (a presence check).
- [ ] 2.4 Run `cargo test -p clipd` and confirm the full suite is green.

## 3. `clipd`: `WaylandDaemonBackend` composition (`wayland-daemon-backend`)

- [ ] 3.1 Add `pub struct WaylandDaemonBackend` to `crates/clipd/src/app.rs`, composing
      `clip_platform::wayland::WaylandBackend<clip_platform::wayland::RealWaylandConnection>` for capture
      and `clip_platform::focus::UnsupportedFocusTracker` for focus, with a `connect() -> anyhow::Result<Self>`
      constructor mirroring `X11DaemonBackend::connect()` (wrap `RealWaylandConnection::connect()`'s and
      `WaylandBackend::new()`'s errors with `anyhow::anyhow!`). Not test-driven - needs a live Wayland
      compositor, same carve-out as `X11DaemonBackend::connect()`/`RealWaylandConnection` (see design.md's
      Test strategy).
- [ ] 3.2 Implement `Backend` for `WaylandDaemonBackend`: `start`/`focused_app`/`capabilities` delegate
      straight through to the composed pieces; `simulate_paste` resolves the paste text via
      `clip_platform::paste::resolve_paste_text` and calls the Wayland connection's `write_selection`
      directly, always returning `Ok(())` (clipboard-only, no key synthesis attempted), per
      `specs/wayland-daemon-backend/spec.md`. Thin composition glue over already-tested pieces - no new
      fake-driven unit test (see design.md's Test strategy).
- [ ] 3.3 Run `cargo check -p clipd` and confirm `WaylandDaemonBackend` compiles and satisfies `Backend`.

## 4. `clipd`: backend selection in `main.rs`

- [ ] 4.1 In `crates/clipd/src/main.rs`, read `WAYLAND_DISPLAY` via `std::env::var_os`, pass its presence to
      `app::is_wayland_session`, and branch: under Wayland, construct
      `app::WaylandDaemonBackend::connect()` and `clip_platform::hotkeys::UnsupportedHotkeyBackend::new()`
      with `backend_name = "wayland"`; otherwise construct `app::X11DaemonBackend::connect()` and
      `clip_platform::hotkeys::GlobalHotkeyBackend::new()` with `backend_name = "x11"` (today's behavior,
      unchanged). Wrap `WaylandDaemonBackend::connect()`'s underlying `Box<dyn Error>` with
      `anyhow::anyhow!` where needed so `?` type-checks.
- [ ] 4.2 Run `cargo check -p clipd` and confirm `main.rs` compiles with both branches type-checking
      identically against `app::run`'s existing `backend`/`hotkeys`/`backend_name` parameters.

## 5. Crate-level and manual verification

- [ ] 5.1 Run `cargo test --workspace` and confirm every test passes, including the new tests from
      sections 1-2.
- [ ] 5.2 Run `cargo check --workspace` and confirm the rest of the workspace still compiles.
- [ ] 5.3 Run `cargo clippy -p clipd -p clip-platform --all-targets -- -D warnings` and fix any lints
      introduced by this change.
- [ ] 5.4 Manually verify on the user's real Wayland session (`sway` or another `wlr-data-control`-
      supporting compositor): `cargo run -p clipd` starts successfully with `WAYLAND_DISPLAY` set, copying
      text is captured (visible via the UI or `SearchClips`), `GetDiagnostics` reports `backend: "wayland"`
      with `hotkeys: false, focus_detection: false`, and pasting a clip places it on the clipboard (manual
      Ctrl+V completes the paste). Record the result in the PR description.
- [ ] 5.5 Manually verify the X11 path is unaffected: on an X11 session (or with `WAYLAND_DISPLAY`
      unset), `cargo run -p clipd` still selects `X11DaemonBackend` and behaves exactly as before this
      change.
