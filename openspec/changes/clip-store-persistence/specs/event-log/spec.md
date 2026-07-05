## ADDED Requirements

### Requirement: Events are recorded with an optional clip association
The system SHALL support appending an event with an `event_type`, optional `clip_id`, and optional JSON
payload, matching the PRD's `events` table, and SHALL allow `clip_id` to be `NULL` for events not tied to
a specific clip (e.g. `CapturePaused`).

#### Scenario: Recording an event tied to a clip
- **WHEN** an event of type `"ClipCaptured"` is recorded with a `clip_id` pointing at an existing clip
- **THEN** fetching events for that clip includes the recorded event

#### Scenario: Recording an event with no clip association
- **WHEN** an event of type `"CapturePaused"` is recorded with no `clip_id`
- **THEN** the event is recorded successfully and appears in event-type queries

### Requirement: Events are queryable by clip and by type, newest first
The system SHALL support listing events for a given `clip_id` ordered by `created_at DESC`, and listing
events for a given `event_type` ordered by `created_at DESC`, matching the PRD's `idx_events_clip_id` and
`idx_events_type` indexes.

#### Scenario: Listing events for a clip returns only its events, newest first
- **WHEN** two events are recorded for one clip and one event is recorded for a different clip
- **THEN** listing events for the first clip returns exactly its two events with the most recent first

#### Scenario: Listing events by type returns only matching events
- **WHEN** events of type `"ClipCaptured"` and `"ClipDeleted"` both exist
- **THEN** listing events of type `"ClipDeleted"` excludes the `"ClipCaptured"` events

### Requirement: A deleted clip's events are retained with the association cleared
When a clip referenced by an event is hard-deleted from the database, the event's `clip_id` SHALL be set
to `NULL` rather than the event row being removed, matching `events.clip_id ... ON DELETE SET NULL`.

#### Scenario: Hard-deleting a clip nulls out its events' clip_id
- **WHEN** a clip with associated events is hard-deleted from the database
- **THEN** its events still exist afterward with `clip_id = NULL`
