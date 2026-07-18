## 1. Protocol DTOs (`ipc-protocol`)

- [x] 1.1 Write failing round-trip serde tests for every `Command` variant and every `Event` variant, per
      `specs/ipc-protocol/spec.md`.
- [x] 1.2 Run `cargo test -p clip-ipc protocol::` and confirm failure (types don't exist).
- [x] 1.3 Implement the `Command` and `Event` enums in `crates/clip-ipc/src/protocol.rs` with serde
      derives - minimum code to pass.
- [x] 1.4 Write a failing test asserting a request envelope preserves a unique request id.
- [x] 1.5 Implement the request envelope wrapping `Command` + request id.
- [x] 1.6 Write failing tests for Ok/Err response envelope round trips.
- [x] 1.7 Implement the response envelope; run `cargo test -p clip-ipc` and confirm all green.

## 2. Server transport (`ipc-server`)

- [x] 2.1 Write a failing test that binds the server on a fresh temp runtime dir and asserts the socket
      file exists, per `specs/ipc-server/spec.md`.
- [x] 2.2 Run `cargo test -p clip-ipc server::` and confirm failure.
- [x] 2.3 Implement socket path resolution and bind logic in `crates/clip-ipc/src/server.rs` - minimum
      code to pass.
- [x] 2.4 Write a failing test asserting the server binds successfully over a stale leftover socket file.
- [x] 2.5 Implement stale-socket removal before bind.
- [x] 2.6 Write a failing test with two concurrent client connections both getting served.
- [x] 2.7 Implement per-connection `tokio::spawn` accept loop.
- [x] 2.8 Write a failing test asserting the response envelope echoes the request's id, and a failing test
      asserting a handler error yields an `Err` response without closing the connection.
- [x] 2.9 Implement command dispatch to a registered handler and response correlation.
- [x] 2.10 Write a failing test asserting an event published on the server is delivered to a second,
      uninvolved connected client.
- [x] 2.11 Implement broadcast-to-all-clients event delivery.
- [x] 2.12 Write a failing test asserting an over-long line is rejected with an error response and the
      connection is closed (message-size guard from the design doc).
- [x] 2.13 Implement the per-line max-length guard.
- [x] 2.14 Run `cargo test -p clip-ipc` and confirm all green.

## 3. Client transport (`ipc-client`)

- [x] 3.1 Write a failing test asserting concurrent requests each receive their own correlated response,
      per `specs/ipc-client/spec.md`.
- [x] 3.2 Run `cargo test -p clip-ipc client::` and confirm failure.
- [x] 3.3 Implement `IpcClient::call` with request-id correlation in `crates/clip-ipc/src/client.rs` -
      minimum code to pass.
- [x] 3.4 Write a failing test asserting `IpcClient::subscribe()` yields a broadcast event.
- [x] 3.5 Implement the event subscription stream.
- [x] 3.6 Write a failing test asserting connecting to a nonexistent socket path returns a
      distinguishable "daemon not running" error.
- [x] 3.7 Implement that distinguishable error variant/check.
- [x] 3.8 Run `cargo test -p clip-ipc` and confirm all green.

## 4. Local-only auth (`ipc-auth`)

- [x] 4.1 Write a failing test asserting the bound socket file's permission bits deny group/other access,
      per `specs/ipc-auth/spec.md`.
- [x] 4.2 Run `cargo test -p clip-ipc auth::` and confirm failure.
- [x] 4.3 Implement permission-restricted directory/socket creation in `crates/clip-ipc/src/auth.rs` -
      minimum code to pass.
- [x] 4.4 Write failing tests for same-UID-accepted and different-UID-rejected using a fakeable
      peer-credential trait.
- [x] 4.5 Implement the peer-credential check trait and wire it into the server's accept loop.
- [x] 4.6 Run `cargo test -p clip-ipc` and confirm all green.

## 5. Crate-level verification

- [x] 5.1 Run `cargo test -p clip-ipc` and confirm every test from sections 1-4 passes.
- [x] 5.2 Run `cargo check --workspace` and confirm the rest of the workspace still compiles.
- [x] 5.3 Run `cargo clippy -p clip-ipc -- -D warnings` and fix any lints introduced by this change.
