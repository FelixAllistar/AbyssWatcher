# Refactor Specification: Backend Architecture

## Objective
Refactor the backend codebase to eliminate the monolithic "God function" in `src-tauri/src/lib.rs` by separating concerns into distinct, testable modules.

## Context
Currently, `src-tauri/src/lib.rs` contains a single `async move` closure that handles:
1.  Polling the file system for log changes.
2.  Managing a collection of `TrackedGamelog` instances.
3.  Parsing raw lines into `CombatEvent`s.
4.  Feeding events into `EngineState`.
5.  Triggering DPS analysis.
6.  Emitting Tauri events.

This coupling makes unit testing the orchestration logic impossible and increases the risk of regressions.

## Requirements

### 1. New Modules
- **`src/core/watcher.rs`**: A dedicated component responsible for scanning the directory and managing the lifecycle of `TrackedGamelog` instances. It should yield a stream or iterator of new `CombatEvent`s.
- **`src/core/coordinator.rs`** (or similar): A high-level struct that owns the `Watcher` and the `EngineState`. It exposes a simple `tick()` or `run()` method that performs one update cycle (read logs -> update engine -> compute stats).

### 2. Error Handling
- Remove `unwrap()` calls from the runtime loop. File IO errors should be logged but should not crash the background thread.

### 3. Tauri Integration
- `src-tauri/src/lib.rs` should act strictly as the "Interface Layer".
- It should initialize the Coordinator.
- It should map the Coordinator's outputs (DPS stats, logs) to Tauri events (`emit`).

## Success Criteria
- `src-tauri/src/lib.rs` logic loop is reduced to high-level function calls.
- Logic is testable without running the full Tauri application.
- All existing tests pass.
