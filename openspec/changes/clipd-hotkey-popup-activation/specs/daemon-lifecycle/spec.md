## MODIFIED Requirements

### Requirement: Startup applies migrations before accepting IPC connections
The daemon SHALL apply `clip-store` migrations and confirm they succeeded before binding the IPC server,
so no client can ever observe a partially-migrated database through a command. After migrations succeed
and settings become readable, startup SHALL attempt to register the persisted global hotkey binding (per
`hotkey-registration`) before the IPC server begins accepting connections; a registration failure SHALL
NOT prevent the server from binding or accepting connections.

#### Scenario: IPC server does not bind until migrations complete
- **WHEN** the daemon starts against a fresh, unmigrated database
- **THEN** the migration step completes successfully before the IPC socket becomes connectable

#### Scenario: Startup proceeds to accept connections even if hotkey registration fails
- **WHEN** the persisted `hotkey_binding` fails to register during startup
- **THEN** the IPC socket still becomes connectable and commands are served normally
