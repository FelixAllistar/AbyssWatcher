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
    - `parser.rs`: Right-to-left positional parsing of combat logs.
      - **Right-to-Left Strategy**: Pops known Quality (Hits, etc.), then pops Weapon, then joins remainder as Entity. This correctly handles target names with dashes (e.g., "Habitation Module - Breeding Facility") and optional quality suffixes.
      - **Neut Direction Detection**: Uses HTML color codes to distinguish direction (since text is identical for both parties):
        - `0xffe57f7f` (Reddish) = Incoming (being neuted).
        - `0xff7fffff` (Cyan) = Outgoing (doing the neuting).
    - `log_io.rs`: Efficient log tailing and historical scanning.
    - `analysis.rs`: DPS computation and time-series aggregation.
    - `state.rs`: The `EngineState` that holds combat history.
    - `config.rs`: Settings management and persistence (JSON-based).
    - `coordinator.rs`: Orchestrates log watching and DPS computation loop.
    - `watcher.rs`: Manages multiple `TrackedGamelog` instances.
    - `tracker.rs`: Wraps a single log file with tailer + parser.
    - `replay_engine.rs`: Log replay with merged streams and speed control.
    - `discovery.rs`: Unified log header extraction for Gamelogs and Chatlogs.
    - `chatlog/`: Chat log parsing for Abyss run detection.
      - `parser.rs`: Parses "Channel changed to Local : X" lines.
      - `watcher.rs`: Tails Local chat logs in real-time.
    - `inline_bookmarks.rs`: Inline bookmark system for marking Abyss runs.
      - Appends bookmark lines directly to gamelog files in EVE log format.
      - Types: `RunStart`, `RunEnd`, `RoomStart`, `RoomEnd`, `Highlight`.
    - `alerts/`: Modular alert system for combat notifications.
      - `model.rs`: Alert types (`AlertRuleId`, `AlertSound`, `AlertEvent`, `CharacterRoles`).
      - `triggers.rs`: Trigger evaluation logic for each rule type.
      - `engine.rs`: Orchestrates triggers, manages cooldowns, holds `AlertEngineConfig`.
- **Frontend (Web)**:
  - Located in `ui/`.
  - **Stack**: React 18, TypeScript, Vite 7 (configured at root).
  - **State Management**: React Hooks (`useState`, `useEffect`) avoiding full DOM repaints.
  - **Communication**: Communicates with Rust via `@tauri-apps/api` (Commands & Events).
  - **Types**: `ui/src/types.ts` (Single source of truth for TypeScript interfaces mirroring Rust core).
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
    - `StatusBar.tsx`: Top summary metrics (DPS, REP, CAP, NEUT). Derives totals from `combat_actions_by_character`.
    - `CombatBreakdown.tsx`: Collapsible character list with `CharacterCard`. Each card computes its own totals.
    - `CharacterSelector.tsx`: Dropdown overlay for toggling active log tracking.
    - `SettingsModal.tsx`: Configuration overlay for log paths and analysis windows.
    - `AlertSettings.tsx`: Alert configuration UI (character roles, rule toggles).
    - `Tooltip.tsx`: Accessible, styled tooltip wrapper for buttons (replacing native `title` attributes).
    - `ReplayControls.tsx`: Timeline slider and playback controls.
    - `LogBrowser.tsx` & `RawLogViewer.tsx`: Replay file selection and debugging.
    - `WindowFrame.tsx`: Custom window decoration system (TitleBar + Resize Handles).

## Data Flow Architecture

### Sum-of-Sums Pattern
The UI uses a **single source of truth** for combat metrics:
- **Backend** (`analysis.rs`): Computes `combat_actions_by_character` — a per-character map of all combat actions (DPS, HPS, CAP, NEUT) with their values and direction (incoming/outgoing).
- **Frontend**: Both `StatusBar` and `CombatBreakdown` derive their totals from this same data:
  - `StatusBar` sums ALL character actions to show global totals.
  - `CharacterCard` sums only its own character's actions.
  - This guarantees: **StatusBar totals = Σ(CharacterCard totals)**

### Sliding Window Algorithm
`analysis.rs` uses an efficient O(n) sliding window algorithm:
- Maintains running sums (`char_actions_map`) that are incrementally updated as events enter/exit the time window.
- Handles all 4 event types uniformly: `Damage`, `Repair`, `Capacitor`, `Neut`.
- Both incoming and outgoing events are tracked and properly expired when they leave the window.

### Alert System

The alert system provides audio notifications for critical combat situations.

**Alert Engine Configuration:**
- **Per-Rule Cooldowns**: Customizable durations (default 3s) managed via `cooldown_seconds` in `AlertRuleConfig`.
- **Squashed Queue Architecture**: The engine evaluates ALL enabled rules every tick. It no longer uses a global cooldown or early-exit "first rule wins" logic. This ensures multi-event clusters (e.g., being neuted while taking damage) trigger all relevant alerts.
- **Independent Vorton Filters**: `FriendlyFire` and `LogiTakingDamage` have separate `ignore_vorton` toggles to exclude chain-lightning damage.
- **Data Flow**: `coordinator.tick()` -> `AlertEngine::evaluate()` -> Frontend `alert-triggered` event -> Web Audio Playback (sequential queuing via `rodio::Sink`).

