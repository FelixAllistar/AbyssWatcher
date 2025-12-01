# Development

Your new bare-bones project includes minimal organization with a single `main.rs` file and a few assets.

```
project/
├─ assets/ # Any assets that are used by the app should be placed here
├─ src/
│  ├─ main.rs # main.rs is the entry point to your application and currently contains all components for the app
├─ Cargo.toml # The Cargo.toml file defines the dependencies and feature flags for your project
```

### Running the App

From the project root, build and run the native overlay:

```bash
cargo run
```

This launches the always-on-top `egui` overlay window that reads EVE gamelogs and shows DPS.
