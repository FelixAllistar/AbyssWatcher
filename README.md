# AbyssWatcher


**AbyssWatcher** is a high-performance, always-on-top DPS Meter for EVE Online. Built with [Tauri](https://tauri.app/) (Rust) and React, it provides a zero-clutter HUD for real-time combat analysis.

## Features

- **Real-Time HUD**: Transparent overlay for DPS, HPS, Capacitor, and Neut tracking.
- **Multiboxing**: Automatically aggregates logs from multiple characters.
- **Abyss Tracking**: Auto-bookmarks for runs/rooms and manual highlight markers.
- **Smart Alerts**: Audible warnings for Neuts, Logi under attack, and Friendly Fire.
- **Log Replay**: Full time-scrubbing suite for post-fight review.

## Installation

### Download
Get the latest build from [Releases](../../releases).

- **Windows**: `.msi` or `.exe`
- **Linux**: `.AppImage` or `.deb`
- **macOS**: `.dmg` or `.app`

> **Note for macOS:** Builds are currently **unsigned**. You may need to allow the application manually in System Settings or via terminal to run it.

## Building from Source

Requires [Rust](https://www.rust-lang.org/), [Node.js](https://nodejs.org/), and [pnpm](https://pnpm.io/).

```bash
# Install dependencies
pnpm install

# Build for Production (Recommended)
cargo tauri build

# Artifacts will be in:
# - Windows: target/release/bundle/msi/
# - Linux: target/release/bundle/appimage/
# - macOS: target/release/bundle/macos/
```
*Note: Do not use `cargo install`. Always use `cargo tauri build` to correctly embed frontend assets.*

## Usage

1. **Log Dir**: Point to `Documents/EVE/logs/Gamelogs`.
2. **Characters**: Use the header dropdown to toggle pilots.
3. **Window**: Drag header to move; use edges to resize. Right-click header for Settings/Close.

## License
MIT
