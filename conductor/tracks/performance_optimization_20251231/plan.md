# Implementation Plan - Performance Optimization

## Phase 1: Implementation
Replace linear iteration with binary search.

- [x] Task: Create Stress Test Benchmark 0870d49
  - Create a new test in `src/core/analysis.rs` that generates a large history (e.g., 50k events) and measures execution time of `compute_dps_series`.
  - Ensure this fails or is visibly slow with the current implementation (or at least establishes a baseline).
- [x] Task: Implement Binary Search Windowing 5032acb
  - Refactor `compute_dps_series` to use `partition_point` to find the relevant slice of events.
  - Only iterate over events falling within `[now - window, now]`.
- [ ] Task: Conductor - User Manual Verification 'Implementation' (Protocol in workflow.md)

## Phase 2: Verification [~]
Confirm correctness and performance.

- [~] Task: Verify Correctness with existing tests
  - Run the standard test suite to ensure no regression.
- [ ] Task: Verify Performance with Stress Test
  - Run the stress test again to confirm the speedup.
- [ ] Task: Conductor - User Manual Verification 'Verification' (Protocol in workflow.md)
