## MODIFIED Requirements

### Requirement: Read-only query commands return live data without mutating state
`SearchClips`, `GetClip`, `ListRules`, `GetSettings`, and `GetDiagnostics` SHALL be handled by
querying `clip-store`/`clip-core`/`clip-platform` directly and returning current data, without side effects
on the underlying data. `ListGroups` is no longer part of this set, per the removal of group management.

#### Scenario: SearchClips reflects a just-ingested clip
- **WHEN** a clip is ingested and then a `SearchClips` command matching its text is issued
- **THEN** the response includes that clip

#### Scenario: GetClip does not mutate last_used_at
- **WHEN** `GetClip` is issued for an existing clip
- **THEN** the clip's `last_used_at` is unchanged by the query itself

#### Scenario: ListRules returns every rule regardless of enabled state
- **WHEN** one enabled rule and one disabled rule both exist
- **THEN** `ListRules` returns both

#### Scenario: ListRules reflects a rule saved earlier in the session
- **WHEN** `SaveRule` creates a rule and `ListRules` is issued afterward
- **THEN** the response includes that rule

### Requirement: Mutating commands persist their change and publish the matching event
`PinClip` and `DeleteClip` SHALL apply their change via `clip-store` and SHALL publish a
`ClipUpdated` event (for pin) or `ClipDeleted` event (for delete) so connected clients stay in
sync. `AssignGroup` is no longer part of this set, per the removal of group management.

#### Scenario: PinClip publishes ClipUpdated
- **WHEN** `PinClip { id, pinned: true }` is handled for an existing clip
- **THEN** the clip's `is_pinned` becomes `true` and a `ClipUpdated` event referencing it is published

#### Scenario: DeleteClip publishes ClipDeleted
- **WHEN** `DeleteClip { id }` is handled for an existing clip
- **THEN** the clip is removed from default listings and a `ClipDeleted` event referencing it is published
