//! Watcher for EVE Online Local chat logs.
//!
//! Tails Local chat logs to detect location changes in real-time.

use std::io;
use std::path::{Path, PathBuf};
use std::collections::HashMap;

use super::parser::{ChatlogParser, LocationChange};
use crate::core::discovery::{self, LogType};
use crate::core::log_io::LogTailer;

/// Watches a single Local chat log file for location changes.
pub struct LocalChatlogTracker {
    character: String,
    character_id: u64,
    tailer: LogTailer,
    parser: ChatlogParser,
    /// Last known location (for detecting Abyss entry/exit)
    last_location: Option<String>,
}

impl LocalChatlogTracker {
    /// Create a new tracker for a Local chat log file.
    pub fn new(character: String, character_id: u64, path: PathBuf) -> io::Result<Self> {
        let tailer = LogTailer::open(&path)?;
        Ok(Self {
            character,
            character_id,
            tailer,
            parser: ChatlogParser::new(),
            last_location: None,
        })
    }

    /// Poll for new location changes.
    pub fn read_location_changes(&mut self) -> io::Result<Vec<LocationChange>> {
        let lines = self.tailer.read_new_lines()?;
        let changes = self.parser.parse_lines(&lines);

        // Update last known location
        if let Some(last) = changes.last() {
            self.last_location = Some(last.location.clone());
        }

        Ok(changes)
    }

    /// Get the last known location.
    pub fn last_location(&self) -> Option<&str> {
        self.last_location.as_deref()
    }

    /// Check if currently in the Abyss (last location was "Unknown").
    pub fn is_in_abyss(&self) -> bool {
        self.last_location.as_deref() == Some("Unknown")
    }

    /// Rewind to the start of the file (for full replay).
    pub fn rewind(&mut self) -> io::Result<()> {
        self.tailer.rewind()?;
        self.last_location = None;
        Ok(())
    }

    pub fn character(&self) -> &str {
        &self.character
    }

    pub fn character_id(&self) -> u64 {
        self.character_id
    }
}

/// Manages multiple Local chat log trackers for multiple characters.
pub struct ChatlogWatcher {
    trackers: HashMap<u64, LocalChatlogTracker>,
}

impl ChatlogWatcher {
    pub fn new() -> Self {
        Self {
            trackers: HashMap::new(),
        }
    }

    /// Start tracking Local chat for a character.
    ///
    /// Automatically finds the most recent Local chat log file.
    pub fn start_tracking(
        &mut self,
        chatlog_dir: &Path,
        character_name: &str,
        character_id: u64,
    ) -> io::Result<bool> {
        // Check if already tracking
        if self.trackers.contains_key(&character_id) {
            return Ok(false);
        }

        // Find the most recent Local chat log
        let path = match discovery::find_local_chatlog(chatlog_dir, character_id)? {
            Some(p) => p,
            None => {
                // Try by name as fallback
                match discovery::find_local_chatlog_by_name(chatlog_dir, character_name)? {
                    Some(p) => p,
                    None => return Ok(false),
                }
            }
        };

        let tracker = LocalChatlogTracker::new(
            character_name.to_string(),
            character_id,
            path,
        )?;
        self.trackers.insert(character_id, tracker);
        Ok(true)
    }

    /// Stop tracking a character.
    pub fn stop_tracking(&mut self, character_id: u64) -> bool {
        self.trackers.remove(&character_id).is_some()
    }

    /// Poll all trackers for location changes.
    ///
    /// Returns a map of character_id -> location changes.
    pub fn read_all_changes(&mut self) -> HashMap<u64, Vec<LocationChange>> {
        let mut results = HashMap::new();

        for (char_id, tracker) in &mut self.trackers {
            match tracker.read_location_changes() {
                Ok(changes) if !changes.is_empty() => {
                    results.insert(*char_id, changes);
                }
                Ok(_) => {}
                Err(e) => {
                    log::warn!("Error reading chatlog for {}: {}", tracker.character, e);
                }
            }
        }

        results
    }

    /// Get the last known location for a character.
    pub fn last_location(&self, character_id: u64) -> Option<&str> {
        self.trackers.get(&character_id).and_then(|t| t.last_location())
    }

    /// Check if a character is currently in the Abyss.
    pub fn is_in_abyss(&self, character_id: u64) -> bool {
        self.trackers.get(&character_id).map(|t| t.is_in_abyss()).unwrap_or(false)
    }

    /// Get all tracked character IDs.
    pub fn tracked_characters(&self) -> Vec<u64> {
        self.trackers.keys().copied().collect()
    }
}

impl Default for ChatlogWatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    fn create_local_chatlog(path: &Path, char_name: &str) {
        let mut file = File::create(path).unwrap();
        writeln!(file, "---------------------------------------------------------------").unwrap();
        writeln!(file).unwrap();
        writeln!(file, "  Channel ID:      local").unwrap();
        writeln!(file, "  Channel Name:    Local").unwrap();
        writeln!(file, "  Listener:        {}", char_name).unwrap();
        writeln!(file, "  Session started: 2026.01.03 11:26:30").unwrap();
        writeln!(file, "---------------------------------------------------------------").unwrap();
        writeln!(file).unwrap();
        writeln!(file, "[ 2026.01.03 11:26:33 ] EVE System > Channel changed to Local : Torrinos").unwrap();
    }

    #[test]
    fn test_local_chatlog_tracker() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("Local_20260103_112630_12345.txt");
        create_local_chatlog(&path, "TestChar");

        let mut tracker = LocalChatlogTracker::new(
            "TestChar".to_string(),
            12345,
            path.clone(),
        ).unwrap();

        // Rewind to read from the beginning (LogTailer starts at end by default)
        tracker.rewind().unwrap();

        // Initial read should get the Torrinos location
        let changes = tracker.read_location_changes().unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].location, "Torrinos");
        assert!(!tracker.is_in_abyss());

        // Add more lines
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        writeln!(file, "[ 2026.01.03 11:30:05 ] EVE System > Channel changed to Local : Unknown").unwrap();
        file.sync_all().unwrap();

        // Read again
        let changes = tracker.read_location_changes().unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].location, "Unknown");
        assert!(tracker.is_in_abyss());
    }

    #[test]
    fn test_chatlog_watcher() {
        let dir = tempdir().unwrap();
        
        // Create a local chatlog
        let path = dir.path().join("Local_20260103_112630_12345.txt");
        create_local_chatlog(&path, "TestChar");

        let mut watcher = ChatlogWatcher::new();

        // Start tracking (by ID)
        let started = watcher.start_tracking(dir.path(), "TestChar", 12345).unwrap();
        assert!(started);

        // Rewind to read from the beginning for test
        if let Some(tracker) = watcher.trackers.get_mut(&12345) {
            tracker.rewind().unwrap();
        }

        // Read changes
        let all_changes = watcher.read_all_changes();
        assert_eq!(all_changes.len(), 1);
        assert_eq!(all_changes.get(&12345).unwrap().len(), 1);

        // Check last location
        assert_eq!(watcher.last_location(12345), Some("Torrinos"));
        assert!(!watcher.is_in_abyss(12345));

        // Stop tracking
        assert!(watcher.stop_tracking(12345));
        assert!(watcher.tracked_characters().is_empty());
    }
}
