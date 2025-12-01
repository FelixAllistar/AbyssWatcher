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
