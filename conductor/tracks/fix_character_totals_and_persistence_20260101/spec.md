# Feature Specification: Character List Improvements

## Objective
Improve the character list UI to support persistent rows for tracked characters and display split totals for different metric types (DPS, HPS, etc.).

## Context
Users reported that:
1. "Totals at bottom are wrong": The single total value summed up incompatible metrics (Damage + Reps).
2. "Characters disappear": Characters would vanish from the list when their activity dropped to zero, making it hard to monitor specific fleet members.

## Requirements

### 1. Persistent Rows
- Characters that are explicitly "tracked" (checked in settings) must remain in the list even if their current activity is 0.
- The UI must be aware of the tracking status of each character.

### 2. Split Totals
- The header row for each character must display separate totals for:
  - **DPS** (Damage)
  - **REP** (Repair)
  - **CAP** (Capacitor)
  - **NEUT** (Neutralization)
- If a metric is 0, it can be hidden (except DPS which serves as a primary anchor, or maybe just show what's relevant).

### 3. Tracking Status
- The settings/character selection list must accurately reflect the current tracking status (checkboxes checked/unchecked).

## Success Criteria
- Tracked characters are always visible.
- Character headers show broken-down totals (e.g., "100.0  REP 50.0").
- Toggling a character in settings immediately updates the main list.
