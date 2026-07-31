## MODIFIED Requirements

### Requirement: All PRD commands and events are represented as protocol variants
The protocol SHALL define one variant per command (`SearchClips`, `GetClip`, `PasteClip`, `PinClip`,
`DeleteClip`, `ClearHistory`, `SaveRule`, `DeleteRule`, `GetSettings`,
`UpdateSettings`, `GetDiagnostics`, `PauseCapture`) and one variant per event (`ClipCaptured`,
`ClipUpdated`, `ClipDeleted`, `CapturePaused`, `DiagnosticsChanged`, `HotkeyPressed`), matching the PRD's
IPC contract, so no command or event needs to be bolted on ad hoc later by a downstream crate.
`AssignGroup` and `ListGroups` are no longer part of the protocol, per the removal of group management.

#### Scenario: Every command variant round-trips through the wire format
- **WHEN** each `Command` variant (constructed with representative field values) is serialized and then
  deserialized
- **THEN** the result is equal to the original value for every variant

#### Scenario: Every event variant round-trips through the wire format
- **WHEN** each `Event` variant (constructed with representative field values) is serialized and then
  deserialized
- **THEN** the result is equal to the original value for every variant
