## MODIFIED Requirements

### Requirement: Rules can be created, enabled/disabled, and deleted from settings
The settings screen SHALL let the user create a new app/window/MIME exclusion rule, toggle an existing
rule's enabled state, and delete a rule, issuing `SaveRule`/`DeleteRule` accordingly. The settings screen
SHALL fetch the existing rule list via `ListRules` on mount, so rules from a prior session (or created
outside the current session) are visible without requiring a create/delete action first.

#### Scenario: Creating a rule issues SaveRule
- **WHEN** the user fills in an app-exclusion rule and confirms creation
- **THEN** a `SaveRule` command is issued with the entered `app_match` and `enabled: true`

#### Scenario: Deleting a rule issues DeleteRule and removes it from the list
- **WHEN** the user deletes an existing rule and the operation succeeds
- **THEN** a `DeleteRule` command was issued for that rule's id and it no longer appears in the rules list

#### Scenario: Rules persisted in a prior session appear on load
- **WHEN** the daemon's `ListRules` returns a rule that was never created during the current session
- **THEN** that rule renders in the settings screen's rule list without any user action
