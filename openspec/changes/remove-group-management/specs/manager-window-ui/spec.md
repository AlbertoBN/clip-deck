## MODIFIED Requirements

### Requirement: Manager lists clips with type/pinned/group filters applied via IPC
The manager window SHALL render the results of a `SearchClips` call whose filters reflect the user's
current filter selections (MIME type, pinned-only), re-issuing the query whenever a filter changes.
The group filter is removed, per the removal of group management; only MIME type and pinned-only
remain.

#### Scenario: Selecting pinned-only re-queries with that filter
- **WHEN** the user checks "Pinned only" in the filter bar
- **THEN** a `SearchClips` command with `pinned_only: true` is issued and the list updates to its
  results

### Requirement: Pin, delete, and group assignment are available inline without leaving the manager
The manager SHALL let the user pin/unpin or delete a clip directly from the list (row action or
context menu), issuing the corresponding `PinClip` or `DeleteClip` command without navigating away
from the manager window. Group reassignment is removed, per the removal of group management.

#### Scenario: Pinning a clip from the manager issues PinClip
- **WHEN** the user triggers the pin action on a clip row
- **THEN** a `PinClip` command with that clip's id and `pinned: true` is issued

#### Scenario: Deleting a clip from the manager removes it from the visible list
- **WHEN** the user triggers delete on a clip row and the corresponding `ClipDeleted` event is received
- **THEN** that clip no longer appears in the manager's list
