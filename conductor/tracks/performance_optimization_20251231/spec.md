# Feature Specification: Performance Optimization - DPS Analysis

## Objective
Optimize the backend DPS calculation logic to eliminate the O(N) complexity per update frame, ensuring the application remains responsive during long gaming sessions (e.g., 4+ hours).

## Context
The current implementation of `compute_dps_series` in `src/core/analysis.rs` iterates through the entire history of combat events (from index 0) every time it runs (4Hz).
- **Current Complexity:** O(N) per frame, where N is total events.
- **Target Complexity:** O(log N) or O(1) per frame (finding window bounds).

## Requirements

### Core Logic Changes
- **Binary Search:** Implement `slice::partition_point` or a custom binary search to locate the `start_index` (events entering the window) and `end_index` (events leaving the window / current time) efficiently.
- **Sorted Guarantee:** Ensure the event vector is strictly sorted by timestamp before performing binary search. (The current `EngineState` already handles sorting, but we must rely on it).

### Success Criteria
- **Passes Existing Tests:** The optimization must not change the mathematical result of the DPS calculation.
- **Benchmarked Improvement:** (Optional) A benchmark or high-load test showing sub-millisecond execution time even with 10,000+ events in history.
