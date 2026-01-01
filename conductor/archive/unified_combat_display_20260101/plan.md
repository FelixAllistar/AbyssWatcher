# Plan: Unified Combat Action Display

## Phase 1: Analysis & Data Modeling
- [x] Task: Analyze `src/core/model.rs` and `src/core/parser.rs` to identify current distinct structures for Weapons, Reps, and Cap effects.
- [x] Task: Design and Implement a unified `CombatAction` enum/struct in `src/core/model.rs` that can represent Damage, Repair (Local/Remote), and Capacitor effects.
- [ ] Task: Conductor - User Manual Verification 'Analysis & Data Modeling' (Protocol in workflow.md)

## Phase 2: Parser & Aggregation Logic
- [x] Task: Refactor `src/core/parser.rs` to normalize parsed events into the new `CombatAction` structure.
    - [x] Sub-task: Ensure Weapons are mapped correctly.
    - [x] Sub-task: Ensure Remote Reps are mapped correctly.
    - [x] Sub-task: Ensure Local Reps (Self-repair) are mapped correctly.
    - [x] Sub-task: Ensure Capacitor effects (Neut/Nos/Transfer) are mapped correctly.
- [x] Task: Update character aggregation logic in `src/core/tracker.rs` (or equivalent) to store a unified list of actions per character.
- [x] Task: Write/Update unit tests in `src/core/parser.rs` and `src/core/model.rs` to verify unified handling.
- [ ] Task: Conductor - User Manual Verification 'Parser & Aggregation Logic' (Protocol in workflow.md)

## Phase 3: Frontend Integration & Display
- [x] Task: Update the JSON serialization of the application state (passed to frontend) to reflect the new unified list structure.
- [x] Task: Update `ui/main.js` (and `index.html` if needed) to render the unified "Combat Actions" list.
    - [x] Sub-task: Implement dynamic label/value display based on action type (DPS for damage, HPS for repair, GJ/s for cap).
    - [x] Sub-task: Ensure visual consistency (icons/styles) across all types.
- [ ] Task: Conductor - User Manual Verification 'Frontend Integration & Display' (Protocol in workflow.md)
