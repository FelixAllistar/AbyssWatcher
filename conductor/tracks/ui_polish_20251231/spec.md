# Feature Specification: UI Polish & Stabilization

## Objective
Improve the visual quality of the AbyssWatcher overlay and resolve rendering artifacts ("ghosting") caused by window transparency.

## Context
The application uses a transparent Tauri window. When toggling large UI elements like the character selection list, some OS compositors fail to clear the buffer immediately, leaving a "ghost" image of the hidden element. Additionally, the current UI is a functional prototype that lacks the "visually appealing" and "polished" feel required for a high-quality product.

## Requirements

### 1. Rendering Stabilization
- **Ghosting Fix:** Implement a workaround for transparency rendering artifacts. Potential solutions:
    - Forcing a browser repaint by toggling a CSS property or triggering a layout thrash on toggle.
    - Using `will-change: transform` or `transform: translateZ(0)` on the container to promote it to a new compositor layer.
    - Briefly resizing the window by 1px and back (last resort).

### 2. Visual Refinement
- **Typography:** Use a cleaner, more readable font stack.
- **Color Palette:** Refine the "Technical and Precise" aesthetic with better contrast and accent colors.
- **Glassmorphism:** Implement a more modern "glass" effect for the background (backdrop-filter: blur) to improve readability against complex game backgrounds.
- **Animations:** Add subtle transitions for opening/closing the settings panel.

## Success Criteria
- Toggling the character list no longer leaves visual artifacts.
- The UI feels modern, polished, and adheres to the "Data-Dense and Compact" guideline.
- All interactions (hover, toggle) have visual feedback.
