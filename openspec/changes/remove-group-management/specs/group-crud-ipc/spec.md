## REMOVED Requirements

### Requirement: CreateGroup command validates and persists a new group
**Reason**: The client-side group picker that would exercise this command (see
`group-management-ui` removal) proved unreliable in the real Tauri/WebKitGTK environment across two
independent implementations; the whole feature is being cut rather than kept half-working.
**Migration**: None. `Command::CreateGroup` is removed from the `clip-ipc` wire protocol; any
persisted groups become unreachable once `group-management`'s store layer is also removed.

The daemon SHALL handle `Command::CreateGroup { group }` by re-validating the group server-side via
`Group::new(group.id, group.name, group.parent_group_id)` (rejecting a self-referential
`parent_group_id` the same way the existing model constructor already does) before persisting it, rather
than trusting the client-constructed struct outright.

#### Scenario: A valid group is created and then listed
- **WHEN** the daemon receives `Command::CreateGroup` with a valid group (not its own parent)
- **THEN** a subsequent `Command::ListGroups` includes that group

#### Scenario: A self-referential group is rejected without being persisted
- **WHEN** the daemon receives `Command::CreateGroup` with `parent_group_id` equal to the group's own `id`
- **THEN** the command returns an error and the group is not persisted

### Requirement: DeleteGroup command deletes a group without deleting its clips
**Reason**: Same as above - the feature this command supports is being removed entirely.
**Migration**: None. `Command::DeleteGroup` is removed from the `clip-ipc` wire protocol.

The daemon SHALL handle `Command::DeleteGroup { id }` by deleting the group, relying on the existing
schema-level cascade/detach behavior (`clips.group_id ... ON DELETE SET NULL`) so clips referencing the
deleted group are detached, not deleted.

#### Scenario: Deleting a group detaches its clips instead of deleting them
- **WHEN** the daemon receives `Command::DeleteGroup` for a group that a clip is assigned to
- **THEN** the group is removed from a subsequent `Command::ListGroups`, and the clip still exists with
  its `group_id` cleared
