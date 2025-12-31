use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{Emitter, Manager, State};
use abyss_watcher::core::{log_io, coordinator};

const DEFAULT_GAMELOG_PATH: &str =
    "/home/felix/Games/eve-online/drive_c/users/felix/My Documents/EVE/logs/Gamelogs";

struct AppState {
    tracked_paths: Mutex<HashSet<PathBuf>>,
}

#[tauri::command]
fn get_available_characters() -> Vec<log_io::CharacterLog> {
    let log_dir = PathBuf::from(DEFAULT_GAMELOG_PATH);
    log_io::scan_gamelogs_dir(&log_dir).unwrap_or_default()
}

#[tauri::command]
fn toggle_tracking(path: PathBuf, state: State<'_, AppState>) {
    let mut tracked = state.tracked_paths.lock().unwrap();
    if tracked.contains(&path) {
        println!("Stopping tracking for: {:?}", path);
        tracked.remove(&path);
    } else {
        println!("Requested tracking for: {:?}", path);
        tracked.insert(path);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            tracked_paths: Mutex::new(HashSet::new()),
        })
        .setup(|app| {
            let handle = app.handle().clone();
            
            // Start the background log watcher
            tauri::async_runtime::spawn(async move {
                println!("Background log watcher started. Monitoring: {:?}", DEFAULT_GAMELOG_PATH);
                let log_dir = PathBuf::from(DEFAULT_GAMELOG_PATH);
                let mut coordinator = coordinator::Coordinator::new(log_dir);
                let dps_window = Duration::from_secs(5);

                loop {
                    // Get the shared state from the handle
                    let active_paths = {
                        let app_state = handle.state::<AppState>();
                        let tracked = app_state.tracked_paths.lock().unwrap();
                        tracked.clone()
                    };

                    let output = coordinator.tick(&active_paths, dps_window);

                    // Emit logs
                    for msg in output.logs {
                        println!("{}", msg);
                        let _ = handle.emit("backend-log", msg);
                    }

                    // Emit DPS
                    if let Some(sample) = output.dps_sample {
                        let _ = handle.emit("dps-update", sample);
                    }

                    tokio::time::sleep(Duration::from_millis(250)).await;
                }
            });

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            app.handle().plugin(tauri_plugin_dialog::init())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_available_characters, toggle_tracking])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}