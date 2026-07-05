## ADDED Requirements

### Requirement: Tray menu exposes show, hide, pause/resume, clear history, settings, and quit
The tray menu SHALL expose actions for showing the popup/manager, hiding open windows, pausing or
resuming capture, clearing history, opening settings, and quitting the application, matching the PRD's
tray-menu requirement.

#### Scenario: Selecting "Pause capture" issues PauseCapture
- **WHEN** the user selects "Pause capture" from the tray menu
- **THEN** a `PauseCapture { paused: true }` command is issued

#### Scenario: Selecting "Quit" exits the application
- **WHEN** the user selects "Quit" from the tray menu
- **THEN** the UI application process exits

### Requirement: Tray reflects the current pause state
The tray icon or its tooltip/menu label SHALL reflect whether capture is currently paused or active,
updating when a `CapturePaused` event is received, so the user does not have to open the app to know
capture's state.

#### Scenario: Receiving CapturePaused updates the tray's displayed state
- **WHEN** a `CapturePaused { paused: true }` event is received
- **THEN** the tray's displayed state (icon/tooltip/menu label) reflects that capture is paused

#### Scenario: Tray menu label toggles between "Pause capture" and "Resume capture"
- **WHEN** capture is currently paused
- **THEN** the tray menu shows a "Resume capture" action instead of "Pause capture"
