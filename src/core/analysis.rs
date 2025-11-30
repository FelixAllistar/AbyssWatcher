use std::collections::HashMap;
use std::time::Duration;

use super::model::{CombatEvent, DpsSample, EntityName, WeaponName};

pub fn compute_dps_series(
    events: &[CombatEvent],
    window: Duration,
    end: Duration,
) -> Vec<DpsSample> {
    if events.is_empty() {
        return Vec::new();
    }

    let window_millis = window.as_millis().max(1) as u64;
    let step_millis: u64 = 1_000;

    let max_event_timestamp_millis = events
        .iter()
        .map(|event| event.timestamp.as_millis() as u64)
        .max()
        .unwrap_or(0);
    let end_millis = end.as_millis() as u64;
    let max_millis = std::cmp::max(max_event_timestamp_millis, end_millis);
    let slot_count = (max_millis / step_millis + 1) as usize;

    let window_seconds = window.as_secs_f32().max(f32::EPSILON);

    let mut samples = Vec::with_capacity(slot_count);
    for index in 0..slot_count {
        let time = Duration::from_millis(index as u64 * step_millis);
        samples.push(DpsSample {
            time,
            outgoing_dps: 0.0,
            incoming_dps: 0.0,
            outgoing_by_weapon: HashMap::<WeaponName, f32>::new(),
            outgoing_by_target: HashMap::<EntityName, f32>::new(),
            incoming_by_source: HashMap::<EntityName, f32>::new(),
        });
    }

    let mut events_sorted: Vec<&CombatEvent> = events.iter().collect();
    events_sorted.sort_by_key(|event| event.timestamp.as_millis() as u64);

    let mut start_idx: usize = 0;
    let mut end_idx: usize = 0;

    let mut outgoing_sum = 0.0_f32;
    let mut incoming_sum = 0.0_f32;
    let mut outgoing_by_weapon_damage: HashMap<WeaponName, f32> = HashMap::new();
    let mut outgoing_by_target_damage: HashMap<EntityName, f32> = HashMap::new();
    let mut incoming_by_source_damage: HashMap<EntityName, f32> = HashMap::new();

    for (i, sample) in samples.iter_mut().enumerate() {
        let center_millis = i as u64 * step_millis;
        let window_start_millis = center_millis.saturating_sub(window_millis);

        while end_idx < events_sorted.len()
            && events_sorted[end_idx].timestamp.as_millis() as u64 <= center_millis
        {
            let event = events_sorted[end_idx];
            if event.incoming {
                incoming_sum += event.damage;
                *incoming_by_source_damage
                    .entry(event.source.clone())
                    .or_insert(0.0) += event.damage;
            } else {
                outgoing_sum += event.damage;
                *outgoing_by_weapon_damage
                    .entry(event.weapon.clone())
                    .or_insert(0.0) += event.damage;
                *outgoing_by_target_damage
                    .entry(event.target.clone())
                    .or_insert(0.0) += event.damage;
            }
            end_idx += 1;
        }

        while start_idx < end_idx
            && (events_sorted[start_idx].timestamp.as_millis() as u64) < window_start_millis
        {
            let event = events_sorted[start_idx];
            if event.incoming {
                incoming_sum -= event.damage;
                if let Some(value) = incoming_by_source_damage.get_mut(&event.source) {
                    *value -= event.damage;
                    if *value <= 0.0 {
                        incoming_by_source_damage.remove(&event.source);
                    }
                }
            } else {
                outgoing_sum -= event.damage;
                if let Some(value) = outgoing_by_weapon_damage.get_mut(&event.weapon) {
                    *value -= event.damage;
                    if *value <= 0.0 {
                        outgoing_by_weapon_damage.remove(&event.weapon);
                    }
                }
                if let Some(value) = outgoing_by_target_damage.get_mut(&event.target) {
                    *value -= event.damage;
                    if *value <= 0.0 {
                        outgoing_by_target_damage.remove(&event.target);
                    }
                }
            }
            start_idx += 1;
        }

        sample.outgoing_dps = outgoing_sum / window_seconds;
        sample.incoming_dps = incoming_sum / window_seconds;

        sample.outgoing_by_weapon = outgoing_by_weapon_damage
            .iter()
            .map(|(weapon, damage)| (weapon.clone(), damage / window_seconds))
            .collect();
        sample.outgoing_by_target = outgoing_by_target_damage
            .iter()
            .map(|(target, damage)| (target.clone(), damage / window_seconds))
            .collect();
        sample.incoming_by_source = incoming_by_source_damage
            .iter()
            .map(|(source, damage)| (source.clone(), damage / window_seconds))
            .collect();
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

        let samples = compute_dps_series(
            &events,
            Duration::from_secs(1),
            Duration::from_secs(3),
        );

        assert_eq!(samples.len(), 4);
        assert!(samples[3].outgoing_dps > 0.0, "latest timestamp should fill slot 3");
        assert!(samples[1].outgoing_dps > 0.0, "middle slot should also exist");
    }
}
