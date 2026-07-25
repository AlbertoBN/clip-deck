# clip-deck
A Linux clipboard  manager inspired by Ditto for windows, written with a rust backend and a Tauri /React frontend

## Project layout

Cargo workspace with six crates under `crates/`:

- `clip-core` — shared domain models, MIME normalization, hashing, config
- `clip-store` — SQLite persistence (schema, FTS5 search, retention)
- `clip-platform` — Linux integration: X11 (primary) and Wayland adapters, hotkeys, focus, paste
- `clip-ipc` — daemon↔UI protocol over a Unix socket
- `clipd` — the daemon binary (background capture, IPC server)
- `clip-ui-tauri` — Tauri 2 + React/TypeScript UI (popup, manager, settings, tray); the Rust host lives in `crates/clip-ui-tauri/src-tauri`, the frontend in `crates/clip-ui-tauri/src`

The daemon (`clipd`) owns clipboard capture, hotkeys, and paste so it keeps working even if the UI is closed; the UI is a thin IPC client over a local Unix socket.

## Prerequisites

- Rust (stable) and Cargo
- Node.js + npm (for the frontend)
- Tauri's Linux system dependencies:
  ```sh
  sudo apt install pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev
  ```
- The Tauri CLI, as a Cargo subcommand:
  ```sh
  cargo install tauri-cli --version "^2"
  ```
- A running X11 session. `clipd`'s only wired backend today is X11 (`clip-platform`'s Wayland adapter exists but isn't yet selected by the daemon), so clipboard capture, global hotkeys, and paste need a real X server.

## Building

```sh
cargo check --workspace          # fastest correctness check across all Rust crates
cargo build --workspace          # build every Rust crate
cargo build -p clipd              # build just the daemon
```

The frontend has its own dependencies, installed separately:

```sh
cd crates/clip-ui-tauri
npm install
```

## Running

**1. Start the daemon** (needs an X11 session; keep it running in its own terminal):

```sh
cargo run -p clipd
```

It creates its SQLite database and Unix socket under the standard XDG paths (`~/.local/share/clipdeck/`, `~/.config/clipdeck/` by default — override with the `CLIPDECK_TEST_HOME` env var to sandbox them elsewhere).

**2. Run the UI** against that daemon, from `crates/clip-ui-tauri` (after `npm install`):

```sh
cd crates/clip-ui-tauri
cargo tauri dev
```

This starts the Vite dev server and launches the Tauri app; the app connects to `clipd`'s socket on startup and will fail to launch if the daemon isn't already running.

To build a production bundle instead of a dev session:

```sh
cd crates/clip-ui-tauri
cargo tauri build
```

## Testing

```sh
cargo test --workspace            # all Rust crates
cd crates/clip-ui-tauri && npm test   # frontend (Vitest)
```

Some tests are `#[ignore]`d because they need real hardware (a live X11/Wayland session) rather than a fake connection — run them explicitly with `cargo test -- --ignored` on a machine that has one.
