# Tech Stack - AbyssWatcher

## Core Technologies
- **Rust (2021 Edition):** The primary programming language, chosen for its performance, safety, and excellent handling of concurrent tasks like log parsing and UI rendering.
- **eframe / egui:** The immediate-mode GUI framework used for building the always-on-top overlay. `egui` is lightweight and provides a responsive experience.
- **egui_plot:** A specialized library for `egui` to handle the interactive DPS and performance charts.
- **tokio:** The asynchronous runtime used to manage non-blocking I/O operations, specifically for watching and reading multiple log files simultaneously.
- **tao:** A window creation and management library (cross-platform) that allows for the creation of specialized windows like the always-on-top overlay.

## Data & Utilities
- **serde / serde_json:** The standard for serialization and deserialization in Rust, used for managing application state and configuration.
- **chrono:** Used for accurate parsing and manipulation of timestamps within EVE Online combat logs.
- **regex:** Employed for robust pattern matching to extract combat data from raw log strings.
- **lazy_static:** Used for defining global or expensive-to-initialize constants, such as pre-compiled regular expressions.
