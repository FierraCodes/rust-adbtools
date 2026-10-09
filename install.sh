#!/bin/sh
# Build and install Rusty ADB for the current user (binary, icon, launcher entry).
set -e
cd "$(dirname "$0")"
cargo build --release
install -Dm755 target/release/rusty-adb "$HOME/.local/bin/rusty-adb"
install -Dm644 assets/icon.png "$HOME/.local/share/icons/hicolor/256x256/apps/rusty-adb.png"
install -Dm644 assets/icon.svg "$HOME/.local/share/icons/hicolor/scalable/apps/rusty-adb.svg"
install -Dm644 assets/rusty-adb.desktop "$HOME/.local/share/applications/rusty-adb.desktop"
# Absolute icon path so launchers with limited theme lookup still find it
sed -i "s|^Icon=.*|Icon=$HOME/.local/share/icons/hicolor/256x256/apps/rusty-adb.png|" "$HOME/.local/share/applications/rusty-adb.desktop"
gtk-update-icon-cache -qtf "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
echo "Installed. Launch 'Rusty ADB' from your app menu or run rusty-adb."
