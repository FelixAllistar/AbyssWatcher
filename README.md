# AbyssWatcher

![AbyssWatcher](assets/header.svg)

**AbyssWatcher** is a high-performance, always-on-top DPS Meter for EVE Online. Built with [Tauri](https://tauri.app/) (Rust) and React, it offers a transparent, zero-clutter HUD designed for "Immediate Cognitive Clarity" during high-intensity Abyssal Deadspace runs.

## Features

- **Real-Time Tracking**: Monitors DPS, HPS (Repairs), Capacitor, and Neut pressure instantly.
- **Unified Zero-Container HUD**: No bulky windows. Data floats directly on your screen with high-contrast visibility.
- **Multiboxing Support**: Automatically detects and aggregates logs from multiple clients into a single view.
- **Abyss Run Tracking**:
  - **Auto-Bookmarks**: Automatically marks `RUN_START`, `RUN_END`, and room changes in your game logs.
  - **Inline Bookmarks**: Manually add "Highlight" or "Room" markers with a single click for post-fight review.
- **Smart Alerts**:
  - **Neut Warning**: Audible alerts when specific "Neut Sensitive" characters are drained.
  - **Logi Support**: Dedicated alerts when your Logi pilots are attacked or neuted.
  - **Friendly Fire**: Detects accidental damage to fleet members (with Vorton projector filtering).
- **Log Replay**: Full time-scrubbing replay suite to analyze past fights tick-by-tick.

## Installation

### Download
Go to the [Releases Page](../../releases) and download the installer for your OS:
- **Windows**: `.msi` or `.exe`
- **Linux**: `.AppImage` or `.deb` (See Linux Setup below)
- **macOS**: `.dmg` or `.app`

### Linux Setup
If using the AppImage, ensure you have `libwebkit2gtk` installed (standard on most modern distros like Ubuntu 22.04+).

**Window Manager Quirks:**
- **KDE/Plasma**: The "Always on Top" feature works, but the window might flicker once on startup as we force the window manager to respect the setting.
- **Wayland**: We strictly force XWayland (`GDK_BACKEND=x11`) to prevent resizing artifacts common with transparent windows on native Wayland.

## Building from Source

**Prerequisites:**
- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- [Node.js](https://nodejs.org/) & [pnpm](https://pnpm.io/)
- **Linux only**: `libwebkit2gtk-4.1-dev`, `build-essential`, `curl`, `wget`, `file`, `libssl-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`

```bash
# 1. Clone the repo
git clone https://github.com/your-username/AbyssWatcher.git
cd AbyssWatcher

# 2. Install Frontend Dependencies
pnpm install

# 3. Build & Run (Dev Mode)
# This requires two terminals or backgrounding the dev process
cargo tauri dev

# 4. Build for Production
# Creates an optimized binary in target/release/bundle/
cargo tauri build
```

**Note:** Do not use `cargo install .`. You must use `cargo tauri build` to ensure the frontend assets are correctly compiled and embedded into the binary.

## Usage

1.  **Launch** AbyssWatcher.
2.  **Select Log Directory**: Point it to your `Documents/EVE/logs/Gamelogs` folder.
3.  **Active Characters**: Click the character dropdown in the header to toggle which pilots to track.
4.  **Overlay**:
    - **Drag** the header to move the window.
    - **Resize** using the edges/corners (invisible handles).
    - **Right-Click** the header to access Settings or Close.

## Architecture

- **Backend**: Rust (Tokio-based async log watching, Regex parsing, State management).
- **Frontend**: React 18 + TypeScript (Vite).
- **Communication**: Tauri Commands & Events.

## License

MIT