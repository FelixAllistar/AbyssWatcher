//! Unified log discovery for EVE Online log files.
//!
//! This module provides shared functionality for discovering and parsing
//! both Gamelogs (combat) and Chatlogs (Local chat).

use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

/// Type of EVE log file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogType {
    /// Combat/game log (Gamelogs directory)
    Gamelog,
    /// Chat log (Chatlogs directory, specifically Local chat)
    Chatlog,
}

/// Unified header information extracted from any EVE log file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogHeader {
    /// Character name from "Listener:" field
    pub character: String,
    /// Character ID extracted from filename (if available)
    pub character_id: Option<u64>,
    /// Session start time from header
    pub session_start: SystemTime,
    /// Full path to the log file
    pub path: PathBuf,
    /// File modification time
    pub last_modified: SystemTime,
    /// File size in bytes
    pub file_size: u64,
    /// Type of log file
    pub log_type: LogType,
}

/// Extract character ID from filename patterns like:
/// - `20250101_120000.txt` (Gamelog, no ID)
/// - `Local_20260103_174237_2112699440.txt` (Chatlog with ID)
fn extract_character_id_from_filename(filename: &str) -> Option<u64> {
    // Chatlog pattern: Local_YYYYMMDD_HHMMSS_CHARACTERID.txt
    // The character ID is the last numeric segment before .txt
    let name = filename.strip_suffix(".txt").unwrap_or(filename);
    let parts: Vec<&str> = name.split('_').collect();
    
    // For chatlog: ["Local", "20260103", "174237", "2112699440"]
    if parts.len() >= 4 && parts[0] == "Local" {
        return parts.last().and_then(|s| s.parse().ok());
    }
    
    None
}

/// Extract header information from an EVE log file.
///
/// Handles both Gamelog and Chatlog formats:
/// - Gamelog: "Listener:", "Session Started:"
/// - Chatlog: "Listener:", "Session started:" (note: lowercase 's')
pub fn extract_header(path: &Path, log_type: LogType) -> io::Result<Option<LogHeader>> {
    let file = File::open(path)?;
    let metadata = fs::metadata(path)?;
    let mut reader = BufReader::new(file);
    let mut buffer = String::new();

    let mut character = None;
    let mut session_start = None;

    // Read up to 20 lines to find header info
    for _ in 0..20 {
        buffer.clear();
        let bytes_read = reader.read_line(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        let trimmed = buffer.trim();

        // Character name: "Listener: CharName"
        if let Some(rest) = trimmed.strip_prefix("Listener:") {
            character = Some(rest.trim().to_string());
        }

        // Session time - try both formats
        // Gamelog: "Session Started: 2025.01.01 12:00:00"
        // Chatlog: "Session started: 2026.01.03 11:26:30"
        let time_str = trimmed
            .strip_prefix("Session Started:")
            .or_else(|| trimmed.strip_prefix("Session started:"));

        if let Some(time_str) = time_str {
            let time_str = time_str.trim();
            if let Ok(naive) = NaiveDateTime::parse_from_str(time_str, "%Y.%m.%d %H:%M:%S") {
                session_start = Some(SystemTime::from(Utc.from_utc_datetime(&naive)));
            }
        }

        // Early exit if we have both
        if character.is_some() && session_start.is_some() {
            break;
        }
    }

    // Require at least character name
    let character = match character {
        Some(c) => c,
        None => return Ok(None),
    };

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    let character_id = extract_character_id_from_filename(filename);

    Ok(Some(LogHeader {
        character,
        character_id,
        session_start: session_start.unwrap_or(SystemTime::UNIX_EPOCH),
        path: path.to_path_buf(),
        last_modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        file_size: metadata.len(),
        log_type,
    }))
}

/// Scan a directory for log files matching a pattern.
///
/// # Arguments
/// * `dir` - Directory to scan
/// * `prefix` - Optional filename prefix filter (e.g., "Local" for chat logs)
/// * `log_type` - Type of logs being scanned
pub fn scan_logs_dir(
    dir: impl AsRef<Path>,
    prefix: Option<&str>,
    log_type: LogType,
) -> io::Result<Vec<LogHeader>> {
    let mut logs = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        // Check extension
        let is_txt = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("txt"))
            .unwrap_or(false);

        if !is_txt {
            continue;
        }

        // Check prefix if specified
        if let Some(prefix) = prefix {
            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !filename.starts_with(prefix) {
                continue;
            }
        }

        if let Some(header) = extract_header(&path, log_type)? {
            logs.push(header);
        }
    }

    // Sort by session start, newest first
    logs.sort_by(|a, b| b.session_start.cmp(&a.session_start));
    Ok(logs)
}

/// Derive the Chatlogs directory from a Gamelogs directory.
///
/// EVE log structure:
/// ```text
/// .../EVE/logs/
/// ├── Gamelogs/
/// └── Chatlogs/
/// ```
pub fn derive_chatlog_dir(gamelog_dir: &Path) -> PathBuf {
    // Go up one level from Gamelogs, then into Chatlogs
    gamelog_dir
        .parent()
        .map(|p| p.join("Chatlogs"))
        .unwrap_or_else(|| gamelog_dir.join("../Chatlogs"))
}

