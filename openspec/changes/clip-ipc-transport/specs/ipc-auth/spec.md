## ADDED Requirements

### Requirement: Socket file permissions restrict access to the owning user
The server SHALL create the socket file (and its containing runtime directory, if it creates it) with
permissions restricted to the owning user only, so no other local account can open the socket at the
filesystem level.

#### Scenario: Socket file is not group- or world-accessible
- **WHEN** the server binds its socket
- **THEN** the socket file's permission bits grant no access to group or other

### Requirement: Server rejects connections from a different local user
On platforms where peer credentials are available, the server SHALL check the connecting peer's UID
against its own UID and reject (close without dispatching any command) a connection from a different
user, providing defense in depth beyond filesystem permissions.

#### Scenario: Connection from the same UID is accepted
- **WHEN** a client connects with the same UID as the running server
- **THEN** the connection is accepted and commands are dispatched normally

#### Scenario: Connection from a different UID is rejected
- **WHEN** a simulated peer-credential check reports a UID different from the server's own UID
- **THEN** the server closes the connection without dispatching any command from it
