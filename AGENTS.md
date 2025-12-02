# AbyssWatcher DPS Meter

This project is a DPS (Damage Per Second) Meter for EVE Online. It reads combat logs from local files and displays real-time damage tracking data in an overlay window. The overlay is a transparent, always-on-top HUD that shows metrics like health, DPS, and other combat stats without interfering with the game.

## Project Purpose
- **Functionality**: Monitors EVE Online combat logs by parsing local log files.
- **Display**: Shows data in a standalone overlay window for easy viewing during gameplay.
- **Scope**: Only the overlay window is needed; no separate main window or hidden processes. The application runs as a single, visible overlay.

## Architecture Overview

- The crate builds a **single desktop binary** (`abyss-watcher`) with one visible `egui` overlay window (via `eframe`).
- Core logic is separated from UI:
  - `src/core/` holds **domain code** only (no UI framework types): data models, log parsing, log I/O, DPS math, and engine state.
    - `model.rs` – combat events, DPS samples, fight summaries.
    - `parser.rs` – log line → `CombatEvent` (pure string parsing).
    - `log_io.rs` – tailing live log files and reading full historical logs.
    - `analysis.rs` – transforming events into DPS time series and aggregates.
    - `state.rs` – long-lived engine state (`EngineState`) that owns events and exposes computed views.
  - `src/overlay_egui.rs` holds **egui/eframe UI and window management**:
    - Creating the transparent, always-on-top, borderless window.
    - Owning the long-lived `EngineState`, log tailers, and a small HUD view model.
    - Drag-to-move behavior, resize grip, and window state persistence (size/position, opacity, DPS window, tracked characters, gamelog folder).
- `src/main.rs` should remain minimal: wire `core` and the overlay together and call `overlay_egui::run_overlay()`.

## Coding Guidelines for This Project

- Keep **domain logic** (parsing, math, log reading, state) inside `core::*` modules and **never in UI components**.
- The overlay UI (`overlay_egui`) should:
  - Consume already-computed values (e.g. DPS samples, totals) via `EngineState` and small view-model structs.
  - Only perform simple presentation logic, formatting, and user interaction wiring.
- Prefer **owned strings** (`String`) and simple collections (`Vec`, `HashMap`) in the core layer for clarity and safety.
- Use `std::time::Duration` for timestamps and time math; do not mix wall-clock time directly into DPS calculations.
- When adding live log tailing:
 - Prefer a simple polling-based tailer using `LogTailer` in `core::log_io`.
 - Keep background work in `overlay_egui` or a small non-UI helper that owns `EngineState`, but never push parsing/math into the drawing code.
- Preserve the overlay constraints:
 - Transparent/semi-transparent background, always on top, no extra windows.
 - Dragging the custom title bar moves the window; closing the window saves state and exits.

## Gamelog UX Notes

- On startup we automatically scan a default gamelog directory (currently a hardcoded path in the overlay). If files are found, the characters list is populated immediately without showing the folder input.
- If no characters are detected, the UI shows a typed gamelog folder field plus “Scan Gamelog Folder”; users can edit the path and rescan when empty.
- `scan_gamelogs_dir` returns every gamelog it finds, and the overlay keeps them sorted by last-modified timestamp. Only files explicitly toggled to “Track” feed data downstream; the rest remain untracked.
- This lets users start with hundreds of archived files, pick the characters they care about, and still ensure the newest logs appear at the top of the list.
- Character attribution comes from the `Listener:` header; each detected `CharacterInfo` stores the listener name plus path/mtime so the parser and tailer can link events to the right pilot.
- Future work: replace the hardcoded default with OS-specific known gamelog locations (Windows/macOS/Linux) and expose the rescan UI even when characters already exist.

## Combat Parsing & Message Filtering

- All combat logic lives in `core::parser`, `core::analysis`, and `core::model`. UI must not parse or interpret raw log lines.
- Parsing pipeline:
  - Only lines containing `(combat)` are considered for DPS.
  - HTML-like formatting (`<color=...>`, `<font ...>`, `<b>`, `<u>`, `<a ...>`, etc.) is stripped before analysis.
  - Session start is detected via `Session Started: YYYY.MM.DD HH:MM:SS`; timestamps are parsed with `chrono` and normalized to `Duration` since session start.
- Direction classification:
  - **Outgoing damage**: cleaned text contains `" to "` (or `" against "`), e.g. `523 to Starving Damavik - Small Focused Beam Laser II - Penetrates`.
    - `CombatEvent.source` is set to the listener character name.
    - `CombatEvent.target` is the enemy/entity name.
    - `CombatEvent.incoming` is `false`.
  - **Incoming damage**: cleaned text contains `" from "`, e.g. `44 from Guristas Heavy Missile Battery - Inferno Heavy Missile - Hits`.
    - `CombatEvent.source` is the attacking entity (NPC, turret, etc.).
    - `CombatEvent.target` is the listener character name.
    - `CombatEvent.incoming` is `true`.
- Current filters:
  - Lines containing `"remote armor repaired"` are **ignored** for DPS (these will later be handled as separate “logi” metrics).
  - Lines that do not begin with a numeric damage amount after tag stripping (e.g. `Your group of ... misses ... completely`) are **ignored**.
  - Non-combat log categories (`hint`, `notify`, `info`, etc.) are ignored entirely by the parser.
- Data model:
  - `CombatEvent` includes `incoming: bool` so any future logic can branch on direction without re-parsing.
  - `DpsSample` splits into:
    - `outgoing_dps` and `outgoing_by_weapon`/`outgoing_by_target`.
    - `incoming_dps` and `incoming_by_source` (per-attacker).
  - `EngineState::total_damage` sums only outgoing damage by design (incoming is for survivability/pressure, not player DPS).
- Future extensions (logi, neuts, nos):
  - Do **not** overload `CombatEvent.incoming` or the DPS buckets for logi/neuts/nos — add separate flags or categories.
  - When adding these:
    - Extend the parser with additional classifiers (e.g. detect `" remote armor repaired "`, `"energy neutralized"`, `"energy nosferatu"`), but keep the existing outgoing/incoming damage semantics unchanged.
    - Add new per-entity aggregates in `DpsSample` (e.g. `rep_by_target`, `neut_by_source`) rather than mixing them into DPS maps.
  - Message filtering rules should stay explicit and centralized in `core::parser`, so overlay/graph components always consume already-filtered `CombatEvent`s.
