## ADDED Requirements

### Requirement: Commands resolve or reject consistent with the daemon's response
The frontend state layer SHALL resolve a command call with the response payload on an `Ok` response and
reject it with the error message on an `Err` response, so UI code can use standard promise/async
success/failure handling rather than inspecting a raw envelope.

#### Scenario: A successful command resolves with its payload
- **WHEN** the daemon responds `Ok` with a list of clips for a `SearchClips` call
- **THEN** the state layer's call resolves with that list of clips

#### Scenario: A failed command rejects with the daemon's error message
- **WHEN** the daemon responds `Err` with `"clip not found"` for a `GetClip` call
- **THEN** the state layer's call rejects with that message

### Requirement: Daemon events update local state reactively
The state layer SHALL subscribe to the daemon's event stream and update its local store in response -
appending on `ClipCaptured`, updating on `ClipUpdated`, and removing on `ClipDeleted` - without requiring
any view to manually re-fetch.

#### Scenario: ClipCaptured appends to the local clip list
- **WHEN** a `ClipCaptured` event referencing a new clip is received
- **THEN** the local state's clip list includes that clip afterward

#### Scenario: ClipDeleted removes the clip from local state
- **WHEN** a `ClipDeleted` event referencing an existing clip is received
- **THEN** that clip is no longer present in the local state's clip list

### Requirement: Daemon-not-running is a distinct, surfaced connection state
When the underlying `clip-ipc` client reports its "daemon not running" error, the state layer SHALL expose
a distinct disconnected/daemon-not-running state that views can render as a reconnect prompt, rather than
surfacing it as an ordinary failed command or crashing the UI.

#### Scenario: Daemon-not-running surfaces as a distinct state, not a crash
- **WHEN** the IPC client reports its "daemon not running" error while a view is active
- **THEN** the state layer exposes a disconnected state and the UI remains responsive (no unhandled
  exception)
