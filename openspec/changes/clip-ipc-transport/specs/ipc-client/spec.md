## ADDED Requirements

### Requirement: Client sends a command and receives its correlated response
The client SHALL send a command over the socket and return the response envelope matching that command's
request id, even when other requests are in flight concurrently on the same connection.

#### Scenario: Client receives the response matching its request
- **WHEN** the client sends a `SearchClips` command and the server responds
- **THEN** the client's call returns the response payload for that specific request, not a response meant
  for a different in-flight request

### Requirement: Client exposes a stream of broadcast events
The client SHALL expose an event stream/subscription that yields `Event` values as the server broadcasts
them, decoupled from the request/response command flow.

#### Scenario: Client observes a HotkeyPressed event
- **WHEN** the server broadcasts a `HotkeyPressed` event while the client is subscribed
- **THEN** the client's event stream yields that event

### Requirement: Client reports a distinct connection error when the daemon is not running
The client SHALL return a distinct "daemon not running" error rather than panicking or returning a generic
I/O error indistinguishable from other failures, when it cannot connect because the daemon's socket does
not exist or refuses the connection.

#### Scenario: Connecting to a non-existent socket path returns a distinguishable error
- **WHEN** the client attempts to connect to a socket path with no listener
- **THEN** it returns an error that can be programmatically identified as "daemon not running"
