## Why

`clip-core`, `clip-store`, `clip-ipc`, and `clip-platform::x11` now each exist independently, but nothing
wires them together into a running service. Per the PRD's build order, `clipd` is next: it is the daemon
that owns the watch loop, ingest pipeline, IPC command handlers, and background jobs, and its completion is
what makes Milestone 1 ("end-to-end text capture on X11") actually demoable.

## What Changes

- Implement the clipboard watch loop (`clipd::watch_loop`) that consumes `ClipboardBackend` capture events
  and feeds them into ingest.
- Implement the ingest pipeline (`clipd::ingest`): normalize captured content via `clip-core`, evaluate
  `clip-store` exclusion rules, compute the dedup hash, and persist via `clip-store`.
- Implement IPC command handlers (`clipd::commands`) for every command in the PRD's IPC contract
  (`SearchClips`, `GetClip`, `PasteClip`, `PinClip`, `AssignGroup`, `DeleteClip`, `ClearHistory`,
  `ListGroups`, `SaveRule`, `DeleteRule`, `GetSettings`, `UpdateSettings`, `GetDiagnostics`,
  `PauseCapture`), each backed by `clip-store`/`clip-platform` and publishing the matching `clip-ipc` event.
- Implement background jobs (`clipd::jobs`): scheduled retention pruning and cleanup.
- Implement structured logging/telemetry (`clipd::telemetry`).
- Implement daemon startup/lifecycle wiring (`clipd::app`, `clipd::main`): config load, DB/migration init,
  backend selection, IPC server start, graceful shutdown.

## Capabilities

### New Capabilities
- `clipboard-watch-loop`: Consumes backend capture events and forwards them to ingest, one clip at a time,
  without dropping events under normal operation.
- `clip-ingest-pipeline`: Normalizes, rule-filters, dedups, and persists a captured clipboard snapshot into
  a stored `Clip`.
- `ipc-command-handlers`: Implementations of every PRD IPC command, each translating an IPC `Command` into
  `clip-store`/`clip-platform` calls and returning the correct response/event.
- `background-jobs`: Scheduled retention pruning (calling `clip-store`'s retention policy on an interval)
  and cleanup tasks.
- `daemon-telemetry`: Structured, filterable logging (`tracing`) covering capture, ingest, and command
  handling.
- `daemon-lifecycle`: Startup wiring (config, DB migration, backend selection, IPC server bind) and
  graceful shutdown, matching the PRD's "daemon starts automatically and is ready soon after login" and
  "UI restarts must not interrupt clipboard capture" non-functional requirements.

### Modified Capabilities
(none - this change composes existing crates' capabilities rather than changing their requirements)

## Impact

- Affected code: `crates/clipd/src/{app,commands,ingest,jobs,telemetry,watch_loop,main}.rs`,
  `crates/clipd/Cargo.toml`.
- Depends on: `clip-core-foundations`, `clip-store-persistence`, `clip-ipc-transport`,
  `clip-platform-x11-adapter` (all four must exist for `clipd` to compile and run).
- Downstream: unlocks `clip-ui-tauri-shell` (a real daemon to connect to) and completes the PRD's
  Milestone 1 acceptance criteria (copy stores automatically, popup opens and pastes back, search works).

### Amendment (discovered during implementation)

Implementing `ipc-command-handlers` and `daemon-lifecycle` surfaced small gaps in the three dependency
crates that this change's original scope didn't anticipate - each was a small, additive, TDD-covered
extension to an already-completed change, not a design change to this proposal's own capabilities:

- `clip-store-persistence` (`crates/clip-store/src/{clips,rules,retention,groups}.rs`, new
  `crates/clip-store/src/settings.rs`): added `clips::touch_last_used`/`set_group`/`get_by_hash`,
  `rules::upsert`/`delete`, `retention::clear_with_ids` (returns removed clip ids, for per-clip
  `ClipDeleted` events), `groups::list_all`, and a `settings` key/value CRUD module - none of these
  existed yet, but `PasteClip`, `AssignGroup`, `ClearHistory`, `SaveRule`/`DeleteRule`, `ListGroups`, and
  `GetSettings`/`UpdateSettings` all require them per `specs/ipc-command-handlers/spec.md`.
- `clip-ipc-transport` (`crates/clip-ipc/src/server.rs`): added `Server::run_with_shutdown`, since the
  original transport had no graceful-drain primitive and `daemon-lifecycle`'s "in-flight command
  completes before shutdown finishes" requirement has no way to be satisfied without one.
- `clip-platform-x11-adapter` (`crates/clip-platform/src/paste.rs`, `src/clipboard.rs`): added
  `PasteSimulator::paste_to_focused_window` (paste target resolved fresh at call time, since this change
  has no popup-activation moment to capture a retained focus snapshot from) and
  `Serialize`/`Deserialize` on `BackendCapabilities` (needed to return it from `GetDiagnostics`).

Each addition has its own failing-test-first coverage in its owning crate's test suite (see that crate's
`cargo test -p <crate>` run), not just an incidental side effect of `clipd`'s own tests.
