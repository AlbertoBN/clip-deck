## Why

The Manager view already has a group filter dropdown and a per-clip group-assignment dropdown, both
populated by the daemon's `ListGroups` command - but nothing anywhere lets a user create or delete a
group in the first place. Both dropdowns are permanently empty in practice. Groups can currently only be
created by hand-inserting rows into SQLite, which isn't a real feature.

## What Changes

- `clip-ipc` (`crates/clip-ipc`): protocol gains `Command::CreateGroup { group: Group }` and
  `Command::DeleteGroup { id: String }`, alongside the existing `ListGroups`/`AssignGroup`.
- `clipd` (`crates/clipd`): the `Store` trait gains `create_group`/`delete_group`, implemented on both
  `SqliteStore` (delegating to `clip-store::groups::insert`/`delete`, both of which already exist) and the
  test `FakeStore`. `CommandHandler::handle` gains matching arms - `CreateGroup` re-validates the group
  server-side via `Group::new(id, name, parent_group_id)` before persisting (same pattern as
  `UpdateSettings` re-validating `hotkey_binding` via `parse_binding`), rather than trusting the
  client-constructed struct outright.
- `clip-ui-tauri` (`crates/clip-ui-tauri/src-tauri`): new `create_group`/`delete_group` Tauri commands
  (mirroring the existing `save_rule`/`delete_rule` wrappers), registered in the invoke handler.
- `clip-ui-tauri` frontend: a new "Groups" section in Settings, mirroring the existing Rules section's
  shape exactly - a name input + "Add group" button, and a list of existing groups each with a "Delete
  group" button.

## Capabilities

### New Capabilities
- `group-crud-ipc` (owned by `clip-ipc` + `clipd`): `CreateGroup`/`DeleteGroup` commands, handled by the
  daemon with server-side re-validation on create.
- `group-management-ui` (owned by `clip-ui-tauri`): a Settings section for creating and deleting groups.

### Modified Capabilities
- None. `group-management` (owned by `clip-store`, from the already-archived `clip-store-persistence`
  change) already fully covers the store-layer CRUD and the cascade/detach-on-delete behavior this change
  relies on - it isn't being changed, only consumed from a new layer above it.

## Impact

- Affected crates: `clip-ipc`, `clipd`, `clip-ui-tauri` (both the Tauri host and the React frontend).
  `clip-core` and `clip-store` are unchanged - `Group::new`'s validation and `clip-store::groups`'
  `insert`/`delete` functions already exist and are reused as-is.
- Sits after `clip-store-persistence` (already implemented, provides the store layer this builds on) in
  the PRD's build order; this is UI/IPC-layer work analogous to `clip-rules-listing`'s prior treatment of
  rules.
- Non-goal: group rename, reparenting/nesting UI, or drag-to-reorder. The `Group` model supports
  `parent_group_id`/`sort_order`, but the Settings UI only creates flat, top-level groups
  (`parent_group_id: null`) - hierarchy management isn't part of this change.
- Non-goal: live cross-view sync between Settings' group list and Manager's group dropdowns. Manager
  already only loads groups once on its own mount (same pre-existing behavior as its Rules-adjacent code);
  this change doesn't add a refresh-on-change mechanism.
- Depends on already-implemented work: `clip-store-persistence` (`Group` model, `clip-store::groups`
  module, DB schema's `ON DELETE CASCADE`/`ON DELETE SET NULL`), `clip-rules-listing` (the `SaveRule`/
  `DeleteRule` + Settings-section pattern this change mirrors).
