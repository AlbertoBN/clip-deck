## 1. `clip-ipc`: `CreateGroup`/`DeleteGroup` commands (`group-crud-ipc` protocol shape)

- [x] 1.1 Write a failing round-trip serde test: add `Command::CreateGroup { group: Group::new(...) }`
      and `Command::DeleteGroup { id }` to the existing `all_commands()` fixture in
      `crates/clip-ipc/src/protocol.rs`'s test module.
- [x] 1.2 Run `cargo test -p clip-ipc`, confirm it fails to compile (variants don't exist yet).
- [x] 1.3 Add `CreateGroup { group: Group }` and `DeleteGroup { id: String }` to the `Command` enum.
- [x] 1.4 Run `cargo test -p clip-ipc`, confirm the suite is green.

## 2. `clipd`: `Store` trait gains `create_group`/`delete_group`

- [x] 2.1 Write a failing test in `crates/clipd/src/app.rs`'s test module: seed a `FakeStore`, call
      `store.create_group(&group)`, assert `store.list_groups()` includes it.
- [x] 2.2 Run `cargo test -p clipd`, confirm it fails to compile (`create_group` doesn't exist on `Store`
      or `FakeStore`).
- [x] 2.3 Add `create_group(&self, group: &Group) -> Result<(), StoreError>` and
      `delete_group(&self, id: &str) -> Result<(), StoreError>` to the `Store` trait; implement on
      `FakeStore` (insert into / remove from its `groups: Mutex<HashMap<String, Group>>`) and on
      `SqliteStore` (delegating to `clip_store::groups::insert`/`delete`). Also added a temporary stub
      `Command::CreateGroup { .. } | Command::DeleteGroup { .. } => unimplemented!()` arm in
      `commands.rs` since `clip-ipc`'s enum growth in task group 1 already made that `match`
      non-exhaustive - task group 3 replaces it with the real implementation.
- [x] 2.4 Run `cargo test -p clipd`, confirm the suite is green.
- [x] 2.5 Write a failing test: seed `FakeStore` with a group and a clip assigned to it, call
      `store.delete_group(&group.id)`, assert `list_groups()` no longer includes it and the clip is still
      retrievable (not deleted). This exercises `FakeStore`'s own detach behavior, not the real SQLite FK
      cascade (already proven in `clip-store`'s own tests) - `FakeStore`'s `delete_group` must
      explicitly clear `group_id` on any of its in-memory clips referencing the deleted group to model
      this, since a `HashMap`-backed fake has no FK semantics of its own.
- [x] 2.6 The detach logic was written directly into `FakeStore::delete_group` as part of task 2.3 (not a
      separate cycle), so this test passed immediately rather than failing first - noting the deviation
      from strict red-first ordering rather than silently glossing over it. `cargo test -p clipd` confirms
      green (50 passed).

## 3. `clipd`: `CommandHandler` handles `CreateGroup`/`DeleteGroup` (`group-crud-ipc`)

- [x] 3.1 Write a failing test in `crates/clipd/src/commands.rs`'s test module:
      `handler.handle(Command::CreateGroup { group })` with a valid group, then
      `handler.handle(Command::ListGroups)` and assert the created group is present.
- [x] 3.2 Confirmed red via the task-2.3 `unimplemented!()` stub panicking (not a compile failure, since
      that stub already made the match exhaustive) - same underlying "missing behavior" signal.
- [x] 3.3 Add the `CreateGroup` match arm: reconstruct via
      `Group::new(group.id, group.name, group.parent_group_id).map_err(|e| e.to_string())?`, then
      `self.store.create_group(&group).map_err(|e| e.to_string())?`.
- [x] 3.4 Run `cargo test -p clipd`, confirm the new test passes.
- [x] 3.5 Write a failing test: `handle(Command::CreateGroup { group })` where `group.parent_group_id ==
      Some(group.id.clone())` (self-referential) - assert the call returns an error and a subsequent
      `ListGroups` does not include it.
- [x] 3.6 Confirmed red via the same `unimplemented!()` stub, then green once 3.3 landed (passed on the
      first real run, as a direct consequence of `Group::new`'s existing self-parent check).
- [x] 3.7 Write a failing test for `DeleteGroup`: create a group, delete it via
      `handle(Command::DeleteGroup { id })`, assert a subsequent `ListGroups` no longer includes it.
- [x] 3.8 Confirmed red via the stub, then added the `DeleteGroup` match arm
      (`self.store.delete_group(&id).map_err(|e| e.to_string())?`).
- [x] 3.9 Run `cargo test -p clipd`, confirm the full suite is green (53 passed).
- [x] 3.10 Run `cargo clippy -p clipd --all-targets -- -D warnings` - clean.

## 4. `clip-ui-tauri` src-tauri: `create_group`/`delete_group` Tauri commands

- [x] 4.1 Write a failing test in `crates/clip-ui-tauri/src-tauri/src/commands.rs`'s test module:
      `create_group_with(&fake_client, group).await`, assert `fake_client.calls()` recorded
      `Command::CreateGroup { group }` (mirrors the existing `save_rule_with` test).
- [x] 4.2 Run `cargo test -p clip-ui-tauri --no-default-features`, confirm it fails to compile
      (`create_group_with` doesn't exist).
- [x] 4.3 Add `create_group_with`/`delete_group_with` (mirroring `save_rule_with`/`delete_rule_with`
      exactly) and the `#[tauri::command] create_group`/`delete_group` wrappers.
- [x] 4.4 Run `cargo test -p clip-ui-tauri --no-default-features`, confirm the suite is green.
- [x] 4.5 Write a failing test for `delete_group_with` mirroring 4.1, confirm red then green the same way.
- [x] 4.6 Register `commands::create_group`/`commands::delete_group` in `lib.rs`'s `generate_handler!`
      list.
- [x] 4.7 Run `cargo test -p clip-ui-tauri --no-default-features`, `cargo clippy -p clip-ui-tauri
      --no-default-features --all-targets -- -D warnings`, and `cargo check --workspace`; confirm all
      clean.

## 5. `clip-ui-tauri` frontend: Groups section in Settings (`group-management-ui`)

- [x] 5.1 Write a failing test in `Settings.test.tsx`: "shows a group returned by ListGroups on initial
      mount" (mirrors the existing equivalent Rules test). (Written together with 5.5/5.9 in one batch,
      not three separate cycles - noting the deviation from the strict per-behavior ordering.)
- [x] 5.2 Run `npx vitest run src/views/settings/Settings.test.tsx`, confirm it fails for the expected
      reason (no groups state/rendering yet) - confirmed together with 5.6/5.10 (3 failed, 8 passed).
- [x] 5.3 Add a "Groups" section to `Settings.tsx`: `groups: Group[]` state, loaded via
      `callCommand<Group[]>('list_groups')` in the mount effect, rendered as a `<ul>` of `<li>` rows
      showing each group's `name`.
- [x] 5.4 Run the test, confirm green.
- [x] 5.5 Write a failing test: "creating a group issues CreateGroup with the entered name" (mirrors the
      existing Rules creation test).
- [x] 5.6 Confirmed red together with 5.2/5.10 above.
- [x] 5.7 Add a labeled "New group name" input + "Add group" button and `handleCreateGroup` (generates an
      id the same way `newRuleId()` does, builds `{ id, name, parent_group_id: null, sort_order: 0 }`,
      calls `create_group`, optimistically appends to local state, clears the input).
- [x] 5.8 Run the test, confirm green.
- [x] 5.9 Write a failing test: "deleting a group issues DeleteGroup and removes it from the list" (mirrors
      the existing Rules deletion test).
- [x] 5.10 Confirmed red together with 5.2/5.6 above.
- [x] 5.11 Add a "Delete group {id}"-labeled button per row calling `handleDeleteGroup` (calls
      `delete_group`, filters it out of local state).
- [x] 5.12 Run the test, confirm green - all 11 Settings tests pass (implemented 5.3/5.7/5.11 together in
      one pass, confirmed green together rather than three separate green checkpoints).
- [x] 5.13 Run the full frontend suite (`npm test -- --run`), `npx tsc -b`, and `npx oxlint`; confirm all
      clean - 44 tests passed, tsc clean, oxlint clean.

## 6. Final verification

- [x] 6.1 Run `cargo test --workspace` - all green (`clip-core` 44, `clip-ipc` 19, `clip-platform` 58,
      `clipd` 53, `clip-ui-tauri` 15 Rust + 44 frontend).
- [x] 6.2 Run `cargo clippy -p clip-ipc -p clipd --all-targets -- -D warnings` and
      `cargo clippy -p clip-ui-tauri --no-default-features --all-targets -- -D warnings` - both clean.
- [x] 6.3 Run `cargo check --workspace` - clean.
- [x] 6.4 Manual smoke test: rebuilt and restarted a single clean `clipd` + Tauri UI, then exercised the
      full flow directly against the real running daemon's socket (CreateGroup → AssignGroup on a real
      existing clip → confirmed via GetClip → DeleteGroup → confirmed ListGroups no longer includes it →
      confirmed via GetClip the clip still exists with `group_id: null`). Full command-level verification
      done this way rather than clicking through the GUI, since this environment has no input-injection
      tool - the Settings/Manager UI wiring itself is already covered by the frontend test suite (task 5).
