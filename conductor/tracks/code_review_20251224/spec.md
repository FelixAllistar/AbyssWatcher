# Feature Specification: Code Review and Stabilization

## Objective
Review the existing codebase to identify potential issues, clean up implementation details, and stabilize the multi-character support functionality. This track aims to bring the project from a "mostly functioning" state to a robust, well-structured, and verifiable state.

## Context
The project is currently a "Brownfield" Rust application with an `egui` overlay (`src/overlay_egui.rs`) and core logic (`src/core/`). The user reports that it is "mostly functioning correctly" but has "a few issues". Recent changes regarding multi-character UI support were just committed. The goal is to audit these changes and the overall architecture, refactor where necessary, and ensure reliability.

## Requirements

### Functional Requirements
- **Multi-Character Support:** Ensure the application correctly detects, tracks, and displays metrics for multiple concurrent EVE Online clients.
- **Log Parsing Accuracy:** Verify that the log parsing logic (`src/core/parser.rs`, `src/core/log_io.rs`) is robust and handles edge cases without crashing or reporting incorrect data.
- **UI Responsiveness:** Ensure the overlay remains responsive and unobtrusive, even when tracking multiple characters.

### Non-Functional Requirements
- **Code Quality:** Adhere to Rust best practices and the project's style guides.
- **Maintainability:** Refactor complex or duplicated logic into reusable components.
- **Test Coverage:** Increase test coverage for core logic to meet the >80% goal.

## User Constraints
- The user is an EVE Online player and potentially a multiboxer.
- The overlay must be "always-on-top" and "click-through" (when not interacting) to avoid interfering with gameplay.

## Success Criteria
- A comprehensive code audit is completed and documented.
- Identified critical bugs and code smells are resolved.
- Multi-character support is verified through testing.
- The codebase is clean, formatted, and lint-free.
