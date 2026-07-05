## Why

`clipd` and `clip-ui-tauri` are separate processes by design (so clip capture survives UI restarts), and
per the PRD's build order `clip-ipc` must exist before `clipd` (which hosts the IPC server) or
`clip-ui-tauri` (which is an IPC client) can be implemented. Without a shared protocol crate, the daemon
and UI would drift on message shapes independently.

## What Changes

- Define the command/event protocol DTOs (`SearchClips`, `GetClip`, `PasteClip`, `PinClip`, `AssignGroup`,
  `DeleteClip`, `ClearHistory`, `ListGroups`, `SaveRule`, `DeleteRule`, `GetSettings`, `UpdateSettings`,
  `GetDiagnostics`, `PauseCapture` / `ClipCaptured`, `ClipUpdated`, `ClipDeleted`, `CapturePaused`,
  `DiagnosticsChanged`, `HotkeyPressed`) in `clip-ipc::protocol`, matching the PRD's IPC contract exactly.
- Implement the daemon-side Unix domain socket server (`clip-ipc::server`): accepting connections, framing
  requests/responses, and broadcasting events to connected clients.
- Implement the UI-side client (`clip-ipc::client`): connecting to the socket, sending commands and
  awaiting responses, and subscribing to the event stream.
- Implement local-only auth/scope (`clip-ipc::auth`): restricting the socket to the current user (file
  permissions / peer credential check) so no other local user can issue commands.

## Capabilities

### New Capabilities
- `ipc-protocol`: Serializable command and event DTOs and the request/response envelope/framing format
  shared by server and client.
- `ipc-server`: Daemon-side Unix socket listener that accepts multiple client connections, dispatches
  commands to a handler, and broadcasts events to all connected clients.
- `ipc-client`: UI-side connection that sends commands and awaits typed responses, and receives a stream
  of broadcast events.
- `ipc-auth`: Local-only connection scoping - only the invoking user's processes may connect and issue
  commands.

### Modified Capabilities
(none)

## Impact

- Affected code: `crates/clip-ipc/src/{protocol,server,client,auth}.rs`, `crates/clip-ipc/Cargo.toml`.
- Depends on: `clip-core-foundations` (domain types referenced by DTOs, e.g. `Clip`, `PasteMode`) and
  `clip-store-persistence` conceptually (DTO shapes mirror what `clip-store` can answer), though this
  change does not call `clip-store` directly - it only defines and transports messages. Command handlers
  that actually call `clip-store` are implemented in `clipd-daemon-core`.
- Downstream: unlocks `clipd-daemon-core` (hosts `ipc-server` + implements handlers) and
  `clip-ui-tauri-shell` (uses `ipc-client`).
