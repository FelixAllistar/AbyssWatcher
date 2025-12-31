use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::model::CombatEvent;
use super::parser;

#[allow(dead_code)]
pub struct LogTailer {
    file: File,
    position: u64,
    path: PathBuf,
}

impl LogTailer {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path_ref = path.as_ref();
        let file = File::open(path_ref)?;
        let metadata = file.metadata()?;
        let position = metadata.len();
        Ok(Self {
            file,
            position,
            path: path_ref.to_path_buf(),
        })
    }

    pub fn read_new_lines(&mut self) -> io::Result<Vec<String>> {
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

    #[allow(dead_code)]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CharacterLog {
    pub character: String,
    pub path: PathBuf,
    pub last_modified: SystemTime,
    #[allow(dead_code)]
    pub file_size: u64,
}

fn extract_listener_name(path: &Path) -> io::Result<Option<String>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut buffer = String::new();

    for _ in 0..20 {
        buffer.clear();
        let bytes_read = reader.read_line(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        let trimmed = buffer.trim();
        if let Some(rest) = trimmed.strip_prefix("Listener:") {
            return Ok(Some(rest.trim().to_string()));
        }
    }

    Ok(None)
}

pub fn scan_gamelogs_dir(dir: impl AsRef<Path>) -> io::Result<Vec<CharacterLog>> {
    let mut per_character: HashMap<String, CharacterLog> = HashMap::new();

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

        if let Some(character) = extract_listener_name(&path)? {
            match per_character.get(&character) {
                Some(existing) if existing.last_modified >= last_modified => {}
                _ => {
                    per_character.insert(
                        character.clone(),
                        CharacterLog {
                            character,
                            path: path.clone(),
                            last_modified,
                            file_size,
                        },
                    );
                }
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
