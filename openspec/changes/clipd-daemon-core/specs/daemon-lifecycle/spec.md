## ADDED Requirements

### Requirement: Startup applies migrations before accepting IPC connections
The daemon SHALL apply `clip-store` migrations and confirm they succeeded before binding the IPC server,
so no client can ever observe a partially-migrated database through a command.

#### Scenario: IPC server does not bind until migrations complete
- **WHEN** the daemon starts against a fresh, unmigrated database
- **THEN** the migration step completes successfully before the IPC socket becomes connectable

### Requirement: Startup fails fast and clearly when the IPC socket is already bound
Startup SHALL fail immediately with a clear "already running" error, rather than silently running a
second, conflicting instance, when another instance of the daemon is already running (its IPC socket is
already bound and live).

#### Scenario: Second daemon instance refuses to start
- **WHEN** a daemon instance is already running and bound to the IPC socket, and a second instance is
  started
- **THEN** the second instance exits immediately with an "already running" error

### Requirement: Shutdown signal triggers a graceful stop
On receiving a shutdown signal (SIGTERM or SIGINT), the daemon SHALL stop accepting new IPC connections,
allow in-flight command handling to finish, and close the clipboard watch loop cleanly before exiting.

#### Scenario: Shutdown signal drains in-flight work before exit
- **WHEN** a shutdown signal is received while a command is being handled
- **THEN** that command's handling completes before the process exits

### Requirement: Clipboard capture continues independent of UI client connections
The daemon's watch loop and ingest pipeline SHALL keep running and persisting clips whether zero, one, or
multiple UI clients are connected over IPC, matching the PRD's requirement that UI restarts must not
interrupt clipboard capture.

#### Scenario: Capture continues with no clients connected
- **WHEN** no IPC client is connected and a clipboard change occurs
- **THEN** the clip is still ingested and persisted normally

#### Scenario: Capture is unaffected by a client disconnecting
- **WHEN** a connected IPC client disconnects and a clipboard change occurs immediately after
- **THEN** the clip is still ingested and persisted normally
