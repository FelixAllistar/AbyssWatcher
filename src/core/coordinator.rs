use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use super::model::{DpsSample, CombatEvent, NotifyEvent};
use super::state::EngineState;
use super::watcher::LogWatcher;
use super::chatlog::watcher::ChatlogWatcher;
use super::chatlog::parser::LocationChange;
use super::discovery;

/// A location change event with character context.
#[derive(Debug, Clone)]
pub struct CharacterLocationChange {
    pub character_name: String,
    pub character_id: u64,
    pub gamelog_path: PathBuf,
    pub change: LocationChange,
}

pub struct CoordinatorOutput {
    pub dps_sample: Option<DpsSample>,
    pub logs: Vec<String>,
    /// Location changes detected from Local chat logs
    pub location_changes: Vec<CharacterLocationChange>,
    /// New combat events since last tick (for alert evaluation)
    pub new_combat_events: Vec<CombatEvent>,
    /// New notify events since last tick (for alert evaluation)
    pub new_notify_events: Vec<NotifyEvent>,
}

pub struct Coordinator {
    watcher: LogWatcher,
    chatlog_watcher: ChatlogWatcher,
    engine: EngineState,
    log_dir: PathBuf,
    
    // State for time tracking
    last_event_timestamp: Option<Duration>,
    last_event_wallclock: Option<SystemTime>,
    current_tracked_set: HashSet<PathBuf>,
    
    /// Maps gamelog path -> (character_name, character_id) for chatlog tracking
    tracked_characters: std::collections::HashMap<PathBuf, (String, u64)>,
}

impl Coordinator {
    pub fn new(log_dir: PathBuf) -> Self {
        Self {
            watcher: LogWatcher::new(),
            chatlog_watcher: ChatlogWatcher::new(),
            engine: EngineState::new(),
            log_dir,
            last_event_timestamp: None,
            last_event_wallclock: None,
            current_tracked_set: HashSet::new(),
            tracked_characters: std::collections::HashMap::new(),
        }
    }

