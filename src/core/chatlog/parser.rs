//! Parser for EVE Online Local chat logs.
//!
//! Handles extracting location change events from Local chat.

use std::time::Duration;
use chrono::{NaiveDateTime, TimeZone, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};

/// A location change event from Local chat.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocationChange {
    /// Timestamp of the location change (Duration from epoch, matching combat events)
    pub timestamp: Duration,
    /// The new location name ("Torrinos", "Unknown", etc.)
    pub location: String,
}

impl LocationChange {
    /// Returns true if this location change represents entering the Abyss.
    pub fn is_abyss_entry(&self) -> bool {
        self.location == "Unknown"
    }

    /// Returns true if this location change represents exiting the Abyss.
    pub fn is_abyss_exit(&self) -> bool {
        !self.is_abyss_entry()
    }
}

/// Parser for Local chat log lines.
pub struct ChatlogParser {
    location_regex: Regex,
}

impl ChatlogParser {
    pub fn new() -> Self {
        // Pattern: [ 2026.01.03 11:26:33 ] EVE System > Channel changed to Local : Torrinos
        let location_regex = Regex::new(
            r"^\s*\[\s*(\d{4}\.\d{2}\.\d{2}\s+\d{2}:\d{2}:\d{2})\s*\]\s*EVE System\s*>\s*Channel changed to Local\s*:\s*(.+)$"
        ).expect("Invalid location regex");

        Self { location_regex }
    }

    /// Parse a single line for a location change event.
    pub fn parse_line(&self, line: &str) -> Option<LocationChange> {
        // Strip BOM and trim whitespace
        let line = line.trim().trim_start_matches('\u{feff}');
        let caps = self.location_regex.captures(line)?;

        let time_str = caps.get(1)?.as_str();
        let location = caps.get(2)?.as_str().trim().to_string();

        // Parse timestamp to Duration (from epoch, like combat events)
        let naive = NaiveDateTime::parse_from_str(time_str, "%Y.%m.%d %H:%M:%S").ok()?;
        let dt = Utc.from_utc_datetime(&naive);
        let timestamp = Duration::from_secs(dt.timestamp() as u64);

        Some(LocationChange { timestamp, location })
    }

    /// Parse all location changes from a list of lines.
    pub fn parse_lines(&self, lines: &[String]) -> Vec<LocationChange> {
        lines.iter().filter_map(|line| self.parse_line(line)).collect()
    }
}

impl Default for ChatlogParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Segment representing an Abyss run (entry to exit).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbyssRun {
    /// Entry timestamp
    pub entry_time: Duration,
    /// Exit timestamp (None if still in Abyss)
    pub exit_time: Option<Duration>,
    /// Location before entering (e.g., "Torrinos")
    pub origin_location: Option<String>,
}

/// Detect Abyss runs from a sequence of location changes.
pub fn detect_abyss_runs(changes: &[LocationChange]) -> Vec<AbyssRun> {
    let mut runs = Vec::new();
    let mut current_run: Option<AbyssRun> = None;
    let mut last_known_location: Option<String> = None;

    for change in changes {
        if change.is_abyss_entry() {
            // Starting a new run
            if current_run.is_none() {
                current_run = Some(AbyssRun {
                    entry_time: change.timestamp,
                    exit_time: None,
                    origin_location: last_known_location.clone(),
                });
            }
        } else {
            // Exiting the Abyss
            if let Some(mut run) = current_run.take() {
                run.exit_time = Some(change.timestamp);
                runs.push(run);
            }
            last_known_location = Some(change.location.clone());
        }
    }

    // If there's an unclosed run, include it
    if let Some(run) = current_run {
        runs.push(run);
    }

    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_location_change() {
        let parser = ChatlogParser::new();

        let line = "[ 2026.01.03 11:26:33 ] EVE System > Channel changed to Local : Torrinos";
        let change = parser.parse_line(line).expect("Should parse");
        assert_eq!(change.location, "Torrinos");
        assert!(!change.is_abyss_entry());
        assert!(change.is_abyss_exit());
    }

    #[test]
    fn test_parse_unknown_location() {
        let parser = ChatlogParser::new();

        let line = "[ 2026.01.03 11:30:05 ] EVE System > Channel changed to Local : Unknown";
        let change = parser.parse_line(line).expect("Should parse");
        assert_eq!(change.location, "Unknown");
        assert!(change.is_abyss_entry());
    }

    #[test]
    fn test_parse_with_bom() {
        let parser = ChatlogParser::new();

        // EVE logs sometimes have BOM characters
        let line = "\u{feff}[ 2026.01.03 11:26:33 ] EVE System > Channel changed to Local : Torrinos";
        let change = parser.parse_line(line).expect("Should parse with BOM");
        assert_eq!(change.location, "Torrinos");
    }

    #[test]
    fn test_parse_non_location_line() {
        let parser = ChatlogParser::new();

        let line = "[ 2026.01.03 11:26:33 ] Felix Allistar > Hello world";
        assert!(parser.parse_line(line).is_none());
    }

    #[test]
    fn test_detect_abyss_runs() {
        let changes = vec![
            LocationChange {
                timestamp: Duration::from_secs(100),
                location: "Torrinos".to_string(),
            },
            LocationChange {
                timestamp: Duration::from_secs(200),
                location: "Unknown".to_string(),
            },
            LocationChange {
                timestamp: Duration::from_secs(800),
                location: "Torrinos".to_string(),
            },
            LocationChange {
                timestamp: Duration::from_secs(900),
                location: "Unknown".to_string(),
            },
            LocationChange {
                timestamp: Duration::from_secs(1500),
                location: "Torrinos".to_string(),
            },
        ];

        let runs = detect_abyss_runs(&changes);
        assert_eq!(runs.len(), 2);

        assert_eq!(runs[0].entry_time, Duration::from_secs(200));
        assert_eq!(runs[0].exit_time, Some(Duration::from_secs(800)));
        assert_eq!(runs[0].origin_location, Some("Torrinos".to_string()));

        assert_eq!(runs[1].entry_time, Duration::from_secs(900));
        assert_eq!(runs[1].exit_time, Some(Duration::from_secs(1500)));
    }

    #[test]
    fn test_detect_unclosed_run() {
        let changes = vec![
            LocationChange {
                timestamp: Duration::from_secs(100),
                location: "Jita".to_string(),
            },
            LocationChange {
                timestamp: Duration::from_secs(200),
                location: "Unknown".to_string(),
            },
            // No exit
        ];

        let runs = detect_abyss_runs(&changes);
        assert_eq!(runs.len(), 1);
        assert!(runs[0].exit_time.is_none());
        assert_eq!(runs[0].origin_location, Some("Jita".to_string()));
    }

    #[test]
    fn test_parse_full_log_sample() {
        let parser = ChatlogParser::new();
        let lines: Vec<String> = vec![
            "[ 2026.01.03 11:26:33 ] EVE System > Channel changed to Local : Torrinos",
            "[ 2026.01.03 11:30:05 ] EVE System > Channel changed to Local : Unknown",
            "[ 2026.01.03 11:39:03 ] EVE System > Channel changed to Local : Torrinos",
            "[ 2026.01.03 11:40:02 ] EVE System > Channel changed to Local : Unknown",
            "[ 2026.01.03 11:53:19 ] EVE System > Channel changed to Local : Torrinos",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let changes = parser.parse_lines(&lines);
        assert_eq!(changes.len(), 5);

        let runs = detect_abyss_runs(&changes);
        assert_eq!(runs.len(), 2);
    }
}
