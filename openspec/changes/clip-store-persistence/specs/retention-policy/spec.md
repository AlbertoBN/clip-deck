## ADDED Requirements

### Requirement: Retention pruning removes clips older than the configured window, excluding pinned clips
The system SHALL permanently remove clips whose `created_at` is older than the configured retention
window when a prune is run, and SHALL exclude pinned clips from pruning regardless of age, matching the
PRD's retention-settings requirement.

#### Scenario: Old unpinned clip is pruned
- **WHEN** the retention window is 30 days and an unpinned clip was created 40 days ago
- **THEN** running prune removes that clip

#### Scenario: Old pinned clip survives pruning
- **WHEN** the retention window is 30 days and a pinned clip was created 40 days ago
- **THEN** running prune does not remove that clip

#### Scenario: No retention window configured means prune is a no-op
- **WHEN** no retention window is configured (keep forever)
- **THEN** running prune removes no clips regardless of age

### Requirement: Bulk clear supports an all-clips scope and an excluding-pinned scope
The system SHALL support clearing history with a selectable scope of "all clips" or "all except pinned",
matching the PRD's `ClearHistory { scope }` command, removing the matching rows immediately rather than
only soft-deleting them.

#### Scenario: Clearing with "all" scope removes pinned clips too
- **WHEN** one pinned and one unpinned clip exist and clear is called with scope "all"
- **THEN** neither clip exists afterward

#### Scenario: Clearing with "excluding pinned" scope keeps pinned clips
- **WHEN** one pinned and one unpinned clip exist and clear is called with scope "excluding pinned"
- **THEN** the pinned clip still exists and the unpinned clip does not

### Requirement: Single-clip deletion is available independent of bulk clear or retention
The system SHALL support deleting one specific clip by id on demand, independent of the retention window
or a bulk clear operation, matching the PRD's per-clip `DeleteClip` command.

#### Scenario: Deleting one clip does not affect others
- **WHEN** two clips exist and one is deleted by id
- **THEN** the deleted clip no longer appears in listings and the other clip is unaffected
