# Implementation Plan - Backend Refactor

## Phase 1: Logic Extraction
Isolate the file watching and coordination logic from the Tauri binding.

- [x] Task: Create Watcher Module d8c9895
  - Implement `src/core/watcher.rs` to handle directory scanning and `TrackedGamelog` management.
  - It should provide a method like `poll(&mut self) -> Vec<CombatEvent>`.
- [ ] Task: Create Coordinator Module
  - Implement `src/core/coordinator.rs` that combines `Watcher` and `EngineState`.
  - Implement a `tick()` method that runs the pipeline and returns `Option<DpsSample>` and log messages.
- [ ] Task: Conductor - User Manual Verification 'Logic Extraction'

## Phase 2: Integration
Connect the new modules to the Tauri backend.

- [ ] Task: Wire up Tauri Backend
  - Replace the loop in `src-tauri/src/lib.rs` with the new `Coordinator`.
  - Ensure events are emitted correctly to the frontend.
- [ ] Task: Conductor - User Manual Verification 'Integration'

## Phase 3: Cleanup
Polishing and verifying the new architecture.

- [ ] Task: Error Handling Audit
  - Scan the new modules for `unwrap()` and replace with `Result` or error logging.
- [ ] Task: Verify Refactor
  - Run all tests to ensure no regressions.
- [ ] Task: Conductor - User Manual Verification 'Cleanup'
