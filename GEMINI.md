# AbyssWatcher

AbyssWatcher is a high-performance DPS Meter for EVE Online, built as a modern desktop overlay using Tauri and Rust. It reads combat logs from local files and displays real-time damage tracking data in a transparent, always-on-top HUD using web technologies (HTML/CSS/JS).

## Project Purpose
- **Functionality**: Monitors EVE Online combat logs by parsing local log files in real-time.
- **Display**: Shows data in a standalone overlay window designed for "Immediate Cognitive Clarity" – high contrast, green/red indicators, and data-dense layouts.
- **Scope**: A single, unobtrusive overlay window. No separate main window or hidden background processes.

## Architecture Overview

- **Tauri Application**: The project is structured as a standard single-package Tauri app where the root directory contains both the Rust backend (`Cargo.toml`) and the frontend configuration, with source code in the `ui` directory.
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
  - **Stack**: React 18, TypeScript, Vite 7 (configured at root).
  - **State Management**: React Hooks (`useState`, `useEffect`) avoiding full DOM repaints.
  - **Communication**: Communicates with Rust via `@tauri-apps/api` (Commands & Events).
  - **Entry Point**: `ui/src/main.tsx` → `ui/src/App.tsx`
  - **Routing**: `App.tsx` automatically detects the window label (`main` or `replay`) to render:
    - **Live Overlay** (`MainApp`): The transparent DPS tracking HUD.
    - **Replay Suite** (`ReplayWindow`): The log replay, analysis, and timeline tool.
  - **Styling** (`ui/src/styles/`):
    - `theme.css`: Single source of truth for variables (colors, fonts).
    - `common.css`: Shared utilities and base styles.
    - `main.css`: Layout-specific styles for the overlay.
    - `window.css`: Custom window frame and resize handle styles.
  - **Components** (`ui/src/components/`):
    - `StatusBar.tsx`: Top summary metrics (DPS, REP, CAP, NEUT).
    - `CombatBreakdown.tsx`: Collapsible character list with `CharacterCard`.
    - `ReplayControls.tsx`: Timeline slider and playback controls.
    - `LogBrowser.tsx` & `RawLogViewer.tsx`: Replay file selection and debugging.
    - `WindowFrame.tsx`: Custom window decoration system (TitleBar + Resize Handles).

## Design & UX Principles

Follow these guidelines from `conductor/product-guidelines.md`:
- **Technical and Precise**: No "fluff". Prioritize data accuracy and "no-nonsense" aesthetics.
- **Data-Dense and Compact**: Maximize information per pixel. Critical for multiboxers monitoring multiple clients.
  - **Compaction**: Styles must be optimized to work in a small, compact window (e.g., 420x260). Use tight margins and minimal padding.
- **Visual-First Status**: Use color (Green=Good/Dealt, Red=Bad/Received) and trends rather than just raw numbers where possible.
- **Modern Dark Aesthetic**: Avoid harsh "pure black" backgrounds. Use subtle deep-blue/off-black gradients and "Glassmorphism" (transparency + blur + thin highlights) to provide depth.
- **Strict Log Validation**: "Truth-in-data". Do not guess or infer missing data. If the log is ambiguous, flag it or ignore it.

## Tech Stack

- **Core**: Rust (2021 Edition) for performance and safety.
- **Framework**: Tauri (v2) for the application shell and system integration.
- **Frontend**: React 18, TypeScript, Vite 7 (Single-Repo Structure).
- **Async Runtime**: `tokio` for non-blocking I/O (log watching).
- **Serialization**: `serde` / `serde_json`.
- **Time**: `chrono` for EVE log timestamp parsing.
- **Pattern Matching**: `regex` for parsing combat log lines.
  - `tauri-plugin-log`: For internal application logging.

## Testing

- **Strategy**: Unit tests are co-located with the code they test (inline `#[cfg(test)]` modules).
- **Core Tests**:
  - `src/core/parser.rs`: Extensive pattern matching tests for combat log lines.
  - `src/core/analysis.rs`: DPS computation and windowing logic verification.
  - `src/core/state.rs`: Event storage and sorting tests.
- **Dedicated Test Modules**:
  - `src/core/sim_test.rs`: End-to-end simulation of multi-character log streams.
  - `src/core/bench_analysis.rs`: Performance benchmarks for high-volume event processing.
- **Running Tests**: Execute `cargo test` in the root directory to run all backend tests.

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

### 4. Custom Window Decorations
- **Configuration**: Native OS decorations are disabled (`decorations: false` in `tauri.conf.json`).
- **Implementation**: Managed by `WindowFrame.tsx` component.
  - **Dragging**: Uses manual `appWindow.startDragging()` on the custom title bar to ensure reliability across Linux distros.
  - **Resizing**: Implements 8 invisible edge/corner handles that call `appWindow.startResizeDragging()`.
  - **Compacted Header**: Controls (Chars, Settings) are integrated directly into the title bar to save vertical space.