**Alert Rules:**

| Rule ID | Trigger Condition | Default Sound |
|---------|-------------------|---------------|
| `EnvironmentalDamage` | Incoming damage from "Unstable Abyssal Depths" | `boundary.wav` |
| `FriendlyFire` | Tracked char hits tracked char (optional Vorton filter) | `friendly_fire.wav` |
| `LogiTakingDamage` | Logi-designated character receives damage | `logi_attacked.wav` |
| `NeutSensitiveNeuted` | Neut-sensitive character is neuted | `neut.wav` |
| `CapacitorFailure` | Module activation fails due to low cap (`(notify)`) | `capacitor_empty.wav` |
| `LogiNeuted` | Logi-designated character is neuted | `logi_neuted.wav` |

**Configuration:** Stored in `settings.json` under `alert_settings` with role designations (logi, neut-sensitive).

**Embedded Audio:** All alert sounds are compiled directly into the binary using `include_bytes!` (in `src/app.rs`). This ensures the single executable works from any location without external sound files. Source `.wav` files are located in `ui/public/sounds/`.

## Tracking & Session Lifecycle

### Dynamic Log Tracking
Tracking is managed dynamically via shared state in `src/app.rs` without requiring restarts:
1. **State**: `AppState` holds a `tracked_paths` set guarded by a Mutex.
2. **Frontend Action**: Toggling a character in the UI triggers the `toggle_tracking` command.
3. **Coordinator Loop**: The main background loop retrieves the current `tracked_paths` snapshot on every tick.
4. **Hot-Reloading**: The `Coordinator` automatically picks up new paths and drops old ones during its processing cycle, ensuring seamless transitions between characters.

### Replay Window & Cleanup
The Replay system uses a self-cleaning lifecycle to prevent resource leaks:
1. **Process Isolation**: Replay sessions run in a dedicated `tokio::spawn` loop, separate from the main live tracking loop.
2. **Window Events**: `src/app.rs` implements an `on_window_event` handler.
   - When the **Replay Window** is closed (User clicks 'X' or `Command+W`), the `WindowEvent::Destroyed` event fires.
   - This handler explicitly acquires the generic `ReplaySession` lock and drops the active session.
3. **Loop Termination**: The background replay loop checks for session validity on every iteration. If the session has been dropped (due to window close or explicit stop), the loop terminates immediately.

## Design & UX Principles: The "Unified Zero-Container HUD"

AbyssWatcher follows the **Unified Zero-Container HUD** design language, prioritizing raw tactical data over UI "chrome".

- **Unified Zero-Container Aesthetic**: Data readouts must never be trapped in bulky boxes, "badges", or button-like containers. Metrics should float integrated directly into the parent strip (e.g., `char-strip`, `status-bar-strip`) to eliminate visual clutter and focus on "Immediate Cognitive Clarity".
- **Visual Hierarchy (Directional Dominance)**: Outgoing tactical data (Damage dealt, Repairs given) is the primary focus and MUST be visually dominant—larger and bolder (e.g., 12px/900) than secondary incoming tactical data (e.g., 9px).
- **HUD-Grade Dividers**: Use minimalist slash dividers (`/`) with low opacity (e.g., 0.15) to separate paired data points. Avoid heavy vertical bars (`|`) or solid borders that create "boxiness".
- **Technical and Precise**: No "fluff". Prioritize data accuracy and "no-nonsense" professional aesthetics.
- **Data-Dense and Compact**: Maximize information per pixel. Critical for multiboxers monitoring multiple clients simultaneously.
  - **Compaction**: Styles must be optimized to work in a small, compact windows (e.g., 420x260). Use tight margins and minimal padding.
- **Modern Dark Aesthetic**: Use subtle deep-blue/off-black gradients and "Glassmorphism" (transparency + blur + thin highlights) to provide depth without relying on literal "pure black" backgrounds.
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
- **Type Synchronization**: `ui/src/types.ts` must be manually kept in sync with `src/core/model.rs` and `src/core/config.rs`. Always update the TypeScript mirror when changing backend data structures.
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

## CI/CD & Release

The project uses GitHub Actions for automated cross-platform releases.

- **Workflow**: `.github/workflows/release.yml`
- **Triggers**:
  - **Tags**: Pushing a tag starting with `v*` (e.g., `v1.0.0`) triggers a full release build.
  - **Manual**: Can be triggered manually via `workflow_dispatch` with a custom version string.
- **Platforms**:
  - **Windows**: `windows-latest`
  - **Linux**: `ubuntu-22.04` (Builds AppImage/Deb)
  - **macOS**: `macos-latest` (Universal Binary: x86_64 + aarch64)
- **Code Signing**:
  - macOS builds require Apple Developer secrets (`APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, etc.) to be valid. Currently, these secrets must be configured in the repo settings for notarization to work.
