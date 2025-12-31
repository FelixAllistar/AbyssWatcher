use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use tauri::{Emitter, Manager, State};
use abyss_watcher::core::{log_io, model, state, tracker};

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
            let state = app.state::<AppState>();
            // Background task needs its own "pointer" to the state
            // In Tauri v2, we can just use the handle to get it back later, 
            // but for the move closure, we'll get the reference now.
            
            // Start the background log watcher
            tauri::async_runtime::spawn(async move {
                println!("Background log watcher started. Monitoring: {:?}", DEFAULT_GAMELOG_PATH);
                let mut engine = state::EngineState::new();
                let mut trackers: HashMap<PathBuf, tracker::TrackedGamelog> = HashMap::new();
                
                let mut last_event_timestamp: Option<Duration> = None;
                let mut last_event_wallclock: Option<SystemTime> = None;
                let mut current_tracked_set: HashSet<PathBuf> = HashSet::new();

                let dps_window = Duration::from_secs(5);

                loop {
                    // Get the shared state from the handle
                    let active_paths = {
                        let app_state = handle.state::<AppState>();
                        let tracked = app_state.tracked_paths.lock().unwrap();
                        tracked.clone()
                    };

                    // Handle changes in the tracked set
                    if active_paths != current_tracked_set {
                        let removed = current_tracked_set.difference(&active_paths).next().is_some();
                        if removed {
                            engine = state::EngineState::new();
                            last_event_timestamp = None;
                            last_event_wallclock = None;
                        }
                        
                        for path in &active_paths {
                            if !trackers.contains_key(path) {
                                if let Ok(logs) = log_io::scan_gamelogs_dir(DEFAULT_GAMELOG_PATH) {
                                    if let Some(log) = logs.iter().find(|l| &l.path == path) {
                                        if let Ok(tr) = tracker::TrackedGamelog::new(log.character.clone(), path.clone()) {
                                            let msg = format!("Started tracking: {}", log.character);
                                            println!("{}", msg);
                                            let _ = handle.emit("backend-log", msg);
                                            trackers.insert(path.clone(), tr);
                                        }
                                    }
                                }
                            }
                        }
                        current_tracked_set = active_paths;
                    }

                    // 2. Read new events
                    for (path, tracker) in trackers.iter_mut() {
                        if !current_tracked_set.contains(path) { continue; }
                        
                        if let Ok(new_events) = tracker.read_new_events() {
                            if !new_events.is_empty() {
                                let msg = format!("Read {} new events for {}", new_events.len(), tracker.source);
                                println!("{}", msg);
                                let _ = handle.emit("backend-log", msg);
                                let now_wallclock = SystemTime::now();
                                
                                for event in new_events {
                                    last_event_timestamp = Some(match last_event_timestamp {
                                        Some(prev) => std::cmp::max(prev, event.timestamp),
                                        None => event.timestamp,
                                    });
                                    engine.push_event(event);
                                }
                                last_event_wallclock = Some(now_wallclock);
                            }
                        }
                    }

                    // 3. Compute DPS and Emit
                    let end_time = match (last_event_timestamp, last_event_wallclock) {
                        (Some(timestamp), Some(seen_at)) => {
                            if let Ok(elapsed) = SystemTime::now().duration_since(seen_at) {
                                timestamp + elapsed
                            } else {
                                timestamp
                            }
                        }
                        (Some(timestamp), None) => timestamp,
                        (None, _) => Duration::from_secs(0),
                    };

                    let samples = engine.dps_series(dps_window, end_time);
                    if let Some(latest) = samples.last() {
                        let _ = handle.emit("dps-update", latest);
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
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_available_characters, toggle_tracking])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}