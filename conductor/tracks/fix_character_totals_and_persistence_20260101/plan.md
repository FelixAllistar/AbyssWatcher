# Implementation Plan - Character List Improvements

## Phase 1: Backend Updates
Update `get_available_characters` to return tracking status.

- [x] Task: Define `CharacterUIState` struct in `lib.rs` 15edeff
- [x] Task: Update `get_available_characters` to return tracking info 15edeff

## Phase 2: Frontend Updates
Update `ui/main.js` to use the new data and improve rendering.

- [x] Task: Update `renderSelection` to use `.checked` state 15edeff
- [x] Task: Implement "Persistent Rows" logic (merging tracked chars) 15edeff
- [x] Task: Implement "Split Totals" rendering 15edeff

## Phase 3: Verification
Manual verification of the UI behavior.

- [x] Task: Conductor - User Manual Verification 'UI Improvements' 15edeff
