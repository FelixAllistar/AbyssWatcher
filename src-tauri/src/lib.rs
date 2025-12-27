use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tauri::Emitter;
use abyss_watcher::core::{log_io, model, state, tracker};

const DEFAULT_GAMELOG_PATH: &str =
    "/home/felix/Games/eve-online/drive_c/users/felix/My Documents/EVE/logs/Gamelogs";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            
            // Start the background log watcher
            tauri::async_runtime::spawn(async move {
                println!("Background log watcher started. Monitoring: {:?}", DEFAULT_GAMELOG_PATH);
                let mut engine = state::EngineState::new();
                let mut trackers: HashMap<PathBuf, tracker::TrackedGamelog> = HashMap::new();
                let mut events_by_path: HashMap<PathBuf, Vec<model::CombatEvent>> = HashMap::new();
                
                let mut last_event_timestamp: Option<Duration> = None;
                let mut last_event_wallclock: Option<SystemTime> = None;
                
                let log_dir = PathBuf::from(DEFAULT_GAMELOG_PATH);
                let dps_window = Duration::from_secs(5);

                loop {
                    // 1. Scan for new logs (simple version for prototype: scan every loop)
                    if let Ok(logs) = log_io::scan_gamelogs_dir(&log_dir) {
                        // println!("Scanned {} logs in {:?}", logs.len(), log_dir); // excessive logging
                        for log in logs {
                            if !trackers.contains_key(&log.path) {
                                println!("Found new log for character: {}", log.character);
                                if let Ok(tr) = tracker::TrackedGamelog::new(log.character.clone(), log.path.clone()) {
                                    trackers.insert(log.path.clone(), tr);
                                    events_by_path.entry(log.path.clone()).or_default();
                                }
                            }
                        }
                    } else {
                        println!("Failed to scan log dir: {:?}", log_dir);
                    }

                    // 2. Read new events
                    for (path, tracker) in trackers.iter_mut() {
                        if let Ok(new_events) = tracker.read_new_events() {
                            if !new_events.is_empty() {
                                println!("Read {} new events for {}", new_events.len(), tracker.source);
                                let now_wallclock = SystemTime::now();
                                let entry_events = events_by_path.entry(path.clone()).or_default();
                                
                                for event in new_events {
                                    entry_events.push(event.clone());
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
                        // Only emit if there's actual activity to report?
                        // For now, emit every time to ensure UI updates even if DPS drops to 0
                        // println!("Emitting DPS update: Out={}", latest.outgoing_dps);
                        if let Err(e) = handle.emit("dps-update", latest) {
                             println!("Failed to emit event: {:?}", e);
                        }
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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
