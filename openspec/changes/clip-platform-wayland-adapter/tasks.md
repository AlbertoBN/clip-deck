## 1. Wayland connection fake and dependency setup

- [x] 1.1 Add `wayland-client` and `wayland-protocols-wlr` to `crates/clip-platform/Cargo.toml` and define
      the internal `WaylandConnection` trait (selection read/write, selection-change subscription) in
      `crates/clip-platform/src/wayland/mod.rs`.
- [x] 1.2 Implement an in-memory fake `WaylandConnection` under a test-only module, including a
      configuration flag for "compositor has no data-control support."

## 2. Wayland clipboard capture (`wayland-clipboard-capture`)

- [x] 2.1 Write failing tests (against the fake connection) for read/write round trip, per
      `specs/wayland-clipboard-capture/spec.md`.
- [x] 2.2 Run `cargo test -p clip-platform wayland::` and confirm failure.
- [x] 2.3 Implement `read_current`/`set_current` against `WaylandConnection` - minimum code to pass.
- [x] 2.4 Write a failing test for the watch loop emitting exactly one event on a genuine content change.
- [x] 2.5 Implement the dedup-by-hash watch loop (reusing `clip-core::hashing`, matching the X11 adapter's
      approach).
- [x] 2.6 Write a failing test asserting backend construction fails clearly when the fake connection is
      configured with no data-control support.
- [x] 2.7 Implement the construction-time capability check and its error; run `cargo test -p clip-platform`
      and confirm all green.
- [x] 2.8 Implement the real `WaylandConnection` using `wayland-client`/`wayland-protocols-wlr`. Add a
      documented `#[ignore]`d integration test exercising it against a real compositor.

## 3. Hotkey degradation (`global-hotkey-registration`, added requirement)

- [x] 3.1 Write a failing test asserting registration against a fake "no shortcut mechanism" backend
      returns the distinct unsupported result, per this change's addition to
      `specs/global-hotkey-registration/spec.md`.
- [x] 3.2 Run `cargo test -p clip-platform hotkeys::` and confirm failure.
- [x] 3.3 Implement the unsupported-mechanism detection and result path.
- [x] 3.4 Write a failing test asserting `capabilities()` reports hotkeys unsupported in that case.
- [x] 3.5 Wire the capability flag; run `cargo test -p clip-platform` and confirm all green.

## 4. Focus-detection degradation (`focused-window-detection`, added requirement)

- [x] 4.1 Write failing tests asserting `focused_app` returns `None` and `capabilities()` reports
      unsupported against a fake connection with no focus information available, per this change's
      addition to `specs/focused-window-detection/spec.md`.
- [x] 4.2 Run `cargo test -p clip-platform focus::` and confirm failure.
- [x] 4.3 Implement the unsupported-focus-detection path; run `cargo test -p clip-platform` and confirm
      all green.

## 5. Paste degradation (`paste-simulation`, modified)

- [x] 5.1 Re-run the existing `paste-simulation` suite (from `clip-platform-x11-adapter` and
      `clip-platform-rich-content`) and confirm it still passes before making changes.
- [x] 5.2 Write a failing test asserting `simulate_paste` still errors on a focus-detection-supported fake
      backend with no captured window (regression guard), per this change's modified
      `specs/paste-simulation/spec.md`.
- [x] 5.3 Write a failing test asserting `simulate_paste` succeeds with clipboard-only fallback on a
      focus-detection-unsupported fake backend with no captured window.
- [x] 5.4 Implement the capability-gated branch in `crates/clip-platform/src/paste.rs`; run
      `cargo test -p clip-platform` and confirm all green.

## 6. Diagnostics (`platform-diagnostics`, modified)

- [x] 6.1 Write a failing test asserting the report's backend identifier is `"wayland"` when the Wayland
      backend is active, per this change's modified `specs/platform-diagnostics/spec.md`.
- [x] 6.2 Run `cargo test -p clip-platform diagnostics::` and confirm failure.
- [x] 6.3 Implement the Wayland backend identifier.
- [x] 6.4 Write a failing test asserting a fake backend reporting mixed supported/unsupported capabilities
      produces a report listing each individually.
- [x] 6.5 Implement per-capability listing in the report generator; run `cargo test -p clip-platform` and
      confirm all green.

## 7. Crate-level verification

- [x] 7.1 Run `cargo test -p clip-platform` and confirm every non-`#[ignore]`d test from sections 1-6
      passes, alongside the unmodified X11 test suites.
- [ ] 7.2 Manually run the `#[ignore]`d Wayland integration test at least once on a `wlr-data-control`-
      supporting compositor (e.g. sway) and record the result in the PR description.
- [ ] 7.3 Manually verify graceful degradation on a compositor without data-control support (or by
      disabling it) - construction should fail clearly rather than hang.
- [x] 7.4 Run `cargo check --workspace` and confirm the rest of the workspace still compiles.
- [x] 7.5 Run `cargo clippy -p clip-platform -- -D warnings` and fix any lints introduced by this change.

## 8. Amendments: extensions discovered during implementation

- [x] 8.1 Add a defaulted `is_supported(&self) -> bool { true }` method to `hotkeys::HotkeyBackend`
      (`crates/clip-platform/src/hotkeys.rs`) and a new `UnsupportedHotkeyBackend` overriding it to `false`,
      as the concrete realization of "hotkeys gain a capability probe" - driven by tasks 3.1-3.5's tests.
- [x] 8.2 Add a standalone `focus::UnsupportedFocusTracker` (`crates/clip-platform/src/focus.rs`, not
      generic over any connection) with `focused_app() -> None` and `is_supported() -> false` - driven by
      tasks 4.1-4.3's tests.
- [x] 8.3 Add a `focus_detection_supported: bool` field to `paste::PasteSimulator` (default `true` via the
      existing `new`, preserving every existing X11 call site) and a new `without_focus_detection`
      constructor for the degraded path - driven by tasks 5.2-5.4's tests.
- [x] 8.4 Implement `RealWaylandConnection` (`crates/clip-platform/src/wayland/real.rs`) against the
      `wlr-data-control` protocol via `wayland-client`'s `Dispatch` mechanism; not test-driven (needs a live
      compositor, same carve-out as `RealX11Connection`) - verify it compiles and passes `clippy`, and rely
      on tasks 7.2/7.3 for correctness.
