#!/usr/bin/env bash
# Registers the PhysicSense native messaging host with Chrome and Firefox.
# Run as root after `cargo build --release`.

set -euo pipefail

BINARY_PATH="$(cd "$(dirname "$0")" && pwd)/target/release/physicsense-native"
MANIFEST_NAME="com.physicsense.native"

CHROME_DIR_USER="$HOME/.config/google-chrome/NativeMessagingHosts"
FIREFOX_DIR_USER="$HOME/.mozilla/native-messaging-hosts"
CHROME_DIR_SYSTEM="/etc/opt/chrome/native-messaging-hosts"

MANIFEST=$(cat <<EOF
{
  "name": "${MANIFEST_NAME}",
  "description": "PhysicSense WiFi monitor-mode bridge",
  "path": "${BINARY_PATH}",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://YOUR_EXTENSION_ID_HERE/"
  ]
}
EOF
)

install_manifest() {
  local dir="$1"
  mkdir -p "$dir"
  echo "$MANIFEST" > "${dir}/${MANIFEST_NAME}.json"
  echo "  installed → ${dir}/${MANIFEST_NAME}.json"
}

echo "PhysicSense native host installer"
echo "Binary: ${BINARY_PATH}"

if [[ ! -f "$BINARY_PATH" ]]; then
  echo "ERROR: binary not found. Run: cargo build --release"
  exit 1
fi

chmod +x "$BINARY_PATH"

install_manifest "$CHROME_DIR_USER"
install_manifest "$FIREFOX_DIR_USER"

echo "Done. Replace YOUR_EXTENSION_ID_HERE with your Chrome extension ID."
