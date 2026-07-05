## 1. Test doubles for clip-store/clip-platform/clip-ipc

- [ ] 1.1 Define the internal `Store`, `Backend`, and `EventPublisher` traits in `crates/clipd/src/app.rs`
      that the real `clip-store`/`clip-platform`/`clip-ipc` types will implement.
- [ ] 1.2 Implement in-memory fakes of `Store`, `Backend`, and `EventPublisher` under a test-only module,
      for use by the tests in sections 2-5.

## 2. Ingest pipeline (`clip-ingest-pipeline`)

- [ ] 2.1 Write a failing test asserting a mixed-case MIME representation is normalized before being
      passed to the fake `Store`, per `specs/clip-ingest-pipeline/spec.md`.
- [ ] 2.2 Run `cargo test -p clipd ingest::` and confirm failure.
- [ ] 2.3 Implement MIME normalization in `crates/clipd/src/ingest.rs` - minimum code to pass.
- [ ] 2.4 Write failing tests for exclusion-rule-skips-persistence and non-matching-app-persists-normally.
- [ ] 2.5 Implement rule evaluation against the fake `Store`'s enabled rules.
- [ ] 2.6 Write a failing test asserting a dedup conflict updates `last_used_at` instead of erroring.
- [ ] 2.7 Implement the dedup-conflict-as-reuse branch.
- [ ] 2.8 Write a failing test asserting a multi-representation snapshot becomes one clip.
- [ ] 2.9 Implement multi-representation persistence as a single clip.
- [ ] 2.10 Write a failing test asserting successful ingest publishes `ClipCaptured` via the fake
      `EventPublisher`.
- [ ] 2.11 Implement event publication on successful ingest; run `cargo test -p clipd` and confirm green.

## 3. Watch loop (`clipboard-watch-loop`)

- [ ] 3.1 Write failing tests for in-order forwarding and surviving one ingest failure, per
      `specs/clipboard-watch-loop/spec.md`.
- [ ] 3.2 Run `cargo test -p clipd watch_loop::` and confirm failure.
- [ ] 3.3 Implement the watch loop in `crates/clipd/src/watch_loop.rs` against a fake event source and
      fake ingest function - minimum code to pass.
- [ ] 3.4 Write failing tests for paused-state skip and resume-after-unpause.
- [ ] 3.5 Implement the paused-state check; run `cargo test -p clipd` and confirm green.

## 4. IPC command handlers (`ipc-command-handlers`)

- [ ] 4.1 Write failing tests for `SearchClips` reflecting a just-ingested clip and `GetClip` not mutating
      `last_used_at`, per `specs/ipc-command-handlers/spec.md`.
- [ ] 4.2 Run `cargo test -p clipd commands::` and confirm failure.
- [ ] 4.3 Implement the read-only query handlers in `crates/clipd/src/commands.rs` against the fake
      `Store`/`Backend` - minimum code to pass.
- [ ] 4.4 Write failing tests for `PasteClip` success updating `last_used_at` and failure not updating it.
- [ ] 4.5 Implement the `PasteClip` handler.
- [ ] 4.6 Write failing tests for `PinClip` publishing `ClipUpdated` and `DeleteClip` publishing
      `ClipDeleted`.
- [ ] 4.7 Implement `PinClip`, `AssignGroup`, and `DeleteClip` handlers.
- [ ] 4.8 Write a failing test for `ClearHistory` scope handling and per-clip `ClipDeleted` events.
- [ ] 4.9 Implement the `ClearHistory` handler.
- [ ] 4.10 Write a failing test asserting a newly saved exclusion rule applies to the very next ingest
      (combine the `commands` and `ingest` fakes).
- [ ] 4.11 Implement `SaveRule`/`DeleteRule` handlers.
- [ ] 4.12 Write a failing test asserting `PauseCapture` publishes `CapturePaused` and the watch loop's
      paused state changes accordingly.
