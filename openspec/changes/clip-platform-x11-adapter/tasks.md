## 1. Trait and supporting types (`clipboard-backend-trait`)

- [ ] 1.1 Write a failing test defining a minimal fake type and asserting it satisfies a
      `ClipboardBackend` trait bound, per `specs/clipboard-backend-trait/spec.md`.
- [ ] 1.2 Run `cargo test -p clip-platform clipboard::` and confirm failure (trait doesn't exist).
- [ ] 1.3 Define the `ClipboardBackend` trait in `crates/clip-platform/src/clipboard.rs` - minimum code to
      pass.
- [ ] 1.4 Write failing tests for `BackendCapabilities` default-all-false and independent-flag-setting.
- [ ] 1.5 Implement `BackendCapabilities`.
- [ ] 1.6 Write failing tests for `ClipboardSnapshot` empty-vs-populated.
- [ ] 1.7 Implement `ClipboardSnapshot`; run `cargo test -p clip-platform` and confirm all green.

## 2. X11 connection fake and dependency setup

- [ ] 2.1 Add `x11rb` to `crates/clip-platform/Cargo.toml` and define the internal `X11Connection` trait
      (selection read/write, selection-change subscription, window property lookup, key grab/synthesize)
      in `crates/clip-platform/src/x11/mod.rs`.
- [ ] 2.2 Implement an in-memory fake `X11Connection` under a test-only module for use by the tests in
      sections 3-5.

## 3. X11 clipboard capture (`x11-clipboard-capture`)

- [ ] 3.1 Write failing tests (against the fake connection) for reading populated/empty clipboard content,
      per `specs/x11-clipboard-capture/spec.md`.
- [ ] 3.2 Run `cargo test -p clip-platform x11::` and confirm failure.
- [ ] 3.3 Implement `read_current` against `X11Connection` - minimum code to pass.
- [ ] 3.4 Write a failing test for `set_current` then `read_current` round trip.
- [ ] 3.5 Implement `set_current`.
- [ ] 3.6 Write failing tests for the watch loop: new-content emits one event, unchanged-content-
      notification emits no duplicate (using `clip-core::hashing` for comparison).
- [ ] 3.7 Implement the `start` watch loop's dedup-by-hash logic.
- [ ] 3.8 Write a failing test asserting X11 `capabilities()` reports all baseline flags supported.
- [ ] 3.9 Implement `capabilities()` for the X11 backend; run `cargo test -p clip-platform` and confirm
      green.
- [ ] 3.10 Implement the real `X11Connection` using `x11rb` (selection ownership, `XFixes` change
      notifications). Add an `#[ignore]`d integration test exercising it against a real X server and
      document how to run it (`Xvfb :99 & DISPLAY=:99 cargo test -p clip-platform -- --ignored`).

## 4. Global hotkey registration (`global-hotkey-registration`)

- [ ] 4.1 Write failing tests for hotkey binding-string parsing (valid combo, invalid combo rejected), per
      `specs/global-hotkey-registration/spec.md`.
- [ ] 4.2 Run `cargo test -p clip-platform hotkeys::` and confirm failure.
- [ ] 4.3 Implement the pure binding-string parser in `crates/clip-platform/src/hotkeys.rs`.
- [ ] 4.4 Write a failing test asserting a registered hotkey triggers its callback exactly once when
      "pressed" via a fake registration backend.
- [ ] 4.5 Write a failing test asserting a conflicting registration returns an error.
- [ ] 4.6 Add the `global-hotkey` crate dependency and implement registration (against a fake backend
      trait for these tests); run `cargo test -p clip-platform` and confirm green.
- [ ] 4.7 Wire the real `global-hotkey`-backed registration; add a documented `#[ignore]`d manual
      integration test.

## 5. Focused window detection (`focused-window-detection`)

- [ ] 5.1 Write failing tests for `AppContext` parsing from canned `WM_CLASS`/`_NET_WM_NAME` fixtures via
      the fake connection, and the desktop-focus-returns-None case, per
      `specs/focused-window-detection/spec.md`.
- [ ] 5.2 Run `cargo test -p clip-platform focus::` and confirm failure.
- [ ] 5.3 Implement `focused_app` against `X11Connection` - minimum code to pass.
- [ ] 5.4 Write a failing test for popup-open-time capture: capture focus, simulate a focus change, assert
      the retained capture still identifies the original window.
- [ ] 5.5 Implement the capture-at-activation-time retention mechanism; run `cargo test -p clip-platform`
      and confirm green.

## 6. Paste simulation (`paste-simulation`)

- [ ] 6.1 Write a failing test asserting content is placed on the clipboard before the paste key
      combination is synthesized, per `specs/paste-simulation/spec.md`.
- [ ] 6.2 Run `cargo test -p clip-platform paste::` and confirm failure.
- [ ] 6.3 Implement `simulate_paste`'s clipboard-then-synthesize ordering against the fake connection.
- [ ] 6.4 Write a failing test for `PasteMode::PlainText` stripping down to plain text.
- [ ] 6.5 Implement plain-text-mode handling.
- [ ] 6.6 Write a failing test asserting a missing previously-focused window yields an error.
- [ ] 6.7 Implement that error path; run `cargo test -p clip-platform` and confirm green.
- [ ] 6.8 Wire the real XTest-based synthetic key delivery; add a documented `#[ignore]`d manual
      integration test (copy in one app, trigger paste, confirm it lands in the target app).

## 7. Platform diagnostics (`platform-diagnostics`)

- [ ] 7.1 Write failing tests asserting the diagnostics report mirrors a fake backend's `capabilities()`
      output and includes the correct backend identifier, per `specs/platform-diagnostics/spec.md`.
- [ ] 7.2 Run `cargo test -p clip-platform diagnostics::` and confirm failure.
- [ ] 7.3 Implement the diagnostics report generator in `crates/clip-platform/src/diagnostics.rs`.
- [ ] 7.4 Run `cargo test -p clip-platform` and confirm all green.

## 8. Crate-level verification

- [ ] 8.1 Run `cargo test -p clip-platform` and confirm every non-`#[ignore]`d test from sections 1-7
      passes.
- [ ] 8.2 Manually run the `#[ignore]`d Xvfb-backed integration tests at least once on an X11 session and
      record the result in the PR description.
- [ ] 8.3 Run `cargo check --workspace` and confirm the rest of the workspace still compiles.
- [ ] 8.4 Run `cargo clippy -p clip-platform -- -D warnings` and fix any lints introduced by this change.
