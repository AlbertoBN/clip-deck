## ADDED Requirements

### Requirement: GSettings-based hotkey backend registers a GNOME custom keybinding
When `register` is called on the GSettings-based `HotkeyBackend`, it SHALL write a GNOME custom-keybinding
entry (via the `org.gnome.settings-daemon.plugins.media-keys.custom-keybindings` list and its child
schema) with `name`, `command` (the hotkey-trigger CLI's path), and `binding` (translated from the app's
`Ctrl+Shift+V`-style format to GSettings accelerator syntax, e.g. `<Control><Shift>v`) set to the resolved
values, and SHALL report success only if that write succeeds.

#### Scenario: Registration writes name, command, and binding
- **WHEN** `register` is called with a parsed binding and the trigger command's path
- **THEN** the backend writes `name`, `command`, and `binding` to a custom-keybinding entry via its
  `GSettingsRunner`

#### Scenario: Binding is translated to GSettings accelerator syntax
- **WHEN** the parsed binding is `Ctrl+Shift+V`
- **THEN** the written `binding` value is `<Control><Shift>v`

#### Scenario: Registration is idempotent across repeated calls
- **WHEN** `register` is called twice with the same trigger command path
- **THEN** the `custom-keybindings` list contains ClipDeck's entry path only once

### Requirement: GSettings-based backend reports itself as supported
`is_supported()` SHALL return `true` for the GSettings-based backend, since GSettings/DConf is available on
any GNOME session capable of running this backend.

#### Scenario: Capabilities reflect the GSettings backend as supported
- **WHEN** `is_supported()` is queried on the GSettings-based backend
- **THEN** it returns `true`
