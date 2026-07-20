## 1. Test doubles for clip-store/clip-platform/clip-ipc

- [x] 1.1 Define the internal `Store`, `Backend`, and `EventPublisher` traits in `crates/clipd/src/app.rs`
      that the real `clip-store`/`clip-platform`/`clip-ipc` types will implement.
- [x] 1.2 Implement in-memory fakes of `Store`, `Backend`, and `EventPublisher` under a test-only module,
      for use by the tests in sections 2-5.

## 2. Ingest pipeline (`clip-ingest-pipeline`)

- [x] 2.1 Write a failing test asserting a mixed-case MIME representation is normalized before being
      passed to the fake `Store`, per `specs/clip-ingest-pipeline/spec.md`.
- [x] 2.2 Run `cargo test -p clipd ingest::` and confirm failure.
- [x] 2.3 Implement MIME normalization in `crates/clipd/src/ingest.rs` - minimum code to pass.
- [x] 2.4 Write failing tests for exclusion-rule-skips-persistence and non-matching-app-persists-normally.
- [x] 2.5 Implement rule evaluation against the fake `Store`'s enabled rules.
- [x] 2.6 Write a failing test asserting a dedup conflict updates `last_used_at` instead of erroring.
- [x] 2.7 Implement the dedup-conflict-as-reuse branch.
- [x] 2.8 Write a failing test asserting a multi-representation snapshot becomes one clip.
- [x] 2.9 Implement multi-representation persistence as a single clip.
- [x] 2.10 Write a failing test asserting successful ingest publishes `ClipCaptured` via the fake
      `EventPublisher`.
- [x] 2.11 Implement event publication on successful ingest; run `cargo test -p clipd` and confirm green.

## 3. Watch loop (`clipboard-watch-loop`)

- [x] 3.1 Write failing tests for in-order forwarding and surviving one ingest failure, per
      `specs/clipboard-watch-loop/spec.md`.
- [x] 3.2 Run `cargo test -p clipd watch_loop::` and confirm failure.
- [x] 3.3 Implement the watch loop in `crates/clipd/src/watch_loop.rs` against a fake event source and
      fake ingest function - minimum code to pass.
- [x] 3.4 Write failing tests for paused-state skip and resume-after-unpause.
- [x] 3.5 Implement the paused-state check; run `cargo test -p clipd` and confirm green.

## 4. IPC command handlers (`ipc-command-handlers`)

- [x] 4.1 Write failing tests for `SearchClips` reflecting a just-ingested clip and `GetClip` not mutating
      `last_used_at`, per `specs/ipc-command-handlers/spec.md`.
- [x] 4.2 Run `cargo test -p clipd commands::` and confirm failure.
- [x] 4.3 Implement the read-only query handlers in `crates/clipd/src/commands.rs` against the fake
      `Store`/`Backend` - minimum code to pass.
- [x] 4.4 Write failing tests for `PasteClip` success updating `last_used_at` and failure not updating it.
- [x] 4.5 Implement the `PasteClip` handler.
- [x] 4.6 Write failing tests for `PinClip` publishing `ClipUpdated` and `DeleteClip` publishing
      `ClipDeleted`.
- [x] 4.7 Implement `PinClip`, `AssignGroup`, and `DeleteClip` handlers.
- [x] 4.8 Write a failing test for `ClearHistory` scope handling and per-clip `ClipDeleted` events.
- [x] 4.9 Implement the `ClearHistory` handler.
- [x] 4.10 Write a failing test asserting a newly saved exclusion rule applies to the very next ingest
      (combine the `commands` and `ingest` fakes).
- [x] 4.11 Implement `SaveRule`/`DeleteRule` handlers.
- [x] 4.12 Write a failing test asserting `PauseCapture` publishes `CapturePaused` and the watch loop's
      paused state changes accordingly.
- [x] 4.13 Implement the `PauseCapture` handler wired to the watch loop's paused state.
- [x] 4.14 Write a failing test for `UpdateSettings` then `GetSettings` round trip.
- [x] 4.15 Implement `GetSettings`/`UpdateSettings` handlers.
- [x] 4.16 Write a failing test asserting `GetDiagnostics` mirrors the fake backend's `capabilities()`.
- [x] 4.17 Implement the `GetDiagnostics` handler; run `cargo test -p clipd` and confirm all green.

## 5. Background jobs (`background-jobs`)

- [x] 5.1 Write a failing test using `tokio::time::pause()`/`advance()` asserting prune is invoked again
      after one interval elapses, per `specs/background-jobs/spec.md`.
- [x] 5.2 Run `cargo test -p clipd jobs::` and confirm failure.
- [x] 5.3 Implement the interval-driven retention job in `crates/clipd/src/jobs.rs` - minimum code to pass.
- [x] 5.4 Write a failing test asserting one failed prune run does not stop later scheduled runs.
- [x] 5.5 Implement per-run error isolation (log and continue).
- [x] 5.6 Write a failing test asserting a no-retention-window run deletes nothing.
- [x] 5.7 Confirm the existing no-op path from `clip-store-persistence` is exercised correctly; run
      `cargo test -p clipd` and confirm green.

## 6. Telemetry (`daemon-telemetry`)

