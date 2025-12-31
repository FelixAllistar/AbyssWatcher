# Tauri Implementation Audit Report
**Date:** 2025-12-31
**Reviewer:** Gemini Agent

## Executive Summary
The Tauri implementation provides a functional and lightweight frontend for the AbyssWatcher. The core architecture is sound, utilizing a push-based event system to update the UI at a steady 4Hz. However, a critical performance bottleneck was identified in the backend analysis logic that will degrade performance over long sessions. Additionally, standard security configurations (CSP) are missing, and the transparent window setting is causing minor visual artifacts.

## Critical Findings (High Priority)

### 1. O(N) Complexity in DPS Analysis
- **Location:** `src/core/analysis.rs` -> `compute_dps_series`
- **Issue:** The function iterates through the entire event history (from index 0) for every single update frame, even though only the last few seconds of data are needed for the moving average.
- **Impact:** As a gaming session lengthens (e.g., 2+ hours), the array size `N` grows. Since this runs 4 times per second, CPU usage will creep up linearly, eventually causing the UI to stutter or the app to become unresponsive.
- **Recommendation:** Implement a binary search or maintain a "window start index" state to only process relevant events.

### 2. Missing Content Security Policy (CSP)
- **Location:** `tauri.conf.json`
- **Issue:** The security configuration is null (`"csp": null`).
- **Impact:** Leaves the application vulnerable to XSS attacks if malicious data (e.g., crafted logs) were ever processed and rendered without escaping.
- **Recommendation:** Add a strict CSP to `tauri.conf.json` allowing only `self` sources.

## Moderate Findings (Medium Priority)

### 3. Visual Artifacts (Ghosting)
- **Location:** UI Rendering
- **Issue:** Toggling the "Characters" list leaves a ghost image on Linux/Windows.
- **Cause:** Known issue with `"transparent": true` windows and certain OS compositors not clearing the buffer immediately when DOM elements are hidden.
- **Recommendation:** 
    - *Short term:* Force a window repaint or CSS layout trashing when toggling.
    - *Long term:* Consider if transparency is strictly necessary or can be simulated.

### 4. Global Tauri API Exposure
- **Location:** `tauri.conf.json` -> `withGlobalTauri: true`
- **Issue:** The entire backend API is exposed to `window.__TAURI__`.
- **Recommendation:** Disable this and use the `@tauri-apps/api` package with a bundler in the future to reduce the attack surface.

## Low Priority / Housekeeping

### 5. Frontend Cleanup
- **Location:** `ui/main.js`
- **Issue:** Event listeners (`listen()`) return a promise resolving to an unlisten function, which is currently ignored.
- **Recommendation:** Store these functions and call them if the app were to support page navigation or component unmounting.

### 6. Unused Assets
- **Location:** `assets/`
- **Issue:** `main.css` and `tailwind.css` exist but are not linked. The app uses inline styles.
- **Recommendation:** Delete unused files to reduce confusion.

## Next Steps
1.  **Refactor `analysis.rs`** to use binary search (Immediate).
2.  **Add CSP** to `tauri.conf.json`.
3.  **Clean up** unused assets.