- [ ] 4.13 Implement the `PauseCapture` handler wired to the watch loop's paused state.
- [ ] 4.14 Write a failing test for `UpdateSettings` then `GetSettings` round trip.
- [ ] 4.15 Implement `GetSettings`/`UpdateSettings` handlers.
- [ ] 4.16 Write a failing test asserting `GetDiagnostics` mirrors the fake backend's `capabilities()`.
- [ ] 4.17 Implement the `GetDiagnostics` handler; run `cargo test -p clipd` and confirm all green.

## 5. Background jobs (`background-jobs`)

- [ ] 5.1 Write a failing test using `tokio::time::pause()`/`advance()` asserting prune is invoked again
      after one interval elapses, per `specs/background-jobs/spec.md`.
- [ ] 5.2 Run `cargo test -p clipd jobs::` and confirm failure.
- [ ] 5.3 Implement the interval-driven retention job in `crates/clipd/src/jobs.rs` - minimum code to pass.
- [ ] 5.4 Write a failing test asserting one failed prune run does not stop later scheduled runs.
- [ ] 5.5 Implement per-run error isolation (log and continue).
- [ ] 5.6 Write a failing test asserting a no-retention-window run deletes nothing.
- [ ] 5.7 Confirm the existing no-op path from `clip-store-persistence` is exercised correctly; run
      `cargo test -p clipd` and confirm green.

## 6. Telemetry (`daemon-telemetry`)

- [ ] 6.1 Write a failing test using a capturing `tracing` subscriber asserting an ingested clip's log
      event carries its clip id, per `specs/daemon-telemetry/spec.md`.
- [ ] 6.2 Run `cargo test -p clipd telemetry::` and confirm failure.
- [ ] 6.3 Implement structured log events for capture/ingest/command handling in
      `crates/clipd/src/telemetry.rs` and call sites - minimum code to pass.
- [ ] 6.4 Write a failing test asserting an excluded capture logs an explicit exclusion event.
- [ ] 6.5 Implement that log event.
- [ ] 6.6 Write a failing test asserting an `EnvFilter` directive surfaces debug-level ingest logs.
- [ ] 6.7 Wire `tracing-subscriber`'s `EnvFilter`; run `cargo test -p clipd` and confirm green.

## 7. Daemon lifecycle (`daemon-lifecycle`)

- [ ] 7.1 Write a failing integration test (real `clip-store` temp DB, real `clip-ipc` temp socket, fake
      backend) asserting the IPC socket is not connectable until migrations complete, per
      `specs/daemon-lifecycle/spec.md`.
- [ ] 7.2 Run `cargo test -p clipd app::` and confirm failure.
- [ ] 7.3 Implement `crates/clipd/src/app.rs` startup ordering (migrate, then bind) - minimum code to pass.
- [ ] 7.4 Write a failing test asserting a second instance started against an already-bound socket path
      exits with an "already running" error.
- [ ] 7.5 Implement the already-running check.
- [ ] 7.6 Write a failing test asserting an in-flight command completes before shutdown finishes when a
      shutdown signal fires mid-handling.
- [ ] 7.7 Implement graceful shutdown draining.
- [ ] 7.8 Write failing tests asserting capture continues with zero clients connected and after a client
      disconnects.
- [ ] 7.9 Confirm/adjust wiring so the watch loop's lifetime is independent of IPC client connections; run
      `cargo test -p clipd` and confirm green.
- [ ] 7.10 Replace `crates/clipd/src/main.rs`'s `todo!()` with real startup calling into `app::run()`.

## 8. Crate-level and milestone verification

- [ ] 8.1 Run `cargo test -p clipd` and confirm every test from sections 1-7 passes.
- [ ] 8.2 Run `cargo check --workspace` and confirm the rest of the workspace still compiles.
- [ ] 8.3 Run `cargo clippy -p clipd -- -D warnings` and fix any lints introduced by this change.
- [ ] 8.4 Manually verify the PRD's Milestone 1 acceptance criteria end-to-end on a real X11 session
      (copying plain text stores it automatically; a raw IPC client can search and issue `PasteClip` to
      paste it back into the previously focused window) and record the result in the PR description.
