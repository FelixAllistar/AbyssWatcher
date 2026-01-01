# Specification: Unified Combat Action Display

## 1. Overview
The goal of this track is to refactor the backend parsing and frontend display logic to treat all combat actions—Weapons, Remote Repairs (Reps), and Capacitor Warfare (Cap/Neut)—uniformly. Currently, only weapons are displayed under the character name in the UI. This update will ensure that Reps (both Local and Remote) and Cap Warfare modules appear in the same list, providing a complete view of a character's contribution to the battlefield.

## 2. Functional Requirements

### 2.1 Backend Refactoring
- **Unified Data Structure:** Introduce a common abstraction (e.g., a `CombatAction` trait or enum) in the Rust backend to represent any active module effect:
    -   **Damage:** Weapons, Drones.
    -   **Repair:** Shield Boosters (Local/Remote), Armor Repairers (Local/Remote), Hull Repairers (Local/Remote).
    -   **Capacitor:** Energy Neutralizers, Nosferatus, Remote Capacitor Transmitters.
- **Log Parsing:** Ensure the parser correctly identifies and classifies these events from EVE Online gamelogs into the unified structure.
- **Aggregation:** Update the aggregation logic to group all these actions under the respective source character.

### 2.2 Frontend Display
- **Unified List:** Modify the character card in the UI to render a single list of "Combat Actions" instead of just weapons.
- **Contextual Metrics:** Display the appropriate metric based on the action type:
    -   **Damage:** DPS (Damage Per Second).
    -   **Repair:** HPS (Healing/Repair Per Second).
    -   **Capacitor:** GJ/s (GigaJoules Per Second).
- **Visual Consistency:** Ensure Reps and Cap items share the same visual style (icon + text + value) as weapons currently do.

## 3. Non-Functional Requirements
- **Code Clarity:** The backend refactor should simplify the data models, making it easier to add new types of combat actions in the future.
- **Performance:** The unified aggregation should not negatively impact the real-time parsing speed.

## 4. Out of Scope
- Adding support for E-War modules (Webs, Painters, etc.) is not part of this specific track unless they naturally fall into the same parsing logic without extra effort.
