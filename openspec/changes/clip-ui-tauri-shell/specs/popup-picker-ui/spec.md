## ADDED Requirements

### Requirement: Popup opens with the search field focused
The popup SHALL focus its search input automatically when it opens (via the global hotkey), so the user
can start typing immediately without an extra click, matching the PRD's "search is focused on open" UX
principle.

#### Scenario: Opening the popup focuses the search field
- **WHEN** the popup is opened
- **THEN** the search input has focus without any additional user interaction

### Requirement: Empty query on open shows results in the order the backend returns them
On open, before the user types anything, the popup SHALL issue a search with an empty query and render the
results in the order `SearchClips` returns them, without re-sorting them client-side.

#### Scenario: Popup renders backend-provided ordering unchanged
- **WHEN** the popup opens and `SearchClips` with an empty query returns a pinned clip followed by two
  unpinned clips
- **THEN** the popup's list renders them in that same order

### Requirement: Keyboard navigation moves the selection without a mouse
Arrow-up/arrow-down SHALL move the highlighted selection through the result list, and every other popup
action (search, select, paste, dismiss) SHALL also be reachable from the keyboard alone, matching the
PRD's "common actions must be reachable with keyboard only" principle.

#### Scenario: Arrow-down moves selection to the next result
- **WHEN** the first result is selected and arrow-down is pressed
- **THEN** the second result becomes selected

#### Scenario: Arrow-up at the first result does not select past the top
- **WHEN** the first result is selected and arrow-up is pressed
- **THEN** the first result remains selected (selection does not wrap to an invalid index)

### Requirement: Enter pastes the selected clip and closes the popup
Pressing Enter SHALL issue `PasteClip` for the currently selected result and close the popup once the
paste command has been sent, matching Ditto's core Enter-to-paste behavior.

#### Scenario: Enter on a selected result triggers paste and closes the popup
- **WHEN** a result is selected and Enter is pressed
- **THEN** a `PasteClip` command is issued for that result's clip id and the popup closes

### Requirement: Typing updates results incrementally
As the user types into the search field, the popup SHALL issue an updated `SearchClips` query reflecting
the current input (debounced to avoid one request per keystroke) and re-render the list with the latest
results.

#### Scenario: Typing a query updates the rendered results
- **WHEN** the user types `"ssh"` into the search field
- **THEN** a `SearchClips` command with query `"ssh"` is issued and the list renders that query's results
