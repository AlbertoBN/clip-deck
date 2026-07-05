## ADDED Requirements

### Requirement: X11 backend reads the current clipboard text content
The X11 `ClipboardBackend` implementation SHALL read the current clipboard selection's text content via
`read_current` and return it as a plain-text representation, returning an empty snapshot when the
clipboard holds no text.

#### Scenario: Reading a populated clipboard returns its text
- **WHEN** the X11 clipboard selection holds the text `"ssh user@host"`
- **THEN** `read_current` returns a snapshot whose plain-text representation is `"ssh user@host"`

#### Scenario: Reading an empty clipboard returns an empty snapshot
- **WHEN** the X11 clipboard selection holds no content
- **THEN** `read_current` returns a snapshot with no representations

### Requirement: X11 backend writes content back to the clipboard
`set_current` SHALL place the given plain-text content onto the X11 clipboard selection such that a
subsequent `read_current` observes the same content.

#### Scenario: Written content is observable on next read
- **WHEN** `set_current` is called with plain-text content `"paste me"`
- **THEN** a subsequent `read_current` call returns a snapshot containing `"paste me"`

### Requirement: Watch loop emits a capture event only when content actually changes
The X11 backend's `start` watch loop SHALL emit a capture event only when the clipboard's content hash
differs from the previously observed hash, so copying the same text twice in a row (or an X11 selection-
owner churn event with unchanged content) does not produce duplicate capture events.

#### Scenario: Copying new content emits one event
- **WHEN** the watch loop is running and the clipboard content changes from empty to `"first copy"`
- **THEN** exactly one capture event is emitted with content `"first copy"`

#### Scenario: An unchanged-content notification does not emit a duplicate event
- **WHEN** the watch loop has already emitted an event for content `"same text"` and the underlying X11
  layer signals a selection event with the same content again
- **THEN** no additional capture event is emitted

### Requirement: X11 backend reports full baseline capability support
As the PRD's baseline fully-supported backend, the X11 `ClipboardBackend`'s `capabilities()` SHALL report
capture, paste-simulation, hotkey-registration, and focus-detection all as supported.

#### Scenario: X11 capabilities report every baseline flag supported
- **WHEN** `capabilities()` is called on the X11 backend
- **THEN** capture, paste-simulation, hotkeys, and focus-detection are all reported as supported
