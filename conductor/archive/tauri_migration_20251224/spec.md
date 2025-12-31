# Feature Specification: Tauri Migration Prototype

## Objective
Create a functional prototype of AbyssWatcher using **Tauri** for the frontend. The goal is to leverage CSS for superior layout control (auto-resizing text, responsive boxes) while retaining the existing Rust core logic for high-performance log parsing. The existing `egui` implementation must be preserved as a fallback.

## Context
The current `egui`-based overlay struggles with responsive layouting at very small window sizes. The user finds manual font sizing and constraint management in immediate mode GUI tedious. Tauri allows using standard Web Technologies (HTML/CSS) which natively handle these responsiveness challenges.

## Requirements

### Functional Requirements
- **Tauri Integration:** Initialize a new Tauri project structure within the existing repo without deleting the old code.
- **Core Logic Reuse:** Refactor `src/core` so it can be shared between the old `egui` app and the new Tauri app (likely by moving it to a library crate or module shared by both).
- **Frontend Prototype:** Create a basic HTML/CSS/JS frontend that:
    - Displays the DPS overlay.
    - Resizes text and elements fluidly based on window size.
    - Supports transparency and "always-on-top" behavior (Tauri features).
- **Data Flow:** Implement a Tauri Command or Event system to stream parsed combat events from Rust to the Frontend.

### Non-Functional Requirements
- **Performance:** Ensure that sending events from Rust to the Webview doesn't introduce significant lag.
- **Preservation:** Do NOT delete `src/main.rs` or `src/overlay_egui.rs`. They should remain buildable (perhaps behind a feature flag or as a separate binary target).

## Success Criteria
- A generic "Hello World" style DPS overlay runs via Tauri.
- Resizing the window scales the text and UI elements correctly using CSS.
- Real (simulated or actual) log data flows from the Rust backend to the HTML frontend.
