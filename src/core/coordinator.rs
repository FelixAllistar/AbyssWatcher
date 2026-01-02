use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use super::model::DpsSample;
use super::state::EngineState;
use super::watcher::LogWatcher;

pub struct CoordinatorOutput {
    pub dps_sample: Option<DpsSample>,
    pub logs: Vec<String>,
}

pub struct Coordinator {
    watcher: LogWatcher,
    engine: EngineState,
    log_dir: PathBuf,
    
    // State for time tracking
    last_event_timestamp: Option<Duration>,
    last_event_wallclock: Option<SystemTime>,
    current_tracked_set: HashSet<PathBuf>,
}

impl Coordinator {
    pub fn new(log_dir: PathBuf) -> Self {
        Self {
            watcher: LogWatcher::new(),
            engine: EngineState::new(),
            log_dir,
            last_event_timestamp: None,
            last_event_wallclock: None,
            current_tracked_set: HashSet::new(),
        }
    }

    pub fn tick(&mut self, active_paths: &HashSet<PathBuf>, dps_window: Duration) -> CoordinatorOutput {
        let mut logs = Vec::new();

        // 1. Update Tracked Paths
        if *active_paths != self.current_tracked_set {
            // If any path was removed, we reset the engine state entirely (as per original logic)
            // Original logic: "let removed = current_tracked_set.difference(&active_paths).next().is_some(); if removed { engine = ... }"
            let removed = self.current_tracked_set.difference(active_paths).next().is_some();
            if removed {
                self.engine = EngineState::new();
                self.last_event_timestamp = None;
                self.last_event_wallclock = None;
            }

            let msgs = self.watcher.update_active_paths(active_paths, &self.log_dir);
            logs.extend(msgs);
            self.current_tracked_set = active_paths.clone();
        }

        // 2. Poll Events
        let (new_events, poll_msgs) = self.watcher.read_events();
        logs.extend(poll_msgs);

        if !new_events.is_empty() {
            let now_wallclock = SystemTime::now();
            for event in new_events {
                self.last_event_timestamp = Some(match self.last_event_timestamp {
                    Some(prev) => std::cmp::max(prev, event.timestamp),
                    None => event.timestamp,
                });
                self.engine.push_event(event);
            }
            self.last_event_wallclock = Some(now_wallclock);
        }

        // 3. Compute DPS
        // Calculate the "simulation time" end point
        let end_time = match (self.last_event_timestamp, self.last_event_wallclock) {
            (Some(timestamp), Some(seen_at)) => {
                // If we have seen events, we project forward based on how much real time passed since the last event
                if let Ok(elapsed) = SystemTime::now().duration_since(seen_at) {
                    timestamp + elapsed
                } else {
                    timestamp
                }
            }
            (Some(timestamp), None) => timestamp,
            (None, _) => Duration::from_secs(0),
        };

        let samples = self.engine.dps_series(dps_window, end_time);
        let dps_sample = samples.into_iter().last();

        CoordinatorOutput {
            dps_sample,
            logs,
        }
    }

    pub fn replay_logs(&mut self) {
        self.engine = EngineState::new();
        self.last_event_timestamp = None;
        self.last_event_wallclock = None;
        self.watcher.rewind_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_coordinator_flow() {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("20250101_120000.txt");
        
        let mut file = File::create(&log_path).unwrap();
        writeln!(file, "------------------------------------------------------------").unwrap();
        writeln!(file, "  Gamelog").unwrap();
        writeln!(file, "  Listener: TestChar").unwrap();
        writeln!(file, "  Session Started: 2025.01.01 12:00:00").unwrap();
        writeln!(file, "------------------------------------------------------------").unwrap();

        let mut coord = Coordinator::new(dir.path().to_path_buf());
        let mut active_paths = HashSet::new();
        active_paths.insert(log_path.clone());

        // Tick 1: Start tracking
        let output = coord.tick(&active_paths, Duration::from_secs(5));
        assert!(output.logs.iter().any(|m| m.contains("Started tracking")));
        assert!(output.dps_sample.is_none()); // No events yet

        // Write event
        writeln!(file, "[ 2025.01.01 12:01:00 ] (combat) 100 from TestChar to Enemy [ Gun ]").unwrap();
        file.sync_all().unwrap();

        // Tick 2: Read event
        let output = coord.tick(&active_paths, Duration::from_secs(5));
        // assert!(output.logs.iter().any(|m| m.contains("Read 1 new events")));
        
        let sample = output.dps_sample.unwrap();
        assert!(sample.outgoing_dps > 0.0);
        
        // Check character isolation
        let char_dps = sample.outgoing_by_character.get("TestChar").unwrap();
        assert!(*char_dps > 0.0);
    }
}
