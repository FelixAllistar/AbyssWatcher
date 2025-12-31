# Implementation Plan - Configurable Settings

## Phase 1: Backend Infrastructure
Implement settings persistence and the dialog plugin.

- [x] Task: Add Dialog Plugin 4e223b2
- [x] Task: Implement Settings Manager d19a861
  - Create `src/core/config.rs` with `Settings` struct and `load`/`save` methods.
  - Integrate persistence in `src-tauri/src/lib.rs` (load on startup).
- [ ] Task: Conductor - User Manual Verification 'Backend Infrastructure'

## Phase 2: Tauri Commands & Integration
Expose functionality to the frontend.

- [ ] Task: Implement Config Commands
  - Add Tauri commands: `get_settings`, `save_settings`, `pick_gamelog_dir`.
  - `pick_gamelog_dir` should use the dialog plugin to return a path string.
- [ ] Task: Hot-Reload Logic
  - Ensure that saving settings updates the running `Coordinator` (changes the watched directory).
- [ ] Task: Conductor - User Manual Verification 'Tauri Commands'

## Phase 3: Frontend UI
Build the settings interface.

- [ ] Task: Build Settings UI
  - Add a "Settings" (Gear icon) button to the header.
  - Create a modal/panel overlay in `index.html`.
  - Implement the "Browse" button logic in `main.js`.
- [ ] Task: Connect UI to Backend
  - Load settings on app init.
  - Call `save_settings` when the user confirms changes.
- [ ] Task: Conductor - User Manual Verification 'Frontend UI'
