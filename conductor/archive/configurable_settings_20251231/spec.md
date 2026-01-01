# Feature Specification: Configurable Settings & File Browser

## Objective
Allow users to configure the application's behavior and persistence paths via the UI, specifically enabling the selection of the EVE Online gamelogs directory using a native file browser.

## Context
Currently, the `DEFAULT_GAMELOG_PATH` is hardcoded in the Rust binary. This limits the application to a specific user/machine. Additionally, the DPS analysis window (5 seconds) cannot be changed by the user.

## Requirements

### 1. Backend (Rust)
- **Settings Model:** Create a `Settings` struct containing:
    - `gamelog_dir`: PathBuf
    - `dps_window_seconds`: u64 (default: 5)
- **Persistence:** Implement logic to load settings from a JSON file on startup and save them when changed.
    - Location: Standard AppConfig directory (e.g., `~/.config/com.abysswatcher.app/settings.json`).
- **Tauri Plugin:** Integrate `tauri-plugin-dialog` to open a native directory selection dialog.
- **State Management:** Update the `Coordinator` to accept dynamic configuration changes.

### 2. Frontend (UI)
- **Settings Panel:** Create a modal or dedicated view for settings (distinct from the Character list).
- **Interactions:**
    - "Browse..." button triggering the native dialog.
    - Input field for DPS window duration.
    - "Save" button to persist changes.
- **Feedback:** UI should update immediately upon saving.

## Success Criteria
- User can click "Browse" and select any directory on their system.
- The selected path is saved and persists across app restarts.
- Changing the path immediately updates the backend watcher to scan the new directory.
