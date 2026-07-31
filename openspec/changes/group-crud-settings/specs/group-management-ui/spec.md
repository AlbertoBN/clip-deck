## ADDED Requirements

### Requirement: Groups can be created and deleted from settings
Settings SHALL fetch the existing group list via `ListGroups` on mount, let the user create a new
top-level group by entering a name, and delete any existing group.

#### Scenario: Groups persisted in a prior session appear on load
- **WHEN** the daemon's `ListGroups` returns a group that was never created during the current session
- **THEN** that group appears in the Settings groups list

#### Scenario: Creating a group issues CreateGroup with the entered name
- **WHEN** the user enters a name and submits the new-group form
- **THEN** `CreateGroup` is issued with a group whose `name` matches the entered text and whose
  `parent_group_id` is `null`

#### Scenario: Deleting a group issues DeleteGroup and removes it from the list
- **WHEN** the user deletes a group from the list
- **THEN** `DeleteGroup` is issued for that group's `id`, and the group no longer appears in the list
