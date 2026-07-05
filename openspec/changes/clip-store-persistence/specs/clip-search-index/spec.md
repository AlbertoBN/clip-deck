## ADDED Requirements

### Requirement: FTS index stays synchronized with the clips table
The system SHALL keep `clips_fts` synchronized with `clips` on insert, update, and delete - whether via
SQL triggers or equivalent application-code sync - such that the FTS index never returns a clip that no
longer exists or misses a clip that does.

#### Scenario: Inserting a clip makes it searchable
- **WHEN** a clip with `display_text = "deploy staging via ssh"` is inserted
- **THEN** searching for `"deploy"` returns that clip

#### Scenario: Updating display text updates search results
- **WHEN** an existing clip's `display_text` is updated from `"foo"` to `"bar"`
- **THEN** searching for `"foo"` no longer returns it and searching for `"bar"` does

#### Scenario: Deleting a clip removes it from search results
- **WHEN** a clip is soft-deleted
- **THEN** it no longer appears in search results

### Requirement: Incremental search uses prefix matching on the last term
Search SHALL support prefix matching so an in-progress query (e.g. the user has typed `"depl"`) returns
clips whose indexed text starts with that prefix, matching the PRD's incremental-search requirement.

#### Scenario: Prefix of a word matches the full word
- **WHEN** a clip with `display_text = "deploy staging"` is indexed and the search query is `"depl"`
- **THEN** the clip is included in the results

### Requirement: Empty query falls back to recency with pinned-first ordering
When the search query is empty, the system SHALL return clips ordered with pinned clips first, then by
`created_at DESC`, rather than returning no results or an arbitrary order.

#### Scenario: Empty query returns pinned clips before unpinned ones
- **WHEN** one pinned clip created earlier and one unpinned clip created later both exist, and search is
  called with an empty query
- **THEN** the pinned clip appears before the unpinned clip in the results

#### Scenario: Empty query orders unpinned clips by recency
- **WHEN** two unpinned clips exist with different `created_at` values and search is called with an
  empty query
- **THEN** the more recently created clip appears first

### Requirement: Search results can be filtered by type, pinned state, and group
The system SHALL support narrowing search results by MIME family, pinned-only, group id, favorite state,
and source app, applying filters whether or not a text query is present.

#### Scenario: Filtering by group excludes clips outside that group
- **WHEN** two clips exist in different groups and search is called with a `group_id` filter matching
  only one of them
- **THEN** only the matching clip is returned

#### Scenario: Filtering by pinned-only excludes unpinned clips
- **WHEN** one pinned and one unpinned clip exist and search is called with `pinned_only = true`
- **THEN** only the pinned clip is returned

### Requirement: Ranking combines relevance with recency and pinned boosts
Non-empty-query search results SHALL be ranked using a combination of FTS5 BM25 relevance and the
recency/pinned boost inputs produced by `clip-core`'s search-query parsing, rather than raw BM25 alone.

#### Scenario: Pinned clip ranks above an equally relevant unpinned clip
- **WHEN** a pinned clip and an unpinned clip both match a query with equal text relevance
- **THEN** the pinned clip is ranked first
