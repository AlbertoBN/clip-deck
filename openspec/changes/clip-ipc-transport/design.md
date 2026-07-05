## Context

`clip-ipc`'s modules (`protocol`, `server`, `client`, `auth`) are currently one-line stubs. This is a
cross-cutting change (defines the contract two future crates - `clipd` and `clip-ui-tauri` - depend on),
and picks the wire format now so both sides are built against the same framing from day one.

## Goals / Non-Goals

**Goals:**
- Full `Command`/`Event` enum coverage matching the PRD's IPC contract list exactly.
- A request/response framing that works over a `tokio::net::UnixStream` and supports concurrent
  in-flight requests on one connection.
- Local-only auth via filesystem permissions plus a peer-credential check.

**Non-Goals:**
- No actual command handling logic (what happens when `PasteClip` is received) - that's `clipd-daemon-
  core`. This crate only transports already-decoded `Command`/`Event` values to/from a registered handler
  function/callback.
- No DBus transport - the PRD allows adding it later behind the same protocol abstraction; this change
  only implements the Unix-socket transport.

## Decisions

- **Wire format**: newline-delimited JSON (one JSON object per line) over the Unix stream, rather than a
  binary framing (e.g. length-prefixed bincode). Rationale: `serde_json` is already a workspace dependency,
  NDJSON is trivial to frame/debug (can `nc` the socket by hand), and message volume/size for a clipboard
  manager's IPC is small enough that JSON overhead is irrelevant. Revisit only if profiling shows IPC
  framing is a bottleneck.
- **Concurrency model**: each server connection is handled on its own `tokio::spawn`'d task reading a
  loop of NDJSON requests and dispatching each to the handler; responses are written back as they
  complete rather than serialized to be strictly in-order, which is why every request carries an id.
- **Auth mechanism**: Unix filesystem permissions (0700 dir / 0600 socket) as the primary control, plus
  `SO_PEERCRED`-style UID check (via a small trait so the check can be faked in tests without a real
  cross-user connection) as defense in depth - matches the PRD's "local-only auth" framing without adding
  a token/password scheme for a single-user local socket.
- **Client API shape**: an async `IpcClient` with a `call(Command) -> Response` method plus a
  `subscribe() -> impl Stream<Item = Event>`, rather than a single multiplexed callback API, so UI code can
  `await` a command result directly while a separate task consumes the event stream.

## Test strategy

Per component, using real `tokio::net::UnixListener`/`UnixStream` pairs over temp-dir socket paths (not
mocked transports), so framing bugs show up in tests rather than only at runtime:

- `protocol`: round-trip serde tests for every `Command` and `Event` variant, envelope request-id
  preservation test, and Ok/Err response round-trip tests. Run with `cargo test -p clip-ipc protocol::`.
- `server`: bind-on-fresh-dir test, bind-over-stale-socket test, two-concurrent-clients test,
  request-id-correlation test, handler-error-keeps-connection-open test, broadcast-to-other-clients test.
  Run with `cargo test -p clip-ipc server::`.
- `client`: response-correlation-under-concurrency test, event-stream-receives-broadcast test,
  connect-to-nonexistent-socket-returns-distinguishable-error test. Run with
  `cargo test -p clip-ipc client::`.
- `auth`: socket-permission-bits test, same-UID-accepted / different-UID-rejected tests (using a fakeable
  peer-credential trait so the "different UID" case doesn't require actually running as two users). Run
  with `cargo test -p clip-ipc auth::`.

Red-green-refactor: write each test against a real (but not-yet-implemented) `UnixListener`/`UnixStream`
based API first, confirm it fails to compile or fails at runtime, implement the minimum to pass, run the
full `cargo test -p clip-ipc` suite, then refactor with tests green.

## Risks / Trade-offs

- [Risk] NDJSON has no built-in message-size guard → Mitigation: server enforces a per-line max length
  and closes the connection with an error response if exceeded, covered by a task in `tasks.md`.
- [Risk] Peer-credential checks are Linux/Unix-specific → Mitigation: acceptable since this is an Ubuntu-
  only v1 product per the PRD; the check is behind a small trait so it can be swapped or no-op'd if the UI
  shell ever needs to run cross-platform.
- [Risk] One task per connection could leak if a client hangs without closing → Mitigation: add a read
  timeout/idle-disconnect as a task; covered in `tasks.md` rather than left implicit.
