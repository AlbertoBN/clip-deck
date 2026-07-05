## ADDED Requirements

### Requirement: ClipboardBackend trait defines a uniform capture/paste/diagnostics surface
The system SHALL define a `ClipboardBackend` trait with `start`, `read_current`, `set_current`,
`focused_app`, `simulate_paste`, and `capabilities` methods, matching the PRD's adapter trait, so `clipd`
can depend on the trait object rather than a concrete X11 or Wayland type.

#### Scenario: A minimal fake backend satisfies the trait
- **WHEN** a test-only fake type implements all six `ClipboardBackend` methods
- **THEN** it can be used anywhere a `Box<dyn ClipboardBackend>` is expected, and calling each method
  returns the fake's configured value

### Requirement: BackendCapabilities defaults to no support until explicitly set
`BackendCapabilities` SHALL default to reporting no optional capability as supported (capture, paste,
hotkeys, focus-detection all `false`), so a new or partially-implemented adapter never silently claims
support it doesn't have.

#### Scenario: Default capabilities report nothing supported
- **WHEN** `BackendCapabilities::default()` is constructed
- **THEN** every capability flag on it is `false`

#### Scenario: Capability flags are independently settable
- **WHEN** a `BackendCapabilities` is constructed with only `paste_simulation = true`
- **THEN** `paste_simulation` reports `true` while every other flag remains `false`

### Requirement: ClipboardSnapshot models a captured clipboard read
`read_current` SHALL return a `ClipboardSnapshot` that can represent "clipboard empty" distinctly from "one
or more representations present", so callers never confuse an empty clipboard with a read failure.

#### Scenario: Empty clipboard produces an empty snapshot, not an error
- **WHEN** a fake backend's underlying clipboard has no content and `read_current` is called
- **THEN** it returns `Ok` with a snapshot reporting no representations, not an `Err`

#### Scenario: Non-empty clipboard produces a snapshot with representations
- **WHEN** a fake backend's underlying clipboard has plain-text content and `read_current` is called
- **THEN** it returns `Ok` with a snapshot containing one representation matching that content
