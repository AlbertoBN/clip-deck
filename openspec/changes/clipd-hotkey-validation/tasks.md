## 1. Hotkey binding validation (`ipc-command-handlers`, `clipd`)

- [ ] 1.1 Write a failing test in `crates/clipd/src/commands.rs` asserting `UpdateSettings` with
      `hotkey_binding: "NotAKey+++"` against a `FakeStore` returns `Err(_)` and leaves the fake store's
      settings unchanged, per `specs/ipc-command-handlers/spec.md`.
- [ ] 1.2 Run `cargo test -p clipd commands::` and confirm it fails because today's handler persists the
      value unconditionally.
- [ ] 1.3 Implement the guard in the `UpdateSettings` match arm: call
      `clip_platform::hotkeys::parse_binding(&settings.hotkey_binding)` and return its error (as a
      `String`) before any store write when it fails - minimum code to pass.
- [ ] 1.4 Write a failing test asserting a valid binding (`"Ctrl+Shift+V"`) still persists and round-trips
      through a subsequent `GetSettings`.
- [ ] 1.5 Run `cargo test -p clipd` and confirm the full suite (existing + 2 new tests) is green.
- [ ] 1.6 Run `cargo check --workspace` and `cargo clippy -p clipd --all-targets -- -D warnings` and fix any
      issues introduced by this change.
