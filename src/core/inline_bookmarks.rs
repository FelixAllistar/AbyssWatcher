//! Inline bookmarks - appends bookmark lines directly to gamelog files.
//!
//! Writes bookmark lines in EVE log format:
//! `[ 2026.01.04 03:56:49 ] (bookmark) TYPE: label`
//!
//! This allows bookmarks to travel with the log file and be parsed during replay.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::Path;

/// Types of bookmarks that can be placed in a gamelog
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BookmarkType {
    /// Abyss run started (entered Unknown)
    RunStart,
    /// Abyss run ended (exited Unknown)
    RunEnd,
    /// Room timer started
    RoomStart,
    /// Room timer ended
    RoomEnd,
    /// User-placed highlight marker
    Highlight,
}

impl BookmarkType {
    /// Get the string representation for log files
    pub fn as_str(&self) -> &'static str {
        match self {
            BookmarkType::RunStart => "RUN_START",
            BookmarkType::RunEnd => "RUN_END",
            BookmarkType::RoomStart => "ROOM_START",
            BookmarkType::RoomEnd => "ROOM_END",
            BookmarkType::Highlight => "HIGHLIGHT",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim() {
            "RUN_START" => Some(BookmarkType::RunStart),
            "RUN_END" => Some(BookmarkType::RunEnd),
            "ROOM_START" => Some(BookmarkType::RoomStart),
            "ROOM_END" => Some(BookmarkType::RoomEnd),
            "HIGHLIGHT" => Some(BookmarkType::Highlight),
            _ => None,
        }
    }
}

/// A parsed inline bookmark from a gamelog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineBookmark {
    /// Timestamp in seconds from epoch
    pub timestamp_secs: u64,
    /// Type of bookmark
    pub bookmark_type: BookmarkType,
    /// Optional label (for Highlight bookmarks)
    pub label: Option<String>,
}

/// Append a bookmark line to a gamelog file.
pub fn append_bookmark(
    gamelog_path: &Path,
    bookmark_type: &str,
    label: Option<&str>,
) -> io::Result<()> {
    let mut file = OpenOptions::new().append(true).open(gamelog_path)?;

    // Format timestamp like EVE logs: "2026.01.04 03:56:49"
    let now: DateTime<Utc> = Utc::now();
    let timestamp = now.format("%Y.%m.%d %H:%M:%S");

    // Format: [ TIMESTAMP ] (bookmark) TYPE: label
    let line = if let Some(lbl) = label {
        format!("[ {} ] (bookmark) {}: {}\n", timestamp, bookmark_type, lbl)
    } else {
        format!("[ {} ] (bookmark) {}\n", timestamp, bookmark_type)
    };

    file.write_all(line.as_bytes())?;
    file.sync_all()?;

    Ok(())
}

/// Add a highlight bookmark
pub fn add_highlight(gamelog_path: &Path, label: Option<&str>) -> io::Result<()> {
    append_bookmark(gamelog_path, "HIGHLIGHT", label)
}

/// Add a room start marker
pub fn add_room_start(gamelog_path: &Path) -> io::Result<()> {
    append_bookmark(gamelog_path, "ROOM_START", None)
}

/// Add a room end marker
pub fn add_room_end(gamelog_path: &Path) -> io::Result<()> {
    append_bookmark(gamelog_path, "ROOM_END", None)
}

/// Add a run start marker
pub fn add_run_start(gamelog_path: &Path) -> io::Result<()> {
    append_bookmark(gamelog_path, "RUN_START", None)
}

/// Add a run end marker
pub fn add_run_end(gamelog_path: &Path) -> io::Result<()> {
    append_bookmark(gamelog_path, "RUN_END", None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_append_bookmark() {
        let dir = tempdir().unwrap();
        let log = dir.path().join("test.txt");

        // Create initial log file
        fs::write(
            &log,
            "[ 2026.01.04 03:00:00 ] (combat) 100 from Me to Target\n",
        )
        .unwrap();

        // Add bookmarks
        add_highlight(&log, Some("Important!")).unwrap();
        add_room_start(&log).unwrap();
        add_room_end(&log).unwrap();

        // Read back
        let content = fs::read_to_string(&log).unwrap();
        assert!(content.contains("(bookmark) HIGHLIGHT: Important!"));
        assert!(content.contains("(bookmark) ROOM_START"));
        assert!(content.contains("(bookmark) ROOM_END"));
    }
}
