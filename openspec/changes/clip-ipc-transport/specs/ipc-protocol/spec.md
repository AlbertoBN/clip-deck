## ADDED Requirements

### Requirement: All PRD commands and events are represented as protocol variants
The protocol SHALL define one variant per command (`SearchClips`, `GetClip`, `PasteClip`, `PinClip`,
`AssignGroup`, `DeleteClip`, `ClearHistory`, `ListGroups`, `SaveRule`, `DeleteRule`, `GetSettings`,
`UpdateSettings`, `GetDiagnostics`, `PauseCapture`) and one variant per event (`ClipCaptured`,
`ClipUpdated`, `ClipDeleted`, `CapturePaused`, `DiagnosticsChanged`, `HotkeyPressed`), matching the PRD's
IPC contract, so no command or event needs to be bolted on ad hoc later by a downstream crate.

#### Scenario: Every command variant round-trips through the wire format
- **WHEN** each `Command` variant (constructed with representative field values) is serialized and then
  deserialized
- **THEN** the result is equal to the original value for every variant

#### Scenario: Every event variant round-trips through the wire format
- **WHEN** each `Event` variant (constructed with representative field values) is serialized and then
  deserialized
- **THEN** the result is equal to the original value for every variant

### Requirement: Commands carry a request id for response correlation
Every outgoing command SHALL be wrapped in an envelope carrying a unique request id, and every response
SHALL echo the request id it answers, so a client issuing multiple concurrent commands over one connection
can match each response to its request.

#### Scenario: Two concurrent requests get distinguishable responses
- **WHEN** two `SearchClips` commands are wrapped in envelopes with different request ids
- **THEN** the two envelopes have different request ids and each preserves its own command payload

### Requirement: Responses model success and error uniformly
The protocol SHALL define a single response envelope with an `Ok(payload)` / `Err(error)` shape shared by
every command, rather than a bespoke result type per command, so client dispatch code has one response
path to handle.

#### Scenario: A successful response round-trips its payload
- **WHEN** a success response wrapping a `GetClip` result is serialized and deserialized
- **THEN** the deserialized response is `Ok` and contains the original payload

#### Scenario: An error response round-trips its error message
- **WHEN** an error response (e.g. "clip not found") is serialized and deserialized
- **THEN** the deserialized response is `Err` and contains the original error message
