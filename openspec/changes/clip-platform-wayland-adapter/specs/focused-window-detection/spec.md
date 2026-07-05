## ADDED Requirements

### Requirement: Focus detection reports unsupported on compositors that restrict focus information
`focused_app` SHALL report unsupported via `capabilities()` and return `None` consistently, rather than
guessing at a focused window or crashing, on a Wayland compositor that does not expose focused-window
information to clients (by design, per Wayland's security model).

#### Scenario: Capabilities reflect unsupported focus detection
- **WHEN** `capabilities()` is queried on a compositor that exposes no focused-window information
- **THEN** it reports focus-detection as unsupported

#### Scenario: focused_app returns None rather than guessing
- **WHEN** `focused_app` is called on a compositor that exposes no focused-window information
- **THEN** it returns `None` rather than a fabricated or stale `AppContext`
