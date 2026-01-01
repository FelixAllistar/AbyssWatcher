# Feature Specification: UI - Capacitor & Neut Metrics

## Objective
Display real-time Capacitor Transfer (CAP) and Energy Neutralization (NEUT) metrics on the main dashboard.

## Context
The backend already parses and aggregates `outgoing_cap` and `outgoing_neut` in the `DpsSample` payload. However, the frontend currently only displays Damage Out (OUT), Repair Out (REP), and Damage In (IN). Adding CAP and NEUT boxes will complete the "Logi/Ewar" support visualization.

## Requirements

### 1. UI Layout
- **Add Boxes:** Add two new metric boxes to the `.dps-summary` row in `index.html`:
    - **CAP:** For remote capacitor transfer.
    - **NEUT:** For energy neutralization and nosferatu.
- **Styling:**
    - **CAP:** Use a distinct color (e.g., Electric Yellow or Light Blue).
    - **NEUT:** Use a distinct color (e.g., Purple or Void Grey).
    - Maintain the existing glassmorphism and typography style.

### 2. Data Wiring
- **Update Logic:** Modify `updateUI` in `main.js` to read `outgoing_cap` and `outgoing_neut` from the event payload and update the new DOM elements.

## Success Criteria
- The dashboard shows 5 boxes: OUT, REP, CAP, NEUT, IN.
- The boxes populate with data when replaying logs containing those events.
- The layout remains responsive and readable.
