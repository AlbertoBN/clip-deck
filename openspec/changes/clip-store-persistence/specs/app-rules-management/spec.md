## ADDED Requirements

### Requirement: App rules support CRUD operations
The system SHALL support creating, updating, enabling/disabling, and deleting app exclusion/privacy rules,
matching the PRD's `app_rules` table (`app_match`, `window_match`, `mime_match`, `action`, `enabled`).

#### Scenario: Created rule is fetchable by id
- **WHEN** a rule excluding app `"1Password"` is created
- **THEN** fetching it by id returns a rule with `app_match = "1Password"`

#### Scenario: Disabling a rule updates only its enabled flag
- **WHEN** an existing enabled rule is updated with `enabled = false`
- **THEN** re-fetching it shows `enabled == false` and all other fields unchanged

### Requirement: Enabled rules are queryable for ingest-time evaluation
The system SHALL support listing all currently-enabled rules ordered for evaluation, so the ingest
pipeline can check a captured clip's `AppContext` and MIME type against every active rule without loading
disabled rules.

#### Scenario: Listing enabled rules excludes disabled ones
- **WHEN** one enabled rule and one disabled rule both exist
- **THEN** listing enabled rules returns only the enabled one

#### Scenario: Listing enabled rules includes rules with no window or MIME match set
- **WHEN** a rule has only `app_match` set and no `window_match`/`mime_match`
- **THEN** it is included in the enabled-rules listing
