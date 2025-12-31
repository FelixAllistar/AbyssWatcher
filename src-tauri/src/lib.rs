use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use abyss_watcher::core::{log_io, coordinator, config::{ConfigManager, Settings}};

struct AppState {
    tracked_paths: Mutex<HashSet<PathBuf>>,
    settings: Mutex<Settings>,
    config_manager: ConfigManager,
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn save_settings(settings: Settings, state: State<'_, AppState>) -> Result<(), String> {
    let mut current = state.settings.lock().unwrap();
    *current = settings.clone();
    state.config_manager.save(&settings).map_err(|e| e.to_string())
}

#[tauri::command]
async fn pick_gamelog_dir(app: tauri::AppHandle) -> Result<Option<PathBuf>, String> {
    // Async dialog to avoid freezing the main thread
    match app.dialog().file().pick_folder().await {
        Some(file_path) => file_path.into_path().map(Some).map_err(|e| e.to_string()),
        None => Ok(None),
    }
}

#[tauri::command]
fn get_available_characters(state: State<'_, AppState>) -> Vec<log_io::CharacterLog> {
    let settings = state.settings.lock().unwrap();
    log_io::scan_gamelogs_dir(&settings.gamelog_dir).unwrap_or_default()
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
        .setup(|app| {
            let handle = app.handle().clone();
            
            // Initialize Config
            let config_dir = app.path().app_config_dir().unwrap_or(PathBuf::from("."));
            let config_manager = ConfigManager::new(config_dir);
            let settings = config_manager.load();
            let initial_settings = settings.clone();
            
            app.manage(AppState {
                tracked_paths: Mutex::new(HashSet::new()),
                settings: Mutex::new(settings),
                config_manager,
            });

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            app.handle().plugin(tauri_plugin_dialog::init())?;

            // Start the background log watcher
            tauri::async_runtime::spawn(async move {
                let mut current_log_dir = initial_settings.gamelog_dir.clone();
                let mut coordinator = coordinator::Coordinator::new(current_log_dir.clone());
                println!("Background watcher started. Monitoring: {:?}", current_log_dir);

                loop {
                    // Get the shared state
                    let (active_paths, current_settings) = {
                        let app_state = handle.state::<AppState>();
                        let tracked = app_state.tracked_paths.lock().unwrap();
                        let settings = app_state.settings.lock().unwrap();
                        (tracked.clone(), settings.clone())
                    };

                    // Hot-reload: Check if log directory changed
                    if current_settings.gamelog_dir != current_log_dir {
                        println!("Log directory changed to: {:?}", current_settings.gamelog_dir);
                        current_log_dir = current_settings.gamelog_dir.clone();
                        // Recreate coordinator with new path
                        // Note: This resets the DPS history, which is acceptable when changing logs
                        coordinator = coordinator::Coordinator::new(current_log_dir.clone());
                    }

                    // Hot-reload: DPS Window
                    let dps_window = Duration::from_secs(current_settings.dps_window_seconds);

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

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_available_characters, 
            toggle_tracking,
            get_settings,
            save_settings,
            pick_gamelog_dir
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}