## ADDED Requirements

### Requirement: Hotkey binding can be viewed and changed, with invalid bindings rejected before saving
The settings screen SHALL display the current global hotkey binding and let the user change it, validating
the new binding (via `clip-platform`'s parser, surfaced through `UpdateSettings`) and showing the returned
error instead of saving when the binding is invalid or conflicts with another registration.

#### Scenario: A valid new binding is saved
- **WHEN** the user enters a valid, non-conflicting hotkey binding and confirms
- **THEN** `UpdateSettings` is issued with the new binding and the screen reflects it as saved

#### Scenario: An invalid binding is not saved
- **WHEN** the user enters an invalid hotkey binding and confirms
- **THEN** the screen shows the validation error and does not report the binding as saved

### Requirement: Diagnostics screen shows unsupported capabilities explicitly, not hidden
The settings/diagnostics screen SHALL display every capability from `GetDiagnostics`'s report, explicitly
labeling unsupported capabilities as unsupported rather than omitting them, matching the PRD's requirement
to surface environment support state clearly instead of failing silently.

#### Scenario: An unsupported capability is shown, not hidden
- **WHEN** `GetDiagnostics` reports hotkey registration as unsupported on the current backend
- **THEN** the diagnostics screen displays hotkey registration with an unsupported/explicit-gap indicator,
  rather than omitting that row

### Requirement: Rules can be created, enabled/disabled, and deleted from settings
The settings screen SHALL let the user create a new app/window/MIME exclusion rule, toggle an existing
rule's enabled state, and delete a rule, issuing `SaveRule`/`DeleteRule` accordingly.

#### Scenario: Creating a rule issues SaveRule
- **WHEN** the user fills in an app-exclusion rule and confirms creation
- **THEN** a `SaveRule` command is issued with the entered `app_match` and `enabled: true`

#### Scenario: Deleting a rule issues DeleteRule and removes it from the list
- **WHEN** the user deletes an existing rule and the operation succeeds
- **THEN** a `DeleteRule` command was issued for that rule's id and it no longer appears in the rules list
