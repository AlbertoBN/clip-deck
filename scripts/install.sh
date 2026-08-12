#!/usr/bin/env bash
# Builds ClipDeck from source and installs both halves of it:
#   - the `clipd` daemon + `clip-hotkey-trigger` helper, onto $PATH via
#     `cargo install`, plus a systemd --user service so `clipd` starts
#     automatically at login and restarts itself if it ever crashes
#   - the ClipDeck desktop app, as a native .deb package (`cargo tauri build`
#     + `apt install`)
#
# See README.md's "Installing `clipd` as a background daemon" and "Installing
# the desktop app" sections for what this automates and why, and for the
# manual steps if you'd rather run them yourself.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$REPO_ROOT"

INSTALL_DAEMON=1
INSTALL_UI=1

usage() {
  cat <<'EOF'
Usage: scripts/install.sh [--daemon-only|--ui-only] [-h|--help]

Options:
  --daemon-only   Install/start only the clipd daemon and its systemd service.
  --ui-only       Build and install only the desktop app package (assumes a
                  clipd daemon is already installed and reachable).
  -h, --help      Show this help.

With no options, installs both.
EOF
}

for arg in "$@"; do
  case "$arg" in
    --daemon-only) INSTALL_UI=0 ;;
    --ui-only) INSTALL_DAEMON=0 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $arg" >&2; usage >&2; exit 1 ;;
  esac
done

step() { printf '\n==> %s\n' "$1"; }

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: '$1' is required but not found on PATH ($2)" >&2
    exit 1
  fi
}

require_cmd cargo "install Rust via https://rustup.rs/"

if [ "$INSTALL_DAEMON" = 1 ]; then
  step "Stopping and removing any existing clipd installation"
  # Must happen before the new binary is built/installed below: `cargo
  # install --force` only overwrites the file on disk, it does not touch an
  # already-running process, and starting the service further down is a
  # no-op if it's already active. Without stopping it first, a previously
  # running clipd keeps serving the OLD binary indefinitely after an
  # upgrade - the new build silently never actually runs until someone
  # notices and restarts it by hand.
  systemctl --user stop clipd.service 2>/dev/null || true
  cargo uninstall clipd 2>/dev/null || true

  step "Building and installing clipd + clip-hotkey-trigger onto \$PATH"
  cargo install --path crates/clipd --force

  CARGO_BIN="${CARGO_HOME:-$HOME/.cargo}/bin"
  case ":$PATH:" in
    *":$CARGO_BIN:"*) ;;
    *) echo "warning: $CARGO_BIN is not on your PATH - add it in your shell profile" >&2 ;;
  esac

  step "Installing the systemd --user service"
  SERVICE_DIR="$HOME/.config/systemd/user"
  SERVICE_FILE="$SERVICE_DIR/clipd.service"
  mkdir -p "$SERVICE_DIR"
  if [ -f "$SERVICE_FILE" ]; then
    echo "  $SERVICE_FILE already exists - leaving it untouched"
  else
    cat > "$SERVICE_FILE" <<EOF
[Unit]
Description=ClipDeck clipboard daemon
After=graphical-session.target

[Service]
ExecStart=$CARGO_BIN/clipd
Restart=on-failure
RestartSec=2

[Install]
WantedBy=graphical-session.target
EOF
    echo "  wrote $SERVICE_FILE"
  fi

  systemctl --user daemon-reload
  systemctl --user enable clipd.service
  # `restart` rather than `enable --now`: `--now` only starts the unit if
  # it isn't already active, which would silently no-op (and leave the old
  # binary running) if the stop above somehow didn't take effect. `restart`
  # unconditionally (re)launches it against the binary just installed,
  # whether or not it was already running.
  systemctl --user restart clipd.service

  step "clipd status"
  systemctl --user --no-pager status clipd.service || true
fi

if [ "$INSTALL_UI" = 1 ]; then
  require_cmd npm "install Node.js 18+ from https://nodejs.org/"
  if ! cargo tauri --version >/dev/null 2>&1; then
    echo 'error: the Tauri CLI is required but not found. Install it with:' >&2
    echo '  cargo install tauri-cli --version "^2"' >&2
    exit 1
  fi

  MISSING_DEPS=()
  for pkg in pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev; do
    dpkg -s "$pkg" >/dev/null 2>&1 || MISSING_DEPS+=("$pkg")
  done
  if [ "${#MISSING_DEPS[@]}" -gt 0 ]; then
    echo "error: missing Tauri Linux system dependencies: ${MISSING_DEPS[*]}" >&2
    echo "  sudo apt install ${MISSING_DEPS[*]}" >&2
    exit 1
  fi

  step "Installing frontend dependencies"
  (cd crates/clip-ui-tauri && npm install)

  step "Building the desktop app bundle"
  (cd crates/clip-ui-tauri && cargo tauri build)

  BUNDLE_DIR="crates/clip-ui-tauri/src-tauri/target/release/bundle/deb"
  DEB_FILE="$(ls -t "$BUNDLE_DIR"/*.deb 2>/dev/null | head -n1 || true)"
  if [ -z "$DEB_FILE" ]; then
    echo "error: no .deb package found under $BUNDLE_DIR" >&2
    exit 1
  fi
  DEB_FILE="$(realpath "$DEB_FILE")"

  step "Installing $DEB_FILE (requires sudo)"
  sudo apt install "$DEB_FILE"
fi

step "Done"
