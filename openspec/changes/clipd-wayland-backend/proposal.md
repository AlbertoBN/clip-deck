## Why

`clip-platform-wayland-adapter` fully implemented a Wayland `ClipboardBackend` (capture, hotkey/focus
degradation, paste degradation, diagnostics), but `clipd`'s `main.rs` still unconditionally constructs
`X11DaemonBackend` and never selects the Wayland pieces at all - `clipd` cannot run on a Wayland-only
session today. The user runs Wayland and wants to actually exercise ClipDeck end-to-end there, so the
daemon needs to detect the session type and compose/select a Wayland-backed daemon backend instead of
always assuming X11.

## What Changes

- Add a `WaylandDaemonBackend` in `clipd` (crate `clipd`, `app.rs`), mirroring `X11DaemonBackend`'s
  composition pattern: `clip_platform::wayland::WaylandBackend<RealWaylandConnection>` for capture,
  `clip_platform::focus::UnsupportedFocusTracker` for focus (always `None`, matching Wayland's security
  model), and a small Wayland-specific `simulate_paste` that places content on the clipboard only - no
  synthetic key delivery, since neither `wlr-data-control` nor anything else wired in this workspace can
  synthesize input on Wayland.
- `clipd`'s `main.rs` detects the running session type at startup (`$WAYLAND_DISPLAY` present vs not) and
  selects `WaylandDaemonBackend` + `clip_platform::hotkeys::UnsupportedHotkeyBackend` under Wayland,
  instead of always constructing `X11DaemonBackend::connect()` + `GlobalHotkeyBackend`.
- `GetDiagnostics`'s backend identifier reports `"wayland"` when the Wayland backend is selected (the
  report generator is already backend-name-generic per `clip-platform-wayland-adapter`; this only requires
  `main.rs` to pass the right `backend_name` string through).

## Capabilities

### New Capabilities
- `wayland-daemon-backend`: `clipd` composes a Wayland-backed `ClipboardBackend` (capture + focus + paste),
  mirroring `X11DaemonBackend`'s composition, so the daemon can run end-to-end on a Wayland session.

### Modified Capabilities
- `daemon-lifecycle` (owned by `clipd-daemon-core`): startup now detects the session type and selects
  between the X11 and Wayland backend/hotkey-backend pairs, rather than always constructing X11's.

## Impact

- Affected code: `crates/clipd/src/app.rs` (new `WaylandDaemonBackend`), `crates/clipd/src/main.rs`
  (session detection + backend selection).
- Depends on: `clip-platform-wayland-adapter` (`WaylandBackend`, `RealWaylandConnection`,
  `UnsupportedFocusTracker`, `UnsupportedHotkeyBackend` - all already implemented and merged),
  `clipd-daemon-core` (`X11DaemonBackend`'s composition precedent), `clipd-hotkey-popup-activation`
  (`app::run`'s existing `hotkeys: Arc<dyn HotkeyBackend>` parameter, reused unchanged here).
- Non-goals carried over from `clip-platform-wayland-adapter`: global hotkeys remain unsupported on
  Wayland in this version (no portal-based shortcut integration); paste is clipboard-only (the user
  completes the paste manually with their own Ctrl+V) since Wayland's security model prevents synthetic
  input delivery to other clients.
- Build/runtime: requires the system's Wayland compositor to support `wlr-data-control` (e.g. `sway`,
  other wlroots-based compositors) for capture/paste to work at all; on a non-supporting compositor,
  `WaylandDaemonBackend` construction fails clearly (per `wayland-clipboard-capture`'s existing spec),
  which `main.rs` must surface as a startup error rather than silently falling back to X11.

### Amendment (discovered during implementation)

- **Backend selection is X11-first, not session-type-first.** Manual verification on the requesting
  user's own machine (GNOME Shell/Mutter, a non-wlroots compositor) revealed that `WAYLAND_DISPLAY`-only
  detection was a real regression: `$DISPLAY` was also set and reachable via XWayland, so before this
  change `X11DaemonBackend` connected successfully through it - but the original design would have
  switched that same session straight to `WaylandDaemonBackend`, which fails outright on Mutter (no
  `wlr-data-control` support), leaving a previously-working session unable to start at all.
- `main.rs` now attempts `X11DaemonBackend::connect()` first, unconditionally (covering native X11 *and*
  XWayland-backed Wayland sessions transparently). Only when that connection attempt fails does it check
  `is_wayland_session` and fall back to `WaylandDaemonBackend` + `UnsupportedHotkeyBackend`; if neither is
  available, startup fails with the original X11 connection error, same as pre-this-change behavior for a
  no-display environment.
- `is_wayland_session` itself is unchanged (still a pure `WAYLAND_DISPLAY`-presence predicate); only its
  role shifted, from "the" selector to a fallback-eligibility check consulted after X11 connection fails.
- This means a genuinely wlroots-only session (no XWayland, or XWayland unreachable) still gets the native
  Wayland backend exactly as originally designed - the amendment only reorders preference to avoid
  regressing sessions where XWayland already provides a working path.