/// Find the most recent Local chat log for a character.
pub fn find_local_chatlog(chatlog_dir: &Path, character_id: u64) -> io::Result<Option<PathBuf>> {
    let logs = scan_logs_dir(chatlog_dir, Some("Local"), LogType::Chatlog)?;

    // Find logs matching the character ID
    let matching: Vec<_> = logs
        .into_iter()
        .filter(|h| h.character_id == Some(character_id))
        .collect();

    // Return the most recent (already sorted by session_start desc)
    Ok(matching.into_iter().next().map(|h| h.path))
}

/// Find the most recent Local chat log by character name.
pub fn find_local_chatlog_by_name(chatlog_dir: &Path, character_name: &str) -> io::Result<Option<PathBuf>> {
    let logs = scan_logs_dir(chatlog_dir, Some("Local"), LogType::Chatlog)?;

    // Find logs matching the character name
    let matching: Vec<_> = logs
        .into_iter()
        .filter(|h| h.character == character_name)
        .collect();

    Ok(matching.into_iter().next().map(|h| h.path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    fn create_gamelog(path: &Path, char_name: &str, time_str: &str) {
        let mut file = File::create(path).unwrap();
        writeln!(file, "------------------------------------------------------------").unwrap();
        writeln!(file, "  Gamelog").unwrap();
        writeln!(file, "  Listener: {}", char_name).unwrap();
        writeln!(file, "  Session Started: {}", time_str).unwrap();
        writeln!(file, "------------------------------------------------------------").unwrap();
    }

    fn create_chatlog(path: &Path, char_name: &str, time_str: &str) {
        let mut file = File::create(path).unwrap();
        writeln!(file, "---------------------------------------------------------------").unwrap();
        writeln!(file).unwrap();
        writeln!(file, "  Channel ID:      local").unwrap();
        writeln!(file, "  Channel Name:    Local").unwrap();
        writeln!(file, "  Listener:        {}", char_name).unwrap();
        writeln!(file, "  Session started: {}", time_str).unwrap();
        writeln!(file, "---------------------------------------------------------------").unwrap();
    }

    #[test]
    fn test_extract_character_id_from_filename() {
        assert_eq!(
            extract_character_id_from_filename("Local_20260103_174237_2112699440.txt"),
            Some(2112699440)
        );
        assert_eq!(
            extract_character_id_from_filename("20250101_120000.txt"),
            None
        );
        assert_eq!(
            extract_character_id_from_filename("Local_20260103_174237.txt"),
            None // Not enough parts
        );
    }

    #[test]
    fn test_extract_gamelog_header() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("20250101_120000.txt");
        create_gamelog(&path, "TestChar", "2025.01.01 12:00:00");

        let header = extract_header(&path, LogType::Gamelog).unwrap().unwrap();
        assert_eq!(header.character, "TestChar");
        assert_eq!(header.character_id, None);
        assert_eq!(header.log_type, LogType::Gamelog);
    }

    #[test]
    fn test_extract_chatlog_header() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("Local_20260103_174237_2112699440.txt");
        create_chatlog(&path, "Felix Allistar 2", "2026.01.03 11:26:30");

        let header = extract_header(&path, LogType::Chatlog).unwrap().unwrap();
        assert_eq!(header.character, "Felix Allistar 2");
        assert_eq!(header.character_id, Some(2112699440));
        assert_eq!(header.log_type, LogType::Chatlog);
    }

    #[test]
    fn test_scan_logs_with_prefix() {
        let dir = tempdir().unwrap();
        
        // Create various logs
        create_chatlog(
            &dir.path().join("Local_20260103_174237_111.txt"),
            "CharA",
            "2026.01.03 11:26:30",
        );
        create_chatlog(
            &dir.path().join("Local_20260103_180000_222.txt"),
            "CharB",
            "2026.01.03 12:00:00",
        );
        create_chatlog(
            &dir.path().join("Corp_20260103_174237_111.txt"),
            "CharA",
            "2026.01.03 11:26:30",
        );

        // Should only find Local logs
        let logs = scan_logs_dir(dir.path(), Some("Local"), LogType::Chatlog).unwrap();
        assert_eq!(logs.len(), 2);
        
        // All logs if no prefix
        let all_logs = scan_logs_dir(dir.path(), None, LogType::Chatlog).unwrap();
        assert_eq!(all_logs.len(), 3);
    }

    #[test]
    fn test_derive_chatlog_dir() {
        let gamelog = PathBuf::from("/home/user/EVE/logs/Gamelogs");
        let chatlog = derive_chatlog_dir(&gamelog);
        assert_eq!(chatlog, PathBuf::from("/home/user/EVE/logs/Chatlogs"));
    }

    #[test]
    fn test_find_local_chatlog_by_id() {
        let dir = tempdir().unwrap();
        
        create_chatlog(
            &dir.path().join("Local_20260103_100000_111.txt"),
            "CharA",
            "2026.01.03 10:00:00",
        );
        create_chatlog(
            &dir.path().join("Local_20260103_120000_111.txt"),
            "CharA",
            "2026.01.03 12:00:00", // Newer
        );
        create_chatlog(
            &dir.path().join("Local_20260103_110000_222.txt"),
            "CharB",
            "2026.01.03 11:00:00",
        );

        let result = find_local_chatlog(dir.path(), 111).unwrap();
        assert!(result.is_some());
        // Should return the newer log
        let path = result.unwrap();
        assert!(path.to_string_lossy().contains("120000"));
    }
}
