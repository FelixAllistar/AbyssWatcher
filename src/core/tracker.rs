use std::io;
use std::path::{Path, PathBuf};

use super::log_io;
use super::model;
use super::parser;

/// Result of reading new log lines: combat events and notify events
pub struct TrackerReadResult {
    pub combat_events: Vec<model::CombatEvent>,
    pub notify_events: Vec<model::NotifyEvent>,
}

#[allow(dead_code)]
pub struct TrackedGamelog {
    tailer: log_io::LogTailer,
    parser: parser::LineParser,
    pub source: String,
    path: PathBuf,
}

impl TrackedGamelog {
    pub fn new(source: impl Into<String>, path: impl AsRef<Path>) -> io::Result<Self> {
        let pathbuf = path.as_ref().to_path_buf();
        let tailer = log_io::LogTailer::open(&pathbuf)?;
        Ok(Self {
            tailer,
            parser: parser::LineParser::new(),
            source: source.into(),
            path: pathbuf,
        })
    }

    /// Read new log lines and parse both combat and notify events
    pub fn read_new_events(&mut self) -> io::Result<TrackerReadResult> {
        let mut combat_events = Vec::new();
        let mut notify_events = Vec::new();
        
        for line in self.tailer.read_new_lines()? {
            // Try parsing as combat event
            if let Some(event) = self.parser.parse_line(&line, &self.source) {
                combat_events.push(event);
            }
            // Also try parsing as notify event (for capacitor failures, etc.)
            if let Some(notify) = self.parser.parse_notify_line(&line, &self.source) {
                notify_events.push(notify);
            }
        }
        
        Ok(TrackerReadResult {
            combat_events,
            notify_events,
        })
    }

    pub fn rewind(&mut self) -> io::Result<()> {
        self.tailer.rewind()
    }

    #[allow(dead_code)]
    pub fn path(&self) -> &Path {
        &self.path
    }
}