- [x] 6.1 Write a failing test using a capturing `tracing` subscriber asserting an ingested clip's log
      event carries its clip id, per `specs/daemon-telemetry/spec.md`.
- [x] 6.2 Run `cargo test -p clipd telemetry::` and confirm failure.
- [x] 6.3 Implement structured log events for capture/ingest/command handling in
      `crates/clipd/src/telemetry.rs` and call sites - minimum code to pass.
- [x] 6.4 Write a failing test asserting an excluded capture logs an explicit exclusion event.
- [x] 6.5 Implement that log event.
- [x] 6.6 Write a failing test asserting an `EnvFilter` directive surfaces debug-level ingest logs.
- [x] 6.7 Wire `tracing-subscriber`'s `EnvFilter`; run `cargo test -p clipd` and confirm green.

## 7. Daemon lifecycle (`daemon-lifecycle`)

- [x] 7.1 Write a failing integration test (real `clip-store` temp DB, real `clip-ipc` temp socket, fake
      backend) asserting the IPC socket is not connectable until migrations complete, per
      `specs/daemon-lifecycle/spec.md`.
- [x] 7.2 Run `cargo test -p clipd app::` and confirm failure.
- [x] 7.3 Implement `crates/clipd/src/app.rs` startup ordering (migrate, then bind) - minimum code to pass.
- [x] 7.4 Write a failing test asserting a second instance started against an already-bound socket path
      exits with an "already running" error.
- [x] 7.5 Implement the already-running check.
- [x] 7.6 Write a failing test asserting an in-flight command completes before shutdown finishes when a
      shutdown signal fires mid-handling.
- [x] 7.7 Implement graceful shutdown draining.
- [x] 7.8 Write failing tests asserting capture continues with zero clients connected and after a client
      disconnects.
- [x] 7.9 Confirm/adjust wiring so the watch loop's lifetime is independent of IPC client connections; run
      `cargo test -p clipd` and confirm green.
- [x] 7.10 Replace `crates/clipd/src/main.rs`'s `todo!()` with real startup calling into `app::run()`.

## 8. Crate-level and milestone verification

- [x] 8.1 Run `cargo test -p clipd` and confirm every test from sections 1-7 passes.
- [x] 8.2 Run `cargo check --workspace` and confirm the rest of the workspace still compiles.
- [x] 8.3 Run `cargo clippy -p clipd -- -D warnings` and fix any lints introduced by this change.
- [ ] 8.4 Manually verify the PRD's Milestone 1 acceptance criteria end-to-end on a real X11 session
      (copying plain text stores it automatically; a raw IPC client can search and issue `PasteClip` to
      paste it back into the previously focused window) and record the result in the PR description.

## 9. Amendments: prerequisite extensions to dependency crates

Discovered mid-implementation (see `proposal.md`'s "Amendment" note under Impact): `ipc-command-handlers`
and `daemon-lifecycle` need store/transport operations that didn't exist yet in the already-completed
`clip-store-persistence`, `clip-ipc-transport`, and `clip-platform-x11-adapter` changes. Each was added
with its own failing-test-first cycle in its owning crate, not as a side effect of `clipd`'s tests.

- [x] 9.1 `clip-store`: write failing tests then implement `clips::touch_last_used`, `clips::set_group`,
      `clips::get_by_hash` (`crates/clip-store/src/clips.rs`); run `cargo test -p clip-store` and confirm
      green.
- [x] 9.2 `clip-store`: write failing tests then implement `rules::upsert`, `rules::delete`
      (`crates/clip-store/src/rules.rs`); run `cargo test -p clip-store` and confirm green.
- [x] 9.3 `clip-store`: write a failing test then implement `retention::clear_with_ids`, returning removed
      clip ids for per-clip `ClipDeleted` events (`crates/clip-store/src/retention.rs`); run
      `cargo test -p clip-store` and confirm green.
- [x] 9.4 `clip-store`: write a failing test then implement `groups::list_all`
      (`crates/clip-store/src/groups.rs`); run `cargo test -p clip-store` and confirm green.
- [x] 9.5 `clip-store`: write failing tests then implement a new `settings` key/value CRUD module
      (`crates/clip-store/src/settings.rs`), backing `GetSettings`/`UpdateSettings`; run
      `cargo test -p clip-store` and confirm green.
- [x] 9.6 `clip-ipc`: write a failing test then implement `Server::run_with_shutdown`
      (`crates/clip-ipc/src/server.rs`) - stops accepting new connections and drains in-flight handler
      calls (tracked via a counter, not whole idle connections) before returning; run
      `cargo test -p clip-ipc` and confirm green.
- [x] 9.7 `clip-platform`: write failing tests then implement `PasteSimulator::paste_to_focused_window`
      (`crates/clip-platform/src/paste.rs`) and add `Serialize`/`Deserialize` to `BackendCapabilities`
      (`crates/clip-platform/src/clipboard.rs`); run `cargo test -p clip-platform` and confirm green.
- [x] 9.8 Re-run `cargo test` for `clip-store`, `clip-ipc`, and `clip-platform`, `cargo check --workspace`,
      and `cargo clippy --all-targets -- -D warnings` for each touched crate, to confirm these amendments
      didn't regress their owning changes.
