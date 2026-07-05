## Why

`clipd` now exposes a full IPC contract and actually captures/serves clips end-to-end, but there is no way
for a human to use ClipDeck without a UI. Per the PRD's build order, `clip-ui-tauri` is next: a thin Tauri
2 + React/TypeScript shell over `clip-ipc`, giving users the popup-search-and-paste workflow the whole
product exists for.

## What Changes

- Scaffold the parts of `clip-ui-tauri` the workspace intentionally left out (`build.rs`/`tauri.conf.json`
  via `cargo tauri init`, and the `src/` frontend), per `CLAUDE.md`'s note that this should be generated
  by the Tauri CLI rather than hand-authored.
- Implement the popup picker (`src/views/popup`): search-first, keyboard-navigable list, Enter-to-paste.
- Implement the manager window (`src/views/manager`): browse/filter/group/delete/inspect.
- Implement the preview pane/dialog (`src/components` preview): full text/HTML/image preview.
- Implement tray integration (Tauri host `src-tauri`): tray icon, menu (show/hide/pause/clear/settings/
  quit), event handling.
- Implement the settings view (`src/views/settings`): rules, retention, hotkey binding, diagnostics
  display.
- Implement IPC-backed state management (`src/state`): a thin client wrapper around `clip-ipc-transport`'s
  wire client (proxied through Tauri commands) plus event-driven state updates.

## Capabilities

### New Capabilities
- `popup-picker-ui`: Search-first popup list with keyboard navigation and Enter-to-paste, opened via the
  global hotkey.
- `manager-window-ui`: Full manager window for browsing, filtering, grouping, deleting, and inspecting
  clips.
- `preview-pane-ui`: Full preview of long text, HTML, or image content.
- `tray-integration`: Tray icon and menu (show, hide, pause capture, clear history, settings, quit).
- `settings-ui`: Settings screens for rules, retention, hotkey binding, and backend diagnostics display.
- `ui-ipc-state`: Frontend state layer backed by `clip-ipc`'s commands/events (via Tauri commands
  proxying to the `clip-ipc` client), keeping UI state in sync with daemon-published events.

### Modified Capabilities
(none)

## Impact

- Affected code: `crates/clip-ui-tauri/{build.rs,tauri.conf.json,src-tauri/**}` (generated then extended),
  `crates/clip-ui-tauri/src/**` (new React/TS frontend), `crates/clip-ui-tauri/Cargo.toml`,
  `crates/clip-ui-tauri/package.json` (new).
- Depends on: `clip-ipc-transport` (client), `clipd-daemon-core` (a running daemon to connect to),
  `clip-core-foundations` (shared types surfaced in the UI, e.g. `PasteMode`).
- Requires the system dependencies already installed for this workspace (`pkg-config`, `libgtk-3-dev`,
  `libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`) plus Node/npm (already present)
  for the frontend build.
- Completes the PRD's Milestone 1 UI slice (popup + Enter-to-paste) and lays the surfaces that Milestone 2
  (preview) and Milestone 3 (organization/lifecycle UI) extend rather than replace.
