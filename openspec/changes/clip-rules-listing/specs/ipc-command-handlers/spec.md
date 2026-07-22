## MODIFIED Requirements

### Requirement: Read-only query commands return live data without mutating state
`SearchClips`, `GetClip`, `ListGroups`, `ListRules`, `GetSettings`, and `GetDiagnostics` SHALL be handled by
querying `clip-store`/`clip-core`/`clip-platform` directly and returning current data, without side effects
on the underlying data.

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
