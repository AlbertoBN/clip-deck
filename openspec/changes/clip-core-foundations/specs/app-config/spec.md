## ADDED Requirements

### Requirement: Application settings have a typed model with defaults
The system SHALL model user-configurable settings (global hotkey binding, retention window, capture-paused
flag, default paste mode) as a typed `AppSettings` structure, and SHALL provide a documented default value
for every field so a fresh install has sane behavior without requiring the user to configure anything
first.

#### Scenario: Default settings have capture enabled
- **WHEN** `AppSettings::default()` is constructed
- **THEN** its capture-paused flag is `false`

#### Scenario: Default settings have no retention window (keep forever)
- **WHEN** `AppSettings::default()` is constructed
- **THEN** its retention window is `None`

### Requirement: Settings round-trip through the key/value storage shape
Each field of `AppSettings` SHALL be (de)serializable to/from an individual JSON value keyed by a stable
string key, matching the PRD's `settings(key TEXT PRIMARY KEY, value_json TEXT NOT NULL)` table, so
`clip-store` can persist and reload settings one key at a time without a schema migration per field.

#### Scenario: A single setting serializes to a JSON value under its key
- **WHEN** the retention-window field of an `AppSettings` is serialized via its settings key
- **THEN** the result is a JSON value that deserializes back to the original field value

#### Scenario: Missing key falls back to the field's default
- **WHEN** `AppSettings` is loaded from a key/value source that has no entry for the hotkey binding key
- **THEN** the loaded `AppSettings` uses the default hotkey binding

### Requirement: Standard config/data/cache directories are resolved via `directories`
The system SHALL resolve the application's config, data, and cache directories using the `directories`
crate's per-OS conventions (not hardcoded paths), exposing them through a single `AppPaths` accessor used
by every crate that needs to read or write to disk (database file, blob store, logs).

#### Scenario: AppPaths exposes distinct config, data, and cache directories
- **WHEN** `AppPaths::resolve()` is called
- **THEN** it returns non-empty, distinct paths for config, data, and cache directories

#### Scenario: AppPaths is overridable for tests
- **WHEN** `AppPaths::resolve()` is called with an override environment variable set (e.g. to a temp
  directory)
- **THEN** it returns paths rooted under that override rather than the real user directories
