## Context

Group management was built incrementally across four prior changes (`clip-core-foundations`,
`clip-store-persistence`, `clipd-daemon-core`/`clip-ipc-transport`, `clip-ui-tauri-shell`, and most
recently `group-crud-settings`), ending with a per-clip group-assignment control in the Manager
window. That control was implemented twice this session - first as a native HTML `<select>`, then
(after live debugging showed the `<select>`'s `change` event reported an empty value even when a
real option was visibly picked, confirmed via request/response logging added directly to the
daemon's `CommandHandler` and the Tauri client bridge) as a custom `<details>`/button popover. The
second implementation still produced no visible, correct effect when tested live. Both root causes
were narrowed down using real IPC-level evidence, not guesswork, but a third UI approach is not
worth the cost: the user has decided to remove group management rather than continue.

This project has no shipped release and no external users - `clip-store`'s "migration" is a single
idempotent `CREATE TABLE IF NOT EXISTS` script (`run_migrations` just re-executes
`migrations/001_init.sql` on every startup; see `crates/clip-store/src/db.rs`), not a real
versioned-migration engine. That materially simplifies this removal: there is no production data to
preserve or migrate away from, only the developer's own local test database.

## Goals / Non-Goals

**Goals:**
- Remove every group-related type, table, IPC command/event, store method, Tauri command, and UI
  element, leaving no dead code or dangling references.
- Remove the temporary diagnostic logging added this session specifically to debug group assignment
  (`eprintln!` in `clip-ui-tauri`'s `IpcClient` `Client` impl; the `tracing::info!`/`error!`
  before/after wrapper around `clipd`'s `CommandHandler::handle`), since its only purpose was
  debugging the feature being removed.
- Leave every other capability (capture, search, pin, delete, rules, settings, hotkeys, bulk clear)
  fully intact and passing.

**Non-Goals:**
- Preserving existing local group data. Pre-release, single-developer software; no migration
  compatibility guarantee is being made.
- Building any replacement organizational feature (tags, favorites-only-groups, etc.) - out of
  scope, not requested.
- Retrying the group-assignment UI with a third implementation approach.

## Decisions

### Decision 1: Remove in the reverse of the original build order
The PRD's build order for *adding* features is `clip-core -> clip-store -> clip-ipc -> clipd ->
clip-ui-tauri` (each layer depends on the one before it existing). Removal must go the other
direction: **`clip-ui-tauri` (frontend, then Rust) -> `clipd` -> `clip-ipc` -> `clip-store` ->
`clip-core`**. Removing `clip-core`'s `Group` type first would break every downstream crate at once
and make it impossible to verify each crate's suite independently as the change progresses.
Removing leaf-first means each crate's `cargo test`/`cargo check` stays meaningful as a checkpoint:
after the frontend and `clip-ui-tauri` Rust side are done, `cargo check --workspace` will still show
`clipd` still compiling against the old protocol/store surface (expected, not yet touched), and so
on inward.

**Alternatives considered**: Remove top-down (`clip-core` first). Rejected - would immediately break
`cargo check --workspace` across every crate simultaneously, losing the ability to gate each step's
correctness independently, which conflicts with this repo's mandatory per-step TDD/verification
workflow.

### Decision 2: Edit `migrations/001_init.sql` directly rather than adding a new migration file
Since `run_migrations` re-applies the single init script idempotently and there is no real
migration-numbering scheme yet (`002_seed_defaults.sql` is the only other file, for seed data, not
schema evolution), the `groups` table, `clips.group_id` column, and `idx_clips_group_id` index are
removed directly from `001_init.sql`'s `CREATE TABLE`/`CREATE INDEX` statements. A brand-new
database (fresh `cargo test`, or a fresh local install) never creates them.

For an *already-migrated* local database (e.g. this session's `~/.config/clipdeck/clipd.sock`
companion `.db` file), `CREATE TABLE IF NOT EXISTS` is a no-op, so the old `groups` table and
`clips.group_id` column remain physically present but inert - nothing in the codebase reads or
writes them anymore. This is acceptable per the Non-Goals above; the task list includes a manual
verification step against the developer's real local database at the end of this change, not a
migration script.

**Alternatives considered**: A real `ALTER TABLE clips DROP COLUMN group_id` + `DROP TABLE groups`
migration, version-gated so it only runs once. Rejected as disproportionate engineering effort for
software with zero production installs - the exact kind of complexity this removal is trying to
shed, not add back in a different form.

### Decision 3: Delete now-dead tests rather than skip or comment them out
Every test that exercises removed behavior (group CRUD, `AssignGroup`/`ListGroups` round-trips,
group filter/picker UI, the `FakeStore`'s group bookkeeping) is deleted outright as part of removing
the code it tests. No `#[ignore]`, no commented-out blocks - CLAUDE.md's guidance against
half-finished states and backwards-compatibility shims applies equally to test debt.

## Risks / Trade-offs

- **[Risk]** An already-migrated local dev database retains an orphaned `groups` table/column
  (Decision 2) → **Mitigation**: harmless (nothing references it), and explicitly called out as a
  Non-Goal; a developer who wants a fully clean schema can delete their local `.db` file.
- **[Risk]** Deleting tests instead of updating them could silently drop coverage for something
  unrelated that happened to share a test file with group logic → **Mitigation**: task list requires
  reading each touched test file before deleting, removing only the group-specific test functions/
  assertions, not whole files, unless the whole file is group-only (e.g. `clip-store/src/groups.rs`).
- **[Risk]** Removing `Clip.group_id` changes the `Clip` struct's serialized shape, which any
  external consumer (none currently exist beyond this workspace) would need to handle →
  **Mitigation**: no external consumers exist; this is an internal wire format between `clipd` and
  `clip-ui-tauri`, both updated together in this same change.

## Test strategy per component

- **`clip-core`**: delete `Group`/`CoreError::InvalidGroupParent` and their unit tests; remove
  `group_id` from `Clip` and its constructor/tests. Red signal: `cargo test -p clip-core` fails to
  compile wherever a deleted test still references `Group`/`InvalidGroupParent`/`group_id` - delete
  those references, confirm green.
- **`clip-store`**: delete `src/groups.rs` (and its `mod groups;` declaration) entirely, remove
  `group_id` handling from `clips.rs`'s insert/query functions and their tests, edit
  `migrations/001_init.sql` per Decision 2. Red signal: `cargo test -p clip-store` fails to compile
  on the now-missing `groups` module and `group_id` field/column; fix call sites, confirm green.
- **`clip-ipc`**: remove `Command::ListGroups`/`CreateGroup`/`DeleteGroup`/`AssignGroup` and
  `Event::GroupsChanged` from `protocol.rs`, plus their entries in the `all_commands()`/
  `all_events()` round-trip test fixtures. Red signal: `cargo test -p clip-ipc` fails to compile on
  the removed variants; confirm green once the fixtures no longer reference them.
- **`clipd`**: remove the four `Store` trait methods, their `SqliteStore`/`FakeStore` impls, and the
  matching `CommandHandler::handle` arms, plus every test exercising them (both `app.rs` and
  `commands.rs`); revert the diagnostic `tracing::info!`/`error!` wrapper added around `handle` back
  to a plain single-function `handle`. Red signal: `cargo test -p clipd` fails to compile once
  `clip-ipc`'s variants are gone (non-exhaustive match) until the arms are deleted; confirm green,
  then `cargo clippy -p clipd --all-targets -- -D warnings`.
- **`clip-ui-tauri` (Rust)**: remove the four `_with` functions/`#[tauri::command]` wrappers and
  their `generate_handler!` registrations in `lib.rs`, plus their tests in `commands.rs`; revert the
  diagnostic `eprintln!` added to the `Client` impl for `IpcClient` in `client.rs`. Red signal:
  `cargo test -p clip-ui-tauri --no-default-features` fails to compile against the removed
  `clip-ipc`/`clip-core` symbols; confirm green, then clippy.
- **`clip-ui-tauri` (frontend)**: remove `Group`/`GroupsChanged` from `state/types.ts`; remove
  `groups`/`loadGroups`/`setClipGroup` from `state/store.ts` and its tests; remove Manager's group
  filter dropdown and per-clip group picker (JSX, `handleAssignGroup`, `assignGroupError`, CSS) and
  their tests; remove Settings' "Groups" section (state, handlers, JSX) and its tests. Red signal:
  `npx tsc -b` fails on removed type references; `npm test -- --run` fails on tests still exercising
  removed UI. Delete those tests, confirm green, then `npx oxlint`.
- **Final gate**: `cargo test --workspace`, `cargo clippy` per touched crate, `cargo check
  --workspace`, `npx tsc -b`, `npx oxlint`, `npm test -- --run`, all clean - matching this repo's
  mandatory pre-completion checklist.
