## Context

Three already-completed changes each stopped one layer short of a full rule-listing path:
`clip-store-persistence` implemented `rules::list_enabled` (ingest-pipeline-facing, filters to
`enabled = true`) but no unfiltered listing; `clip-ipc-transport` defined `SaveRule`/`DeleteRule` but no
list query in `Command`; `clip-ui-tauri-shell` built a full rules UI (`Settings.tsx`) against that gap by
keeping the list in local `useState`, seeded empty and only ever mutated by the session's own
`save_rule`/`delete_rule` calls. This change closes the gap end-to-end by mirroring the exact pattern
`ListGroups` already established across the same three crates (`groups::list_all` -> `Command::ListGroups`
-> `Store::list_groups` -> `list_groups` Tauri command).

## Goals / Non-Goals

**Goals:**
- `clip-store::rules::list_all` returns every rule, enabled or not, so a settings screen can display and
  let a user toggle previously-disabled rules.
- `clipd` exposes this via a new `ListRules` command, handled identically in shape to `ListGroups`.
- `clip-ui-tauri`'s settings screen fetches the real list on mount instead of starting from an empty
  session-local array, while still applying `SaveRule`/`DeleteRule` results to that same list locally for
  immediate feedback (no extra round-trip re-fetch after a save/delete).

**Non-Goals:**
- No new rule fields, no rule-editing (only create/delete already exist), no pagination - `app_rules` is
  expected to stay small (tens, not thousands, of rules) per the PRD, so a flat unfiltered list is
  sufficient, matching `ListGroups`'s existing precedent.
- No change to `rules::list_enabled` or the ingest pipeline's use of it (`clipd-daemon-core`'s `ingest.rs`
  is untouched).

## Decisions

- **Mirror `ListGroups` exactly** rather than inventing a different shape: `Command::ListRules` (no
  payload, like `ListGroups`), handler returns `serde_json::to_value(rules)`, Tauri command
  `list_rules(client) -> Result<Vec<Rule>, String>`. Consistency with the existing precedent minimizes
  review surface and avoids a one-off protocol shape.
- **`list_all` orders by `app_match` then `id`** for stable, deterministic test assertions and a
  predictable UI order (no PRD-specified ordering exists for rules, unlike clips' recency/pin ordering).
- **Settings.tsx fetches on mount only** (not on an event subscription) - there is no `RuleUpdated`/
  `RuleDeleted` broadcast event in the protocol (out of scope to add one here), so the list only refreshes
  via the local optimistic updates already used by `save_rule`/`delete_rule`, same as before this change.

### Test strategy

- **`clip-store`**: red - failing test in `rules.rs`'s test module inserting one enabled and one disabled
  rule and asserting `list_all` returns both (`list_enabled` already has a parallel test asserting it
  returns only the enabled one - this new test lives alongside it). Green - implement `list_all` per the
  `groups::list_all` pattern. Confirm `cargo test -p clip-store` green.
- **`clip-ipc`**: red - failing round-trip serde test for `Command::ListRules` (mirrors existing
  `Command::ListGroups` test). Green - add the enum variant. Confirm `cargo test -p clip-ipc` green.
- **`clipd`**: red - failing test in `commands.rs` calling `CommandHandler::handle(Command::ListRules)`
  against a `FakeStore` seeded with one enabled and one disabled rule, asserting both come back. Green -
  add the `Store` trait method and the match arm (mirrors `ListGroups`'s arm). Confirm `cargo test -p clipd`
  green.
- **`clip-ui-tauri` (Rust)**: red - failing test for a `list_rules_with(client)` core function against a
  `FakeClient` returning canned rules. Green - implement it plus the thin `#[tauri::command]` wrapper.
  Confirm `cargo test -p clip-ui-tauri` green.
- **`clip-ui-tauri` (frontend)**: red - failing Vitest test in `Settings.test.tsx` asserting a
  daemon-returned rule (via a mocked `list_rules` invoke) renders on initial mount, before any
  `save_rule`/`delete_rule` call. Green - fetch `list_rules` in `Settings.tsx`'s mount effect and seed the
  local list from it. Confirm `npm test` green alongside the existing create/delete tests.
