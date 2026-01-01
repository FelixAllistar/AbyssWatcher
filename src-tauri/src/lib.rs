use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{Emitter, Manager, State, WebviewWindowBuilder, WebviewUrl};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::mpsc;
use abyss_watcher::core::{log_io, coordinator, config::{ConfigManager, Settings}};

enum LoopCommand {
    Replay,
}

struct AppState {
    tracked_paths: Mutex<HashSet<PathBuf>>,
    settings: Mutex<Settings>,
    config_manager: ConfigManager,
    loop_tx: mpsc::Sender<LoopCommand>,
}

#[tauri::command]
async fn open_replay_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("replay") {
        window.set_focus().map_err(|e| e.to_string())?;
    } else {
        WebviewWindowBuilder::new(
            &app,
            "replay",
            WebviewUrl::App("replay.html".into())
        )
        .title("AbyssWatcher - Replay")
        .inner_size(800.0, 600.0)
        .build()
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn get_replay_sessions(path: Option<PathBuf>, state: State<'_, AppState>) -> Result<Vec<log_io::Session>, String> {
    let target_dir = if let Some(p) = path {
        p
    } else {
        let settings = state.settings.lock().unwrap();
        settings.gamelog_dir.clone()
    };

    let logs = log_io::scan_all_logs(&target_dir).map_err(|e| e.to_string())?;
    let sessions = log_io::group_sessions(logs);
    Ok(sessions)
}

#[tauri::command]
async fn replay_logs(state: State<'_, AppState>) -> Result<(), String> {
    state.loop_tx.send(LoopCommand::Replay).await.map_err(|e| e.to_string())
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
    // Run blocking dialog on a separate thread to avoid freezing the UI
    let result = tauri::async_runtime::spawn_blocking(move || {
        app.dialog().file().blocking_pick_folder()
    }).await.map_err(|e| e.to_string())?;

    match result {
        Some(file_path) => file_path.into_path().map(Some).map_err(|e| e.to_string()),
        None => Ok(None)
    }
}

#[derive(serde::Serialize)]
struct CharacterUIState {
    character: String,
    path: PathBuf,
    tracked: bool,
}

#[tauri::command]
fn get_available_characters(state: State<'_, AppState>) -> Vec<CharacterUIState> {
    let settings = state.settings.lock().unwrap();
    let logs = log_io::scan_gamelogs_dir(&settings.gamelog_dir).unwrap_or_default();
    let tracked = state.tracked_paths.lock().unwrap();

    logs.into_iter().map(|log| {
        let is_tracked = tracked.contains(&log.path);
        CharacterUIState {
            character: log.character,
            path: log.path,
            tracked: is_tracked,
        }
    }).collect()
}

#[tauri::command]
fn toggle_tracking(path: PathBuf, state: State<'_, AppState>) {
    let mut tracked = state.tracked_paths.lock().unwrap();
    if tracked.contains(&path) {
        tracked.remove(&path);
    } else {
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
            
            // Create a channel for communicating with the background loop
            let (tx, mut rx) = mpsc::channel(32);

            app.manage(AppState {
                tracked_paths: Mutex::new(HashSet::new()),
                settings: Mutex::new(settings),
                config_manager,
                loop_tx: tx,
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

                loop {
                    // Check for commands from the frontend
                    while let Ok(cmd) = rx.try_recv() {
                        match cmd {
                            LoopCommand::Replay => {
                                coordinator.replay_logs();
                            }
                        }
                    }

                    // Get the shared state
                    let (active_paths, current_settings) = {
                        let app_state = handle.state::<AppState>();
                        let tracked = app_state.tracked_paths.lock().unwrap();
                        let settings = app_state.settings.lock().unwrap();
                        (tracked.clone(), settings.clone())
                    };

                    // Hot-reload: Check if log directory changed
                    if current_settings.gamelog_dir != current_log_dir {
                        current_log_dir = current_settings.gamelog_dir.clone();
                        coordinator = coordinator::Coordinator::new(current_log_dir.clone());
                    }

                    // Hot-reload: DPS Window
                    let dps_window = Duration::from_secs(current_settings.dps_window_seconds);

                    let output = coordinator.tick(&active_paths, dps_window);

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
            pick_gamelog_dir,
            replay_logs,
            open_replay_window,
            get_replay_sessions
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}