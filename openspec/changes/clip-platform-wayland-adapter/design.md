## Context

`clip-platform-x11-adapter` established the `ClipboardBackend` trait and `BackendCapabilities`/
diagnostics shape; `clip-platform-rich-content` extended X11 capture and paste-mode selection. This change
adds the second, deliberately-partial backend the PRD calls for, without changing the trait contract other
crates (`clipd`, `clip-ui-tauri`) already depend on - only the capability *values* a Wayland session
reports differ from X11's.

## Goals / Non-Goals

**Goals:**
- A working Wayland adapter for clipboard read/write/watch on compositors supporting `wlr-data-control`.
- Honest, per-capability degradation: every gap (hotkeys, focus-detection) is a reported `false` in
  `capabilities()`, never a crash or a silent no-op that looks like success.
- Diagnostics and paste-simulation behavior that already-existing downstream code (`clipd`,
  `clip-ui-tauri-shell`) can rely on without needing Wayland-specific branches of their own - the
  degradation contract lives entirely in `clip-platform`.

**Non-Goals:**
- No support for compositors without any data-control protocol at all - construction fails clearly (per
  `wayland-clipboard-capture`'s spec) rather than attempting an unsupported fallback mechanism.
- No portal-based (xdg-desktop-portal) global shortcut integration in this change - `capabilities()`
  reporting hotkeys as unsupported is an acceptable v1 outcome per the PRD's risk table; a portal-based
  hotkey path is a candidate follow-up change, not something to half-implement here.

## Decisions

- **Wayland bindings**: `wayland-client` + `wayland-protocols-wlr` (for `zwlr_data_control_manager_v1`),
  the same category of choice as `x11rb` for X11 - actively maintained, protocol-accurate bindings rather
  than shelling out to a CLI tool (e.g. `wl-clipboard`).
- **Testability**: mirrors `clip-platform-x11-adapter`'s approach - an internal `WaylandConnection` trait
  with a real `wayland-client`-backed implementation and an in-memory fake used by unit tests, so capture/
  watch-loop dedup logic is tested without a running compositor; a `#[ignore]`d integration suite (run
  manually against a real Wayland session, e.g. a nested `sway` or `weston` instance) exercises the real
  bindings.
- **Hotkey/focus degradation**: rather than a Wayland-specific error type bubbling up through `clipd` and
  the UI, both `hotkeys` and `focus` gain a construction-time or first-call capability probe that sets the
  relevant `BackendCapabilities` flag to `false` once, so callers only ever need to check `capabilities()`
  - they don't need per-call error handling for "not supported here."
- **Paste degradation**: implemented as a branch in the existing `clip-platform::paste` code keyed off
  `capabilities().focus_detection`, not a Wayland-specific paste function, so `clipd`'s `PasteClip` handler
  (already written against the generic `ClipboardBackend` trait) needs no changes.

## Test strategy

- `wayland` capture logic: unit tests against a fake `WaylandConnection` - read/write round trip, dedup-
  on-unchanged-content watch-loop behavior (reusing `clip-core::hashing`, same approach as the X11 change),
  and a construction-fails-without-data-control test. Run with `cargo test -p clip-platform wayland::`. A
  `#[ignore]`d integration test exercises the real `wayland-client` binding against a real compositor,
  documented the same way as the X11 adapter's manual integration test.
- `platform-diagnostics` (modified): unit tests asserting the backend identifier is `"wayland"` when the
  Wayland backend is active, and that a fake backend reporting a mix of supported/unsupported capabilities
  produces a report listing each individually - both against the existing `diagnostics` test module from
  `clip-platform-x11-adapter`, extended rather than replaced.
- `global-hotkey-registration` (added requirement): unit test asserting a fake "no shortcut mechanism"
  registration backend yields the unsupported result and the corresponding capability flag.
- `focused-window-detection` (added requirement): unit tests asserting `focused_app` returns `None` and
  `capabilities()` reports unsupported against a fake connection configured with no focus information
  available.
- `paste-simulation` (modified): unit test asserting `simulate_paste` still errors on X11-like
  (focus-detection-supported) backends with no captured window (regression test - must still pass), plus a
  new test asserting a focus-detection-unsupported fake backend succeeds with clipboard-only fallback.

Red-green-refactor: write each test against the fake `WaylandConnection` (or the existing X11 fakes,
extended with an "unsupported" configuration) first, confirm failure, implement the minimum degradation/
capture logic to pass, run `cargo test -p clip-platform`, then refactor with tests green.

## Risks / Trade-offs

- [Risk] `wlr-data-control` is wlroots-specific; GNOME (Mutter) and KDE (KWin) support varies → Mitigation:
  construction-time failure is explicit and typed, and the PRD's own risk table already accepts this as a
  known v1 limitation; `tasks.md` includes manually verifying behavior on at least one supporting and one
  non-supporting compositor.
- [Risk] Silent behavior drift between X11 and Wayland paste semantics could confuse users → Mitigation:
  `platform-diagnostics`'s per-capability surfacing (already required to be shown, not hidden, per
  `clip-ui-tauri-shell`'s `settings-ui` spec) is the single source of truth for what's degraded, rather than
  users discovering gaps by trial and error.
