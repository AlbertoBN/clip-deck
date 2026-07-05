## ADDED Requirements

### Requirement: Server listens on a Unix domain socket at a resolved runtime path
The server SHALL bind a Unix domain socket at a path derived from the application's runtime directory
(not a hardcoded path), removing any stale socket file left over from an unclean shutdown before binding.

#### Scenario: Server binds successfully on a fresh runtime directory
- **WHEN** the server is started with a runtime directory that has no existing socket file
- **THEN** it successfully binds and the socket file exists afterward

#### Scenario: Server binds successfully over a stale socket file
- **WHEN** a leftover socket file from a previous run exists at the resolved path and the server starts
- **THEN** it removes the stale file and binds successfully rather than failing with "address in use"

### Requirement: Server accepts multiple concurrent client connections
The server SHALL accept more than one simultaneous client connection on the same socket, handling each
independently, so the popup, manager window, and tray can all be connected at once.

#### Scenario: Two clients connect at the same time
- **WHEN** two separate client connections are opened to a running server
- **THEN** both connections are accepted and each can send a command and receive its own response

### Requirement: Server dispatches decoded commands to a handler and returns the correlated response
The server SHALL decode an incoming command envelope, pass the command to a registered handler function,
and write back a response envelope carrying the same request id as the incoming command.

#### Scenario: A dispatched command's response carries the original request id
- **WHEN** a client sends a `GetClip` command wrapped in an envelope with request id `"r1"`
- **THEN** the response envelope received back also has request id `"r1"`

#### Scenario: Handler error becomes an Err response, not a dropped connection
- **WHEN** the registered handler returns an error for a given command
- **THEN** the client receives an `Err` response envelope and the connection remains open for further
  commands

### Requirement: Server broadcasts events to every connected client
When an event is published to the server, it SHALL be delivered to every currently-connected client, not
only the client that triggered the underlying action.

#### Scenario: An event triggered by one client's command is seen by another client
- **WHEN** client A's `PinClip` command causes a `ClipUpdated` event to be published, and client B is also
  connected
- **THEN** client B receives the `ClipUpdated` event on its connection
