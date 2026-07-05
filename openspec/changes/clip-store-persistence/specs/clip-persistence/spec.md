## ADDED Requirements

### Requirement: Migrations apply the full schema idempotently
The system SHALL apply `migrations/001_init.sql` (clips, clip_representations, groups, app_rules,
settings, events, and the `clips_fts` virtual table) on startup, and running the migration runner against
an already-migrated database SHALL be a no-op rather than an error.

#### Scenario: Fresh database gets the full schema
- **WHEN** the migration runner is applied to a brand-new SQLite file
- **THEN** the `clips`, `clip_representations`, `groups`, `app_rules`, `settings`, `events`, and
  `clips_fts` objects all exist afterward

#### Scenario: Re-running migrations on an already-migrated database is a no-op
- **WHEN** the migration runner is applied twice in a row to the same database file
- **THEN** the second run succeeds without error and does not duplicate any schema objects

### Requirement: Connection setup enables WAL and foreign keys
Every connection opened by `clip-store` SHALL enable `PRAGMA journal_mode = WAL` and
`PRAGMA foreign_keys = ON`, matching the PRD's proposed migration file, so foreign-key cascades (e.g.
group deletion) and concurrent read/write behavior work as designed.

#### Scenario: New connection reports WAL journal mode
- **WHEN** a connection is opened through `clip-store::db`
- **THEN** querying `PRAGMA journal_mode` on it returns `wal`

#### Scenario: New connection enforces foreign keys
- **WHEN** a connection is opened through `clip-store::db` and an insert violates a foreign key
  (e.g. a clip referencing a non-existent `group_id`)
- **THEN** the insert fails with a foreign-key-constraint error

### Requirement: Clip insert enforces content-hash + MIME dedup
Inserting a clip SHALL fail with a dedup conflict (not a generic SQL error) when a non-deleted clip
already exists with the same `(content_hash, primary_mime)`, matching the PRD's
`idx_clips_hash_mime` unique index on `(content_hash, primary_mime, is_deleted)`.

#### Scenario: Inserting a duplicate clip is rejected
- **WHEN** a clip with `content_hash = "abc"`, `primary_mime = "text/plain"` is inserted, and a second
  clip with the same hash and MIME is inserted while the first is not deleted
- **THEN** the second insert returns a dedup-conflict error and no second row is created

#### Scenario: A previously deleted clip does not block re-insertion
- **WHEN** a clip with `content_hash = "abc"`, `primary_mime = "text/plain"` is soft-deleted
  (`is_deleted = 1`), and a new clip with the same hash and MIME is inserted
- **THEN** the new insert succeeds

### Requirement: Clip CRUD supports get, list, update, and soft delete
The system SHALL support fetching a single clip by id, listing clips ordered by `created_at DESC` by
default, updating mutable fields (pin, favorite, group, last_used_at), and soft-deleting a clip by setting
`is_deleted = 1` rather than removing the row outright.

#### Scenario: Get returns a previously inserted clip
- **WHEN** a clip is inserted and then fetched by its id
- **THEN** the fetched clip's fields match what was inserted

#### Scenario: Soft delete hides the clip from default listing
- **WHEN** a clip is soft-deleted
- **THEN** it no longer appears in the default `list_clips()` results but still exists in the database

#### Scenario: Pinning a clip updates only the pin flag
- **WHEN** an existing clip is updated with `is_pinned = true`
- **THEN** re-fetching the clip shows `is_pinned == true` and all other fields unchanged

### Requirement: Representations are stored and retrieved in ordinal order
Inserting a clip with multiple `ClipRepresentation` values SHALL persist each as its own
`clip_representations` row referencing the parent clip, and retrieving a clip's representations SHALL
return them ordered by `ordinal`.

#### Scenario: Two representations round-trip in order
- **WHEN** a clip is inserted with a plain-text representation at ordinal 0 and an HTML representation
  at ordinal 1
- **THEN** fetching the clip's representations returns plain-text first and HTML second
