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

- There is now a typed gamelog folder input (defaulting to your current path) instead of a native picker; the user edits it and clicks “Scan Gamelog Folder” to refresh.
- `scan_gamelogs_dir` returns every gamelog it finds, and `OverlayViewState` keeps them all sorted by last-modified timestamp. Only files explicitly toggled to “Track” feed data downstream; the rest remain untracked.
- This lets users start with hundreds of archived files, pick the characters they care about, and still ensure the newest logs appear at the top of the list.
- Character attribution comes from the `Listener:` header; each detected `CharacterInfo` stores the listener name plus path/mtime so the parser and tailer can link events to the right pilot.

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

# Async

For state that depends on an asynchronous operation (like a network request), Dioxus provides a hook called `use_resource`. This hook manages the lifecycle of the async task and provides the result to your component.

* The `use_resource` hook takes an `async` closure. It re-runs this closure whenever any signals it depends on (reads) are updated
* The `Resource` object returned can be in several states when read:
1. `None` if the resource is still loading
2. `Some(value)` if the resource has successfully loaded

```rust
let mut dog = use_resource(move || async move {
	// api request
});

match dog() {
	Some(dog_info) => rsx! { Dog { dog_info } },
	None => rsx! { "Loading..." },
}
```

# Routing

All possible routes are defined in a single Rust `enum` that derives `Routable`. Each variant represents a route and is annotated with `#[route("/path")]`. Dynamic Segments can capture parts of the URL path as parameters by using `:name` in the route string. These become fields in the enum variant.

The `Router<Route> {}` component is the entry point that manages rendering the correct component for the current URL.

You can use the `#[layout(NavBar)]` to create a layout shared between pages and place an `Outlet<Route> {}` inside your layout component. The child routes will be rendered in the outlet.

```rust
#[derive(Routable, Clone, PartialEq)]
enum Route {
	#[layout(NavBar)] // This will use NavBar as the layout for all routes
		#[route("/")]
		Home {},
		#[route("/blog/:id")] // Dynamic segment
		BlogPost { id: i32 },
}

#[component]
fn NavBar() -> Element {
	rsx! {
		a { href: "/", "Home" }
		Outlet<Route> {} // Renders Home or BlogPost
	}
}

#[component]
fn App() -> Element {
	rsx! { Router::<Route> {} }
}
```

```

## Server Functions

Use the `#[server]` macro to define an `async` function that will only run on the server. On the server, this macro generates an API endpoint. On the client, it generates a function that makes an HTTP request to that endpoint.

```rust
#[server]
async fn double_server(number: i32) -> Result<i32, ServerFnError> {
	tokio::time::sleep(std::time::Duration::from_secs(1)).await;
	Ok(number * 2)
}
```

## Hydration

Hydration is the process of making a server-rendered HTML page interactive on the client. The server sends the initial HTML, and then the client-side runs, attaches event listeners, and takes control of future rendering.

### Errors
The initial UI rendered by the component on the client must be identical to the UI rendered on the server.

* Use the `use_server_future` hook instead of `use_resource`. It runs the future on the server, serializes the result, and sends it to the client, ensuring the client has the data immediately for its first render.
* Any code that relies on browser-specific APIs (like accessing `localStorage`) must be run *after* hydration. Place this code inside a `use_effect` hook.
