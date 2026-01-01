# Implementation Plan - Weapon Parsing Logic

## Phase 1: Core Logic Update
Refactor `model.rs` and `parser.rs` to support repairs.

- [x] Task: Update Combat Model 82a74ba
- [x] Task: Refactor Parser Regex 34747eb
- [x] Task: Conductor - User Manual Verification 'Core Logic'

## Phase 2: Analysis Engine Update [~]
Update the aggregation logic to handle repairs.

- [x] Task: Update Analysis Logic fb4e377
- [x] Task: Verify with Tests 7bd7484
- [x] Task: Conductor - User Manual Verification 'Analysis Engine'

- [x] Task: Conductor - User Manual Verification 'Integration' 82342c2

## Phase 3: Integration [checkpoint: 82342c2]
Ensure the new data flows to the frontend (even if not fully displayed yet).

- [x] Task: Verify Frontend Compatibility d23e72c
  - Ensure the frontend doesn't crash receiving the new JSON structure.
  - (Optional) Add a simple "HPS" display if trivial.
- [x] Task: Implement Replay Button 7adc3f9
  - Add `tokio::sync::mpsc` channel to signal the background loop.
  - Implement `replay_logs` command.
  - Add button to Settings UI.
