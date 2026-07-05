## ADDED Requirements

### Requirement: Manager lists clips with type/pinned/group filters applied via IPC
The manager window SHALL render the results of a `SearchClips` call whose filters reflect the user's
current filter selections (MIME type, pinned-only, group), re-issuing the query whenever a filter changes.

#### Scenario: Selecting a group filter re-queries with that group
- **WHEN** the user selects group `"Work"` in the filter bar
- **THEN** a `SearchClips` command with `group_id` filter `"Work"` is issued and the list updates to its
  results

### Requirement: Pin, delete, and group assignment are available inline without leaving the manager
The manager SHALL let the user pin/unpin, delete, or reassign the group of a clip directly from the list
(row action or context menu), issuing the corresponding `PinClip`, `DeleteClip`, or `AssignGroup` command
without navigating away from the manager window.

#### Scenario: Pinning a clip from the manager issues PinClip
- **WHEN** the user triggers the pin action on a clip row
- **THEN** a `PinClip` command with that clip's id and `pinned: true` is issued

#### Scenario: Deleting a clip from the manager removes it from the visible list
- **WHEN** the user triggers delete on a clip row and the corresponding `ClipDeleted` event is received
- **THEN** that clip no longer appears in the manager's list

### Requirement: Bulk clear invokes ClearHistory with the selected scope
The manager SHALL expose a bulk-clear action that lets the user choose a scope (all clips, or all except
pinned) and issues `ClearHistory` with that scope, updating the list as the resulting `ClipDeleted` events
arrive.

#### Scenario: Bulk clear excluding pinned removes only unpinned clips from the list
- **WHEN** the user triggers bulk clear with scope "excluding pinned" and `ClipDeleted` events arrive for
  the unpinned clips
- **THEN** those clips disappear from the list while pinned clips remain visible

### Requirement: Manager reflects newly captured clips without a manual refresh
When a `ClipCaptured` event is received while the manager window is open, the manager SHALL update its
visible list to include the new clip if it matches the current filters, without requiring the user to
re-trigger the search manually.

#### Scenario: A newly captured matching clip appears live
- **WHEN** the manager is showing unfiltered recent clips and a `ClipCaptured` event arrives for a new
  clip
- **THEN** the new clip appears in the list without the user taking any action
