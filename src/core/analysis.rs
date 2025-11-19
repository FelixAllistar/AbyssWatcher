use std::collections::HashMap;
use std::time::Duration;

use super::model::{CombatEvent, DpsSample, EntityName, WeaponName};

pub fn compute_dps_series(events: &[CombatEvent], window: Duration) -> Vec<DpsSample> {
    if events.is_empty() {
        return Vec::new();
    }

    let window_millis = window.as_millis().max(1) as u64;
    let max_timestamp_millis = events
        .iter()
        .map(|event| event.timestamp.as_millis() as u64)
        .max()
        .unwrap_or(0);
    let slot_count = (max_timestamp_millis / window_millis + 1) as usize;

    let mut samples = Vec::with_capacity(slot_count);
    for index in 0..slot_count {
        let time = Duration::from_millis(index as u64 * window_millis);
        samples.push(DpsSample {
            time,
            outgoing_dps: 0.0,
            incoming_dps: 0.0,
            outgoing_by_weapon: HashMap::<WeaponName, f32>::new(),
            outgoing_by_target: HashMap::<EntityName, f32>::new(),
            incoming_by_source: HashMap::<EntityName, f32>::new(),
        });
    }

    let window_seconds = window.as_secs_f32().max(f32::EPSILON);

    for event in events {
        let timestamp_millis = event.timestamp.as_millis() as u64;
        let slot_index = (timestamp_millis / window_millis) as usize;
        if let Some(sample) = samples.get_mut(slot_index) {
            let dps_contribution = event.damage / window_seconds;
            if event.incoming {
                sample.incoming_dps += dps_contribution;
                *sample
                    .incoming_by_source
                    .entry(event.source.clone())
                    .or_insert(0.0) += dps_contribution;
            } else {
                sample.outgoing_dps += dps_contribution;
                *sample
                    .outgoing_by_weapon
                    .entry(event.weapon.clone())
                    .or_insert(0.0) += dps_contribution;
                *sample
                    .outgoing_by_target
                    .entry(event.target.clone())
                    .or_insert(0.0) += dps_contribution;
            }
        }
    }

    samples
}

#[cfg(test)]
mod tests {
    use super::super::model::CombatEvent;
    use super::*;
    use std::time::Duration;

    fn make_event(seconds: u64, damage: f32, incoming: bool, source: &str, target: &str) -> CombatEvent {
        CombatEvent {
            timestamp: Duration::from_secs(seconds),
            source: source.to_string(),
            target: target.to_string(),
            weapon: "Test".to_string(),
            damage,
            incoming,
        }
    }

    #[test]
    fn keeps_slot_for_max_timestamp_even_if_unsorted() {
        let events = vec![
            make_event(3, 100.0, false, "Pilot", "Enemy"),
            make_event(1, 50.0, false, "Pilot", "Enemy"),
        ];

        let samples = compute_dps_series(&events, Duration::from_secs(1));

        assert_eq!(samples.len(), 4);
        assert!(samples[3].outgoing_dps > 0.0, "latest timestamp should fill slot 3");
        assert!(samples[1].outgoing_dps > 0.0, "middle slot should also exist");
    }
}