    pub fn tick(&mut self, active_paths: &HashSet<PathBuf>, dps_window: Duration) -> CoordinatorOutput {
        let mut logs = Vec::new();
        let mut location_changes = Vec::new();
        let mut new_combat_events = Vec::new();
        let mut new_notify_events = Vec::new();

        // 1. Update Tracked Paths
        if *active_paths != self.current_tracked_set {
            // If any path was removed, we reset the engine state entirely
            let removed = self.current_tracked_set.difference(active_paths).next().is_some();
            if removed {
                self.engine = EngineState::new();
                self.last_event_timestamp = None;
                self.last_event_wallclock = None;
            }

            let msgs = self.watcher.update_active_paths(active_paths, &self.log_dir);
            logs.extend(msgs);
            
            // Start/stop chatlog tracking for each character
            self.update_chatlog_tracking(active_paths, &mut logs);
            
            self.current_tracked_set = active_paths.clone();
        }

        // 2. Poll Combat and Notify Events
        let (combat_events, notify_events, poll_msgs) = self.watcher.read_events();
        logs.extend(poll_msgs);
        
        // Store for alert evaluation
        new_notify_events = notify_events;

        if !combat_events.is_empty() {
            let now_wallclock = SystemTime::now();
            for event in &combat_events {
                self.last_event_timestamp = Some(
                    self.last_event_timestamp
                        .map_or(event.timestamp, |prev| prev.max(event.timestamp))
                );
                self.engine.push_event(event.clone());
            }
            self.last_event_wallclock = Some(now_wallclock);
            new_combat_events = combat_events;
        }

        // 3. Poll Location Changes from Chatlogs
        let all_changes = self.chatlog_watcher.read_all_changes();
        for (char_id, changes) in all_changes {
            // Find the gamelog path for this character
            if let Some((gamelog_path, (char_name, _))) = self.tracked_characters.iter().find(|(_, (_, id))| *id == char_id) {
                for change in changes {
                    logs.push(format!("{} moved to: {}", char_name, change.location));
                    location_changes.push(CharacterLocationChange {
                        character_name: char_name.clone(),
                        character_id: char_id,
                        gamelog_path: gamelog_path.clone(),
                        change,
                    });
                }
            }
        }

        // 4. Compute DPS
        let end_time = match (self.last_event_timestamp, self.last_event_wallclock) {
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

        let samples = self.engine.dps_series(dps_window, end_time);
        let dps_sample = samples.into_iter().last();

        CoordinatorOutput {
            dps_sample,
            logs,
            location_changes,
            new_combat_events,
            new_notify_events,
        }
    }

    /// Update chatlog tracking based on active gamelog paths
    fn update_chatlog_tracking(&mut self, active_paths: &HashSet<PathBuf>, logs: &mut Vec<String>) {
        // Derive chatlog dir from log_dir (gamelog dir)
        let chatlog_dir = discovery::derive_chatlog_dir(&self.log_dir);
        
        if !chatlog_dir.exists() {
            return;
        }
        
        // Track new characters
        for gamelog_path in active_paths {
            if self.tracked_characters.contains_key(gamelog_path) {
                continue;
            }
            
            // Extract character info from gamelog header
            if let Ok(Some(header)) = discovery::extract_header(gamelog_path, discovery::LogType::Gamelog) {
                let char_id = header.character_id.unwrap_or_else(|| {
                    // Fallback to hash if no ID in filename
                    use std::hash::{Hash, Hasher};
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    header.character.hash(&mut hasher);
                    hasher.finish()
                });
                
                // Start tracking chatlog
                match self.chatlog_watcher.start_tracking(&chatlog_dir, &header.character, char_id) {
                    Ok(true) => {
                        logs.push(format!("Started chatlog tracking for {}", header.character));
                        self.tracked_characters.insert(gamelog_path.clone(), (header.character.clone(), char_id));
                    }
                    Ok(false) => {
                        // No chatlog found, still track the character for manual bookmarks
                        self.tracked_characters.insert(gamelog_path.clone(), (header.character.clone(), char_id));
                    }
                    Err(e) => {
                        logs.push(format!("Failed to start chatlog for {}: {}", header.character, e));
                    }
                }
            }
        }
        
        // Stop tracking removed characters
        let to_remove: Vec<_> = self.tracked_characters.keys()
            .filter(|p| !active_paths.contains(*p))
            .cloned()
            .collect();
            
        for path in to_remove {
            if let Some((name, char_id)) = self.tracked_characters.remove(&path) {
                self.chatlog_watcher.stop_tracking(char_id);
                logs.push(format!("Stopped chatlog tracking for {}", name));
            }
        }
    }

    pub fn replay_logs(&mut self) {
        self.engine = EngineState::new();
        self.last_event_timestamp = None;
        self.last_event_wallclock = None;
        self.watcher.rewind_all();
    }
    
    /// Get character info for a tracked gamelog path
    pub fn get_character_info(&self, gamelog_path: &PathBuf) -> Option<(String, u64)> {
        self.tracked_characters.get(gamelog_path).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_coordinator_flow() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("20250101_120000_12345.txt");
        
        let mut file = File::create(&log_path).unwrap();
        writeln!(file, "------------------------------------------------------------").unwrap();
        writeln!(file, "  Gamelog").unwrap();
        writeln!(file, "  Listener: TestChar").unwrap();
        writeln!(file, "  Session Started: 2025.01.01 12:00:00").unwrap();
        writeln!(file, "------------------------------------------------------------").unwrap();

        let mut coord = Coordinator::new(dir.path().to_path_buf());
        let mut active_paths = HashSet::new();
        active_paths.insert(log_path.clone());

        // Tick 1: Start tracking
        let output = coord.tick(&active_paths, Duration::from_secs(5));
        assert!(output.logs.iter().any(|m| m.contains("Started tracking")));
        assert!(output.dps_sample.is_none()); // No events yet

        // Write event
        writeln!(file, "[ 2025.01.01 12:01:00 ] (combat) 100 from TestChar to Enemy [ Gun ]").unwrap();
        file.sync_all().unwrap();

        // Tick 2: Read event
        let output = coord.tick(&active_paths, Duration::from_secs(5));
        
        let sample = output.dps_sample.unwrap();
        assert!(sample.outgoing_dps > 0.0);
        
        // Check character isolation
        let char_dps = sample.outgoing_by_character.get("TestChar").unwrap();
        assert!(*char_dps > 0.0);
    }
}
