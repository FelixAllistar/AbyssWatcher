# Plan: Advanced Multi-Character Replay Suite

## Phase 1: Infrastructure & Componentization
- [x] Task: Refactor `ui/main.js` and `ui/index.html` to separate visual components (DPS bars, tables) from application-specific logic. 8459239
- [x] Task: Create `ui/replay.html` and `ui/replay.js` as the entry point for the new window. cfdd8a1
- [x] Task: Implement a Tauri command `open_replay_window` that spawns the secondary window. cfdd8a1
- [x] Task: Ensure the Replay window can fetch the current `gamelog_dir` from the backend settings. cfdd8a1
- [ ] Task: Conductor - User Manual Verification 'Infrastructure & Componentization' (Protocol in workflow.md) [checkpoint: adf52fe]

## Phase 2: Log Discovery & Session Grouping
- [x] Task: Implement `core::log_io::discover_sessions` to scan a directory and extract character names from log headers. b150f2f
- [x] Task: Implement grouping logic to cluster logs by timestamp (e.g., logs starting within 10 minutes of each other). b150f2f
- [x] Task: Expose session discovery via a Tauri command `get_replay_sessions`. b150f2f
- [x] Task: Build the "Session Picker" UI in the Replay window (List of runs -> List of character logs). b150f2f
- [ ] Task: Conductor - User Manual Verification 'Log Discovery & Session Grouping' (Protocol in workflow.md)

## Phase 3: Synchronized Playback Engine
- [ ] Task: Create `core::replay_engine::MergedStream` to read multiple log files and yield lines in chronological order.
- [ ] Task: Implement `core::replay_engine::ReplayController` to manage playback state (Play/Pause, Speed Multiplier, Manual Ticking).
- [ ] Task: Implement the "Live Simulation" timer logic in the backend to push updates to the frontend at the correct relative intervals.
- [ ] Task: Connect the Replay Engine to the existing `core::analysis` logic to produce DPS/HPS metrics.
- [ ] Task: Conductor - User Manual Verification 'Synchronized Playback Engine' (Protocol in workflow.md)

## Phase 4: Advanced Controls & Debugging
- [ ] Task: Implement the Playback Dashboard in the frontend (Timeline Slider, Speed Selector, Play/Pause).
- [ ] Task: Implement "Timeline Scrubbing" by re-processing the log stream up to the selected timestamp.
- [ ] Task: Implement the toggleable "Raw Log Debug Panel" component.
- [ ] Task: Add "Step Forward" button to advance the simulation by one event or one second.
- [ ] Task: Conductor - User Manual Verification 'Advanced Controls & Debugging' (Protocol in workflow.md)
