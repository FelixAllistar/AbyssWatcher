use std::collections::HashMap;
use std::time::Duration;

use super::model::{CombatEvent, DpsSample, EntityName, WeaponName};

pub fn compute_dps_series(events: &[CombatEvent], window: Duration) -> Vec<DpsSample> {
    if events.is_empty() {
        return Vec::new();
    }

    let window_millis = window.as_millis().max(1) as u64;
    let last_timestamp_millis = events
        .last()
        .map(|event| event.timestamp.as_millis() as u64)
        .unwrap_or(0);
    let slot_count = (last_timestamp_millis / window_millis + 1) as usize;

    let mut samples = Vec::with_capacity(slot_count);
    for index in 0..slot_count {
        let time = Duration::from_millis(index as u64 * window_millis);
        samples.push(DpsSample {
            time,
            total_dps: 0.0,
            by_weapon: HashMap::<WeaponName, f32>::new(),
            by_target: HashMap::<EntityName, f32>::new(),
        });
    }

    let window_seconds = window.as_secs_f32().max(f32::EPSILON);

    for event in events {
        let timestamp_millis = event.timestamp.as_millis() as u64;
        let slot_index = (timestamp_millis / window_millis) as usize;
        if let Some(sample) = samples.get_mut(slot_index) {
            let dps_contribution = event.damage / window_seconds;
            sample.total_dps += dps_contribution;
            *sample
                .by_weapon
                .entry(event.weapon.clone())
                .or_insert(0.0) += dps_contribution;
            *sample
                .by_target
                .entry(event.target.clone())
                .or_insert(0.0) += dps_contribution;
        }
    }

    samples
}

