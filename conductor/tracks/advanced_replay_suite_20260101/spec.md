# Specification: Advanced Multi-Character Replay Suite

## 1. Overview
This track implements a robust, standalone "Replay Window" for AbyssWatcher. The goal is to replace the current broken (instant-flash) replay system with a fully interactive "Time Machine" for EVE Online combat logs. This tool will allow developers and users to simulate past encounters as if they were happening live, synchronized across multiple characters, with precise playback controls for debugging parsing logic and analyzing performance.

## 2. Functional Requirements

### 2.1 Replay Window Architecture
- **Standalone Native Window:** The Replay Suite must open in a separate OS window, independent of the main "Always-on-Top" overlay.
- **Self-Contained State:** The replay window must maintain its own simulation state (`ReplayState`) separate from the live application state to prevent data pollution.
- **Visual Parity:** The primary view within this window must use the same UI components (DPS charts, tables, glassmorphism style) as the main application to ensure what is being debugged matches the live experience.

### 2.2 Log Selection & Grouping
- **Default Directory:** The Replay window must default to the `gamelog_dir` currently configured in the main application settings.
- **Integrated File Browser:** Include a "Browse" button and folder path input (identical in functionality to the main window's settings) to allow users to select different log directories for replay.
- **Smart Discovery:** The system must scan the selected directory and parse file headers to extract Character Names.
- **Character-Centric Grouping:** Logs must be grouped by **Character Name** first, then sorted by timestamp (newest first).
- **Selection UI:**
    - Display a list of detected characters.
    - Users can expand a character to see their history of log files.
    - Users can manually select one or more log files (across multiple characters) to combine into a replay session.
    - (Logic for "Session Grouping" by timestamp is removed in favor of manual composition).

### 2.3 Playback Engine
- **Multi-Log Synchronization:** The engine must read multiple log files simultaneously, merging their events into a single chronological stream based on timestamp.
- **Live Simulation:** Events must be emitted at their original relative timing intervals (e.g., if an event happens 2 seconds after the previous one, the engine waits 2 seconds).
- **Controls:**
    - **Play/Pause:** Toggle playback.
    - **Speed Control:** Adjustable playback multiplier (0.5x, 1x, 2x, 5x, 10x).
    - **Manual Stepping:** "Next Tick" or "Next Event" button for frame-by-frame analysis.
    - **Scrubbing:** A timeline slider allowing the user to jump to any point in the session.

### 2.4 Debugging Tools
- **Optional Raw Log View:** A toggleable side panel (implemented as a distinct UI component) that displays the raw text lines as they are processed.
    - This allows correlating a visual change (e.g., DPS spike) with the exact log line that caused it.
    - Designed for easy removal/disabling in future versions.

## 3. Technical Implementation
- **Tauri Windowing:** Use Tauri's API to spawn a secondary window (`label: "replay"`).
- **Backend Logic:** Extend `core::coordinator` or create a new `core::replay_manager` to handle the merged stream logic.
- **Frontend Components:** Refactor existing UI widgets (DPS Meter, Character Card) into reusable components/templates to be shared between `index.html` (Main) and `replay.html` (New Window).

## 4. Acceptance Criteria
- [ ] Clicking "Replay" in the main app opens the new Replay Window.
- [ ] Replay Window defaults to the main app's log directory.
- [ ] "Browse" button in Replay window correctly triggers a folder picker.
- [ ] The Replay Window correctly groups log files by character name and time.
- [ ] Selecting a group and clicking "Start" begins playback.
- [ ] Playback respects relative timing (no instant completion).
- [ ] Playback is synchronized across multiple characters (events happen in the correct order).
- [ ] The "Raw Log View" scrolls in sync with the visualization.
- [ ] Scrubbing the timeline correctly updates the state to that point in time.
