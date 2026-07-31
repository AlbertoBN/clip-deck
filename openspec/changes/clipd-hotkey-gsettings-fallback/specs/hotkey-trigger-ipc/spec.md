## ADDED Requirements

### Requirement: TriggerHotkey command publishes HotkeyPressed
The daemon SHALL handle a `Command::TriggerHotkey` by publishing `Event::HotkeyPressed` via its event
publisher and returning a success response, without involving any `HotkeyBackend`.

#### Scenario: TriggerHotkey publishes HotkeyPressed
- **WHEN** the daemon receives `Command::TriggerHotkey`
- **THEN** it publishes `Event::HotkeyPressed` and returns a success response

### Requirement: A standalone CLI binary sends TriggerHotkey over the daemon's existing socket
A CLI trigger binary SHALL connect to the daemon's Unix socket at its resolved path, send
`Command::TriggerHotkey`, and exit, so GNOME's custom keybinding can invoke it as a plain shell command.

#### Scenario: Trigger binary sends TriggerHotkey and exits
- **WHEN** the trigger binary is run while the daemon is listening on its socket
- **THEN** it sends `Command::TriggerHotkey` over that socket and exits

#### Scenario: Trigger binary exits quietly when the daemon isn't running
- **WHEN** the trigger binary is run while no daemon is listening on the socket
- **THEN** it exits without publishing anything and without crashing or printing a stack trace
