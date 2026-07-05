## Context

`clipd`'s modules (`app`, `commands`, `ingest`, `jobs`, `telemetry`, `watch_loop`) are one-line stubs, and
`main.rs` is a literal `todo!()`. This change is the composition point: it is the first crate where
`clip-core`, `clip-store`, `clip-ipc`, and `clip-platform::x11` all get wired together into one running
process, completing the PRD's Milestone 1.

## Goals / Non-Goals

**Goals:**
- A real, runnable `clipd` binary implementing the full IPC contract against real `clip-store` data and a
  real X11 backend.
- Ingest and command-handling logic that is unit-testable against fakes of its three dependencies
  (`clip-store`, `clip-platform`, `clip-ipc`), so the bulk of the test suite doesn't require a live X
  server or a real Unix socket.
- Daemon reliability behavior the PRD calls out explicitly: capture survives UI restarts/disconnects,
  fails fast on a duplicate instance, shuts down gracefully.

**Non-Goals:**
- No rich content (HTML/image) handling in ingest yet - `clip-platform-x11-adapter` only captures plain
  text, so ingest only needs to normalize/persist plain-text representations for now; `clip-platform-rich-
  content` extends both the capture side and this pipeline later.
- No UI - this change makes `clipd` fully operable via a raw IPC client (e.g. a test harness or `nc`-style
  tool against the Unix socket), not via `clip-ui-tauri`.

## Decisions

- **Dependency shape for testability**: `clipd::app` wires concrete `clip-store`, `clip-platform`, and
  `clip-ipc` implementations behind small internal traits/closures (a `Store`, a `Backend`, an
  `EventPublisher`) so `ingest`, `commands`, and `watch_loop` unit tests can inject fakes instead of a real
  SQLite file, real X11 connection, or real Unix socket. Integration tests (Milestone 1's actual
  acceptance criteria) exercise the real wiring end-to-end separately.
- **Scheduler for background jobs**: a `tokio::time::interval`-driven loop with an injectable clock/interval
  (via `tokio::time::pause()` in tests) rather than a full cron-style scheduler, since the only scheduled
  job today is retention pruning on a simple fixed interval.
- **Dedup-conflict handling**: ingest distinguishes `clip_store::Error::DedupConflict` from other store
  errors explicitly (matching `clip-store-persistence`'s typed error) so the "reuse, update last_used_at"
  path is a deliberate branch, not a catch-all.
- **Event publication ordering**: `clip-store` write + `clip-ipc` event publish happen inside the same
  handler function, publish-after-commit, so a client never observes an event for a write that hasn't
  actually been committed to SQLite.

## Test strategy

- `ingest`: unit tests against a fake `Store` (in-memory, implementing just enough of `clip-store`'s
  surface to observe what ingest called) and a fake `EventPublisher` - MIME-normalization test,
  exclusion-rule-skips-persistence test, dedup-conflict-updates-last_used_at test, multi-representation-
  one-clip test, successful-ingest-publishes-ClipCaptured test. Run with `cargo test -p clipd ingest::`.
- `watch_loop`: unit tests against a fake backend event source and a fake ingest function - in-order
  forwarding test, survives-one-ingest-failure test, paused-state skip/resume tests. Run with
  `cargo test -p clipd watch_loop::`.
- `commands`: unit tests per handler group against fake `Store`/`Backend`/`EventPublisher` - read-only
  commands don't mutate, PasteClip success/failure paths, PinClip/DeleteClip publish the right event,
  ClearHistory scope + per-clip events, SaveRule/DeleteRule take effect on next ingest (integration-style
  test combining `commands` and `ingest` fakes), PauseCapture toggles state consumed by `watch_loop`,
  Settings round trip, GetDiagnostics passthrough. Run with `cargo test -p clipd commands::`.
- `jobs`: unit tests using `tokio::time::pause()`/`advance()` to simulate interval elapse without real
  sleeping - scheduled-prune-runs-again test, failing-job-does-not-stop-scheduler test, no-window-no-op
  test. Run with `cargo test -p clipd jobs::`.
- `telemetry`: a test `tracing` subscriber capturing emitted events, asserting structured fields (clip id
  on ingest, exclusion indication) and that an `EnvFilter` directive changes what's emitted. Run with
  `cargo test -p clipd telemetry::`.
- `app`/`main` (`daemon-lifecycle`): integration-style tests using real `clip-store` (temp SQLite),
  real `clip-ipc` (temp-dir Unix socket), and a fake/no-op `ClipboardBackend` (a real X11 display isn't
  available in CI) - migrations-before-bind-ordering test, second-instance-fails-fast test (bind the
  socket first, then try to start a second app), shutdown-signal-drains-in-flight test, capture-
  continues-with-no/disconnected-clients test. Run with `cargo test -p clipd app::`.

Red-green-refactor: write each test against the not-yet-implemented handler/loop/job first (fails to
compile or fails the assertion), implement the minimum wiring to pass, run the full `cargo test -p clipd`
suite, then refactor with tests green. Only after `ingest`/`watch_loop`/`commands`/`jobs` are green against
fakes does `app`/`main` wire in the real `clip-store`/`clip-ipc`/`clip-platform` implementations.

## Risks / Trade-offs

- [Risk] Faking all three dependencies for unit tests could let a real wiring bug (e.g. a trait method
  signature mismatch) slip through → Mitigation: `daemon-lifecycle`'s integration tests use the real
  `clip-store` and `clip-ipc` crates (only the X11 backend is faked, since it's the one dependency without
  a headless test mode), catching real-wiring issues before they reach manual testing.
- [Risk] Background job scheduling drift/duplication under real `tokio::time` vs test-paused time →
  Mitigation: explicit `tokio::time::pause()`-based tests plus a manual verification task in `tasks.md`
  running the real daemon for an extended period.
- [Risk] Milestone 1's manual acceptance criteria (copy/paste loop in terminal, browser, editor) aren't
  fully covered by automated tests → Mitigation: `tasks.md` includes a manual verification pass against
  those PRD acceptance criteria before this change is considered done.
