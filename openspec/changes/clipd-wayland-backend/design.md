## Context

`clip-platform-wayland-adapter` already delivered every piece `clipd` needs for a Wayland session:
`wayland::WaylandBackend<C: WaylandConnection>` (capture/watch, text-only), `wayland::RealWaylandConnection`
(the real `wayland-client` binding), `hotkeys::UnsupportedHotkeyBackend`, and `focus::UnsupportedFocusTracker`.
None of it is wired into `clipd` - `main.rs` unconditionally builds `X11DaemonBackend::connect()` and
`GlobalHotkeyBackend::new()`. This change is exactly the "future `clipd` change" both
`clip-platform-wayland-adapter`'s and `clipd-hotkey-popup-activation`'s proposals pointed at.

## Goals / Non-Goals

**Goals:**
- `clipd` runs end-to-end on a Wayland session with `wlr-data-control` support: capture works, paste
  places content on the clipboard, diagnostics report `"wayland"` with honest per-capability flags.
- Session type is detected automatically at startup - no new config/CLI flag required for the common case.
- Zero behavior change on X11: the existing `X11DaemonBackend`/`GlobalHotkeyBackend` path is untouched.

**Non-Goals** (carried over from `clip-platform-wayland-adapter`, not re-litigated here):
- No global hotkey support on Wayland - `capabilities().hotkeys` stays `false`; `main.rs` selects
  `UnsupportedHotkeyBackend`, and `register_hotkey`'s existing degradation path (log + continue) already
  handles that.
- No synthetic paste-key delivery on Wayland - `simulate_paste` places content on the clipboard only; the
  user completes the paste themselves.
- No explicit backend-override flag/env var in this change - if a future need arises (e.g. forcing X11
  under a mixed XWayland session), that's a separate, narrowly-scoped follow-up.

## Decisions

- **Session detection**: check `WAYLAND_DISPLAY` for presence (`std::env::var_os("WAYLAND_DISPLAY")`)
  rather than `XDG_SESSION_TYPE`. `WAYLAND_DISPLAY` is literally the variable `wayland-client`'s own
  `Connection::connect_to_env` reads to find the compositor socket, so it's the more direct signal for
  "can this process actually reach a Wayland compositor" - `XDG_SESSION_TYPE` is a broader session-manager
  label that doesn't guarantee the socket is reachable (nested/manually-launched compositors, mixed
  sessions).
- **Selection is X11-first, not Wayland-first, when both are reachable** (revised from the original
  Wayland-if-`WAYLAND_DISPLAY`-set plan - see the amendment below): `main.rs` always attempts
  `X11DaemonBackend::connect()` first; only on failure does it consult `is_wayland_session` and fall back
  to `WaylandDaemonBackend`. This is deliberate, not an accident of ordering - a successful X11 connection
  works identically whether it's a native X11 session or a Wayland session with XWayland available, and
  covers strictly more capability (hotkeys, focus detection, synthetic paste) than the Wayland backend can
  ever offer. Preferring it whenever it's actually reachable is strictly better for the user than
  preferring Wayland just because `WAYLAND_DISPLAY` happens to be set.

### Amendment (discovered during implementation)

Manual verification (task 5.4) on the requesting user's own machine - GNOME Shell/Mutter, which does not
implement `wlr-data-control` - surfaced that the originally-planned "select on `WAYLAND_DISPLAY` presence
alone" rule was a real regression, not just a theoretical risk: that machine also has `$DISPLAY` set and
reachable via XWayland, so `X11DaemonBackend` connected and worked fine before this change existed at all.
Under the original plan, the same session would have switched straight to `WaylandDaemonBackend`, which
fails to construct at all on Mutter - turning a working session into a broken one.

Fix: reorder the selection to attempt X11 first unconditionally, and only reach for
`is_wayland_session`/`WaylandDaemonBackend` when that X11 attempt fails (see `main.rs`'s updated logic,
task 6 in `tasks.md`). `is_wayland_session`'s own signature and test are unchanged; only how `main.rs` uses
its result changed - from "the" branch condition to a fallback-eligibility check. The
`specs/daemon-lifecycle/spec.md` requirement and scenarios were updated to match (X11-reachable-first,
Wayland-fallback-only-when-X11-unreachable), and are no longer literally "based on session type" in the
original sense - they're based on which backend can actually be reached, preferring X11.
- **The decision itself is a small, pure, directly-testable function** (`fn is_wayland_session(wayland_display:
  Option<&str>) -> bool` in `crates/clipd/src/app.rs`), taking the env value as a parameter rather than
  reading `std::env` itself. This avoids mutating/reading process-wide env vars from a test (a real risk
  already accepted once in this codebase for `AppPaths::resolve`'s own env-var test, not worth repeating
  here) and keeps `main.rs`'s only job as "read the real env var once, call the pure function, branch."
