# Implementation Plan - Character List Improvements

## Phase 1: Backend Updates
Update `get_available_characters` to return tracking status.

- [x] Task: Define `CharacterUIState` struct in `lib.rs` f66a85b
- [x] Task: Update `get_available_characters` to return tracking info f66a85b

## Phase 2: Frontend Updates
Update `ui/main.js` to use the new data and improve rendering.

- [x] Task: Update `renderSelection` to use `.checked` state f66a85b
- [x] Task: Implement "Persistent Rows" logic (merging tracked chars) f66a85b
- [x] Task: Implement "Split Totals" rendering f66a85b

## Phase 3: Verification
Manual verification of the UI behavior.

- [x] Task: Conductor - User Manual Verification 'UI Improvements' f66a85b
