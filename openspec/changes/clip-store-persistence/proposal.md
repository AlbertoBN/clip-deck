## Why

`clip-store` is the second crate in the PRD's build order: once `clip-core` provides domain types, the
daemon needs a durable, searchable place to put them. Without `clip-store`, neither `clip-ipc` (which
returns query results) nor `clipd` (which ingests and serves clips) have anything real to call.

## What Changes

- Implement the SQLite connection/pragma/migration layer (`clip-store::db`) that applies the PRD's
  `001_init.sql` schema (clips, clip_representations, clips_fts, groups, app_rules, settings, events) on
  startup.
- Implement clip CRUD (insert/update/delete/list/get) with content-hash + MIME dedup enforcement
  (`clip-store::clips`).
- Implement FTS5 synchronization (insert/update/delete triggers or equivalent application-code sync) and
  search queries with prefix matching, BM25 + recency/pinned ranking, and the empty-query fallback to
  `created_at DESC` with pinned-first ordering (`clip-store::fts`).
- Implement group CRUD and hierarchy queries (`clip-store::groups`).
- Implement app-rule CRUD for exclusion/privacy rules (`clip-store::rules`).
- Implement the audit/event log (`clip-store::events`).
- Implement retention/pruning queries (auto-delete windows, bulk clear) (`clip-store::retention`).

## Capabilities

### New Capabilities
- `clip-persistence`: Migration runner, connection/pragma setup, and clip CRUD with hash+MIME dedup
  enforcement via the unique index from the PRD schema.
- `clip-search-index`: FTS5 virtual table sync and search queries (prefix matching, ranking, empty-query
  fallback, filters by MIME family/pinned/group/favorite/source app).
- `group-management`: CRUD and hierarchy listing for groups/folders.
- `app-rules-management`: CRUD for per-app/per-MIME exclusion and privacy rules.
- `event-log`: Append-only audit/usage event recording and querying by clip or event type.
- `retention-policy`: Auto-delete window pruning and bulk clear (single clip delete, clear-all, clear-by-
  scope).

### Modified Capabilities
(none - `clip-core`'s capabilities are consumed as-is, not changed)

## Impact

- Affected code: `crates/clip-store/src/{db,clips,fts,groups,rules,events,retention}.rs`,
  `migrations/001_init.sql` (currently a placeholder comment - this change fills in the real DDL from the
  PRD), `crates/clip-store/Cargo.toml`.
- Depends on: `clip-core-foundations` (`Clip`, `ClipRepresentation`, `Group`, `Rule`, `AppSettings`,
  hashing, MIME normalization, search-query parsing types).
- Downstream: unlocks `clip-ipc-transport`'s DTOs to be backed by real data, and `clipd-daemon-core`'s
  ingest pipeline and command handlers.
