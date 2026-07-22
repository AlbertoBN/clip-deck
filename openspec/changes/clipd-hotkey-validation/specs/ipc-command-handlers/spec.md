## MODIFIED Requirements

### Requirement: Settings commands round-trip through the AppSettings model
`GetSettings` SHALL return the current `AppSettings` (via `clip-core`) as persisted, and `UpdateSettings`
SHALL persist the given changes such that a subsequent `GetSettings` reflects them. When the submitted
changes include a `hotkey_binding`, `UpdateSettings` SHALL validate it via `clip-platform`'s
`hotkeys::parse_binding` before persisting, and SHALL reject the command with a descriptive error - without
persisting any part of the update - when validation fails.

#### Scenario: Updated setting is visible on next GetSettings
- **WHEN** `UpdateSettings` changes the retention window and `GetSettings` is issued afterward
- **THEN** the response reflects the new retention window

#### Scenario: Valid hotkey binding is persisted
- **WHEN** `UpdateSettings` submits `hotkey_binding: "Ctrl+Shift+V"`
- **THEN** the command succeeds and a subsequent `GetSettings` returns `hotkey_binding: "Ctrl+Shift+V"`

#### Scenario: Invalid hotkey binding is rejected and not persisted
- **WHEN** `UpdateSettings` submits `hotkey_binding: "NotAKey+++"`
- **THEN** the command returns an error and a subsequent `GetSettings` still returns the previously
  persisted (or default) `hotkey_binding`, unchanged
