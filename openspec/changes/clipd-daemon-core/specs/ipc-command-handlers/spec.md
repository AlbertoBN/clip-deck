## ADDED Requirements

### Requirement: Read-only query commands return live data without mutating state
`SearchClips`, `GetClip`, `ListGroups`, `GetSettings`, and `GetDiagnostics` SHALL be handled by querying
`clip-store`/`clip-core`/`clip-platform` directly and returning current data, without side effects on the
underlying data.

#### Scenario: SearchClips reflects a just-ingested clip
- **WHEN** a clip is ingested and then a `SearchClips` command matching its text is issued
- **THEN** the response includes that clip

#### Scenario: GetClip does not change the clip's last_used_at
- **WHEN** a `GetClip` command is issued for an existing clip
- **THEN** the clip's `last_used_at` is unchanged by the query itself

### Requirement: PasteClip triggers paste and records usage
The `PasteClip` handler SHALL invoke `clip-platform`'s paste simulation with the requested clip's content
and paste mode, and on success SHALL update the clip's `last_used_at` via `clip-store`.

#### Scenario: Successful paste updates last_used_at
- **WHEN** `PasteClip` is issued for an existing clip and paste simulation succeeds
- **THEN** the clip's `last_used_at` is updated to the current time

#### Scenario: Paste failure is returned as an error response, not a partial success
- **WHEN** `PasteClip` is issued and paste simulation returns an error
- **THEN** the command response is an error and `last_used_at` is not updated

### Requirement: Mutating commands persist their change and publish the matching event
`PinClip`, `AssignGroup`, and `DeleteClip` SHALL apply their change via `clip-store` and SHALL publish a
`ClipUpdated` event (for pin/assign-group) or `ClipDeleted` event (for delete) so connected clients stay in
sync.

#### Scenario: PinClip publishes ClipUpdated
- **WHEN** `PinClip { id, pinned: true }` is handled for an existing clip
- **THEN** the clip's `is_pinned` becomes `true` and a `ClipUpdated` event referencing it is published

#### Scenario: DeleteClip publishes ClipDeleted
- **WHEN** `DeleteClip { id }` is handled for an existing clip
- **THEN** the clip is removed from default listings and a `ClipDeleted` event referencing it is published

### Requirement: ClearHistory removes matching clips per scope and publishes an event per removed clip
The `ClearHistory { scope }` handler SHALL remove the clips matching the requested scope via `clip-store`'s
bulk-clear capability and SHALL publish a `ClipDeleted` event for each removed clip.

#### Scenario: Clearing with "excluding pinned" scope leaves pinned clips and their events unpublished
- **WHEN** one pinned and one unpinned clip exist and `ClearHistory { scope: ExcludingPinned }` is handled
- **THEN** the unpinned clip is removed and a `ClipDeleted` event is published for it, while the pinned
  clip remains and no event is published for it

### Requirement: Rule commands take effect on the next ingest without a restart
`SaveRule` and `DeleteRule` SHALL persist the rule change via `clip-store` such that the very next ingest
of a matching capture reflects the change, with no daemon restart required.

#### Scenario: A newly saved exclusion rule applies to the next capture
- **WHEN** `SaveRule` creates an enabled rule excluding app `"1Password"`, and a capture event from that
  app arrives afterward
- **THEN** that capture is excluded from persistence per `clip-ingest-pipeline`'s exclusion behavior

### Requirement: PauseCapture toggles the watch loop and publishes CapturePaused
The `PauseCapture { paused }` handler SHALL set the daemon's paused state (consumed by the watch loop) and
SHALL publish a `CapturePaused` event reflecting the new state.

#### Scenario: Pausing capture is reflected in a published event
- **WHEN** `PauseCapture { paused: true }` is handled
- **THEN** a `CapturePaused` event with `paused: true` is published and subsequent capture events are not
  ingested (per `clipboard-watch-loop`)

### Requirement: Settings commands round-trip through the AppSettings model
`GetSettings` SHALL return the current `AppSettings` (via `clip-core`) as persisted, and `UpdateSettings`
SHALL persist the given changes such that a subsequent `GetSettings` reflects them.

#### Scenario: Updated setting is visible on next GetSettings
- **WHEN** `UpdateSettings` changes the retention window and `GetSettings` is issued afterward
- **THEN** the response reflects the new retention window

### Requirement: GetDiagnostics returns the active backend's capability report
`GetDiagnostics` SHALL return the diagnostics report produced by `clip-platform` for the currently active
backend, unmodified.

#### Scenario: GetDiagnostics reflects the active backend's capabilities
- **WHEN** `GetDiagnostics` is issued while the X11 backend is active
- **THEN** the response's capability flags match the X11 backend's `capabilities()` output
