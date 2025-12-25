# Implementation Plan - Tauri Migration Prototype

## Phase 1: Setup and Scaffolding
This phase focuses on getting the build environment ready and establishing the project structure.

- [x] Task: Restructure Project for Dual Targets
  - Move `src/core` to a shared library module (or ensure it's accessible to both binaries).
  - Rename current `main.rs` to `bin/egui_app.rs` (or similar) or define multiple `[[bin]]` targets in `Cargo.toml`.
- [x] Task: Initialize Tauri
  - Use `cargo tauri init` (or manual setup) to create the `src-tauri` directory.
  - Configure `tauri.conf.json` for "always on top", transparency, and window decorations.
  - Setup a minimal frontend (vanilla HTML/JS or a lightweight framework like Preact/React).
- [x] Task: Conductor - User Manual Verification 'Setup and Scaffolding' (Protocol in workflow.md) [checkpoint: 2dcda02]

## Phase 2: Connecting Backend to Frontend
This phase bridges the gap between the Rust log parser and the Webview.

- [ ] Task: Create Tauri Commands/Events
  - Implement a Tauri command to start the log watcher.
  - Use `tauri::Window::emit` to push `DpsSample` updates to the frontend.
- [ ] Task: Wire up the Frontend
  - Write JavaScript to listen for the DPS events.
  - Update the DOM with the received data.
- [ ] Task: Conductor - User Manual Verification 'Connecting Backend to Frontend' (Protocol in workflow.md)

## Phase 3: UI Prototyping (The "CSS" Part)
This phase addresses the core reason for the migration: responsive layout.

- [ ] Task: Implement Responsive DPS Layout
  - Create a CSS Grid/Flexbox layout for the overlay.
  - Use viewport units (`vw`, `vh`) or `clamp()` for font sizing.
  - Verify that shrinking the window "squishes" the UI gracefully without breaking.
- [ ] Task: Conductor - User Manual Verification 'UI Prototyping' (Protocol in workflow.md)
