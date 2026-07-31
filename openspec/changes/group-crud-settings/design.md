## Context

`crates/clip-store/src/groups.rs` already has everything the store layer needs: `insert(conn, group)`,
`get`, `rename`, `list_all`, `list_children`, `delete(conn, id)`. The DB schema
(`migrations/001_init.sql`) already gives `delete` its cascade/detach behavior for free via
`groups.parent_group_id ... ON DELETE CASCADE` and `clips.group_id ... ON DELETE SET NULL` - confirmed by
the already-passing `deleting_a_parent_group_deletes_its_child_group` and
`deleting_a_group_detaches_its_clips_instead_of_deleting_them` tests. Nothing in this change touches
`clip-store` or `clip-core` - it's purely wiring the existing store-layer primitives through
`clip-ipc` → `clipd` → `clip-ui-tauri`, following the exact shape already established for rules
(`Command::SaveRule`/`DeleteRule`, `Store::save_rule`/`delete_rule`, Settings' Rules section).

## Goals / Non-Goals

**Goals:**
- Let a user create a (flat, top-level) group and delete any group from Settings.
- Re-validate group data server-side on create (mirroring `UpdateSettings`'s `parse_binding` re-validation
  pattern), not just trust whatever the frontend constructed.

**Non-Goals:**
- Group rename, reparenting, or reordering UI - the `Group` model's `parent_group_id`/`sort_order` fields
  exist and are exercised by `clip-store`'s own tests, but no UI surface for them is being added here.
  Groups created through this change always have `parent_group_id: null`.
- Any change to cascade/detach semantics - already fully correct at the schema level, not touched.
- Cross-view live refresh (Settings' group list updating Manager's already-mounted dropdowns) - out of
  scope, same as the existing Rules section's behavior.

## Decisions

### 1. `Command::CreateGroup { group: Group }`, not an upsert-style `SaveGroup`
Unlike rules (where `SaveRule` is reused for both create and toggling `enabled`), groups have no
in-place-update UI in this change, so there's no need for upsert semantics. `CreateGroup` is create-only;
a duplicate `id` surfaces as a raw store error (`StoreError::Sqlite`, via the `PRIMARY KEY` constraint) -
astronomically unlikely since the frontend generates a fresh UUID per group (the same `newRuleId()`-style
helper already used for rules), so no extra dedup handling is added.

**Alternative considered:** name it `SaveGroup` for naming symmetry with `SaveRule`. Rejected - `SaveGroup`
would imply update-in-place semantics that don't exist yet, and would need to be revisited (breaking or
overloading it) the moment rename/reorder is eventually built. `CreateGroup` says exactly what it does
today.

### 2. Server-side re-validation via `Group::new` in `CommandHandler`, not trusting the client struct
`CreateGroup`'s handler reconstructs the group via
`Group::new(group.id, group.name, group.parent_group_id)?` before calling `Store::create_group`, so the
existing self-parent check (`CoreError::InvalidGroupParent`) is enforced regardless of what the frontend
sends, then uses the *reconstructed* group (whose `sort_order` is always the constructor's default `0`)
for persistence - not the raw client-supplied struct. This mirrors `UpdateSettings`'s existing
`parse_binding` re-validation precedent (`crates/clipd/src/commands.rs`) rather than inventing a new
validation approach.

**Alternative considered:** validate only client-side (Settings never sends a self-referential
`parent_group_id` anyway, since the UI never sets one). Rejected - the daemon is the trust boundary for
every other command in this protocol; groups shouldn't be the one exception, especially since IPC clients
aren't necessarily limited to this one frontend.

### 3. `Store` trait gains `create_group`/`delete_group`, delegating directly to `clip-store::groups`
`SqliteStore::create_group` calls `clip_store::groups::insert`; `SqliteStore::delete_group` calls
`clip_store::groups::delete`. No new logic at this layer - it's a pure pass-through, same shape as
`list_groups`/`set_group`'s existing delegation.

### Test strategy per component

- **`clip-ipc` (`Command` variants)**: red - add `Command::CreateGroup`/`Command::DeleteGroup` to the
  existing `all_commands()` round-trip fixture in `protocol.rs`'s test module; run `cargo test -p clip-ipc`
  and confirm it fails to compile (variants don't exist). Green - add the variants.
- **`clipd` (`Store` trait + `FakeStore`)**: red - a failing test in `commands.rs` calling
  `handler.handle(Command::CreateGroup { group })` against a `FakeStore`/`FakeEventPublisher`, asserting
  the group is retrievable via a subsequent `list_groups()`; a second failing test for `DeleteGroup`
  asserting it's gone afterward. Both fail to compile first (new `Command` variants make `handle`'s
  `match` non-exhaustive - same expected-compile-failure pattern already used for `TriggerHotkey` in this
  codebase's history), then green once the match arms + trait methods + `FakeStore`/`SqliteStore` impls
  exist. Add a real-SQLite-backed test too (mirroring existing `SqliteStore` test coverage) asserting
  `delete_group` detaches a clip's `group_id` rather than deleting the clip - even though `clip-store`
  already proves this at its own layer, this closes the loop at the `clipd::Store` trait level the same
  scenario now flows through.
- **`clip-ui-tauri` src-tauri (`commands.rs`)**: red - a failing test calling `create_group_with`/
  `delete_group_with` against a `FakeClient`, asserting the right `Command` was sent (mirrors
  `save_rule_with`'s/`delete_rule_with`'s existing tests exactly). Green - add the wrapper functions and
  `#[tauri::command]`s, register in `lib.rs`'s `generate_handler!`.
- **`clip-ui-tauri` frontend (`Settings.tsx`)**: red - failing RTL tests in `Settings.test.tsx`: "creating a
  group issues CreateGroup with the entered name", "deleting a group issues DeleteGroup and removes it
  from the list", "shows a group returned by ListGroups on initial mount" - mirroring the three equivalent
  existing Rules tests exactly. Green - add the Groups section (state, mount-load, handlers, JSX) to
  `Settings.tsx`.

## Risks / Trade-offs

- **No rename/reparent UI yet, but the model supports it** → Accepted; explicitly scoped out per the
  proposal's non-goals. Revisit only if requested.
- **Duplicate-id insert failures surface as a raw SQLite error string to the user** → Accepted given the
  vanishingly small collision probability of client-generated UUIDs; no special-cased error message is
  added, consistent with how every other command in this protocol propagates store errors today.
