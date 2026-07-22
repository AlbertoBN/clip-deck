## MODIFIED Requirements

### Requirement: Enabled rules are queryable for ingest-time evaluation
The system SHALL support listing all currently-enabled rules ordered for evaluation, so the ingest
pipeline can check a captured clip's `AppContext` and MIME type against every active rule without loading
disabled rules. The system SHALL ALSO support listing every rule regardless of `enabled` state, ordered
deterministically, for display in a settings/management UI.

#### Scenario: Listing enabled rules excludes disabled ones
- **WHEN** one enabled rule and one disabled rule both exist
- **THEN** listing enabled rules returns only the enabled one

#### Scenario: Listing enabled rules includes rules with no window or MIME match set
- **WHEN** a rule has only `app_match` set and no `window_match`/`mime_match`
- **THEN** it is included in the enabled-rules listing

#### Scenario: Listing all rules includes both enabled and disabled rules
- **WHEN** one enabled rule and one disabled rule both exist
- **THEN** listing all rules returns both, in a stable deterministic order
