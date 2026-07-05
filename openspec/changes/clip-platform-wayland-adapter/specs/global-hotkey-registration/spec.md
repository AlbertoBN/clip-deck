## ADDED Requirements

### Requirement: Hotkey registration degrades gracefully when the compositor offers no global-shortcut mechanism
Registering a hotkey SHALL return a distinct "unsupported on this compositor" result, and `capabilities()`
SHALL report hotkeys as unsupported, rather than hanging, panicking, or silently pretending to succeed, on
a Wayland compositor that provides no mechanism the registration layer can use for global shortcuts.

#### Scenario: Registration on an unsupporting compositor reports unsupported, not success
- **WHEN** hotkey registration is attempted on a Wayland compositor with no usable global-shortcut
  mechanism
- **THEN** registration returns the distinct "unsupported on this compositor" result

#### Scenario: Capabilities reflect the unsupported hotkey mechanism
- **WHEN** `capabilities()` is queried on that same session
- **THEN** it reports hotkey registration as unsupported
