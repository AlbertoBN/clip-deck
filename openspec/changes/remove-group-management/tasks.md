## 1. `clip-ui-tauri` frontend: remove group UI and state

- [x] 1.1 In `crates/clip-ui-tauri/src/views/manager/Manager.tsx`, delete the group filter `<select>`,
      the `.group-picker`/`.group-picker-options` JSX, `handleAssignGroup`, `assignGroupError` state,
      and the `groups`/`setClipGroup`/`loadGroups` store selectors (keep pin/delete/bulk-clear/search
      intact).
- [x] 1.2 In `crates/clip-ui-tauri/src/state/store.ts`, delete `groups`, `loadGroups`, `setClipGroup`,
      and the `GroupsChanged` case in `subscribeToEvents`'s switch.
- [x] 1.3 In `crates/clip-ui-tauri/src/state/types.ts`, delete the `Group` interface and the
      `GroupsChanged` member of `DaemonEvent`. Also removed `group_id` from `Clip`/`SearchFilters` -
      not called out explicitly in this task's original wording, but required since those fields
      mirror the backend shape that no longer includes it.
- [x] 1.4 In `crates/clip-ui-tauri/src/views/settings/Settings.tsx`, delete the entire "Groups"
      section (state, `newGroupId`, `handleCreateGroup`, `handleDeleteGroup`, JSX) (keep Hotkey/
      Diagnostics/Rules sections intact).
- [x] 1.5 In `crates/clip-ui-tauri/src/index.css`, delete the `.group-picker`/`.group-picker-options`
      rules.
- [x] 1.6 Run `npm test -- --run` and `npx tsc -b`. Confirm the failures are exactly the tests in
      `Manager.test.tsx`/`Settings.test.tsx`/`store.test.ts` that reference the now-deleted group
      UI/state/types (failing for the expected reason: missing elements/undefined exports), not
      unrelated breakage. 10 failures, all group-related, except one collateral hit
      ("clicking Settings calls show_settings_window" waited on the now-removed Group filter's
      label) - fixed in 1.7 rather than deleted, since that test isn't about groups.
