## Why

Group assignment (organizing clips into named collections, filterable in Manager) has proven
unreliable in this Tauri 2 + WebKitGTK environment despite two independent implementation attempts:
a native `<select>` dropdown whose `change` event reported an empty value even when a real option
was visibly picked, and a replacement custom popover component that still failed to produce a
visible, correct assignment. Both were verified against the real running daemon and UI (not just
mocked tests) via IPC-level logging added specifically to diagnose this. Continuing to chase
WebKitGTK-specific widget/rendering quirks is not worth the engineering cost for a feature that,
per the user, "is not worth the effort." This change removes group functionality entirely rather
than carrying a half-working, low-confidence feature.

## What Changes

- **BREAKING**: Remove the `Group` domain model, `CoreError::InvalidGroupParent`, and
  `Clip::group_id` from `clip-core`.
- **BREAKING**: Remove the `groups` table, `clips.group_id` column, and all group-related queries
  (`insert`/`get`/`rename`/`list_all`/`list_children`/`delete`) from `clip-store`, via a new
  migration that drops them (existing user databases lose any group assignments; clips themselves
  are unaffected).
- **BREAKING**: Remove `Command::ListGroups`/`CreateGroup`/`DeleteGroup`/`AssignGroup` and
  `Event::GroupsChanged` from the `clip-ipc` wire protocol.
- Remove the `Store` trait's `list_groups`/`create_group`/`delete_group`/`set_group` methods and
  their `CommandHandler` match arms in `clipd` (both `SqliteStore` and the test `FakeStore`).
- Remove `list_groups_with`/`create_group_with`/`delete_group_with`/`assign_group_with` and their
  `#[tauri::command]` wrappers, plus their `generate_handler!` registrations, in
  `clip-ui-tauri/src-tauri`.
- Remove the frontend `Group` type, `GroupsChanged` from `DaemonEvent`, the `groups`/`loadGroups`/
  `setClipGroup` store state, Manager's group filter dropdown and per-clip group picker (including
  the `.group-picker*` CSS), and Settings' entire "Groups" section (state, handlers, JSX).
- Remove the temporary diagnostic logging (`eprintln!` in `clip-ui-tauri`'s `Client` impl,
  `tracing::info!`/`error!` wrapper in `clipd`'s `CommandHandler::handle`) added earlier in this
  session specifically to debug group assignment, now moot.
- Delete now-orphaned tests across all touched crates instead of leaving them to rot against
  deleted code.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `group-management` (originally added by `clip-store-persistence`): all requirements removed -
  the capability is deleted outright, not modified.
- `group-crud-ipc` (originally added by `group-crud-settings`): all requirements removed.
- `group-management-ui` (originally added by `group-crud-settings`): all requirements removed.
- `ipc-protocol` (originally added by `clip-ipc-transport`): the command-variant list requirement
  drops `AssignGroup` and `ListGroups`; the rest of the variant list is unaffected.
- `ipc-command-handlers` (originally added by `clipd-daemon-core`, modified by
  `clip-rules-listing`): the read-only-query requirement drops `ListGroups`; the mutating-command
  requirement drops `AssignGroup` (keeping `PinClip`/`DeleteClip`).
- `manager-window-ui` (originally added by `clip-ui-tauri-shell`): the filter requirement drops the
  group filter (keeping type/pinned); the inline-action requirement drops group reassignment
  (keeping pin/delete).

## Impact

Touches every crate in the workspace except `clip-platform`: `clip-core`, `clip-store` (including a
new migration), `clip-ipc`, `clipd`, `clip-ui-tauri` (both the Rust `src-tauri` side and the
React/TS frontend). No PRD milestone is un-done by this - group management was an incremental
addition on top of the already-complete core milestones (capture, search, paste, rules, settings),
none of which depend on groups. The Manager UI loses its group filter and per-clip group picker;
Settings loses its "Groups" section. `openspec/changes/clip-store-persistence` and
`openspec/changes/group-crud-settings` remain in place as historical record of what was built and
why it's now being removed - this change does not rewrite history, only removes the shipped code.
