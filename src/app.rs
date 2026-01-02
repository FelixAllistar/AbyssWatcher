use std::collections::{HashSet, HashMap};
use std::path::PathBuf;
use std::sync::{Mutex, Arc, RwLock};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tauri::{Emitter, Manager, State, WebviewWindowBuilder, WebviewUrl};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::mpsc;
use crate::core::{log_io, coordinator, config::{ConfigManager, Settings}, replay_engine, state::EngineState};

static REPLAY_SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);

enum LoopCommand {
    Replay,
}

struct ReplaySession {
    controller: replay_engine::ReplayController,
    engine: EngineState,
    id: u64,
}

struct AppState {
    tracked_paths: Mutex<HashSet<PathBuf>>,
    settings: Mutex<Settings>,
    config_manager: ConfigManager,
    loop_tx: mpsc::Sender<LoopCommand>,
    replay: Arc<RwLock<Option<ReplaySession>>>,
}

#[tauri::command]
async fn open_replay_window(app: tauri::AppHandle) -> Result<(), String> {
    println!("Opening replay window...");
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
async fn get_logs_by_character(path: Option<PathBuf>, state: State<'_, AppState>) -> Result<HashMap<String, Vec<log_io::CharacterLog>>, String> {
    let target_dir = if let Some(p) = path {
        p
    } else {
        let settings = state.settings.lock().unwrap();
        settings.gamelog_dir.clone()
    };

    println!("Scanning logs in {:?}", target_dir);
    let logs = log_io::scan_all_logs(&target_dir).map_err(|e| e.to_string())?;
    let groups = log_io::group_logs_by_character(logs);
    println!("Found {} characters with logs.", groups.len());
    Ok(groups)
}

#[tauri::command]
async fn start_replay(logs: Vec<(String, PathBuf)>, state: State<'_, AppState>, app: tauri::AppHandle) -> Result<u64, String> {
    println!("Starting replay with {} logs...", logs.len());
    let controller = replay_engine::ReplayController::new(logs).ok_or("Failed to initialize replay controller")?;
    let duration = controller.session_duration().as_secs();
    
    let session_id = REPLAY_SESSION_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;

    {
        let mut replay = state.replay.write().unwrap();
        let mut session = ReplaySession {
            controller,
            engine: EngineState::new(),
            id: session_id,
        };
        session.controller.set_state(replay_engine::PlaybackState::Playing);
        *replay = Some(session);
    }
    
    let handle = app.clone();
    let replay_state = state.replay.clone();
    
    tauri::async_runtime::spawn(async move {
        println!("Replay loop {} started.", session_id);
        loop {
            // Check if this session is still the active one
            {
                let replay_lock = replay_state.read().unwrap();
                match &*replay_lock {
                    Some(s) if s.id == session_id => {} // Continue
                    _ => {
                        println!("Replay loop {} terminating.", session_id);
                        break;
                    }
                }
            }

            let (events, lines, current_sim_time, progress) = {
                let mut replay_lock = replay_state.write().unwrap();
                if let Some(session) = replay_lock.as_mut() {
                    let (events, lines) = session.controller.tick();
                    for event in &events {
                        session.engine.push_event(event.clone());
                    }
                    (events, lines, session.controller.current_sim_time(), session.controller.relative_progress())
                } else {
                    return;
                }
            };

            // Emit updates
            {
                let mut replay_lock = replay_state.write().unwrap();
                if let Some(session) = replay_lock.as_mut() {
                    let dps_window = Duration::from_secs(5);
                    let samples = session.engine.dps_series(dps_window, current_sim_time);
                    if let Some(sample) = samples.into_iter().last() {
                         if !events.is_empty() {
                             println!("Replay loop {}: Processed {} events. Out DPS: {:.1}", session_id, events.len(), sample.outgoing_dps);
                         }
                         let _ = handle.emit("replay-dps-update", sample);
                    }
                    
                    if !lines.is_empty() {
                        let _ = handle.emit("replay-raw-lines", lines);
                    }
                    
                    let status = serde_json::json!({
                        "current_time": current_sim_time.as_secs(),
                        "progress": progress.as_secs(),
                    });
                    let _ = handle.emit("replay-status", status);
                }
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    Ok(duration)
}

#[tauri::command]
fn seek_replay(offset_secs: u64, state: State<'_, AppState>) -> Result<(), String> {
    let mut replay = state.replay.write().unwrap();
    if let Some(session) = replay.as_mut() {
        session.controller.seek(Duration::from_secs(offset_secs)).map_err(|e| e.to_string())?;
        session.engine = EngineState::new(); 
        println!("Seeked replay to {}s", offset_secs);
    }
    Ok(())
}

#[tauri::command]
fn toggle_replay_pause(state: State<'_, AppState>) {
    let mut replay = state.replay.write().unwrap();
    if let Some(session) = replay.as_mut() {
        let current = session.controller.get_state();
        let next = match current {
            replay_engine::PlaybackState::Playing => replay_engine::PlaybackState::Paused,
            replay_engine::PlaybackState::Paused => replay_engine::PlaybackState::Playing,
        };
        session.controller.set_state(next);
        println!("Replay paused: {:?}", next == replay_engine::PlaybackState::Paused);
    }
}

#[tauri::command]
fn step_replay(state: State<'_, AppState>, app: tauri::AppHandle) -> Result<(), String> {
    let mut replay_lock = state.replay.write().unwrap();
    if let Some(session) = replay_lock.as_mut() {
        session.controller.step(Duration::from_secs(1));
        
        // Process any events in that step
        let (events, lines) = session.controller.tick();
        for event in &events {
            session.engine.push_event(event.clone());
        }
        
        let sim_time = session.controller.current_sim_time();
        let progress = session.controller.relative_progress();
        
        // Manual emit for the step
        let dps_window = Duration::from_secs(5);
        let samples = session.engine.dps_series(dps_window, sim_time);
        if let Some(sample) = samples.into_iter().last() {
             let _ = app.emit("replay-dps-update", sample);
        }
        if !lines.is_empty() {
            let _ = app.emit("replay-raw-lines", lines);
        }
        let status = serde_json::json!({
            "current_time": sim_time.as_secs(),
            "progress": progress.as_secs(),
        });
        let _ = app.emit("replay-status", status);
    }
    Ok(())
}

#[tauri::command]
fn set_replay_speed(speed: f64, state: State<'_, AppState>) {
    let mut replay = state.replay.write().unwrap();
    if let Some(session) = replay.as_mut() {
        session.controller.set_speed(speed);
        println!("Replay speed set to {}", speed);
    }
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
async fn get_available_characters(state: State<'_, AppState>) -> Result<Vec<CharacterUIState>, String> {
    let gamelog_dir = {
        let settings = state.settings.lock().unwrap();
        settings.gamelog_dir.clone()
    };
    
    // Run blocking file I/O on a separate thread
    let logs = tauri::async_runtime::spawn_blocking(move || {
        log_io::scan_gamelogs_dir(&gamelog_dir).unwrap_or_default()
    }).await.map_err(|e| e.to_string())?;
    
    let tracked = state.tracked_paths.lock().unwrap();

    Ok(logs.into_iter().map(|log| {
        let is_tracked = tracked.contains(&log.path);
        CharacterUIState {
            character: log.character,
            path: log.path,
            tracked: is_tracked,
        }
    }).collect())
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
            
            // KDE Always-On-Top "Double-Tap" Fix
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_always_on_top(true);
                let w_clone = window.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    let _ = w_clone.set_always_on_top(true);
                });
            }
            
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
                replay: Arc::new(RwLock::new(None)),
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
                println!("Background log watcher started. Monitoring: {:?}", current_log_dir);

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
                        println!("Log directory changed to {:?}", current_log_dir);
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
            get_logs_by_character,
            start_replay,
            toggle_replay_pause,
            set_replay_speed,
            seek_replay,
            step_replay
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
