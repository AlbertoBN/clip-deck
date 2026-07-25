## 1. `clip-platform`: expose `resolve_paste_text` for reuse

- [x] 1.1 Change `resolve_paste_text` in `crates/clip-platform/src/paste.rs` from private to `pub fn` -
      visibility-only change, no behavior change. Run `cargo test -p clip-platform` and confirm the
      existing Auto/PlainText selection tests still pass unmodified (they already characterize its
      behavior; no new test needed for this step).

## 2. `clipd`: session-detection predicate (`daemon-lifecycle`)

- [x] 2.1 Write a failing test in `crates/clipd/src/app.rs` asserting `is_wayland_session(Some("wayland-0"))`
      is `true` and `is_wayland_session(None)` is `false`, per `specs/daemon-lifecycle/spec.md`.
- [x] 2.2 Run `cargo test -p clipd app::` and confirm it fails (function does not exist).
- [x] 2.3 Implement `pub(crate) fn is_wayland_session(wayland_display: Option<&str>) -> bool` in
      `crates/clipd/src/app.rs` - minimum code to pass (a presence check).
- [x] 2.4 Run `cargo test -p clipd` and confirm the full suite is green.

## 3. `clipd`: `WaylandDaemonBackend` composition (`wayland-daemon-backend`)

- [x] 3.1 Add `pub struct WaylandDaemonBackend` to `crates/clipd/src/app.rs`, composing
      `clip_platform::wayland::WaylandBackend<clip_platform::wayland::RealWaylandConnection>` for capture
      and `clip_platform::focus::UnsupportedFocusTracker` for focus, with a `connect() -> anyhow::Result<Self>`
      constructor mirroring `X11DaemonBackend::connect()` (wrap `RealWaylandConnection::connect()`'s and
      `WaylandBackend::new()`'s errors with `anyhow::anyhow!`). Not test-driven - needs a live Wayland
      compositor, same carve-out as `X11DaemonBackend::connect()`/`RealWaylandConnection` (see design.md's
      Test strategy).
- [x] 3.2 Implement `Backend` for `WaylandDaemonBackend`: `start`/`focused_app`/`capabilities` delegate
      straight through to the composed pieces; `simulate_paste` resolves the paste text via
      `clip_platform::paste::resolve_paste_text` and calls the Wayland connection's `write_selection`
      directly, always returning `Ok(())` (clipboard-only, no key synthesis attempted), per
      `specs/wayland-daemon-backend/spec.md`. Thin composition glue over already-tested pieces - no new
      fake-driven unit test (see design.md's Test strategy).
- [x] 3.3 Run `cargo check -p clipd` and confirm `WaylandDaemonBackend` compiles and satisfies `Backend`.

## 4. `clipd`: backend selection in `main.rs`

- [x] 4.1 In `crates/clipd/src/main.rs`, read `WAYLAND_DISPLAY` via `std::env::var_os`, pass its presence to
      `app::is_wayland_session`, and branch: under Wayland, construct
      `app::WaylandDaemonBackend::connect()` and `clip_platform::hotkeys::UnsupportedHotkeyBackend::new()`
      with `backend_name = "wayland"`; otherwise construct `app::X11DaemonBackend::connect()` and
      `clip_platform::hotkeys::GlobalHotkeyBackend::new()` with `backend_name = "x11"` (today's behavior,
      unchanged). Wrap `WaylandDaemonBackend::connect()`'s underlying `Box<dyn Error>` with
      `anyhow::anyhow!` where needed so `?` type-checks.
- [x] 4.2 Run `cargo check -p clipd` and confirm `main.rs` compiles with both branches type-checking
      identically against `app::run`'s existing `backend`/`hotkeys`/`backend_name` parameters.

## 5. Crate-level and manual verification

- [x] 5.1 Run `cargo test --workspace` and confirm every test passes, including the new tests from
      sections 1-2.
- [x] 5.2 Run `cargo check --workspace` and confirm the rest of the workspace still compiles.
- [x] 5.3 Run `cargo clippy -p clipd -p clip-platform --all-targets -- -D warnings` and fix any lints
      introduced by this change.
- [x] 5.4 Manually verify on the user's real Wayland session: attempted on the requesting user's own
      machine (GNOME Shell/Mutter). Result: `WaylandDaemonBackend::connect()` fails outright
      (`this compositor does not support the wlr-data-control protocol` - Mutter is not wlroots-based),
      and that machine's `$DISPLAY` is also set and reachable via XWayland - see section 6's amendment,
      which reorders selection to try X11 first so this specific session keeps working. Capture/paste
      on a genuine wlroots compositor (e.g. sway, with no XWayland/X11 fallback available) remains
      unverified on real hardware in this session - do that verification if/when such a compositor is
      available, per `wayland-clipboard-capture`'s own carve-out.
- [x] 5.5 Manually verify the X11 path is unaffected: confirmed on the requesting user's own machine
      (`$WAYLAND_DISPLAY` set, `$DISPLAY` reachable via XWayland) - `cargo run -p clipd` starts cleanly
      with no error and keeps running, selecting `X11DaemonBackend` exactly as before this change (see
      section 6's amendment for why this now requires the X11-first reordering rather than falling out
      of the original WAYLAND_DISPLAY-based selection).

## 6. Amendment: prefer X11/XWayland over native Wayland when reachable

Discovered during task 5.4's manual verification (see `proposal.md`'s "Amendment" note under Impact and
`design.md`'s Decisions/Risks): selecting Wayland purely on `WAYLAND_DISPLAY` presence regresses sessions
(e.g. GNOME/Mutter) where XWayland already provides a working X11 connection, since `WaylandDaemonBackend`
fails outright on non-wlroots compositors.

- [x] 6.1 Reorder `crates/clipd/src/main.rs`'s selection: attempt `app::X11DaemonBackend::connect()`
      first, unconditionally. On success, select X11 + `GlobalHotkeyBackend` + `backend_name = "x11"`
      (unchanged). On failure, check `app::is_wayland_session` and, if true, select
      `app::WaylandDaemonBackend::connect()` + `UnsupportedHotkeyBackend` + `backend_name = "wayland"`;
      if false, propagate the original X11 connection error (matching pre-existing no-display behavior).
      Not test-driven - same live-connection carve-out as the rest of `main.rs`'s backend construction.
- [x] 6.2 Update `specs/daemon-lifecycle/spec.md`'s requirement and scenarios to describe reachability-
      based, X11-preferring selection instead of session-type-based selection.
- [x] 6.3 Run `cargo check -p clipd` and `cargo clippy -p clipd --all-targets -- -D warnings` and confirm
      both are clean after the reorder.
- [x] 6.4 Re-run the manual smoke test on the requesting user's machine: `cargo run -p clipd` now starts
      and stays running (no immediate error), confirming the X11-first fallback restores the previously-
      working behavior on this GNOME/Mutter session.
