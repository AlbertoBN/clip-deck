## ADDED Requirements

### Requirement: Groups support CRUD operations
The system SHALL support creating, fetching, renaming, reordering (`sort_order`), and deleting groups,
matching the PRD's `groups` table.

#### Scenario: Created group is fetchable by id
- **WHEN** a group named `"SSH commands"` is created
- **THEN** fetching it by id returns a group with that name

#### Scenario: Renaming a group updates its name only
- **WHEN** an existing group is renamed to `"Ops snippets"`
- **THEN** re-fetching it shows the new name and an unchanged id

### Requirement: Group hierarchy queries list children of a parent
The system SHALL support listing the direct child groups of a given parent group id (or the top-level
groups when no parent is given), matching the `groups.parent_group_id` self-reference.

#### Scenario: Listing children returns only direct children
- **WHEN** group `"Work"` has child groups `"SSH"` and `"SQL"`, and `"SSH"` has its own child `"Prod"`
- **THEN** listing children of `"Work"` returns `"SSH"` and `"SQL"` but not `"Prod"`

#### Scenario: Listing top-level groups excludes nested groups
- **WHEN** groups `"Work"` (no parent) and `"SSH"` (parent `"Work"`) both exist
- **THEN** listing top-level groups returns `"Work"` but not `"SSH"`

### Requirement: Deleting a group cascades to child groups and detaches clips
Deleting a group SHALL cascade-delete its descendant groups (matching
`parent_group_id ... ON DELETE CASCADE`) and SHALL set `group_id` to `NULL` on any clips that referenced
the deleted group or its descendants (matching `clips.group_id ... ON DELETE SET NULL`), rather than
failing or leaving orphaned references.

#### Scenario: Deleting a parent group deletes its child group
- **WHEN** group `"Work"` has child group `"SSH"` and `"Work"` is deleted
- **THEN** `"SSH"` no longer exists

#### Scenario: Deleting a group detaches its clips instead of deleting them
- **WHEN** a clip is assigned to group `"Work"` and `"Work"` is deleted
- **THEN** the clip still exists afterward with `group_id = NULL`
