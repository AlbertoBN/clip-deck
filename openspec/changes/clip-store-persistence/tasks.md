## 1. Migration DDL and connection setup (`clip-persistence` part 1)

- [x] 1.1 Write a failing integration test that applies the migration runner to a fresh in-memory SQLite
      database and asserts `clips`, `clip_representations`, `groups`, `app_rules`, `settings`, `events`,
      and `clips_fts` all exist afterward.
- [x] 1.2 Run `cargo test -p clip-store db::` and confirm it fails (no runner/schema yet).
- [x] 1.3 Fill in `migrations/001_init.sql` with the PRD's full DDL and implement the migration runner in
      `crates/clip-store/src/db.rs` - minimum code to pass.
- [x] 1.4 Write a failing test asserting a second application of the migration runner is a no-op.
- [x] 1.5 Implement idempotent guards (`IF NOT EXISTS`, already present in the PRD's DDL) until the test
      passes.
- [x] 1.6 Write failing tests asserting new connections report `journal_mode = wal` and enforce foreign
      keys (insert with a bad `group_id` fails).
- [x] 1.7 Implement pragma setup in connection open; run `cargo test -p clip-store` and confirm green.

## 2. Clip CRUD and dedup (`clip-persistence` part 2)

- [x] 2.1 Write failing tests for: duplicate-clip-insert rejection, reinsertion-after-soft-delete success,
      get/list/update/soft-delete round trips, and representation-ordinal ordering, per
      `specs/clip-persistence/spec.md`.
- [x] 2.2 Run `cargo test -p clip-store clips::` and confirm failure.
- [x] 2.3 Implement `clips::insert/get/list/update/soft_delete` and representation persistence in
      `crates/clip-store/src/clips.rs`, translating the unique-index violation into a `DedupConflict`
      error - minimum code to pass.
- [x] 2.4 Run `cargo test -p clip-store` and confirm all green; refactor if needed while keeping tests
      green.

## 3. FTS synchronization and search (`clip-search-index`)

- [x] 3.1 Write failing tests for insert/update/delete FTS sync, per `specs/clip-search-index/spec.md`.
- [x] 3.2 Run `cargo test -p clip-store fts::` and confirm failure.
- [x] 3.3 Add the `clips_ai`/`clips_au`/`clips_ad` triggers to `migrations/001_init.sql` and implement the
      basic search query in `crates/clip-store/src/fts.rs` - minimum code to pass.
- [x] 3.4 Write failing tests for prefix matching and the empty-query pinned-first/recency fallback.
- [x] 3.5 Implement prefix-query construction and the empty-query fallback path.
- [x] 3.6 Write failing tests for MIME-family/group/pinned/favorite/source-app filters.
- [x] 3.7 Implement filter application in the search query builder.
- [x] 3.8 Write a failing test asserting a pinned clip ranks above an equally relevant unpinned clip.
- [x] 3.9 Implement ranking that combines BM25 with the pinned/recency boost inputs from
      `clip-core::search`.
- [x] 3.10 Run `cargo test -p clip-store` and confirm all green.

## 4. Group management (`group-management`)

- [x] 4.1 Write failing tests for group CRUD, children-of-a-parent listing, top-level listing, cascade
      delete of child groups, and clip detachment on group delete, per `specs/group-management/spec.md`.
- [x] 4.2 Run `cargo test -p clip-store groups::` and confirm failure.
- [x] 4.3 Implement `crates/clip-store/src/groups.rs` - minimum code to pass.
- [x] 4.4 Run `cargo test -p clip-store` and confirm all green.

## 5. App rules management (`app-rules-management`)

- [x] 5.1 Write failing tests for rule CRUD and enabled-only listing, per
      `specs/app-rules-management/spec.md`.
- [x] 5.2 Run `cargo test -p clip-store rules::` and confirm failure.
- [x] 5.3 Implement `crates/clip-store/src/rules.rs` - minimum code to pass.
- [x] 5.4 Run `cargo test -p clip-store` and confirm all green.

## 6. Event log (`event-log`)

- [x] 6.1 Write failing tests for clip-associated and clip-less event recording, by-clip and by-type
      listing, and clip-hard-delete nulling `clip_id`, per `specs/event-log/spec.md`.
- [x] 6.2 Run `cargo test -p clip-store events::` and confirm failure.
- [x] 6.3 Implement `crates/clip-store/src/events.rs` - minimum code to pass.
- [x] 6.4 Run `cargo test -p clip-store` and confirm all green.

## 7. Retention policy (`retention-policy`)

- [x] 7.1 Write failing tests for retention pruning (old-unpinned-removed, old-pinned-kept, no-window-
      no-op), bulk clear scopes (all vs. excluding-pinned), and single-clip delete, per
      `specs/retention-policy/spec.md`.
- [x] 7.2 Run `cargo test -p clip-store retention::` and confirm failure.
- [x] 7.3 Implement `crates/clip-store/src/retention.rs` - minimum code to pass.
- [x] 7.4 Run `cargo test -p clip-store` and confirm all green.

## 8. Crate-level verification

- [x] 8.1 Run `cargo test -p clip-store` and confirm every test from sections 1-7 passes.
- [x] 8.2 Run `cargo check --workspace` and confirm the rest of the workspace still compiles.
- [x] 8.3 Run `cargo clippy -p clip-store -- -D warnings` and fix any lints introduced by this change.
