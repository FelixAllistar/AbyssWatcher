use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use super::model::CombatEvent;
use super::parser::LineParser;

pub struct MergedStream {
    sources: Vec<LogSource>,
}

struct LogSource {
    reader: BufReader<File>,
    character: String,
    next_event: Option<(CombatEvent, String)>,
    parser: LineParser,
}

impl MergedStream {
    pub fn new(paths: Vec<(String, PathBuf)>) -> io::Result<Self> {
        let mut sources = Vec::new();
        for (character, path) in paths {
            let file = File::open(path)?;
            let mut reader = BufReader::new(file);
            let mut parser = LineParser::new();
            
            let next_event = read_next_event(&mut reader, &mut parser, &character);
            
            sources.push(LogSource {
                reader,
                character,
                next_event,
                parser,
            });
        }
        Ok(Self { sources })
    }

    pub fn next_event(&mut self) -> Option<(CombatEvent, String)> {
        let mut earliest_idx = None;
        let mut earliest_time = None;

        for (idx, source) in self.sources.iter().enumerate() {
            if let Some((event, _)) = &source.next_event {
                if earliest_time.is_none() || event.timestamp < earliest_time.unwrap() {
                    earliest_time = Some(event.timestamp);
                    earliest_idx = Some(idx);
                }
            }
        }

        if let Some(idx) = earliest_idx {
            let source = &mut self.sources[idx];
            let result = source.next_event.take();
            source.next_event = read_next_event(&mut source.reader, &mut source.parser, &source.character);
            result
        } else {
            None
        }
    }

    pub fn peek_time(&self) -> Option<Duration> {
        self.sources.iter()
            .filter_map(|s| s.next_event.as_ref().map(|(e, _)| e.timestamp))
            .min()
    }
}

