## ADDED Requirements

### Requirement: Ingest normalizes representations before persisting
The ingest pipeline SHALL run every representation in a captured snapshot through `clip-core`'s MIME
normalization before persisting, so `clip-store` never receives an un-normalized MIME type.

#### Scenario: Mixed-case MIME type is normalized before storage
- **WHEN** a captured snapshot has a representation with MIME type `"TEXT/PLAIN"`
- **THEN** the persisted representation's MIME type is `"text/plain"`

### Requirement: Ingest skips persistence for a snapshot matching an exclusion rule
The ingest pipeline SHALL evaluate the captured `AppContext` and each representation's MIME type against
every enabled `clip-store` rule, and SHALL skip persisting the snapshot entirely (no `Clip` row, no
`ClipCaptured` event) when any rule with action "exclude" matches.

#### Scenario: Excluded app's copy is not persisted
- **WHEN** an enabled rule excludes app `"1Password"` and a capture event arrives with
  `AppContext { app: "1Password", .. }`
- **THEN** no `Clip` is persisted and no `ClipCaptured` event is published

#### Scenario: Non-matching app's copy is persisted normally
- **WHEN** the only enabled rule excludes app `"1Password"` and a capture event arrives from
  `AppContext { app: "gnome-terminal", .. }`
- **THEN** a `Clip` is persisted and a `ClipCaptured` event is published

### Requirement: Ingest treats a dedup conflict as a reuse, not a failure
When `clip-store` reports a dedup conflict for a captured snapshot's content-hash + MIME, ingest SHALL
treat this as "clip already exists" - updating the existing clip's `last_used_at` - rather than surfacing
an error or creating a duplicate row.

#### Scenario: Re-copying identical content updates last_used_at instead of erroring
- **WHEN** a clip with given content already exists (not deleted) and the same content is captured again
- **THEN** ingest completes successfully, no new `Clip` row is created, and the existing clip's
  `last_used_at` is updated

### Requirement: One capture snapshot becomes one clip with multiple representations
Ingest SHALL persist a single capture event's representations as one `Clip` with multiple
`ClipRepresentation` rows, not as separate clips, when the event includes more than one representation
(e.g. plain text and HTML from the same copy).

#### Scenario: Plain text and HTML from one copy become one clip
- **WHEN** a capture event includes both a plain-text and an HTML representation of the same copy
- **THEN** exactly one `Clip` is persisted, and it has both representations attached

### Requirement: Successful ingest publishes a ClipCaptured event
When a snapshot is persisted as a new clip, ingest SHALL publish a `ClipCaptured` event via `clip-ipc` so
connected UI clients update without polling.

#### Scenario: Persisting a new clip publishes ClipCaptured
- **WHEN** a snapshot for genuinely new content is ingested
- **THEN** a `ClipCaptured` event is published referencing the new clip's id
