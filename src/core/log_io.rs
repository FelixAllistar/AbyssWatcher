use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use chrono::{NaiveDateTime, TimeZone, Utc};

use super::model::CombatEvent;
use super::parser;

/// Detected encoding of a log file
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogEncoding {
    Utf8,
    Utf16Le,
}

#[allow(dead_code)]
pub struct LogTailer {
    file: File,
    position: u64,
    path: PathBuf,
    encoding: LogEncoding,
}

impl LogTailer {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path_ref = path.as_ref();
        let mut file = File::open(path_ref)?;
        let metadata = file.metadata()?;
        
        // Detect encoding by checking for UTF-16LE BOM (FF FE)
        let encoding = Self::detect_encoding(&mut file)?;
        
        let position = metadata.len();
        Ok(Self {
            file,
            position,
            path: path_ref.to_path_buf(),
            encoding,
        })
    }
    
    /// Detect file encoding by checking for BOM
    fn detect_encoding(file: &mut File) -> io::Result<LogEncoding> {
        let mut bom = [0u8; 2];
        file.seek(SeekFrom::Start(0))?;
        
        if file.read(&mut bom)? >= 2 {
            // UTF-16LE BOM: FF FE
            if bom[0] == 0xFF && bom[1] == 0xFE {
                return Ok(LogEncoding::Utf16Le);
            }
        }
        
        file.seek(SeekFrom::Start(0))?;
        Ok(LogEncoding::Utf8)
    }

    pub fn read_new_lines(&mut self) -> io::Result<Vec<String>> {
        match self.encoding {
            LogEncoding::Utf8 => self.read_utf8_lines(),
            LogEncoding::Utf16Le => self.read_utf16le_lines(),
        }
    }
    
    fn read_utf8_lines(&mut self) -> io::Result<Vec<String>> {
        let mut lines = Vec::new();

        self.file.seek(SeekFrom::Start(self.position))?;
        let mut reader = BufReader::new(&self.file);
        let mut buffer = String::new();

        loop {
            buffer.clear();
            let bytes_read = reader.read_line(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            self.position += bytes_read as u64;
            let line = buffer.trim_end_matches(&['\r', '\n'][..]).to_string();
            lines.push(line);
        }

        Ok(lines)
    }
    
    fn read_utf16le_lines(&mut self) -> io::Result<Vec<String>> {
        let mut lines = Vec::new();
        
        self.file.seek(SeekFrom::Start(self.position))?;
        
        // Read all remaining bytes
        let mut bytes = Vec::new();
        self.file.read_to_end(&mut bytes)?;
        
        if bytes.is_empty() {
            return Ok(lines);
        }
        
        // Convert UTF-16LE to String
        // Skip BOM if at start of file
        let start = if self.position == 0 && bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
            2
        } else {
            0
        };
        
        let u16_units: Vec<u16> = bytes[start..]
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        
        let text = String::from_utf16_lossy(&u16_units);
        
        for line in text.lines() {
            lines.push(line.to_string());
        }
        
        self.position += bytes.len() as u64;
        
        Ok(lines)
    }

    pub fn rewind(&mut self) -> io::Result<()> {
        self.position = 0;
        self.file.seek(SeekFrom::Start(0))?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn path(&self) -> &Path {
        &self.path
    }
    
    #[allow(dead_code)]
    pub fn encoding(&self) -> LogEncoding {
        self.encoding
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CharacterLog {
    pub character: String,
    pub path: PathBuf,
    pub last_modified: SystemTime,
    pub session_start: SystemTime,
    #[allow(dead_code)]
    pub file_size: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Session {
    pub id: String,
    pub timestamp: SystemTime,
    pub logs: Vec<CharacterLog>,
}

fn extract_header_info(path: &Path) -> io::Result<Option<(String, SystemTime)>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut buffer = String::new();
    
    let mut character = None;
    let mut timestamp = None;

    for _ in 0..20 {
        buffer.clear();
        let bytes_read = reader.read_line(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        let trimmed = buffer.trim();
        if let Some(rest) = trimmed.strip_prefix("Listener:") {
            character = Some(rest.trim().to_string());
        } else if let Some(rest) = trimmed.strip_prefix("Session Started:") {
            // Parse "2025.01.01 12:00:00"
            let time_str = rest.trim();
            if let Ok(naive) = NaiveDateTime::parse_from_str(time_str, "%Y.%m.%d %H:%M:%S") {
                timestamp = Some(SystemTime::from(Utc.from_utc_datetime(&naive)));
            }
        }
        
        if character.is_some() && timestamp.is_some() {
            return Ok(Some((character.unwrap(), timestamp.unwrap())));
        }
    }
    
    // Fallback: If we found character but no timestamp, we might return None or use SystemTime::UNIX_EPOCH?
    // Let's adhere to previous strictness: if we can't fully parse, return None (or at least Character is mandatory).
    if let Some(c) = character {
         return Ok(Some((c, timestamp.unwrap_or(SystemTime::UNIX_EPOCH))));
    }

    Ok(None)
}

pub fn scan_all_logs(dir: impl AsRef<Path>) -> io::Result<Vec<CharacterLog>> {
    let mut logs = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            if !extension.eq_ignore_ascii_case("txt") {
                continue;
            }
        } else {
            continue;
        }

        let metadata = fs::metadata(&path)?;
        let last_modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let file_size = metadata.len();

        if let Some((character, session_start)) = extract_header_info(&path)? {
            logs.push(CharacterLog {
                character,
                path,
                last_modified,
                session_start,
                file_size,
            });
        }
    }
    
    logs.sort_by(|a, b| b.session_start.cmp(&a.session_start));
    Ok(logs)
}

pub fn group_logs_by_character(logs: Vec<CharacterLog>) -> HashMap<String, Vec<CharacterLog>> {
    let mut groups: HashMap<String, Vec<CharacterLog>> = HashMap::new();
    for log in logs {
        groups.entry(log.character.clone()).or_default().push(log);
    }
    // Sort logs within each group
    for list in groups.values_mut() {
        list.sort_by(|a, b| b.session_start.cmp(&a.session_start));
    }
    groups
}

pub fn scan_gamelogs_dir(dir: impl AsRef<Path>) -> io::Result<Vec<CharacterLog>> {
    let mut per_character: HashMap<String, CharacterLog> = HashMap::new();

    // Use scan_all_logs internally to DRY
    let all_logs = scan_all_logs(dir)?;

    for log in all_logs {
        match per_character.get(&log.character) {
            Some(existing) if existing.last_modified >= log.last_modified => {} // Keep existing
            _ => {
                per_character.insert(log.character.clone(), log);
            }
        }
    }

    let mut logs: Vec<CharacterLog> = per_character.into_values().collect();
    logs.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
    Ok(logs)
}

#[allow(dead_code)]
pub fn read_full_lines(path: impl AsRef<Path>) -> io::Result<Vec<String>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines = Vec::new();

    for line_result in reader.lines() {
        let line = line_result?;
        lines.push(line);
    }

    Ok(lines)
}

