# Draft Audit Notes

## Backend Watcher Efficiency
- **Critical Performance Issue:** `compute_dps_series` in `src/core/analysis.rs` has O(N) complexity per call, where N is the total number of recorded events in the session.
    - It iterates from index 0 for every sample calculation, even if the relevant window is at the end of a long list.
    - Called 4 times per second.
    - **Impact:** CPU usage will scale linearly with session duration, potentially causing UI stutter or lag after long play sessions.
    - **Recommendation:** Implement binary search to locate the start/end indices for the analysis window.
- **Watcher Loop:** `scan_gamelogs_dir` is properly conditional (only runs when tracking changes), so it is efficient.
- **Concurrency:** `Mutex<HashSet<PathBuf>>` usage is minimal and correctly scoped. Low deadlock risk.
