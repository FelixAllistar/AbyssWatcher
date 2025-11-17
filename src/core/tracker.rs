use std::io;
use std::path::{Path, PathBuf};

use super::log_io;
use super::model;
use super::parser;

pub struct TrackedGamelog {
    tailer: log_io::LogTailer,
    parser: parser::LineParser,
    source: String,
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

    pub fn read_new_events(&mut self) -> io::Result<Vec<model::CombatEvent>> {
        let mut events = Vec::new();
        for line in self.tailer.read_new_lines()? {
            if let Some(event) = self.parser.parse_line(&line, &self.source) {
                events.push(event);
            }
        }
        Ok(events)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}
