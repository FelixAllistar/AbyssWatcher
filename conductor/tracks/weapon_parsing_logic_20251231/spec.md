# Feature Specification: Weapon Parsing Logic & Logi Support

## Objective
Refactor the log parsing logic to correctly identify and categorize logistics (remote repair) events and improve weapon detection accuracy.

## Context
Currently, the `parser.rs` regex logic is focused on damage dealing. It may misinterpret or ignore remote repair events (Logistics) which are critical for support pilots. Additionally, the weapon detection logic might need refinement to handle edge cases where the weapon name is ambiguous or missing in the log line.

## Requirements

### 1. Logistics Support
- **Identify Repair Events:** Update regex to detect lines where a pilot repairs another entity (shield, armor, hull).
- **Categorize:** Create a new event type or flag in `CombatEvent` for repairs.
    - `type: Damage | Repair`
- **Metrics:** Track "HPS" (Healing Per Second) similar to DPS.
    - Update `EngineState` and `Analysis` to aggregate outgoing repair amounts.

### 2. Weapon Parsing Refinement
- **Robustness:** Review current regex to ensure weapon names are captured correctly even with unusual spacing or characters.
- **Unit Tests:** Add test cases for known difficult log lines (e.g., smartbombs, drones, remote repairers).

## Success Criteria
- New unit tests for repair lines pass.
- `CombatEvent` struct supports repair data.
- The backend correctly aggregates and emits repair stats (though UI display might be a separate/future task, we should at least have the data ready).
