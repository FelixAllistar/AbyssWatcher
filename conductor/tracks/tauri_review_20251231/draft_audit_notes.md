# Draft Audit Notes

## Backend Watcher Efficiency
- **Critical Performance Issue:** `compute_dps_series` in `src/core/analysis.rs` has O(N) complexity per call, where N is the total number of recorded events in the session.
    - It iterates from index 0 for every sample calculation, even if the relevant window is at the end of a long list.
    - Called 4 times per second.
    - **Impact:** CPU usage will scale linearly with session duration, potentially causing UI stutter or lag after long play sessions.
    - **Recommendation:** Implement binary search to locate the start/end indices for the analysis window.
- **Watcher Loop:** `scan_gamelogs_dir` is properly conditional (only runs when tracking changes), so it is efficient.
- **Concurrency:** `Mutex<HashSet<PathBuf>>` usage is minimal and correctly scoped. Low deadlock risk.

## Event Emission Logic
- **Frequency:** `dps-update` emits at 4Hz (250ms interval). This is well within Tauri's IPC capacity.
- **Payload:** The `DpsSample` payload can grow with the number of entities, but at 4Hz, it is unlikely to cause saturation unless the battle is massive (thousands of entities).
- **Correctness:** Continuous emission is required to show DPS decay when no new events are occurring.
- **Conclusion:** Emission logic is safe and efficient enough for the current scope.

## Frontend Audit
- **Unused Assets:** `assets/main.css` and `assets/tailwind.css` are present in the project but NOT linked in `ui/index.html`. The app uses inline styles.
- **Cleanup:** `main.js` sets up event listeners but does not capture the returned `unlisten` functions. This is acceptable for a single-page app that doesn't unmount components, but bad practice generally.
- **Responsiveness:** CSS uses `clamp()` and flexbox correctly for window resizing.
