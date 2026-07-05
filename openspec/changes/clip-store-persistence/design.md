## Context

`clip-store`'s modules (`db`, `clips`, `fts`, `groups`, `rules`, `events`, `retention`) are currently
one-line stubs, and `migrations/001_init.sql` is a placeholder comment pointing at the PRD. `clip-core-
foundations` provides the `Clip`, `ClipRepresentation`, `Group`, `Rule`, and hashing/MIME types this crate
persists. Nothing outside tests depends on `clip-store` yet, so this change is free to iterate on its
internal schema-mapping approach as long as the public API matches what `clipd-daemon-core` and
`clip-ipc-transport` will need per the PRD's IPC contract.

## Goals / Non-Goals

**Goals:**
- Fill in `migrations/001_init.sql` with the PRD's full DDL (clips, clip_representations, clips_fts,
  groups, app_rules, settings, events, indexes, FTS triggers) and a migration runner that applies it
  idempotently.
- Implement clip/group/rule/event CRUD, FTS-backed search, and retention pruning against a real,
  test-covered SQLite database (in-memory or temp-file, per test).
- Keep every operation transactional where the PRD implies atomicity (e.g. clip insert + its
  representations + its FTS row).

**Non-Goals:**
- No IPC wiring (`clip-ipc-transport` defines the wire DTOs separately; this crate exposes plain Rust
  functions/structs).
- No daemon-side scheduling of retention pruning (that's `clipd-daemon-core`'s `jobs` module) - this
  change only provides the prune *query*, not a cron-like scheduler.
- No thumbnail generation (that's `clip-platform-rich-content`).

## Decisions

- **Migration mechanism**: hand-rolled idempotent migration runner using `CREATE TABLE IF NOT EXISTS` /
  `CREATE INDEX IF NOT EXISTS` (as the PRD's migration SQL already does) tracked via a `schema_version`
  pragma or a dedicated table, rather than pulling in `refinery` - the PRD ships a single seed file, and a
  minimal runner avoids adding a migration-framework dependency for one file. Revisit if migration count
  grows past a handful.
- **FTS sync mechanism**: implement via SQL triggers (`clips_ai`/`clips_au`/`clips_ad`) exactly as
  specified in the PRD's migration file, rather than application-code sync, since it keeps insert/update/
  delete atomic with the FTS row without extra application logic; the PRD explicitly allows switching to
  application-code sync later if trigger overhead becomes a problem.
- **Dedup enforcement**: rely on the unique index `idx_clips_hash_mime` and translate the resulting SQLite
  constraint violation into a typed `DedupConflict` error, rather than doing a `SELECT`-then-`INSERT`
  check (avoids a race between check and insert).
- **Connection model**: a single `rusqlite::Connection` wrapped behind a small pool-free handle for now
  (the daemon is single-process and mostly single-writer); revisit pooling only if concurrent-access tests
  reveal contention.

## Test strategy

Every scenario in `specs/*/spec.md` becomes a `#[test]` using a fresh temp-file or in-memory SQLite
database per test (via `AppPaths` override / a test helper that runs migrations against `":memory:"`).
Per component:

- `db`: migration idempotency test (apply twice, assert no error/duplication), WAL + foreign_keys pragma
  assertions, and a foreign-key-violation test. Run with `cargo test -p clip-store db::`.
- `clips`: dedup-conflict test, soft-delete-then-reinsert test, get/list/update/soft-delete tests,
  representation-ordinal round-trip test. Run with `cargo test -p clip-store clips::`.
- `fts`: insert/update/delete sync tests, prefix-match test, empty-query pinned-first-then-recency test,
  filter tests (group/pinned), ranking test (pinned beats equally-relevant unpinned). Run with
  `cargo test -p clip-store fts::`.
- `groups`: CRUD tests, children-listing test, cascade-delete test, detach-clips-on-delete test. Run with
  `cargo test -p clip-store groups::`.
- `rules`: CRUD tests, enabled-only listing test. Run with `cargo test -p clip-store rules::`.
- `events`: clip-associated and clip-less recording tests, by-clip and by-type listing tests, clip-hard-
  delete-nulls-clip_id test. Run with `cargo test -p clip-store events::`.
- `retention`: prune-excludes-pinned test, no-window-is-no-op test, bulk-clear-scope tests, single-delete
  test. Run with `cargo test -p clip-store retention::`.

Red-green-refactor: for every task, write the test against the not-yet-implemented function/table first
(compile failure or assertion failure), confirm the failure reason matches expectations, implement the
minimum SQL/Rust to pass, run the full `cargo test -p clip-store` suite, then refactor with tests green.

## Risks / Trade-offs

- [Risk] SQL trigger-based FTS sync is harder to unit-test in isolation from real SQLite → Mitigation:
  tests run against real (temp) SQLite rather than mocking, so trigger behavior is exercised for real.
- [Risk] Retention pruning is destructive (hard delete) → Mitigation: pinned-clip exemption is a hard
  requirement with its own test, and bulk clear scopes are explicit enums, not a free-form string, so a
  typo can't silently clear everything.
- [Risk] Migration file growing to control single-file DDL forever → Mitigation: acceptable for v1 per the
  PRD; revisit a real migration framework if/when `002_seed_defaults.sql` or later migrations need more
  than "apply once."
