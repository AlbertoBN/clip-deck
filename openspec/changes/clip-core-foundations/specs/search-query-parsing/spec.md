## ADDED Requirements

### Requirement: Raw search input is parsed into structured query terms
The system SHALL parse a raw search string typed by the user into a structured query representation
(a list of terms) that `clip-store` can turn into an FTS5 `MATCH` expression, rather than passing the raw
string straight through to SQL.

#### Scenario: Simple query splits into terms
- **WHEN** `parse_query("ssh deploy")` is called
- **THEN** it returns a query with terms `["ssh", "deploy"]`

#### Scenario: Extra whitespace does not produce empty terms
- **WHEN** `parse_query("  ssh   deploy  ")` is called
- **THEN** it returns a query with exactly terms `["ssh", "deploy"]`

### Requirement: Last term is marked for prefix matching
To support incremental (as-you-type) search, the parsed query SHALL mark the final term as a prefix
match candidate while earlier terms remain exact/phrase terms, matching the PRD's incremental-search
requirement.

#### Scenario: Final term is flagged as a prefix term
- **WHEN** `parse_query("ssh depl")` is called
- **THEN** the term `"depl"` is marked as a prefix term and `"ssh"` is not

### Requirement: Empty or whitespace-only query is recognized explicitly
The parser SHALL recognize an empty or whitespace-only input as an explicit "no query" result (not an
empty term list treated the same as a real query), so callers can implement the PRD's fallback to
`created_at DESC` with pinned-first ordering.

#### Scenario: Blank string produces an explicit empty query
- **WHEN** `parse_query("")` is called
- **THEN** the result reports `is_empty() == true`

#### Scenario: Whitespace-only string produces an explicit empty query
- **WHEN** `parse_query("   ")` is called
- **THEN** the result reports `is_empty() == true`

### Requirement: Filters for type, pinned state, and group are modeled separately from query terms
The system SHALL model search filters (MIME family, pinned-only, group id, favorite, source app) as a
distinct `SearchFilters` structure separate from the free-text query terms, so filters can be applied
whether or not a text query is present.

#### Scenario: Filters can be constructed without any text query
- **WHEN** a `SearchFilters` is constructed with only `pinned_only = true`
- **THEN** it is valid on its own and does not require a non-empty query

### Requirement: Ranking-boost inputs are exposed for recency and pinned status
The system SHALL expose the ranking-boost inputs described in the PRD (recency reference timestamp,
pinned-boost flag) as part of the parsed query/filter output, so `clip-store`'s ranking query can combine
them with FTS5 BM25 without recomputing "now" or duplicating pin logic.

#### Scenario: Ranking inputs default to boosting pinned results
- **WHEN** a query is parsed without explicit ranking overrides
- **THEN** the resulting ranking inputs report pinned-boost enabled
