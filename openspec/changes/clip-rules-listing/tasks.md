## 1. `clip-store`: list all rules (`app-rules-management`)

- [ ] 1.1 Write a failing test in `crates/clip-store/src/rules.rs` inserting one enabled and one disabled
      rule and asserting `rules::list_all` returns both, ordered by `app_match` then `id`, per
      `specs/app-rules-management/spec.md`.
- [ ] 1.2 Run `cargo test -p clip-store rules::` and confirm it fails (`list_all` does not exist yet).
- [ ] 1.3 Implement `rules::list_all` in `crates/clip-store/src/rules.rs`, mirroring `groups::list_all` -
      minimum code to pass.
- [ ] 1.4 Run `cargo test -p clip-store` and confirm the full suite is green.

## 2. `clip-ipc`: `ListRules` command (`ipc-command-handlers` protocol shape)

- [ ] 2.1 Write a failing round-trip serde test in `crates/clip-ipc/src/protocol.rs` for
      `Command::ListRules` (serialize/deserialize with `type: "ListRules"`), mirroring the existing
      `Command::ListGroups` test.
- [ ] 2.2 Run `cargo test -p clip-ipc` and confirm it fails (variant does not exist).
- [ ] 2.3 Add `ListRules` to the `Command` enum in `crates/clip-ipc/src/protocol.rs` - minimum code to
      pass.
- [ ] 2.4 Run `cargo test -p clip-ipc` and confirm the full suite is green.

## 3. `clipd`: handle `ListRules` (`ipc-command-handlers`)

- [ ] 3.1 Write a failing test in `crates/clipd/src/commands.rs` calling
      `CommandHandler::handle(Command::ListRules)` against a `FakeStore` seeded with one enabled and one
      disabled rule, asserting the response contains both, per `specs/ipc-command-handlers/spec.md`.
- [ ] 3.2 Run `cargo test -p clipd commands::` and confirm it fails (no `Store::list_rules` method, no
      match arm).
- [ ] 3.3 Add a `list_rules` method to the `Store` trait in `crates/clipd/src/app.rs`, implement it on
      `SqliteStore` (delegating to `clip_store::rules::list_all`) and on `FakeStore` (crate
      `clipd`'s `app::fakes` module), and add the `ListRules` match arm in `commands.rs` mirroring
      `ListGroups`'s arm - minimum code to pass.
- [ ] 3.4 Run `cargo test -p clipd` and confirm the full suite is green.

## 4. `clip-ui-tauri` (Rust side): `list_rules` Tauri command

- [ ] 4.1 Write a failing test in `crates/clip-ui-tauri/src-tauri/src/commands.rs` calling a
      `list_rules_with(client: &dyn Client)` core function against a `FakeClient` returning canned rules,
      asserting the returned `Vec<Rule>` matches.
- [ ] 4.2 Run `cargo test -p clip-ui-tauri` and confirm it fails (function does not exist).
- [ ] 4.3 Implement `list_rules_with` and the thin `#[tauri::command] list_rules(state)` wrapper, mirroring
      the existing `list_groups`/`list_groups_with` pair - minimum code to pass.
- [ ] 4.4 Add `list_rules` to the `invoke_handler(tauri::generate_handler![...])` list in
      `crates/clip-ui-tauri/src-tauri/src/lib.rs`.
- [ ] 4.5 Run `cargo test -p clip-ui-tauri` and confirm the full Rust suite is green.

## 5. `clip-ui-tauri` (frontend): fetch rules on settings mount

- [ ] 5.1 Write a failing Vitest test in `Settings.test.tsx` asserting a rule returned by a mocked
      `list_rules` invoke renders in the rules list on initial mount, before any create/delete action, per
      `specs/settings-ui/spec.md`.
- [ ] 5.2 Run `npm test` (inside `crates/clip-ui-tauri`) and confirm it fails (list starts empty today).
- [ ] 5.3 Implement the `list_rules` fetch in `src/views/settings/Settings.tsx`'s mount effect, seeding the
      local rules `useState` from the response, keeping the existing optimistic
      create/delete-list-mutation logic unchanged - minimum code to pass.
- [ ] 5.4 Run `npm test` and confirm the full frontend suite (including existing create/delete rule tests)
      is green.

## 6. Crate-level verification

- [ ] 6.1 Run `cargo check --workspace` and confirm the rest of the workspace still compiles.
- [ ] 6.2 Run `cargo clippy -p clip-store -p clip-ipc -p clipd -p clip-ui-tauri --all-targets -- -D
      warnings` and fix any lints introduced by this change.
- [ ] 6.3 Run `npm run build` inside `crates/clip-ui-tauri` and confirm the frontend still type-checks and
      builds.
