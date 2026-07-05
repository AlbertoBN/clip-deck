## Context

`clip-platform`'s modules (`clipboard`, `x11/`, `wayland/`, `hotkeys`, `focus`, `paste`, `tray_support`,
`diagnostics`) are currently one-line stubs, and its `Cargo.toml` deliberately has no platform-specific
dependencies yet (see the scaffold's comment: "left out until the corresponding adapter is implemented").
This change is the first to add real X11 bindings and picks the trait shape that the (deferred) Wayland
adapter must also satisfy.

## Goals / Non-Goals

**Goals:**
- `ClipboardBackend` trait + `BackendCapabilities`/`ClipboardSnapshot` types that both X11 and (later)
  Wayland can implement without either one dictating the other's internals.
- A fully working X11 adapter: read/write clipboard, change-watch loop, hotkey registration, focused-
  window detection, paste simulation, capability reporting - the PRD's Milestone 1 platform slice.
- Unit-testable core logic (dedup-on-change, hotkey-string parsing, capability defaults, paste-mode
  handling) that does not require a real X server to run in CI.

**Non-Goals:**
- Wayland adapter (`clip-platform-wayland-adapter`, deferred milestone).
- HTML/image capture and thumbnailing (`clip-platform-rich-content`).
- Tray icon/menu (`tray_support` - owned by `clip-ui-tauri-shell`, since Tauri's own tray APIs are the
  primary mechanism per the PRD; this crate's `tray_support` module stays a stub until that change needs
  a shared helper).

## Decisions

- **X11 bindings**: `x11rb` for the low-level X11 protocol (selection ownership, `XFixes` selection-
  change notifications, key grabbing) - chosen over `x11` (the older FFI-style crate) for its safer,
  actively maintained async-friendly API. Added to `crates/clip-platform/Cargo.toml` as an X11-only
  dependency (behind no feature flag for now, since Wayland gets its own separate module/dependency set
  when that change lands).
- **Dependency injection for testability**: the X11 module defines a narrow internal `X11Connection` trait
  (selection read/write, selection-change subscription, window property lookup, key grab/synthesize)
  implemented once for real via `x11rb` and once as an in-memory fake used by unit tests. This lets
  `x11-clipboard-capture`, `focused-window-detection`, and `paste-simulation`'s *logic* (dedup-by-hash,
  WM_CLASS parsing, plain-text-mode stripping, previously-focused-window capture) be unit-tested without a
  running X server, while the real `X11Connection` impl itself is exercised only by a small `#[ignore]`d
  integration suite (see Test strategy) run manually / in a CI job with Xvfb.
- **Hotkey crate**: the `global-hotkey` crate for cross-desktop-environment key grabbing, wrapped so the
  binding-string parser (`"Ctrl+Shift+V"` → modifiers + key) is a pure, independently unit-testable
  function rather than embedded in the registration call.
- **Paste key delivery**: synthesize `Ctrl+V` (configurable) via `x11rb`'s `SendEvent`/`XTest` extension
  targeted at the captured previously-focused window, after placing content on the clipboard - matches
  Ditto's paste-back model per the PRD.

## Test strategy

- `clipboard` (trait + supporting types): pure unit tests - fake backend satisfying the trait, default-
  capabilities-all-false test, independent-flag-setting test, empty-vs-populated `ClipboardSnapshot`
  tests. Run with `cargo test -p clip-platform clipboard::`. No X server needed.
- `x11` capture logic: unit tests against the fake `X11Connection` for read/write round trip and the
  dedup-on-unchanged-content watch-loop behavior (using `clip-core::hashing` for the comparison, reusing
  `clip-core-foundations` rather than reimplementing hashing here). A separate `#[ignore]` integration
  test (documented in the module, run via `Xvfb :99 & DISPLAY=:99 cargo test -p clip-platform --
  --ignored`) exercises the real `x11rb`-backed connection end-to-end as a smoke test, not as the primary
  correctness gate.
- `hotkeys`: unit tests for binding-string parsing (valid combo, invalid combo rejected) against the pure
  parser function; registration-triggers-callback and conflicting-registration-errors tests run against a
  fake registration backend, with a `#[ignore]` real-hotkey integration test as a manual smoke test (real
  global hotkey capture needs a running desktop session).
- `focus`: unit tests for `AppContext` parsing from canned `WM_CLASS`/`_NET_WM_NAME` property fixtures via
  the fake `X11Connection`, the desktop-focus-returns-None case, and the popup-open-time-capture test
  (capture then simulate focus change, assert paste still targets the captured window).
- `paste`: unit tests for clipboard-then-synthesize ordering, plain-text-mode stripping, and the no-
  previously-focused-window error case, all against the fake `X11Connection`.
- `diagnostics`: unit tests asserting the report mirrors a fake backend's `capabilities()` output and
  includes the correct backend identifier string.

Red-green-refactor: for every task, write the test against the fake `X11Connection` (or the pure parser
function) first, confirm it fails to compile or fails the assertion, implement the minimum logic to pass,
run `cargo test -p clip-platform`, then refactor with tests green. The real `x11rb` wiring is implemented
only after the fake-backed logic is fully green, and is verified by the `#[ignore]`d integration tests
rather than by inline unit tests (matching the PRD's own "Manual acceptance tests" section for X11 runs).

## Risks / Trade-offs

- [Risk] Faking `X11Connection` in unit tests could hide real X11 protocol bugs → Mitigation: the
  `#[ignore]`d Xvfb-backed integration suite exercises the real implementation before this change is
  considered done; `tasks.md` includes running it manually as a verification step.
- [Risk] `global-hotkey` crate's cross-desktop support varies → Mitigation: `capabilities()` is the
  mechanism for surfacing what actually got registered; a failed registration is a typed error, not a
  silent no-op, per `global-hotkey-registration`'s spec.
- [Risk] XTest-based synthetic paste may behave differently across window managers → Mitigation: scope
  acknowledged in the PRD itself (X11 is the fully-supported baseline precisely because this risk is lower
  there than on Wayland); any WM-specific gaps become diagnostics-reported capability gaps, not crashes.
