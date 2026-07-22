## Why

`clip-ui-tauri-shell`'s settings screen manages app rules (`SaveRule`/`DeleteRule` are both wired end to
end), but the IPC protocol has no query to list existing rules, so the rule list shown to the user is only
ever session-local React state - rules created in a previous session, or by editing the database directly,
never appear until a listing command exists. This was flagged as a known gap in `clip-ui-tauri-shell`'s
proposal. `clip-store` already has `rules::list_enabled` (used by the ingest pipeline to evaluate active
exclusion rules), but nothing lists *all* rules including disabled ones, which is what a settings screen
needs to show and let a user re-enable.

## What Changes

- Add `rules::list_all` to `clip-store` (crate `clip-store`, module `rules.rs`), returning every rule
  (enabled and disabled) ordered for stable display, mirroring the existing `groups::list_all` pattern.
- Add `Command::ListRules` / handle it in `clipd`'s `CommandHandler` (crate `clipd`, module `commands.rs`),
  returning the full rule list via the new `Store` trait method, mirroring the existing `ListGroups`
  handler exactly.
- Add the `list_rules` Tauri command in `clip-ui-tauri` (crate `clip-ui-tauri`, `src-tauri/src/commands.rs`)
  and call it once on mount in `src/views/settings/Settings.tsx`, replacing the current session-only
  `useState<Rule[]>([])` initial value with a real fetch, while keeping `save_rule`/`delete_rule`'s existing
  optimistic local-list updates for responsiveness within a session.

## Capabilities

### Modified Capabilities
- `ipc-command-handlers` (owned by `clipd-daemon-core`): gains a `ListRules` requirement alongside the
  existing `SaveRule`/`DeleteRule` one.
- `app-rules-management` (owned by `clip-store-persistence`): gains a "listing all rules" requirement
  alongside the existing "listing enabled rules" one.
- `settings-ui` (owned by `clip-ui-tauri-shell`): the rules section's "created/deleted rules render in the
  list" requirement is amended to source its initial list from the daemon rather than starting empty every
  session.

### New Capabilities
(none - this only fills in a query path for an existing feature area)

## Impact

- Affected code: `crates/clip-store/src/rules.rs`, `crates/clip-ipc/src/protocol.rs` (new `Command`
  variant), `crates/clipd/src/commands.rs`, `crates/clip-ui-tauri/src-tauri/src/commands.rs`,
  `crates/clip-ui-tauri/src/views/settings/Settings.tsx`, `crates/clip-ui-tauri/src/state/types.ts` (no
  shape change needed - `Rule` already exists there).
- Depends on: `clip-store-persistence` (owns `rules.rs`), `clip-ipc-transport` (owns `protocol.rs`),
  `clipd-daemon-core` (owns `commands.rs`'s handler), `clip-ui-tauri-shell` (owns the settings screen this
  wires into). All four are already-completed changes; this is a small, additive extension across their
  existing seams rather than a new capability area.
- No migration/schema change: `app_rules` already stores `enabled`; this only adds a query that doesn't
  filter on it.
