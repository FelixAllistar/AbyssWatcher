# Implementation Plan - Configurable Settings

## Phase 1: Backend Infrastructure [checkpoint: d19a861]
Implement settings persistence and the dialog plugin.

- [x] Task: Add Dialog Plugin 4e223b2
- [x] Task: Implement Settings Manager d19a861
- [x] Task: Conductor - User Manual Verification 'Backend Infrastructure'

## Phase 2: Tauri Commands & Integration [~]
Expose functionality to the frontend.

- [x] Task: Implement Config Commands 4b1c8f1
- [x] Task: Hot-Reload Logic 4b1c8f1

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
