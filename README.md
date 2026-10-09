<div align="center">

<img src="assets/icon.png" alt="Rusty ADB icon" width="128" height="128">

# Rusty ADB

**An all-in-one ADB & scrcpy multitool with a native GUI, built for app management and debloating.**

![Rust](https://img.shields.io/badge/Rust-2024-orange?logo=rust&logoColor=white)
![GUI](https://img.shields.io/badge/GUI-egui%20%2F%20eframe-blue)
![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey)
![License](https://img.shields.io/badge/license-MIT-green)
![ADB](https://img.shields.io/badge/requires-adb-3DDC84?logo=android&logoColor=white)
![scrcpy](https://img.shields.io/badge/requires-scrcpy-informational)

</div>

---

Rusty ADB wraps `adb` and `scrcpy` in a fast, single-binary desktop app. Manage packages in bulk,
strip bloatware using the community-maintained UAD-ng list, mirror your screen, or turn your
phone's camera into a Linux webcam, all without memorising commands.

## ✨ Features

### 📦 App management
- Browse every package, filtered by **User / System / Disabled / Removed / All**, with live search
- **Real app icons**, extracted from each APK and cached on disk
- Bulk actions on a multi-selection: **Uninstall, Disable, Enable, Restore, Clear data, Force stop, Pull APK**
- Detail panel with version, install times, install location and granted permissions
- Launch an app, jump to its system settings page, or copy its package name

### 🗑 Debloat
- Powered by the **[Universal Android Debloater Next Generation](https://github.com/Universal-Debloater-Alliance/universal-android-debloater-next-generation)** package list (about 5,400 entries), downloaded at runtime and refreshed weekly
- Only shows packages that are **actually on your device**
- Colour-coded removal ratings: **recommended · advanced · expert · unsafe**, with full descriptions and dependency info on hover
- Filter by list (Oem, Google, Aosp, Carrier, Misc) and removal level, or search by name
- Choose **Uninstall for user 0** (reversible) or **Disable**
- Confirmation dialog warns about expert and unsafe removals
- **Export / import** selections so you can reuse a debloat profile across devices
- Every action is logged to `the app config folder (see README)`
- Built-in fallback list when you're offline

### 🖥 Mirror (scrcpy)
- Launch and stop scrcpy with a GUI for the common options: max size, bitrate, FPS, recording, window title, no audio, view-only, screen off, stay awake, always on top, fullscreen, show touches
- **Device camera as source**: choose back, front, external or a specific camera id, plus size, FPS and torch
- **Use as a webcam on Linux** through a `v4l2loopback` virtual camera, from either the screen or the camera
- **⚡ Low-latency preset** (720p, 30 fps, 4M, no audio)

### 🔧 Tools
- Wireless debugging: connect, disconnect, **pair** with a code, enable TCP/IP
- Install APKs, with a file picker
- Take screenshots into a folder you choose
- Reboot to system, recovery, bootloader or sideload

### ⌨ Shell
- Run `adb shell` commands and read the output

## 🚀 Getting started

### Requirements

| Dependency | Why |
|---|---|
| [Rust](https://rustup.rs) (stable, recent) | to build |
| `adb` (android-tools) | device communication |
| `scrcpy` 2.2 or newer | mirroring and camera (camera mirroring also needs Android 12+) |
| `v4l2loopback` *(optional)* | the webcam feature |

| OS | Install adb and scrcpy |
|---|---|
| Arch Linux | `sudo pacman -S android-tools scrcpy` |
| Debian / Ubuntu | `sudo apt install adb scrcpy` |
| macOS | `brew install android-platform-tools scrcpy` |
| Windows | `winget install Genymobile.scrcpy` (it bundles `adb`), or `scoop install adb scrcpy` |

Rusty ADB looks for `adb` and `scrcpy` on your `PATH`, and also in Homebrew's folders and the
default Android SDK `platform-tools` folders, so it works when launched from Finder or the Start menu.

### Build & install

```bash
git clone https://github.com/FierraCodes/rust-adbtools.git
cd rust-adbtools
./install.sh
```

**Linux:** `install.sh` builds a release binary and installs, for your user only:

- `~/.local/bin/rusty-adb`
- the icon into `~/.local/share/icons/hicolor/`
- a launcher entry in `~/.local/share/applications/`

Make sure `~/.local/bin` is on your `PATH`.

**macOS / Windows (and Linux without the script):** build and run with Cargo:

```bash
cargo run --release          # run from source
cargo install --path .       # or install the binary into ~/.cargo/bin
```

Pre-built binaries for all three platforms are produced by the GitHub Actions workflow
(download them from the **Actions** tab, or from **Releases** for tagged versions).

### Connecting a device

1. Enable **Developer options** and **USB debugging** on the phone.
2. Plug it in and accept the RSA prompt.
3. Pick the device in the top bar. The package list loads automatically.

For wireless debugging, use the **Tools** tab to pair and connect.

## 📷 Using your phone as a webcam (Linux)

The virtual-webcam output relies on `v4l2loopback`, so it is **Linux only**. On macOS and Windows the
camera and screen still mirror to a window; capture that window in OBS if you need a virtual camera.

Load the virtual camera module once (it needs root):

```bash
sudo modprobe v4l2loopback devices=1 video_nr=10 card_label="Android Cam" exclusive_caps=1
```

Then, on the **Mirror** tab:

1. Choose **Device camera** (or leave it on **Screen** to share your screen as a camera).
2. Tick **Use as a webcam** and pick the `/dev/video10` device.
3. Press **⚡ Low-latency preset**, then **Start scrcpy**.

Select "Android Cam" in Zoom, OBS, your browser, and so on. If it lags, check that the camera
size isn't the maximum (for example 4096x3072). The preset sets 720p for that reason.

## 🧹 Undoing a removal

Debloating uses `pm uninstall --user 0`, which only removes an app for the current user and leaves
it in the system image. To bring one back, go to **Apps → Removed**, select it and press
**Restore**. Treat "expert" and "unsafe" entries with care: some can break calls, SMS, updates, or
cause a bootloop.

## 🗂 Where things are stored

| What | Linux | macOS | Windows |
|---|---|---|---|
| Cache (`uad_lists.json`, `icons/`) | `~/.cache/rusty-adb` | `~/Library/Caches/rusty-adb` | `%LOCALAPPDATA%\rusty-adb` |
| Config (`actions.log`, a log of uninstall / disable / enable / restore actions) | `~/.config/rusty-adb` | `~/Library/Application Support/rusty-adb` | `%APPDATA%\rusty-adb` |

## 🧱 Project layout

```
src/
├── main.rs      # egui app: tabs, state, background tasks
├── platform.rs  # OS helpers: data dirs, finding adb/scrcpy, no console flash on Windows
├── adb.rs       # adb wrapper: devices, packages, icons, pulls, action log
└── debloat.rs   # UAD-ng list loading, caching and the offline fallback
assets/          # icon (SVG + PNG) and .desktop entry
install.sh       # build and install for the current user
```

Commands run on worker threads and report back over a channel, so the UI stays responsive.

## ⚠️ Disclaimer

Removing system packages can break your device. Back up first, start with **recommended**
entries, and know how to restore. You use this tool at your own risk.

## 📄 License

Released under the [MIT License](LICENSE).

## 🙏 Credits

- [scrcpy](https://github.com/Genymobile/scrcpy) by Genymobile
- [Universal Android Debloater Next Generation](https://github.com/Universal-Debloater-Alliance/universal-android-debloater-next-generation) for the package database
- [egui](https://github.com/emilk/egui) for the UI
