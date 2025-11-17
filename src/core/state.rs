use std::time::Duration;

use super::analysis;
use super::model::{CombatEvent, DpsSample};

pub struct EngineState {
    events: Vec<CombatEvent>,
}

impl EngineState {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn push_event(&mut self, event: CombatEvent) {
        self.events.push(event);
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

    pub fn dps_series(&self, window: Duration) -> Vec<DpsSample> {
        analysis::compute_dps_series(&self.events, window)
    }
}
