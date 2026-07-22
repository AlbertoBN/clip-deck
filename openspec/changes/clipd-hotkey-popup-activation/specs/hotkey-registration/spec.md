## ADDED Requirements

### Requirement: Daemon registers the persisted hotkey binding at startup
The daemon SHALL register a global hotkey, parsed from the persisted `hotkey_binding` setting, with
`clip-platform`'s `HotkeyBackend` during startup, before it begins serving IPC commands.

#### Scenario: Startup registers the persisted binding
- **WHEN** the daemon starts with `hotkey_binding: "Ctrl+Shift+V"` persisted in settings
- **THEN** a hotkey matching that binding is registered with the hotkey backend

### Requirement: Triggering the registered hotkey publishes HotkeyPressed
When the registered hotkey fires, the daemon SHALL publish an `Event::HotkeyPressed` via the existing
event publisher, so connected UI clients can react to it.

#### Scenario: Hotkey trigger publishes an event
- **WHEN** the registered hotkey backend invokes its callback
- **THEN** a `HotkeyPressed` event is published

### Requirement: Hotkey registration failure degrades gracefully without failing startup
The daemon SHALL log and continue past a failure to parse or register the persisted hotkey binding (e.g.
already claimed by another application), rather than aborting startup or crashing.

#### Scenario: Registration failure does not prevent the daemon from starting
- **WHEN** the hotkey backend's registration call returns an error during startup
- **THEN** the daemon still binds its IPC socket and begins serving commands normally
