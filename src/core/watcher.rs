use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use super::tracker::TrackedGamelog;
use super::log_io;
use super::model::CombatEvent;

pub struct LogWatcher {
    trackers: HashMap<PathBuf, TrackedGamelog>,
}

impl LogWatcher {
    pub fn new() -> Self {
        Self {
            trackers: HashMap::new(),
        }
    }

    /// Updates the set of tracked paths.
    /// Returns a list of status messages (e.g., "Started tracking...").
    pub fn update_active_paths(&mut self, active_paths: &HashSet<PathBuf>, log_dir: &Path) -> Vec<String> {
        let mut messages = Vec::new();
        
        // Remove paths not in active_paths
        self.trackers.retain(|path, _| active_paths.contains(path));

        // Find paths that need to be added
        let to_add: Vec<PathBuf> = active_paths.iter()
            .filter(|p| !self.trackers.contains_key(*p))
            .cloned()
            .collect();

        if !to_add.is_empty() {
            // We only scan if we have something to add. 
            // In a real scenario, we might want to cache the scan result, 
            // but for now we follow the existing logic: scan when needed.
            if let Ok(logs) = log_io::scan_gamelogs_dir(log_dir) {
                for path in to_add {
                    if let Some(log) = logs.iter().find(|l| l.path == path) {
                        match TrackedGamelog::new(log.character.clone(), path.clone()) {
                            Ok(tracker) => {
                                messages.push(format!("Started tracking: {}", log.character));
                                self.trackers.insert(path, tracker);
                            }
                            Err(e) => {
                                messages.push(format!("Failed to track {:?}: {}", path, e));
                            }
                        }
                    } else {
                        messages.push(format!("Log file not found in directory scan: {:?}", path));
                    }
                }
            } else {
                 messages.push(format!("Failed to scan log directory: {:?}", log_dir));
            }
        }
        
        messages
    }

    /// Polls all active trackers for new events.
    /// Returns collected events and any log messages (e.g., "Read X new events").
    pub fn read_events(&mut self) -> (Vec<CombatEvent>, Vec<String>) {
        let mut all_events = Vec::new();
        let mut messages = Vec::new();

        for tracker in self.trackers.values_mut() {
            match tracker.read_new_events() {
                Ok(new_events) => {
                    if !new_events.is_empty() {
                        messages.push(format!("Read {} new events for {}", new_events.len(), tracker.source));
                        all_events.extend(new_events);
                    }
                }
                Err(e) => {
                    // Log error but continue
                    messages.push(format!("Error reading logs for {}: {}", tracker.source, e));
                }
            }
        }

        (all_events, messages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_watcher_lifecycle() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("20250101_120000.txt");
        
        // Create a dummy log file
        let mut file = File::create(&log_path).unwrap();
        writeln!(file, "------------------------------------------------------------").unwrap();
        writeln!(file, "  Gamelog").unwrap();
        writeln!(file, "  Listener: TestChar").unwrap();
        writeln!(file, "  Session Started: 2025.01.01 12:00:00").unwrap();
        writeln!(file, "------------------------------------------------------------").unwrap();
        
        let mut watcher = LogWatcher::new();
        let mut active_paths = HashSet::new();
        
        // 1. Add path
        active_paths.insert(log_path.clone());
        let msgs = watcher.update_active_paths(&active_paths, dir.path());
        
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].contains("Started tracking: TestChar"));
        assert!(watcher.trackers.contains_key(&log_path));

        // 2. Read events (empty initially)
        let (events, msgs) = watcher.read_events();
        assert!(events.is_empty());
        assert!(msgs.is_empty());

        // 3. Write event
        writeln!(file, "[ 2025.01.01 12:01:00 ] (combat) 100 from TestChar to Enemy [ Gun ]").unwrap();
        file.sync_all().unwrap();

        // 4. Read events again
        let (events, msgs) = watcher.read_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].amount, 100.0);
        assert!(msgs[0].contains("Read 1 new events"));

        // 5. Remove path
        active_paths.clear();
        let _ = watcher.update_active_paths(&active_paths, dir.path());
        assert!(!watcher.trackers.contains_key(&log_path));
    }
}
