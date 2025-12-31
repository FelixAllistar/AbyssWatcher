# Tech Stack - AbyssWatcher

## Core Technologies
- **Rust (2021 Edition):** The primary programming language, chosen for its performance, safety, and excellent handling of concurrent tasks like log parsing and UI rendering.
- **Tauri:** The primary application framework, used to build a secure and performant desktop overlay using Web Technologies (HTML/CSS/JS) for the frontend and Rust for the backend.
- **HTML5 / CSS3 / JavaScript:** Used for the frontend UI, leveraging modern layout engines (Flexbox, Grid) for fluid responsiveness and auto-scaling text.
- **tokio:** The asynchronous runtime used to manage non-blocking I/O operations, specifically for watching and reading multiple log files simultaneously.

## Data & Utilities
- **serde / serde_json:** The standard for serialization and deserialization in Rust, used for managing application state and configuration.
- **chrono:** Used for accurate parsing and manipulation of timestamps within EVE Online combat logs.
- **regex:** Employed for robust pattern matching to extract combat data from raw log strings.
- **lazy_static:** Used for defining global or expensive-to-initialize constants, such as pre-compiled regular expressions.