//! Data model for bookmarks and Abyss runs.
//!
//! NOTE: TypeScript mirror types should be added to ui/src/types.ts

use std::path::PathBuf;
use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Type of bookmark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BookmarkType {
    /// Auto-detected run start (entered "Unknown" location)
    RunStart,
    /// Auto-detected run end (exited to known location)
    RunEnd,
    /// Manual room start marker
    RoomStart,
    /// Manual room end marker
    RoomEnd,
    /// Generic manual highlight/mark
    Highlight,
}

impl BookmarkType {
    /// Returns true if this is an auto-detected bookmark.
    pub fn is_auto(&self) -> bool {
        matches!(self, BookmarkType::RunStart | BookmarkType::RunEnd)
    }

    /// Returns true if this is a manual bookmark.
    pub fn is_manual(&self) -> bool {
        !self.is_auto()
    }
}

/// A bookmark marking a point in time during a run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    /// Unique ID within the run
    pub id: u64,
    /// Which run this belongs to
    pub run_id: u64,
    /// Timestamp (Duration from epoch, matching combat events)
    pub timestamp: Duration,
    /// Type of bookmark
    pub bookmark_type: BookmarkType,
    /// Optional user-provided label
    pub label: Option<String>,
}

/// State of the room marker toggle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoomMarkerState {
    /// No active room marker
    Idle,
    /// Room marker started, waiting for end
    InRoom,
}

impl Default for RoomMarkerState {
    fn default() -> Self {
        Self::Idle
    }
}

/// An Abyss run (from entry to exit).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    /// Unique run ID
    pub id: u64,
    /// Character name
    pub character: String,
    /// Character ID (from log filename)
    pub character_id: u64,
    /// Path to the gamelog file for this session
    pub gamelog_path: PathBuf,
    /// Path to the chatlog file (if available)
    pub chatlog_path: Option<PathBuf>,
    /// Run start timestamp
    pub start_time: Duration,
    /// Run end timestamp (None if still in progress)
    pub end_time: Option<Duration>,
    /// Origin location (where they entered from)
    pub origin_location: Option<String>,
    /// Bookmarks within this run
    pub bookmarks: Vec<Bookmark>,
    /// Current room marker state
    #[serde(skip)]
    pub room_marker_state: RoomMarkerState,
    /// Timestamp when room marker was started (for auto-close on run end)
    #[serde(skip)]
    pub room_marker_start: Option<Duration>,
}

impl Run {
    /// Create a new run.
    pub fn new(
        id: u64,
        character: String,
        character_id: u64,
        gamelog_path: PathBuf,
        chatlog_path: Option<PathBuf>,
        start_time: Duration,
        origin_location: Option<String>,
    ) -> Self {
        Self {
            id,
            character,
            character_id,
            gamelog_path,
            chatlog_path,
            start_time,
            end_time: None,
            origin_location,
            bookmarks: Vec::new(),
            room_marker_state: RoomMarkerState::Idle,
            room_marker_start: None,
        }
    }

    /// Check if the run is still in progress.
    pub fn is_in_progress(&self) -> bool {
        self.end_time.is_none()
    }

    /// End the run.
    pub fn end(&mut self, end_time: Duration) {
        self.end_time = Some(end_time);
        
        // Auto-close any open room marker
        if self.room_marker_state == RoomMarkerState::InRoom {
            if let Some(_start) = self.room_marker_start.take() {
                self.add_bookmark(BookmarkType::RoomEnd, end_time, Some("Auto-closed on run end".to_string()));
            }
            self.room_marker_state = RoomMarkerState::Idle;
        }
    }

    /// Add a bookmark to this run.
    pub fn add_bookmark(&mut self, bookmark_type: BookmarkType, timestamp: Duration, label: Option<String>) -> u64 {
        let id = self.bookmarks.len() as u64 + 1;
        self.bookmarks.push(Bookmark {
            id,
            run_id: self.id,
            timestamp,
            bookmark_type,
            label,
        });
        id
    }

    /// Toggle the room marker state.
    ///
    /// Returns the new state and the bookmark ID if one was created.
    pub fn toggle_room_marker(&mut self, timestamp: Duration) -> (RoomMarkerState, Option<u64>) {
        match self.room_marker_state {
            RoomMarkerState::Idle => {
                // Start room
                let id = self.add_bookmark(BookmarkType::RoomStart, timestamp, None);
                self.room_marker_state = RoomMarkerState::InRoom;
                self.room_marker_start = Some(timestamp);
                (RoomMarkerState::InRoom, Some(id))
            }
            RoomMarkerState::InRoom => {
                // End room
                let id = self.add_bookmark(BookmarkType::RoomEnd, timestamp, None);
                self.room_marker_state = RoomMarkerState::Idle;
                self.room_marker_start = None;
                (RoomMarkerState::Idle, Some(id))
            }
        }
    }

    /// Get the duration of the run (if ended).
    pub fn duration(&self) -> Option<Duration> {
        self.end_time.map(|end| end.saturating_sub(self.start_time))
    }

