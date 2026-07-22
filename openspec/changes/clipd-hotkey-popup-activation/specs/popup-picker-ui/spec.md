## MODIFIED Requirements

### Requirement: Popup opens with the search field focused
The popup SHALL focus its search input automatically every time it becomes visible - both on its first
mount and on every subsequent activation via the global hotkey (`HotkeyPressed`) - so the user can start
typing immediately without an extra click, matching the PRD's "search is focused on open" UX principle.
The popup window itself SHALL be shown and given OS focus when `HotkeyPressed` is received while it is
hidden.

#### Scenario: Opening the popup focuses the search field
- **WHEN** the popup is opened
- **THEN** the search input has focus without any additional user interaction

#### Scenario: A repeat hotkey press re-focuses the search field
- **WHEN** the popup is already open (or was previously hidden after a paste) and a `HotkeyPressed` event
  arrives again
- **THEN** the search input is re-focused without requiring a manual click

### Requirement: Empty query on open shows results in the order the backend returns them
The popup SHALL issue a fresh empty-query search on every activation (first open and every subsequent
`HotkeyPressed`), before the user types anything, and render the results in the order `SearchClips`
returns them, without re-sorting them client-side.

#### Scenario: Popup renders backend-provided ordering unchanged
- **WHEN** the popup opens and `SearchClips` with an empty query returns a pinned clip followed by two
  unpinned clips
- **THEN** the popup's list renders them in that same order

#### Scenario: A repeat hotkey press re-issues the empty-query search
- **WHEN** the popup was previously hidden (e.g. after a paste) and a `HotkeyPressed` event arrives again
- **THEN** a fresh empty-query `SearchClips` is issued and the list reflects its results, not stale results
  from the previous activation