- [x] 1.7 Delete those now-dead tests: the group-filter and per-clip group-picker tests in
      `Manager.test.tsx`; the three group tests and the `group()` helper in `Settings.test.tsx`; the
      `setClipGroup`/`GroupsChanged` tests in `store.test.ts`. Remove the now-unused `Group` imports
      and `list_groups` mock branches from all three test files. Also fixed the collateral
      "clicking Settings" test's wait condition, and removed a `group_id: null` fixture field found
      in `Popup.test.tsx` during 1.8 (not anticipated by this task's original file list).
- [x] 1.8 Run `npm test -- --run`, `npx tsc -b`, `npx oxlint`. Confirm all clean. 40/40 tests pass,
      tsc clean, oxlint clean.

## 2. `clip-ui-tauri` Rust (`src-tauri`): remove Tauri command wrappers and diagnostic logging

- [x] 2.1 In `crates/clip-ui-tauri/src-tauri/src/commands.rs`, delete `list_groups_with`,
      `create_group_with`, `delete_group_with`, `assign_group_with`, and their `#[tauri::command]`
      wrappers (`list_groups`, `create_group`, `delete_group`, `assign_group`), plus their tests.
- [x] 2.2 In `crates/clip-ui-tauri/src-tauri/src/lib.rs`, remove `commands::list_groups`,
      `commands::create_group`, `commands::delete_group`, `commands::assign_group` from the
      `generate_handler!` list.
- [x] 2.3 In `crates/clip-ui-tauri/src-tauri/src/client.rs`, revert the diagnostic
      `eprintln!("[clip-ui-tauri] -> {command:?}")` / `eprintln!("[clip-ui-tauri] <- {response:?}")`
      added to the `Client` impl for `IpcClient` back to a plain pass-through `call`, now that the
      bug it was added to diagnose is being removed rather than fixed.
- [x] 2.4 Run `cargo test -p clip-ui-tauri --no-default-features`. Deviation from the anticipated
      red step: since 2.1's code and tests were deleted together in one pass rather than
      sequentially, the crate compiled clean on the first run (13/13 passed) rather than failing
      first - noting this rather than silently claiming a red step that didn't occur.
- [x] 2.5 Run `cargo clippy -p clip-ui-tauri --no-default-features --all-targets -- -D warnings`.
      Confirm clean.

## 3. `clipd`: remove Store methods, CommandHandler arms, and diagnostic logging

- [x] 3.1 In `crates/clipd/src/app.rs`, delete `list_groups`/`set_group`/`create_group`/
      `delete_group` from the `Store` trait, their `SqliteStore` implementations, their `FakeStore`
      implementations (including the `groups: Mutex<HashMap<String, Group>>` field and its
      `seed_group()` helper), and every test exercising them.
- [x] 3.2 In `crates/clipd/src/commands.rs`, delete the `Command::ListGroups`/`CreateGroup`/
      `DeleteGroup`/`AssignGroup` match arms, the `use clip_core::models::Group;` import, and every
      test exercising them (create/delete group persistence, self-referential rejection,
      `GroupsChanged` publication, `AssignGroup` publishing `ClipUpdated`).
- [x] 3.3 In `crates/clipd/src/commands.rs`, revert the diagnostic `handle`/`handle_inner` split
      (added to log every command in/out while debugging group assignment) back to a single plain
      `handle` function, now that its purpose is moot.
- [x] 3.4 Run `cargo test -p clipd`. Confirmed red for the expected reason first: non-exhaustive
      match citing `AssignGroup`/`ListGroups`/`CreateGroup`/`DeleteGroup` not covered, since
      `clip-ipc` still had those variants at that point - did task group 4 next, then confirmed
      green (46/46).
- [x] 3.5 Run `cargo clippy -p clipd --all-targets -- -D warnings`. Confirm clean. Note: `app.rs`'s
      `FakeStore::search` still filters on `filters.group_id`/`c.group_id` - left in place since
      those fields belong to `clip-core`'s `SearchFilters`/`Clip`, not yet removed until task group
      6; still compiles fine at this point.

## 4. `clip-ipc`: remove protocol variants

- [x] 4.1 In `crates/clip-ipc/src/protocol.rs`, delete `Command::ListGroups`, `Command::CreateGroup`,
      `Command::DeleteGroup`, `Command::AssignGroup`, and `Event::GroupsChanged`, plus their entries
      in the `all_commands()`/`all_events()` test fixtures and the `use clip_core::models::Group;`
      import. Also fixed three collateral test references that used `Command::ListGroups` merely as
      an arbitrary second/rejected command (not testing groups themselves): one each in
      `protocol.rs`, `client.rs`, and `server.rs` - swapped to `Command::ListRules`.
- [x] 4.2 Run `cargo test -p clip-ipc`. Confirm the round-trip tests still pass for every remaining
      variant. 19/19 passed.
- [x] 4.3 Run `cargo clippy -p clip-ipc --all-targets -- -D warnings`. Confirm clean.

## 5. `clip-store`: remove the groups table, column, and queries

- [x] 5.1 Delete `crates/clip-store/src/groups.rs` entirely and remove `pub mod groups;` from
      `crates/clip-store/src/lib.rs`.
- [x] 5.2 In `crates/clip-store/src/clips.rs`, delete `set_group`, remove `group_id` from every
      insert/query function and struct mapping, and delete every test exercising group assignment
      (including any fixture clip builder's `group_id` field/override). Also removed the group
      filter from `fts.rs`'s `matches_filters` and its dedicated test, and fixed `db.rs`'s
      `fresh_database_gets_the_full_schema` (dropped `"groups"` from the expected table list) and
      `new_connection_enforces_foreign_keys` (rewrote to exercise `clip_representations.clip_id`'s FK
      instead of the removed `clips.group_id` one) - none of these were named in this task's original
      wording but were direct fallout of removing the `groups` table.
- [x] 5.3 In `migrations/001_init.sql`, remove the `groups` table definition, the
      `group_id`/`parent_group_id` columns and their `REFERENCES groups(id)` foreign keys from
      `clips`, and the `idx_clips_group_id` index. Per design.md Decision 2, do not attempt a
      version-gated `DROP TABLE`/`DROP COLUMN` migration - this is dev-stage software re-applying one
      idempotent init script.
- [x] 5.4 Run `cargo test -p clip-store`. Confirmed red for the expected reason first
      (`E0063: missing field group_id in initializer of Clip`, since `clip-core`'s `Clip` still had
      `group_id` at that point but `clips.rs::row_to_clip` no longer set it) - this forced doing task
      group 6 (`clip-core`) immediately, then confirmed green (44/44).
- [x] 5.5 Run `cargo clippy -p clip-store --all-targets -- -D warnings`. Confirm clean.

## 6. `clip-core`: remove the `Group` model and its error variant

- [x] 6.1 In `crates/clip-core/src/models.rs`, delete the `Group` struct, its `Group::new`
      constructor, `group_id` from `Clip` and `Clip::new`, and every test exercising them. Also
      removed `group_id` from `clip-core/src/search.rs`'s `SearchFilters` and adjusted its
      partial-deserialization test - not named in this task's original wording, but the same field
      duplicated onto `SearchFilters`, and leaving it would have been a dangling reference matching
      nothing on the wire.
- [x] 6.2 In `crates/clip-core/src/errors.rs`, delete `CoreError::InvalidGroupParent` and its test.
- [x] 6.3 Run `cargo test -p clip-core`. Confirm clean (41/41) - done together with task group 5 per
      the compiler-forced ordering noted above, rather than 5 waiting on 6.
- [x] 6.4 Run `cargo clippy -p clip-core --all-targets -- -D warnings`. Confirm clean.
- [x] 6.5 (Not in the original plan.) Removing `clip-core`'s fields broke `clipd/src/app.rs`'s
      `FakeStore::search`, which still filtered on `filters.group_id`/`c.group_id` per task 3.5's
      note that it was deliberately left in place until this point. Fixed by deleting that filter
      line; reran `cargo test -p clipd` (46/46) and `cargo test --workspace` (all green, 221 tests)
      to confirm no other crate was affected.

## 7. Final verification

- [x] 7.1 Run `cargo test --workspace`. All green, 221 tests total (clip-core 41, clip-ipc 19,
      clip-platform 58 [4 pre-existing ignored], clip-store 44, clip-ui-tauri 13, clipd 46); no
      group-related test remains anywhere.
- [x] 7.2 Run `cargo check --workspace`. Confirm clean.
- [x] 7.3 Run `cargo clippy --all-targets -- -D warnings` per crate (`clip-core`, `clip-ipc`,
      `clip-store`, `clip-platform`, `clipd` together; `clip-ui-tauri` separately with
      `--no-default-features`, matching this repo's existing convention). Confirm clean.
- [x] 7.4 Run `npm test -- --run`, `npx tsc -b`, `npx oxlint` in `crates/clip-ui-tauri`. 40/40 tests
      pass, tsc clean, oxlint clean.
- [x] 7.5 Grepped the whole repo for `group` (case-insensitive). Found and fixed two real leftovers
      not caught by earlier steps: `clip-core/src/models.rs`'s module doc comment still listed
      `Group`; two stray `if (command === 'list_groups') return []` mock branches remained in
      `Settings.test.tsx` (a `beforeEach` and one test's override) that an earlier `replace_all` had
      missed. Everything else matching is either unrelated (`auth.rs`'s Unix permission-group tests,
      Tauri's own auto-generated capability-*group* schema docs under `gen/schemas/`) or
      intentionally untouched history (`openspec/changes/*`). **Found but deliberately not edited**,
      per this task's own instruction: `docs/ClipDeck-ubuntu-clipboard-manager-prd.md` still
      describes groups extensively (v1 scope, DB schema, IPC commands, module list) - now
      out of sync with the shipped code; flagged for the user in the completion report rather than
      silently rewritten.
- [x] 7.6 Rebuilt and restarted a single clean `clipd` + `cargo tauri dev` (verified via `ps aux`
      that exactly one of each was running). Confirmed via a direct socket round-trip that
      `GetSettings` still works and a raw `ListGroups` request is now unrecognized (empty response,
      connection dropped) rather than served. UI log showed a normal startup (only the pre-existing
      benign `libayatana-appindicator` deprecation warning), no panics or errors.
