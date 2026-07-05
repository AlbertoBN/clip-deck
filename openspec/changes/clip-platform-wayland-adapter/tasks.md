## 1. Wayland connection fake and dependency setup

- [ ] 1.1 Add `wayland-client` and `wayland-protocols-wlr` to `crates/clip-platform/Cargo.toml` and define
      the internal `WaylandConnection` trait (selection read/write, selection-change subscription) in
      `crates/clip-platform/src/wayland/mod.rs`.
- [ ] 1.2 Implement an in-memory fake `WaylandConnection` under a test-only module, including a
      configuration flag for "compositor has no data-control support."

## 2. Wayland clipboard capture (`wayland-clipboard-capture`)

- [ ] 2.1 Write failing tests (against the fake connection) for read/write round trip, per
      `specs/wayland-clipboard-capture/spec.md`.
- [ ] 2.2 Run `cargo test -p clip-platform wayland::` and confirm failure.
- [ ] 2.3 Implement `read_current`/`set_current` against `WaylandConnection` - minimum code to pass.
- [ ] 2.4 Write a failing test for the watch loop emitting exactly one event on a genuine content change.
- [ ] 2.5 Implement the dedup-by-hash watch loop (reusing `clip-core::hashing`, matching the X11 adapter's
      approach).
- [ ] 2.6 Write a failing test asserting backend construction fails clearly when the fake connection is
      configured with no data-control support.
- [ ] 2.7 Implement the construction-time capability check and its error; run `cargo test -p clip-platform`
      and confirm all green.
- [ ] 2.8 Implement the real `WaylandConnection` using `wayland-client`/`wayland-protocols-wlr`. Add a
      documented `#[ignore]`d integration test exercising it against a real compositor.

## 3. Hotkey degradation (`global-hotkey-registration`, added requirement)

- [ ] 3.1 Write a failing test asserting registration against a fake "no shortcut mechanism" backend
      returns the distinct unsupported result, per this change's addition to
      `specs/global-hotkey-registration/spec.md`.
- [ ] 3.2 Run `cargo test -p clip-platform hotkeys::` and confirm failure.
- [ ] 3.3 Implement the unsupported-mechanism detection and result path.
- [ ] 3.4 Write a failing test asserting `capabilities()` reports hotkeys unsupported in that case.
- [ ] 3.5 Wire the capability flag; run `cargo test -p clip-platform` and confirm all green.

## 4. Focus-detection degradation (`focused-window-detection`, added requirement)

- [ ] 4.1 Write failing tests asserting `focused_app` returns `None` and `capabilities()` reports
      unsupported against a fake connection with no focus information available, per this change's
      addition to `specs/focused-window-detection/spec.md`.
- [ ] 4.2 Run `cargo test -p clip-platform focus::` and confirm failure.
- [ ] 4.3 Implement the unsupported-focus-detection path; run `cargo test -p clip-platform` and confirm
      all green.

## 5. Paste degradation (`paste-simulation`, modified)

- [ ] 5.1 Re-run the existing `paste-simulation` suite (from `clip-platform-x11-adapter` and
      `clip-platform-rich-content`) and confirm it still passes before making changes.
- [ ] 5.2 Write a failing test asserting `simulate_paste` still errors on a focus-detection-supported fake
      backend with no captured window (regression guard), per this change's modified
      `specs/paste-simulation/spec.md`.
- [ ] 5.3 Write a failing test asserting `simulate_paste` succeeds with clipboard-only fallback on a
      focus-detection-unsupported fake backend with no captured window.
- [ ] 5.4 Implement the capability-gated branch in `crates/clip-platform/src/paste.rs`; run
      `cargo test -p clip-platform` and confirm all green.

## 6. Diagnostics (`platform-diagnostics`, modified)

- [ ] 6.1 Write a failing test asserting the report's backend identifier is `"wayland"` when the Wayland
      backend is active, per this change's modified `specs/platform-diagnostics/spec.md`.
- [ ] 6.2 Run `cargo test -p clip-platform diagnostics::` and confirm failure.
- [ ] 6.3 Implement the Wayland backend identifier.
- [ ] 6.4 Write a failing test asserting a fake backend reporting mixed supported/unsupported capabilities
      produces a report listing each individually.
- [ ] 6.5 Implement per-capability listing in the report generator; run `cargo test -p clip-platform` and
      confirm all green.

## 7. Crate-level verification

- [ ] 7.1 Run `cargo test -p clip-platform` and confirm every non-`#[ignore]`d test from sections 1-6
      passes, alongside the unmodified X11 test suites.
- [ ] 7.2 Manually run the `#[ignore]`d Wayland integration test at least once on a `wlr-data-control`-
      supporting compositor (e.g. sway) and record the result in the PR description.
- [ ] 7.3 Manually verify graceful degradation on a compositor without data-control support (or by
      disabling it) - construction should fail clearly rather than hang.
- [ ] 7.4 Run `cargo check --workspace` and confirm the rest of the workspace still compiles.
- [ ] 7.5 Run `cargo clippy -p clip-platform -- -D warnings` and fix any lints introduced by this change.
