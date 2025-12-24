use std::time::Duration;

use super::analysis;
use super::model::{CombatEvent, DpsSample};

pub struct EngineState {
    events: Vec<CombatEvent>,
    sorted: bool,
}

impl EngineState {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            sorted: true,
        }
    }

    pub fn push_event(&mut self, event: CombatEvent) {
        self.events.push(event);
        self.sorted = false;
    }

    pub fn push_events(&mut self, mut new_events: Vec<CombatEvent>) {
        if new_events.is_empty() {
            return;
        }
        self.events.append(&mut new_events);
        self.sorted = false;
    }

    pub fn events(&self) -> &[CombatEvent] {
        &self.events
    }

    pub fn total_damage(&self) -> f32 {
        self.events
            .iter()
            .filter(|event| !event.incoming)
            .map(|event| event.damage)
            .sum()
    }

    pub fn dps_series(&mut self, window: Duration, end: Duration) -> Vec<DpsSample> {
        if !self.sorted {
            self.events
                .sort_by_key(|event| event.timestamp.as_millis() as u64);
            self.sorted = true;
        }
        analysis::compute_dps_series(&self.events, window, end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::CombatEvent;

    fn make_event(timestamp_secs: u64, character: &str) -> CombatEvent {
        CombatEvent {
            timestamp: Duration::from_secs(timestamp_secs),
            source: "Source".to_string(),
            target: "Target".to_string(),
            weapon: "Weapon".to_string(),
            damage: 100.0,
            incoming: false,
            character: character.to_string(),
        }
    }

    #[test]
    fn engine_state_sorts_events_before_analysis() {
        let mut state = EngineState::new();
        state.push_event(make_event(10, "A"));
        state.push_event(make_event(5, "A"));

        assert!(!state.sorted);
        let _ = state.dps_series(Duration::from_secs(1), Duration::from_secs(10));
        assert!(state.sorted);
        assert_eq!(state.events[0].timestamp.as_secs(), 5);
        assert_eq!(state.events[1].timestamp.as_secs(), 10);
    }

    #[test]
    fn total_damage_sums_outgoing_only() {
        let mut state = EngineState::new();
        state.push_event(make_event(1, "A")); // Outgoing 100
        state.push_event(CombatEvent {
            incoming: true,
            ..make_event(2, "A")
        }); // Incoming 100

        assert_eq!(state.total_damage(), 100.0);
    }
}