fn read_next_event(reader: &mut BufReader<File>, parser: &mut LineParser, character: &str) -> Option<(CombatEvent, String)> {
    let mut line = String::new();
    while reader.read_line(&mut line).ok()? > 0 {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            if let Some(event) = parser.parse_line(trimmed, character) {
                return Some((event, trimmed.to_string()));
            }
        }
        line.clear();
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlaybackState {
    Playing,
    Paused,
}

pub struct ReplayController {
    stream_paths: Vec<(String, PathBuf)>,
    stream: MergedStream,
    state: PlaybackState,
    speed: f64,
    
    session_start_time: Duration,
    session_duration: Duration,
    session_epoch_start: u64,
    
    current_sim_time: Duration,
    last_update_wall_time: SystemTime,
}

impl ReplayController {
    pub fn new(paths: Vec<(String, PathBuf)>) -> Option<Self> {
        let stream = MergedStream::new(paths.clone()).ok()?;
        
        // Calculate absolute epoch start (earliest session start)
        let mut min_epoch = u64::MAX;
        for source in &stream.sources {
            if let Some(base) = source.parser.get_base_time() {
                 let epoch = base.and_utc().timestamp() as u64;
                 if epoch < min_epoch {
                     min_epoch = epoch;
                 }
            }
        }
        
        // If we found a valid session start, use it.
        // If we didn't find any session headers, we can't really replay.
        if min_epoch == u64::MAX {
            eprintln!("ReplayController: No valid session start found in headers.");
            return None;
        }

        // Try to peek first event time, OR default to 0 duration if no events
        let start_time = stream.peek_time().unwrap_or(Duration::ZERO);
        
        let mut end_time = start_time;
        for (_, path) in &paths {
            if let Ok(events) = super::log_io::read_full_events(path) {
                if let Some(last) = events.last() {
                    if last.timestamp > end_time {
                        end_time = last.timestamp;
                    }
                }
            }
        }

        Some(Self {
            stream_paths: paths,
            stream,
            state: PlaybackState::Paused,
            speed: 1.0,
            session_start_time: start_time,
            session_duration: end_time.saturating_sub(start_time),
            session_epoch_start: min_epoch,
            current_sim_time: start_time,
            last_update_wall_time: SystemTime::now(),
        })
    }

    pub fn seek(&mut self, offset: Duration) -> io::Result<()> {
        self.stream = MergedStream::new(self.stream_paths.clone())?;
        self.current_sim_time = self.session_start_time + offset;
        self.last_update_wall_time = SystemTime::now();
        Ok(())
    }

    pub fn session_duration(&self) -> Duration {
        self.session_duration
    }

    pub fn start_time(&self) -> Duration {
        // Return absolute epoch time + relative start offset
        // This gives the exact time of the FIRST EVENT in the replay
        Duration::from_secs(self.session_epoch_start) + self.session_start_time
    }

    pub fn set_state(&mut self, state: PlaybackState) {
        self.last_update_wall_time = SystemTime::now();
        self.state = state;
    }

    pub fn get_state(&self) -> PlaybackState {
        self.state
    }

    pub fn set_speed(&mut self, speed: f64) {
        self.speed = speed;
    }

    pub fn step(&mut self, delta: Duration) {
        self.current_sim_time += delta;
        self.last_update_wall_time = SystemTime::now(); // Reset wall clock to prevent 'jump' if play resumed
    }

    pub fn tick(&mut self) -> (Vec<CombatEvent>, Vec<String>) {
        let now = SystemTime::now();
        let elapsed_wall = now.duration_since(self.last_update_wall_time).unwrap_or(Duration::ZERO);
        self.last_update_wall_time = now;

        if self.state == PlaybackState::Paused {
            return (Vec::new(), Vec::new());
        }

        let elapsed_sim = Duration::from_secs_f64(elapsed_wall.as_secs_f64() * self.speed);
        self.current_sim_time += elapsed_sim;

        let mut events = Vec::new();
        let mut lines = Vec::new();
        while let Some(next_time) = self.stream.peek_time() {
            if next_time <= self.current_sim_time {
                if let Some((event, line)) = self.stream.next_event() {
                    events.push(event);
                    lines.push(line);
                }
            } else {
                break;
            }
        }
        (events, lines)
    }

    pub fn current_sim_time(&self) -> Duration {
        self.current_sim_time
    }
    
    pub fn relative_progress(&self) -> Duration {
        self.current_sim_time.saturating_sub(self.session_start_time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_merged_stream_chronological() {
        let dir = tempdir().unwrap();
        
        let path_a = dir.path().join("A.txt");
        let mut f_a = File::create(&path_a).unwrap();
        writeln!(f_a, "[ 2024.01.01 12:00:00 ] (combat) 10 from A to X [ Gun ]").unwrap();
        writeln!(f_a, "[ 2024.01.01 12:00:10 ] (combat) 10 from A to X [ Gun ]").unwrap();
        
        let path_b = dir.path().join("B.txt");
        let mut f_b = File::create(&path_b).unwrap();
        writeln!(f_b, "[ 2024.01.01 12:00:05 ] (combat) 10 from B to X [ Gun ]").unwrap();

        let mut stream = MergedStream::new(vec![
            ("CharA".to_string(), path_a),
            ("CharB".to_string(), path_b),
        ]).unwrap();

        let e1 = stream.next_event().unwrap();
        assert_eq!(e1.0.source, "CharA");
        
        let e2 = stream.next_event().unwrap();
        assert_eq!(e2.0.source, "CharB");
        
        let e3 = stream.next_event().unwrap();
        assert_eq!(e3.0.source, "CharA");
        
        assert!(stream.next_event().is_none());
    }

    #[test]
    fn test_replay_controller_speed() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("log.txt");
        let mut f = File::create(&path).unwrap();
        writeln!(f, "[ 2024.01.01 12:00:00 ] (combat) 10 from A to X [ Gun ]").unwrap();
        writeln!(f, "[ 2024.01.01 12:00:01 ] (combat) 10 from A to X [ Gun ]").unwrap();

        let mut ctrl = ReplayController::new(vec![("A".to_string(), path)]).unwrap();
        
        ctrl.set_state(PlaybackState::Playing);
        let events = ctrl.tick();
        assert_eq!(events.0.len(), 1);
        
        ctrl.set_speed(10.0);
        std::thread::sleep(Duration::from_millis(150)); 
        let events = ctrl.tick();
        assert_eq!(events.0.len(), 1);
    }
}
