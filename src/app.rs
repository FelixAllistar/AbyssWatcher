use std::collections::{HashSet, HashMap};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::{Mutex, Arc, RwLock};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tauri::{Emitter, Manager, State, WebviewWindowBuilder, WebviewUrl};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::mpsc;
use crate::core::{
    log_io, coordinator, 
    config::{ConfigManager, Settings}, 
    replay_engine, 
    state::EngineState,
    discovery,
};

static REPLAY_SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generate a stable character ID from the character name.
/// This is used when gamelogs don't have character IDs in filenames.
fn char_id_from_name(name: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    name.hash(&mut hasher);
    hasher.finish()
}



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
    let target_dir = path.unwrap_or_else(|| {
        state.settings.lock().unwrap().gamelog_dir.clone()
    });

    println!("Scanning logs in {:?}", target_dir);
    let logs = log_io::scan_all_logs(&target_dir).map_err(|e| e.to_string())?;
    let groups = log_io::group_logs_by_character(logs);
    println!("Found {} characters with logs.", groups.len());
    Ok(groups)
}

#[derive(serde::Serialize)]
struct ReplaySessionInfo {
    duration: u64,
    start_time: u64,
}

#[tauri::command]
async fn start_replay(logs: Vec<(String, PathBuf)>, state: State<'_, AppState>, app: tauri::AppHandle) -> Result<ReplaySessionInfo, String> {
    println!("Starting replay with {} logs...", logs.len());
    let controller = replay_engine::ReplayController::new(logs).ok_or("Failed to initialize replay controller")?;
    let duration = controller.session_duration().as_secs();
    let start_time = controller.start_time().as_secs();
    
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

    Ok(ReplaySessionInfo {
        duration,
        start_time,
    })
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

// ============================================
// Inline Bookmark Commands
// ============================================

/// Response for room marker toggle
#[derive(serde::Serialize)]
struct SimpleRoomResponse {
    room_open: bool,
}

/// Bookmark for frontend
#[derive(serde::Serialize)]
struct SimpleBookmarkResponse {
    timestamp_secs: u64,
    bookmark_type: String,
    label: Option<String>,
}

#[tauri::command]
async fn create_highlight_bookmark(
    gamelog_path: PathBuf,
    label: Option<String>,
) -> Result<(), String> {
    use crate::core::inline_bookmarks;
    inline_bookmarks::add_highlight(&gamelog_path, label.as_deref())
        .map_err(|e| e.to_string())?;
    println!("Added HIGHLIGHT bookmark to {:?}", gamelog_path);
    Ok(())
}

#[tauri::command]
async fn toggle_room_marker(
    gamelog_path: PathBuf,
    currently_in_room: bool,
) -> Result<SimpleRoomResponse, String> {
    use crate::core::inline_bookmarks;
    
    if currently_in_room {
        // End room
        inline_bookmarks::add_room_end(&gamelog_path).map_err(|e| e.to_string())?;
        println!("Added ROOM_END to {:?}", gamelog_path);
        Ok(SimpleRoomResponse { room_open: false })
    } else {
        // Start room
        inline_bookmarks::add_room_start(&gamelog_path).map_err(|e| e.to_string())?;
        println!("Added ROOM_START to {:?}", gamelog_path);
        Ok(SimpleRoomResponse { room_open: true })
    }
}

#[tauri::command]
async fn get_session_bookmarks(
    gamelog_path: PathBuf,
) -> Result<Vec<SimpleBookmarkResponse>, String> {
    // Read gamelog and parse bookmark lines
    use std::fs;
    use std::io::{BufRead, BufReader};
    
    let file = fs::File::open(&gamelog_path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    
    let mut bookmarks = Vec::new();
    
    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        if let Some(bm) = parse_bookmark_line(&line) {
            bookmarks.push(bm);
        }
    }
    
    Ok(bookmarks)
}

/// Parse a bookmark line like: [ 2026.01.04 03:56:49 ] (bookmark) TYPE: label
fn parse_bookmark_line(line: &str) -> Option<SimpleBookmarkResponse> {
    if !line.contains("(bookmark)") {
        return None;
    }
    
    // Extract timestamp from [ YYYY.MM.DD HH:MM:SS ]
    let timestamp_start = line.find('[')? + 1;
    let timestamp_end = line.find(']')?;
    let timestamp_str = line[timestamp_start..timestamp_end].trim();
    
    // Parse to epoch seconds
    use chrono::NaiveDateTime;
    let naive = NaiveDateTime::parse_from_str(timestamp_str, "%Y.%m.%d %H:%M:%S").ok()?;
    let timestamp_secs = naive.and_utc().timestamp() as u64;
    
    // Extract type and optional label after (bookmark)
    let after_bookmark = line.split("(bookmark)").nth(1)?.trim();
    let (bookmark_type, label) = if let Some(colon_pos) = after_bookmark.find(':') {
        let btype = after_bookmark[..colon_pos].trim().to_string();
        let lbl = after_bookmark[colon_pos + 1..].trim().to_string();
        (btype, Some(lbl))
    } else {
        (after_bookmark.to_string(), None)
    };
    
    Some(SimpleBookmarkResponse {
        timestamp_secs,
        bookmark_type,
        label,
    })
}

#[tauri::command]
async fn detect_filaments(
    gamelog_path: PathBuf,
) -> Result<(), String> {
    println!("detect_filaments called for {:?}", gamelog_path);

    // 1. Extract Header to identify character and time
    let header = discovery::extract_header(&gamelog_path, discovery::LogType::Gamelog)
        .map_err(|e| e.to_string())?
        .ok_or("Failed to parse gamelog header")?;

    println!("Detected character: {} (ID: {:?})", header.character, header.character_id);

    // 2. Find matching Chatlog
    // We try to find the Local chatlog that corresponds to this session.
    // Since session times might slightly differ, we look for a chatlog that started 
    // around the same time or is the most recent one before the gamelog end.
    // For simplicity, we look for a chatlog with the same character name/ID 
    // and picking the one closest in time.
    
    let chatlog_dir = discovery::derive_chatlog_dir(header.path.parent().unwrap());
    println!("Looking for chatlogs in {:?}", chatlog_dir);
    
    // We scan all local chatlogs for this character
    let mut relevant_logs = discovery::scan_logs_dir(&chatlog_dir, Some("Local"), discovery::LogType::Chatlog)
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|h| h.character == header.character)
        .collect::<Vec<_>>();

    // Sort by start time ascending
    relevant_logs.sort_by_key(|h| h.session_start);

    // Find the chatlog that covers the gamelog session
    // Gamelog start: T_g
    // Chatlog start: T_c
    // We want T_c roughly equal to T_g (within reasonable tolerance)
    // Or just the one that was continuously active?
    // Let's pick the one explicitly starting +/- 2 minutes of gamelog, 
    // or if that fails, the one immediately preceding it.
    
    let gamelog_start = header.session_start;
    
    let best_match = relevant_logs.into_iter().rev().find(|h| {
        // Chatlog must have started BEFORE or shortly AFTER gamelog
        // Let's use a loose window: Chatlog started within 24h before gamelog?
        // EVE daily downtime restarts logs.
        // Usually they start at almost exact same second if logged in.
        if let Ok(diff) = gamelog_start.duration_since(h.session_start) {
            // Chatlog started before Gamelog
            diff.as_secs() < 86400 // Less than 24h diff
        } else if let Ok(diff) = h.session_start.duration_since(gamelog_start) {
             // Chatlog started after Gamelog (maybe crash recovery?)
             diff.as_secs() < 300 // 5 min tolerance
        } else {
            false
        }
    });

    let chatlog_path = best_match.ok_or("No matching Local chatlog found for this session")?.path;
    println!("Found matching chatlog: {:?}", chatlog_path);

    // 3. Scan Chatlog for Abyss Runs
    // 3. Scan Chatlog for Abyss Runs
    
    // Check if file exists
    if !chatlog_path.exists() {
        return Err(format!("Chatlog path matches but file missing: {:?}", chatlog_path));
    }

    println!("Reading chatlog content...");
    // Rust read_to_string expects UTF-8. discovery.rs handles reading headers with encoding check.
    // We need to robustly read the whole file. 
    // Let's assume standard UTF-8/Ascii for now or use the BOM check from discovery if needed.
    // EVE logs are often UTF-16LE. We should reuse the read logic or be careful.
    
    // Let's try reading as UTF-8 first, if valid.
    // If not, we might need a helper to read diverse encodings.
    // For now, let's look at how discovery reads it.
    // discovery::extract_header handles UTF-16LE. We should probably expose that read function.
    // OR just try to run `strings` equivalent by filtering valid chars?
    // Let's do a quick encoding check similar to discovery.
    
    let clean_content = {
        use std::io::Read;
        let mut f = std::fs::File::open(&chatlog_path).map_err(|e| e.to_string())?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
        
        if buffer.len() >= 2 && buffer[0] == 0xFF && buffer[1] == 0xFE {
            // UTF-16LE
            let u16_units: Vec<u16> = buffer[2..]
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            String::from_utf16_lossy(&u16_units)
        } else {
            // Assume UTF-8 / ASCII
            String::from_utf8_lossy(&buffer).to_string()
        }
    };

    use crate::core::chatlog::parser::{ChatlogParser, detect_abyss_runs};
    let parser = ChatlogParser::new();
    let changes = parser.parse_lines(&clean_content.lines().map(String::from).collect::<Vec<_>>());
    let runs = detect_abyss_runs(&changes);

    println!("Detected {} Abyss runs in chatlog.", runs.len());

    // 4. Append Match Bookmarks to Gamelog
    // We only append if they don't already exist to avoid duplicates?
    // Actually, inline_bookmarks just appends. Duplicates might be annoying.
    // Ideally we check if they exist.
    // For now, let's just append. The user pressed "Detect", so they want them.
    
    use crate::core::inline_bookmarks::{append_bookmark, BookmarkType};
    use chrono::{DateTime, Utc};

    // Helper to format timestamp exactly as it appears in log for consistency verification?
    // No, inline_bookmarks generates its own timestamp string.
    // Wait! inline_bookmarks::append_bookmark uses `Utc::now()` for the log line timestamp!
    // That's WRONG for historical detection. We need to write the HISTORICAL timestamp.
    // We need to modify `inline_bookmarks` to allow specific timestamp injection.
    
    // Ah, `inline_bookmarks` creates a new line like: [ TIMESTAMP ] (bookmark) ...
    // If we simply call append_bookmark, it will put NOW's timestamp.
    // In Replay, the bookmark time is derived from the parsed line timestamp.
    // So if we write [ NOW ] (bookmark) RUN_START, the replay will see it at `NOW`.
    // It will be WAY after the actual events.
    
    // FIX: We must manually construct the line with the HISTORICAL timestamp.
    
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(&gamelog_path)
        .map_err(|e| e.to_string())?;

    use std::io::Write;
    
    let format_ts = |dur: Duration| -> String {
        let dt: DateTime<Utc> = DateTime::from(std::time::UNIX_EPOCH + dur);
        dt.format("%Y.%m.%d %H:%M:%S").to_string()
    };

    let mut added_count = 0;
    for run in runs {
        // Run Start
        let start_ts = format_ts(run.entry_time);
        let start_line = format!("[ {} ] (bookmark) RUN_START\n", start_ts);
        f.write_all(start_line.as_bytes()).map_err(|e| e.to_string())?;
        added_count += 1;

        // Run End
        if let Some(exit_time) = run.exit_time {
            let end_ts = format_ts(exit_time);
            let end_line = format!("[ {} ] (bookmark) RUN_END\n", end_ts);
            f.write_all(end_line.as_bytes()).map_err(|e| e.to_string())?;
            added_count += 1;
        }
    }
    
    println!("Appended {} bookmark lines to gamelog.", added_count);

    Ok(())
}


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
                    
                    // Print coordinator logs for debugging
                    for log_msg in &output.logs {
                        println!("[Coordinator] {}", log_msg);
                    }

                    // Emit DPS
                    if let Some(sample) = output.dps_sample {
                        let _ = handle.emit("dps-update", sample);
                    }
                    
                    // Handle location changes for auto run management (append to gamelog)
                    if !output.location_changes.is_empty() {
                        use crate::core::inline_bookmarks;
                        
                        for loc_change in output.location_changes {
                            if loc_change.change.is_abyss_entry() {
                                // Entering Abyss - append RUN_START to gamelog
                                if let Err(e) = inline_bookmarks::add_run_start(&loc_change.gamelog_path) {
                                    println!("Error appending run start: {}", e);
                                } else {
                                    println!("{} entered the Abyss", loc_change.character_name);
                                }
                                
                                // Emit event for frontend
                                let _ = handle.emit("abyss-entered", serde_json::json!({
                                    "character": loc_change.character_name
                                }));
                            } else {
                                // Exiting Abyss - append RUN_END to gamelog
                                if let Err(e) = inline_bookmarks::add_run_end(&loc_change.gamelog_path) {
                                    println!("Error appending run end: {}", e);
                                } else {
                                    println!("{} exited the Abyss to {}", loc_change.character_name, loc_change.change.location);
                                }
                                
                                // Emit event for frontend
                                let _ = handle.emit("abyss-exited", serde_json::json!({
                                    "character": loc_change.character_name,
                                    "location": loc_change.change.location
                                }));
                            }
                        }
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
            step_replay,
            // Bookmark commands
            create_highlight_bookmark,
            toggle_room_marker,
            detect_filaments,
            get_session_bookmarks
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