    /// Get all bookmarks sorted by timestamp.
    pub fn bookmarks_sorted(&self) -> Vec<&Bookmark> {
        let mut bookmarks: Vec<_> = self.bookmarks.iter().collect();
        bookmarks.sort_by_key(|b| b.timestamp);
        bookmarks
    }
}

/// Bookmarks for all runs by a single character.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CharacterBookmarks {
    /// Character ID
    pub character_id: u64,
    /// Character name
    pub character_name: String,
    /// All runs for this character
    pub runs: Vec<Run>,
    /// Next run ID to assign
    next_run_id: u64,
}

impl CharacterBookmarks {
    pub fn new(character_id: u64, character_name: String) -> Self {
        Self {
            character_id,
            character_name,
            runs: Vec::new(),
            next_run_id: 1,
        }
    }

    /// Start a new run.
    pub fn start_run(
        &mut self,
        gamelog_path: PathBuf,
        chatlog_path: Option<PathBuf>,
        start_time: Duration,
        origin_location: Option<String>,
    ) -> u64 {
        let id = self.next_run_id;
        self.next_run_id += 1;

        let run = Run::new(
            id,
            self.character_name.clone(),
            self.character_id,
            gamelog_path,
            chatlog_path,
            start_time,
            origin_location,
        );
        self.runs.push(run);
        id
    }

    /// Get a mutable reference to the current active run.
    pub fn active_run_mut(&mut self) -> Option<&mut Run> {
        self.runs.iter_mut().rev().find(|r| r.is_in_progress())
    }

    /// Get the current active run.
    pub fn active_run(&self) -> Option<&Run> {
        self.runs.iter().rev().find(|r| r.is_in_progress())
    }

    /// Get a run by ID.
    pub fn run(&self, run_id: u64) -> Option<&Run> {
        self.runs.iter().find(|r| r.id == run_id)
    }

    /// Get a mutable run by ID.
    pub fn run_mut(&mut self, run_id: u64) -> Option<&mut Run> {
        self.runs.iter_mut().find(|r| r.id == run_id)
    }

    /// Get runs that overlap with a time range.
    pub fn runs_in_range(&self, start: Duration, end: Duration) -> Vec<&Run> {
        self.runs
            .iter()
            .filter(|r| {
                let run_end = r.end_time.unwrap_or(Duration::MAX);
                r.start_time <= end && run_end >= start
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_lifecycle() {
        let mut run = Run::new(
            1,
            "TestChar".to_string(),
            12345,
            PathBuf::from("game.txt"),
            Some(PathBuf::from("chat.txt")),
            Duration::from_secs(100),
            Some("Jita".to_string()),
        );

        assert!(run.is_in_progress());
        assert_eq!(run.duration(), None);

        // Add a highlight
        let id = run.add_bookmark(BookmarkType::Highlight, Duration::from_secs(150), Some("Important!".to_string()));
        assert_eq!(id, 1);

        // Toggle room marker
        let (state, id) = run.toggle_room_marker(Duration::from_secs(200));
        assert_eq!(state, RoomMarkerState::InRoom);
        assert!(id.is_some());

        // End the run (should auto-close room marker)
        run.end(Duration::from_secs(300));
        assert!(!run.is_in_progress());
        assert_eq!(run.duration(), Some(Duration::from_secs(200)));
        assert_eq!(run.room_marker_state, RoomMarkerState::Idle);

        // Should have 4 bookmarks: highlight, room start, room end (auto-closed)
        assert_eq!(run.bookmarks.len(), 3);
    }

    #[test]
    fn test_room_marker_toggle() {
        let mut run = Run::new(
            1,
            "TestChar".to_string(),
            12345,
            PathBuf::from("game.txt"),
            None,
            Duration::from_secs(100),
            None,
        );

        // Start room
        let (state, _) = run.toggle_room_marker(Duration::from_secs(150));
        assert_eq!(state, RoomMarkerState::InRoom);

        // End room
        let (state, _) = run.toggle_room_marker(Duration::from_secs(200));
        assert_eq!(state, RoomMarkerState::Idle);

        assert_eq!(run.bookmarks.len(), 2);
        assert_eq!(run.bookmarks[0].bookmark_type, BookmarkType::RoomStart);
        assert_eq!(run.bookmarks[1].bookmark_type, BookmarkType::RoomEnd);
    }

    #[test]
    fn test_character_bookmarks() {
        let mut cb = CharacterBookmarks::new(12345, "TestChar".to_string());

        // Start a run
        let run_id = cb.start_run(
            PathBuf::from("game.txt"),
            None,
            Duration::from_secs(100),
            Some("Jita".to_string()),
        );
        assert_eq!(run_id, 1);

        // Should have an active run
        assert!(cb.active_run().is_some());

        // Add bookmark to active run
        cb.active_run_mut().unwrap().add_bookmark(
            BookmarkType::Highlight,
            Duration::from_secs(150),
            None,
        );

        // End the run
        cb.run_mut(run_id).unwrap().end(Duration::from_secs(300));
        assert!(cb.active_run().is_none());

        // Start another run
        let run_id2 = cb.start_run(
            PathBuf::from("game2.txt"),
            None,
            Duration::from_secs(400),
            None,
        );
        assert_eq!(run_id2, 2);
        assert!(cb.active_run().is_some());
    }
}
