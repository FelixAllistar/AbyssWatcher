# Code Audit Report - Core Logic & UI Layer

**Date:** December 24, 2025
**Auditor:** Gemini

## Overview
This audit covers the core logic and UI layer of the AbyssWatcher application. The goal is to identify potential issues, race conditions, and areas for improvement, particularly concerning multi-character support.

## Findings

### 1. `src/core/parser.rs`
- **Issue:** The `LineParser` relies on `base_time` being set from a "Session Started:" line. If a log file is tailed *after* this line has been written (e.g., attaching to an existing session), `base_time` will be `None`, and `ensure_base_time` will incorrectly set it to the timestamp of the first combat event. This causes all timestamps to be relative to the first combat event rather than the actual session start, potentially skewing absolute time calculations if they are ever needed across multiple logs.
- **Issue:** `extract_timestamp` assumes a fixed format `[ YYYY.MM.DD HH:MM:SS ]`. While standard for EVE, any deviation (or localization differences) could break parsing.
- **Refactor:** The `parse_line` method is quite long and handles multiple responsibilities (stripping tags, parsing direction, splitting damage, splitting entities). Breaking this down into smaller, testable functions would improve maintainability.
- **Data Integrity:** The parser returns `None` for lines it can't handle. It might be beneficial to have a way to report *why* a line was skipped (e.g., "not a combat line" vs "malformed combat line") for debugging purposes.

### 2. `src/core/analysis.rs`
- **Performance:** `compute_dps_series` iterates through `samples` and inside that loop iterates through `events`. While it attempts to be efficient with `start_idx` and `end_idx`, for very long sessions with thousands of events, this could still be computationally expensive, especially since it re-calculates the entire series every time it's called.
- **Multi-Character Complexity:** The `DpsSample` struct has grown significantly to accommodate multi-character data. The `compute_dps_series` function is complex and loops over many maps for every sample.
- **Logic:** The windowing logic (`window_start_millis`) looks correct for a sliding window.

### 3. `src/core/state.rs`
- **Synchronization:** `EngineState` is used within `AbyssWatcherApp`. Since `eframe` runs the update loop on the main thread and `poll_engine` is called synchronously within `draw_dps`, there are no immediate threading issues *unless* file reading becomes asynchronous in a background thread in the future. Currently, `trackers` read synchronously in `poll_engine`, which might cause UI stutters if I/O blocks.

### 4. `src/overlay_egui.rs`
- **I/O in UI Thread:** `poll_engine` is called every frame (throttled to 250ms). It calls `tracker.read_new_events()`, which performs file I/O. If the file system is slow or logs are huge, this will freeze the UI.
- **State Reset:** When `tracked_paths != self.last_tracked_paths`, the entire `engine` is recreated (`EngineState::new()`), and all events from all paths are re-pushed. This is inefficient if only one character is added/removed. It forces a complete re-sort and re-analysis of potentially thousands of events.
- **UI Logic:** `draw_dps` calculates `peak_out` and `peak_in` *every frame* by iterating over the last sample's values. While not huge, it's unnecessary work.
- **Character Colors:** The hardcoded `CHARACTER_COLORS` array has only 6 colors. If a user tracks >6 characters (rare but possible), colors will cycle.
- **Persistence:** `save_persisted_state` is called only on `close_requested`. If the app crashes or is killed, settings (like tracked files) are lost. It might be better to save on change or periodically.

## Recommendations

1.  **Refactor Parser:** Split `parse_line` into smaller sub-functions. Fix `base_time` issue.
2.  **Move I/O off UI Thread:** Use `tokio::spawn` or `std::thread` to run the log tailing and parsing in a background thread. The UI should only read from a shared, thread-safe queue or state.
3.  **Optimize Engine Updates:** Instead of rebuilding the whole engine on track change, enable adding/removing sources dynamically. Or at least, optimize the re-ingestion.
4.  **UI Performance:** Avoid heavy computations (like sorting targets/weapons) inside the immediate mode draw loop if possible, or cache the results in the `AbyssWatcherApp` struct.
5.  **Persistence:** Save state when critical settings change (e.g., toggling a character), not just on exit.

## Conclusion
The application is functional but suffers from potential UI freezes due to synchronous I/O and inefficient state rebuilding. The parser needs robustness improvements. Refactoring to move I/O to a background task is the most impactful architectural change needed.