# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project status

The Cargo workspace and all six crates are scaffolded per the module plan below, but every module is an
empty stub (a doc comment, or `todo!()` in the two binaries) — there is no real implementation yet. Follow
the workspace layout and module plan rather than inventing a different structure. `cargo check --workspace`
passes cleanly. Building `clip-ui-tauri` requires Tauri's Linux system deps (`pkg-config`, `libgtk-3-dev`,
`libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`), which are installed on this machine.

The `clip-ui-tauri` crate is a bare Rust binary stub only — `build.rs`, `tauri.conf.json`, and the React/TS
frontend under `src/` have **not** been scaffolded. Run `cargo tauri init` (once the Tauri CLI is available)
to generate those rather than hand-authoring the config.

### Common commands

```bash
cargo check --workspace          # fastest correctness check across all crates
cargo build --workspace          # build everything
cargo build -p clipd             # build a single crate
cargo test --workspace           # run tests (none exist yet)
cargo run -p clipd                # currently panics: main() is a todo!()
```

## What ClipDeck is

A Linux clipboard manager for Ubuntu inspired by Ditto (Windows), built as a Rust daemon paired with a
Tauri 2 + React/TypeScript desktop shell. Core workflow: automatic clipboard capture in the background,
global-hotkey popup for fuzzy/full-text search over history, Enter-to-paste back into the previously
focused window.

## Planned architecture

The daemon owns all OS-sensitive integration (clipboard capture, global hotkeys, focused-window detection,
synthetic paste) so clip capture keeps working even if the UI process is closed or crashes. The UI is a thin
client over IPC.

```text
clip-ui (Tauri 2, popup/manager/tray)  <--IPC (Unix socket)-->  clipd (Rust daemon)
                                                                       |
                                                                       v
                                                          SQLite + FTS5, blob store on disk
```

### Rust workspace layout

```text
crates/
├── clip-core/       # Shared domain models (Clip, ClipRepresentation, Group, Rule, AppContext,
│                     # PasteMode), MIME normalization, content hashing, search query helpers, config, errors
├── clip-store/       # SQLite persistence: migrations, CRUD, FTS5 sync, groups, rules, events, retention jobs
├── clip-platform/    # Linux integration boundary: ClipboardBackend trait with separate x11/ and wayland/
│                     # adapters, hotkeys, focused-window discovery, paste simulation, capability diagnostics
├── clip-ipc/         # Daemon<->UI transport: protocol/DTOs, Unix-socket server, client, local-only auth
├── clipd/            # The daemon binary: watch loop, ingest pipeline, IPC command handlers, background jobs
└── clip-ui-tauri/    # Tauri host binary stub only — no build.rs/tauri.conf.json/frontend yet
```

`migrations/` holds the (currently empty placeholder) SQL migration files, and `assets/` is reserved for
app icons/static assets per the PRD's proposed workspace layout.

Key design rules to preserve when implementing:

- **x11 and wayland are separate adapters** behind one `ClipboardBackend` trait (`clip-platform`), each with
  explicit `capabilities()` reporting — never assume feature parity between the two; degrade and surface
  status in Settings instead of failing silently.
- **A clip has multiple representations** (e.g. plain text + HTML) stored as separate `clip_representations`
  rows referencing one `clips` row — don't collapse to a single format field.
- **Large binaries (images) live on disk**, referenced by `blob_path`; keep them out of the main SQLite file.
- **Dedup is by deterministic content hash** (`blake3`) combined with MIME type, enforced via a unique index
  on `(content_hash, primary_mime, is_deleted)`.
- **Search is SQLite FTS5** (`clips_fts` virtual table), kept in sync with the `clips` table via
  `AFTER INSERT/UPDATE/DELETE` triggers (or equivalent application-code sync if trigger overhead becomes an
  issue). Empty query falls back to `created_at DESC` with pinned-first ordering.
- **Preferred crates**: `rusqlite` over `sqlx` (explicit transactional control fits the local-only,
  SQLite-native model), `tokio`, `serde`/`serde_json`, `thiserror`+`anyhow`, `tracing`, `uuid`, `blake3`,
  `directories` for config/data paths, `image` for thumbnails.
- **IPC** is a local Unix domain socket carrying a command/event protocol (`SearchClips`, `PasteClip`,
  `PinClip`, etc. / `ClipCaptured`, `HotkeyPressed`, etc.) — see the PRD's "IPC contract" section for the
  full list before extending it.

### Build order

The PRD specifies an intentional delivery order because later crates depend on earlier ones being stable:
`clip-core` → `clip-store` → `clip-ipc` → `clip-platform::x11` → `clipd` → `clip-ui-tauri` →
`clip-platform::rich` (HTML/image) → `clip-platform::wayland`. X11 is the first fully-supported backend and
baseline for end-to-end tests; Wayland is deliberately deferred to its own milestone due to inconsistent
compositor support for automation.

## Development workflow: Test-Driven Development (mandatory)

This project is built strict TDD. This applies to every code change in this repo — bug fixes, new
features, refactors — not only work started from an OpenSpec change.

For every unit of behavior, follow this loop and do not skip or reorder steps:

1. **Red** — write a test that specifies the desired behavior before writing the implementation. For a
   bug fix, the test is a regression test that reproduces the bug.
2. **Confirm the failure** — run the test (or the crate's test module) and confirm it fails for the
   expected reason (missing type/function, wrong behavior) — not a typo or unrelated compile error.
3. **Green** — write the minimum code needed to make the test pass. Resist adding behavior the current
   test doesn't require.
4. **Confirm the suite is green** — run the full crate's test suite (`cargo test -p <crate>`), not just
   the new test, before moving on.
5. **Refactor** — clean up only while the suite stays green; re-run tests after refactoring.

Rules that follow from this:

- Never write implementation code without a preceding failing test that specifies it. If you catch
  yourself about to add a function/struct/branch with no test driving it, stop and write the test first.
- One logical behavior per red-green cycle — don't batch several behaviors behind one test.
- Before reporting any implementation task as done, run, at minimum:
  - `cargo test -p <crate>` for every crate you touched
  - `cargo check --workspace` (catches breakage in dependent crates)
  - `cargo clippy -p <crate> --all-targets -- -D warnings`
- When working through an OpenSpec change's `tasks.md`, each task is already written as an explicit
  red/green sub-step (see `openspec/config.yaml`'s `rules.tasks`) — follow the checklist as written rather
  than collapsing steps together, and check off `- [ ]` → `- [x]` only once that step's tests are green.
- Prefer fakes/dependency-injected test doubles over real OS/network/display-server state so logic is
  unit-testable (see any `design.md`'s "Test strategy" section for the fake used in that change, e.g.
  `X11Connection`, `Store`/`Backend`/`EventPublisher`); reserve real integration (`#[ignore]`d tests, manual
  verification) for the small surface that can't be faked.

## Reference

For full schema (migration SQL), the IPC command/event list, and the milestone-by-milestone acceptance
criteria, read `docs/ClipDeck-ubuntu-clipboard-manager-prd.md` directly rather than relying on this summary —
it is the source of truth for design decisions.
