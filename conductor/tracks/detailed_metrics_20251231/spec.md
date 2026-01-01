# Feature Specification: Detailed Combat Metrics

## Objective
Enhance the UI to show per-character weapon breakdowns, top targets, and ensure pilot names remain visible. Also perform UI cleanup by removing the debug event logs.

## Context
Currently, the UI only shows total DPS per character. The `DpsSample` emitted by the backend already contains detailed maps for weapons and targets per character, but these are not displayed. Users need this information to identify which drones/weapons are performing and which targets are being prioritized.

## Requirements

### 1. Per-Character Breakdown
- **Weapon Stats:** For each character, show a list of active weapons and their contribution to DPS.
- **Target Stats:** For each character, show the top 3 targets being hit and the DPS dealt to each.
- **Layout:** The breakdown should be compact. Possibly an accordion-style expansion for each pilot or a persistent small-text list.

### 2. Pilot Visibility
- **Persistence:** Ensure that even when many characters are tracked, the pilot names remain legible and don't scroll off-screen if the list grows (though the current UI has a fixed height container).
- **Rework:** Adjust the character list layout to prioritize pilot name visibility.

### 3. Cleanup
- **Remove Event Logs:** Delete the `#logs` div and the `logToScreen` logic from the UI. This was intended for debugging and is now cluttering the refined "glassmorphism" look.

## Success Criteria
- Each pilot row can be expanded to show weapons and targets.
- Weapons and targets are sorted by DPS (descending).
- The debug log area is removed.
- Pilot names are always prominent.