- **`WaylandDaemonBackend`** (`crates/clipd/src/app.rs`) mirrors `X11DaemonBackend`'s composition exactly:
  - `capture: wayland::WaylandBackend<wayland::RealWaylandConnection>` - `start`/`capabilities` delegate
    straight through (`WaylandBackend::capabilities()` already reports the honest
    `hotkeys: false, focus_detection: false` set).
  - `focus: focus::UnsupportedFocusTracker` - `focused_app` delegates straight through (always `None`).
  - `simulate_paste`: a small Wayland-specific implementation, **not** `paste::PasteSimulator<C:
    X11Connection>` reused as-is, since `PasteSimulator` needs `synthesize_key`/`focused_window` from
    `X11Connection` - a trait `WaylandConnection` doesn't (and shouldn't) implement, because no synthetic
    input mechanism is wired for Wayland at all. Instead it resolves the text to paste via
    `clip_platform::paste::resolve_paste_text` (made `pub` - a visibility-only change; its selection
    behavior is already fully covered by `clip-platform`'s existing Auto/PlainText tests, so no new test is
    needed there) and calls the Wayland connection's `write_selection` directly, always succeeding
    (clipboard-only, matching `paste-simulation`'s Wayland scenario from `clip-platform-wayland-adapter`).
  - Alternative considered: give `WaylandConnection` a no-op `synthesize_key`/`focused_window` so
    `PasteSimulator` could be reused unchanged. Rejected - it would let a Wayland compositor silently
    accept a "synthesize key" call that does nothing, undermining the "never assume feature parity, report
    honestly" rule; a dedicated tiny paste path makes the limitation structurally obvious instead.
- **`main.rs` selects the whole (backend, hotkeys, backend_name) triple together**, keyed off
  `is_wayland_session`, rather than mixing backends (e.g. X11 capture with Wayland hotkeys) - the two
  adapters are not designed to be combined, and the PRD doesn't call for it.
- **`RealWaylandConnection::connect()`'s error type doesn't satisfy `anyhow`'s blanket `?` conversion**
  (it returns `Box<dyn std::error::Error>`, not `+ Send + Sync`) - `main.rs` wraps it explicitly
  (`.map_err(|e| anyhow::anyhow!("failed to open Wayland connection: {e}"))?`) rather than changing
  `clip-platform`'s public error type just for this call site.

### Test strategy

- `is_wayland_session` (`clipd`, `app.rs`): red - failing test asserting `is_wayland_session(Some("wayland-0"))
  == true` and `is_wayland_session(None) == false`. Green - implement the one-line predicate. Run
  `cargo test -p clipd app::`.
- `WaylandDaemonBackend::connect()`/real Wayland construction: not unit-tested, same carve-out as
  `X11DaemonBackend::connect()` (needs a live compositor) - covered by this change's manual verification
  task, not a fake-driven test.
- `WaylandDaemonBackend`'s `Backend` trait impl (`start`/`focused_app`/`capabilities` delegation,
  `simulate_paste`'s clipboard-only behavior): thin composition glue over already-tested pieces
  (`WaylandBackend`, `UnsupportedFocusTracker`, `resolve_paste_text`) - no new fake-driven unit test, same
  reasoning `X11DaemonBackend` itself was never separately unit-tested beyond a real-hardware
  `#[ignore]`d path. If this later grows real branching logic of its own, add tests then.
- `clip-platform::paste::resolve_paste_text` visibility change (private -> `pub`): no new test - existing
  `paste.rs` tests already characterize its Auto/PlainText selection behavior; run `cargo test -p
  clip-platform` to confirm nothing broke.
- Full workspace gate: `cargo test --workspace`, `cargo check --workspace`, `cargo clippy -p clipd
  --all-targets -- -D warnings` before calling this done.

## Risks / Trade-offs

- [Risk, resolved via amendment] `WAYLAND_DISPLAY`-only detection would have selected Wayland even in a
  mixed session with a perfectly good XWayland connection available, regressing sessions (like GNOME/
  Mutter) that only work through X11/XWayland today → Mitigation: selection is now X11-first (see
  Decisions' amendment) - X11 is attempted whenever reachable, and Wayland is only a fallback for sessions
  where no X11 connection can be made at all.
- [Risk] A user who deliberately wants the Wayland backend on a machine where XWayland also happens to be
  reachable (e.g. to test native Wayland capture specifically) cannot force it - X11 always wins when both
  are available → Mitigation: accepted as a v1 limitation; an explicit override flag is a narrow, separate
  follow-up if it turns out to matter in practice.
- [Risk] Clipboard-only paste is a real workflow regression versus X11's one-key paste, easy to miss until
  a user actually tries it → Mitigation: already an accepted, spec'd trade-off from
  `clip-platform-wayland-adapter`; this change's manual-verification task explicitly exercises paste on a
  real Wayland session so the behavior is confirmed, not assumed.
- [Risk] `wlr-data-control` support varies by compositor (wlroots-based only) → Mitigation: unchanged from
  `clip-platform-wayland-adapter` - construction fails clearly rather than silently degrading further.
