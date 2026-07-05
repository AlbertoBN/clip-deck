## ADDED Requirements

### Requirement: Hotkey binding is configurable rather than hardcoded
The system SHALL register the popup-activation global hotkey using a binding read from `AppSettings`
(e.g. `"Ctrl+Shift+V"`), parsing it into the underlying key/modifier representation, rather than hardcoding
one fixed combination.

#### Scenario: A configured binding parses into the expected modifiers and key
- **WHEN** the hotkey binding string `"Ctrl+Shift+V"` is parsed
- **THEN** the resulting binding has the Ctrl and Shift modifiers and key `V`

#### Scenario: An invalid binding string is rejected at parse time
- **WHEN** the hotkey binding string `"NotAKey+++"` is parsed
- **THEN** parsing returns an error rather than a partially-valid binding

### Requirement: Registered hotkey triggers popup activation
Once registered, pressing the configured hotkey combination SHALL trigger the popup-activation callback
exactly once per press.

#### Scenario: Pressing the registered combination triggers activation
- **WHEN** the configured hotkey combination is registered and then pressed
- **THEN** the popup-activation callback is invoked exactly once

### Requirement: Registering an already-in-use hotkey surfaces an error
Registration SHALL return an error identifying the conflict, rather than silently failing to register or
silently overriding the other registration, when the requested hotkey combination is already claimed by
another application (or otherwise cannot be registered).

#### Scenario: Conflicting registration returns an error
- **WHEN** a hotkey combination that is already registered elsewhere on the system is requested
- **THEN** registration returns an error rather than reporting success
