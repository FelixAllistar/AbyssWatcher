# AbyssWatcher

AbyssWatcher is a high-performance DPS Meter for EVE Online, built as a modern desktop overlay using Tauri and Rust. It reads combat logs from local files and displays real-time damage tracking data in a transparent, always-on-top HUD using web technologies (HTML/CSS/JS).

## Project Purpose
- **Functionality**: Monitors EVE Online combat logs by parsing local log files in real-time.
- **Display**: Shows data in a standalone overlay window designed for "Immediate Cognitive Clarity" – high contrast, green/red indicators, and data-dense layouts.
- **Scope**: A single, unobtrusive overlay window. No separate main window or hidden background processes.

## Architecture Overview

- **Tauri Application**: The project is structured as a standard Tauri app where the root directory contains the Rust backend (`Cargo.toml`) and the `ui` directory contains the frontend assets.
- **Backend (Rust)**:
  - `src/main.rs`: The entry point that initializes the Tauri runtime via `abyss_watcher::run()`.
  - `src/app.rs`: The core Tauri application logic, command handlers, and state management.
  - `src/core/`: Application domain logic, strictly separated from the UI.
    - `model.rs`: Combat events, DPS samples, fight summaries.
    - `parser.rs`: Regex-based log line parsing (`(combat)` lines).
    - `log_io.rs`: Efficient log tailing and historical scanning.
    - `analysis.rs`: DPS computation and time-series aggregation.
    - `state.rs`: The `EngineState` that holds combat history.
    - `config.rs`: Settings management and persistence (JSON-based).
    - `coordinator.rs`: Orchestrates log watching and DPS computation loop.
    - `watcher.rs`: Manages multiple `TrackedGamelog` instances.
    - `tracker.rs`: Wraps a single log file with tailer + parser.
    - `replay_engine.rs`: Log replay with merged streams and speed control.
- **Frontend (Web)**:
  - Located in `ui/`.
  - Built with HTML5, CSS3, and JavaScript (Vanilla or Framework-light).
  - Communicates with Rust via Tauri Commands (`invoke`) and Events (`emit`).
  - **Windows**:
    - **Live Overlay** (`index.html`): The main transparent DPS tracking window.
    - **Replay Suite** (`replay.html`): The log replay and analysis window.
  - **Styling** (`ui/styles/`):
    - `theme.css`: Single source of truth for all CSS variables (colors, fonts). Edit this for theming.
    - `common.css`: Shared component styles (`.icon-btn`, `.dps-box`, `.value`, utility classes like `.text-dps-out`).
    - `main.css`: Imports theme/common + Live Overlay-specific layout (settings modal, character selection).
    - `replay.css`: Imports theme/common + Replay Suite-specific layout (timeline, controls).
  - **Components** (`ui/components.js`): Shared JS for rendering character lists and DPS updates. Uses CSS classes (e.g., `.text-action-damage`) instead of hardcoded colors.

## Design & UX Principles

Follow these guidelines from `conductor/product-guidelines.md`:
- **Technical and Precise**: No "fluff". Prioritize data accuracy and "no-nonsense" aesthetics.
- **Data-Dense and Compact**: Maximize information per pixel. Critical for multiboxers monitoring multiple clients.
- **Visual-First Status**: Use color (Green=Good/Dealt, Red=Bad/Received) and trends rather than just raw numbers where possible.
- **Strict Log Validation**: "Truth-in-data". Do not guess or infer missing data. If the log is ambiguous, flag it or ignore it.

## Tech Stack

- **Core**: Rust (2021 Edition) for performance and safety.
- **Framework**: Tauri (v2) for the application shell and system integration.
- **Frontend**: HTML/CSS/JS.
- **Async Runtime**: `tokio` for non-blocking I/O (log watching).
- **Serialization**: `serde` / `serde_json`.
- **Time**: `chrono` for EVE log timestamp parsing.
- **Pattern Matching**: `regex` for parsing combat log lines.
- **Key Plugins**:
  - `tauri-plugin-dialog`: For selecting gamelog directories.
  - `tauri-plugin-log`: For internal application logging.

## Core Goals

1.  **Real-time Performance Monitoring**: Unobtrusive, low-latency DPS tracking.
2.  **Historical Visualization**: Review past fights (stored in memory/session).
3.  **Seamless Multiboxing**: Automatically detect and aggregate logs from multiple characters.

## Coding Guidelines

- **Domain vs UI Separation**: Keep all parsing and math in `src/core`. `src/app.rs` should only handle "wiring" – passing data from Core to Frontend.
- **Performance**:
  - Use `std::time::Duration` for all internal time math.
  - Avoid heavy computation on the main thread; use `tokio` tasks for log IO.
- **Reliability**:
  - Prefer owned `String` in Core for safety.
  - Handle all IO errors gracefully (e.g., file permission issues on logs).
## Linux-Specific Fixes


### 2. Always-On-Top "Double-Tap"
- **Problem**: Some Linux window managers (KDE) ignore the `alwaysOnTop` setting in `tauri.conf.json` during initial window creation.
- **Fix**: Implemented in `src/app.rs`. The application explicitly calls `set_always_on_top(true)` twice: once immediately on setup, and again after a 500ms delay to ensure the window manager respects the state once the window is fully mapped.
### 3. Wayland Transparency Resize Fix
- **Problem**: Native Wayland (via WebKitGTK) exhibits resizing synchronization artifacts with transparent windows, causing severe flickering or size instability (e.g., "size crawl").
- **Fix**: Force the application to use XWayland (X11 compatibility mode) by setting `GDK_BACKEND=x11`. This is enforced in:
  - `abyss-watcher.desktop`: `Exec=env GDK_BACKEND=x11 ...`
  - Dev environment: `.cargo/config.toml` sets `[env] GDK_BACKEND = "x11"`