#[allow(dead_code)]
pub fn read_full_events(path: impl AsRef<Path>) -> io::Result<Vec<CombatEvent>> {
    let lines = read_full_lines(path)?;
    let mut events = Vec::new();
    let mut parser = parser::LineParser::new();

    for line in lines {
        if let Some(event) = parser.parse_line(&line, "") {
            events.push(event);
        }
    }

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::io::Write;
    use std::time::Duration;

    fn create_dummy_log(path: PathBuf, char_name: &str, time_str: &str) {
        let mut file = File::create(path).unwrap();
        writeln!(file, "------------------------------------------------------------").unwrap();
        writeln!(file, "  Gamelog").unwrap();
        writeln!(file, "  Listener: {}", char_name).unwrap();
        writeln!(file, "  Session Started: {}", time_str).unwrap();
        writeln!(file, "------------------------------------------------------------").unwrap();
    }

    #[test]
    fn test_scan_all_logs_returns_history() {
        let dir = tempdir().unwrap();
        // Create 2 logs for same char
        create_dummy_log(dir.path().join("old.txt"), "CharA", "2024.01.01 10:00:00");
        create_dummy_log(dir.path().join("new.txt"), "CharA", "2024.01.01 11:00:00");

        // scan_gamelogs_dir only returns the latest per character
        let logs = scan_gamelogs_dir(dir.path()).unwrap();
        assert_eq!(logs.len(), 1); 
        assert_eq!(logs[0].path.file_name().unwrap(), "new.txt");

        // We want a function that returns ALL logs
        let all_logs = scan_all_logs(dir.path()).unwrap();
        assert_eq!(all_logs.len(), 2);
    }

    #[test]
    fn test_group_logs_by_character() {
        let dir = tempdir().unwrap();
        create_dummy_log(dir.path().join("s1_a.txt"), "CharA", "2024.01.01 12:00:00");
        create_dummy_log(dir.path().join("s1_b.txt"), "CharB", "2024.01.01 12:00:05");
        create_dummy_log(dir.path().join("s2_a.txt"), "CharA", "2024.01.01 14:00:00");

        let logs = scan_all_logs(dir.path()).unwrap();
        let groups = group_logs_by_character(logs);

        assert_eq!(groups.len(), 2);
        assert_eq!(groups.get("CharA").unwrap().len(), 2);
        assert_eq!(groups.get("CharB").unwrap().len(), 1);
    }
}