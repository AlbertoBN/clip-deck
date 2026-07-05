## Context

`crates/clip-ui-tauri` is currently a bare Rust binary stub (`Cargo.toml` + a `main.rs` with `todo!()`);
`build.rs`, `tauri.conf.json`, and the entire `src/` frontend were deliberately left unscaffolded (see
`CLAUDE.md`) pending the Tauri CLI. This change is both the scaffolding step (`cargo tauri init`) and the
first real feature work for the UI shell, and is the first change in the workspace to introduce a
JavaScript/TypeScript toolchain (Node/npm are already present per `CLAUDE.md`'s environment notes).

## Goals / Non-Goals

**Goals:**
- Generate the missing Tauri scaffolding via `cargo tauri init` rather than hand-authoring
  `tauri.conf.json`/`build.rs`, per the existing scaffold's own guidance.
- A working popup → search → Enter-to-paste loop against a real running `clipd`, plus the manager,
  preview, tray, and settings surfaces from the PRD's UX requirements.
- Component-level test coverage for the frontend (Vitest + React Testing Library) and unit coverage for
  the Rust-side Tauri command bridge, without requiring a full Tauri runtime/webview in the test process.

**Non-Goals:**
- No new IPC commands/events - this change only consumes what `clip-ipc-transport` and
  `clipd-daemon-core` already define.
- No rich-content capture changes - HTML/image capture itself is `clip-platform-rich-content`'s scope;
  this change's preview pane renders whatever representations already exist by the time it lands
  (plain text from Milestone 1), and is extended, not replaced, once rich capture arrives.

## Decisions

- **Frontend stack**: React + TypeScript + Vite (Tauri's standard React template), state via a small
  store (Zustand) rather than Redux, given the app's modest state surface (clip list, filters, settings,
  connection state) - avoids boilerplate a full Redux setup would add for this scope.
- **IPC bridging**: the Tauri host (`src-tauri`) exposes `#[tauri::command]` wrappers around a single
  `clip-ipc::Client` instance held in Tauri's managed state; the frontend never talks to the Unix socket
  directly, only through Tauri's `invoke`. Events are forwarded from the Rust-side event subscription to
  the frontend via Tauri's event emitter, so `ui-ipc-state` on the frontend only needs to know about
  Tauri's `invoke`/`listen`, not raw IPC framing.
- **HTML sanitization**: `ammonia` (Rust) sanitizes HTML server-side (in the Tauri command that returns
  clip content to the frontend) rather than a client-side JS sanitizer, so untrusted HTML never reaches
  the webview unsanitized even if a future frontend change forgets to sanitize before rendering.
- **Tray**: Tauri's built-in tray APIs (`tauri::tray`), not a custom `clip-platform::tray_support` helper -
  the PRD calls out Tauri 2's tray support as the reason to pick Tauri, so this change uses it directly;
  `clip-platform::tray_support` stays a stub unless a genuine cross-cutting need emerges later.
- **Component testing without a real webview**: frontend components are tested with Vitest + React
  Testing Library against a mocked `invoke`/`listen` (Tauri's JS API), so keyboard navigation, rendering,
  and state-update logic are covered without spinning up an actual Tauri window. The Rust-side command
  bridge is tested by calling the `#[tauri::command]` functions directly (they're plain async functions
  under the attribute) against a fake `clip-ipc::Client`.

## Test strategy

- `popup-picker-ui`, `manager-window-ui`, `preview-pane-ui`, `tray-integration` (tray's web-facing parts),
  `settings-ui`: Vitest + React Testing Library component tests per scenario in each capability's spec,
  with Tauri's `invoke`/`listen` mocked to return canned responses/events. Run with `npm test` inside
  `crates/clip-ui-tauri`.
- `tray-integration`'s Rust-side menu wiring (action → command issued): Rust unit tests calling the tray
  menu-event handler function directly against a fake `clip-ipc::Client`, asserting the right command is
  issued per menu item. Run with `cargo test -p clip-ui-tauri`.
- `ui-ipc-state`: unit tests against the state store directly (not full components) - resolve/reject
  mapping test, per-event-type update tests, daemon-not-running distinct-state test. Run with `npm test`.
- HTML sanitization (part of `preview-pane-ui`): a Rust unit test asserting the `ammonia`-sanitized output
  of a `<script>`-containing input contains no script tag, and a second asserting benign tags survive.
  Run with `cargo test -p clip-ui-tauri`.

Red-green-refactor: for every task, write the component/unit test against the not-yet-implemented
component/function first (fails to render/compile or fails the assertion), implement the minimum to pass,
run the full test command for that side (`npm test` or `cargo test -p clip-ui-tauri`), then refactor with
tests green.

## Risks / Trade-offs

- [Risk] Testing against a mocked `invoke`/`listen` could miss real Tauri wiring bugs (command name
  typos, argument shape mismatches) → Mitigation: `tasks.md` includes a manual end-to-end verification
  pass (popup open → search → paste; tray pause/resume; settings save) against a real running `clipd`
  before this change is considered done.
- [Risk] Introducing a JS toolchain (Vite/Vitest/npm) is new territory for this otherwise Rust-only
  workspace → Mitigation: kept to Tauri's own standard template/tooling rather than a bespoke build setup.
- [Risk] `ammonia` sanitization policy could be too strict (drops legitimate formatting) or too loose →
  Mitigation: the benign-formatting-survives scenario in `preview-pane-ui`'s spec is a concrete test gate
  for the sanitizer's allowlist.
