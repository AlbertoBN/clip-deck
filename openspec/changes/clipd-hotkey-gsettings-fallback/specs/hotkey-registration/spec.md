## MODIFIED Requirements

### Requirement: Daemon registers the persisted hotkey binding at startup
The daemon SHALL register a global hotkey, parsed from the persisted `hotkey_binding` setting, with a
`clip-platform` `HotkeyBackend` during startup, before it begins serving IPC commands. Which backend is
used SHALL depend on session type: a Wayland session (per `is_wayland_session`) SHALL use the
GSettings-based backend; a native X11 session (no `WAYLAND_DISPLAY`) SHALL use `GlobalHotkeyBackend`,
regardless of whether an X11/XWayland connection happens to be reachable.

#### Scenario: Startup registers the persisted binding
- **WHEN** the daemon starts with `hotkey_binding: "Ctrl+Shift+V"` persisted in settings
- **THEN** a hotkey matching that binding is registered with the session-appropriate hotkey backend

#### Scenario: A Wayland session registers via the GSettings backend even when XWayland is reachable
- **WHEN** the daemon starts on a Wayland session (`WAYLAND_DISPLAY` set) where an X11/XWayland connection
  is also reachable
- **THEN** the GSettings-based hotkey backend is used to register the binding, not `GlobalHotkeyBackend`

### Requirement: Triggering the registered hotkey publishes HotkeyPressed
The daemon SHALL publish an `Event::HotkeyPressed` via the existing event publisher whenever the
registered hotkey fires, whether detected via an in-process `HotkeyBackend` callback (native X11) or via
an external trigger command reaching the daemon over IPC (GSettings-based, Wayland).

#### Scenario: Hotkey trigger publishes an event (native X11)
- **WHEN** the registered `GlobalHotkeyBackend`'s callback is invoked
- **THEN** a `HotkeyPressed` event is published

#### Scenario: Hotkey trigger publishes an event (GSettings/Wayland)
- **WHEN** the daemon receives a `Command::TriggerHotkey`
- **THEN** a `HotkeyPressed` event is published
