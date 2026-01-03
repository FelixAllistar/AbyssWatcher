use std::collections::HashMap;
use std::time::Duration;

use super::model::{CombatEvent, DpsSample, EntityName, EventType, WeaponName};

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

    // Keep at most this much history in the DPS series.
    const HISTORY_MILLIS: u64 = 60_000;

    let (start_millis, slot_count) = if max_millis <= HISTORY_MILLIS {
        (0, (max_millis / step_millis + 1) as usize)
    } else {
        let start = max_millis - HISTORY_MILLIS;
        let slots = (HISTORY_MILLIS / step_millis + 1) as usize;
        (start, slots)
    };

    let window_seconds = window.as_secs_f32().max(f32::EPSILON);

    let mut samples = Vec::with_capacity(slot_count);
    for index in 0..slot_count {
        let time = Duration::from_millis(start_millis + index as u64 * step_millis);
        samples.push(DpsSample {
            time,
            outgoing_dps: 0.0,
            incoming_dps: 0.0,
            outgoing_hps: 0.0,
            incoming_hps: 0.0,
            outgoing_cap: 0.0,
            incoming_cap: 0.0,
            outgoing_neut: 0.0,
            incoming_neut: 0.0,
            outgoing_by_weapon: HashMap::<WeaponName, f32>::new(),
            outgoing_by_target: HashMap::<EntityName, f32>::new(),
            incoming_by_source: HashMap::<EntityName, f32>::new(),
            outgoing_by_character: HashMap::<String, f32>::new(),
            incoming_by_character: HashMap::<String, f32>::new(),
            outgoing_by_char_weapon: HashMap::<String, HashMap<WeaponName, f32>>::new(),
            outgoing_by_char_target: HashMap::<String, HashMap<EntityName, f32>>::new(),
            combat_actions_by_character: HashMap::<String, Vec<super::model::CombatAction>>::new(),
        });
    }

    let global_start_cutoff = Duration::from_millis(start_millis.saturating_sub(window_millis));
    
    let mut start_idx = events.partition_point(|e| e.timestamp < global_start_cutoff);
    let mut end_idx = start_idx;

    let mut outgoing_sum = 0.0_f32;
    let mut incoming_sum = 0.0_f32;
    let mut outgoing_hps_sum = 0.0_f32;
    let mut incoming_hps_sum = 0.0_f32;
    let mut outgoing_cap_sum = 0.0_f32;
    let mut incoming_cap_sum = 0.0_f32;
    let mut outgoing_neut_sum = 0.0_f32;
    let mut incoming_neut_sum = 0.0_f32;
    
    let mut outgoing_by_weapon_damage: HashMap<WeaponName, f32> = HashMap::new();
    let mut outgoing_by_target_damage: HashMap<EntityName, f32> = HashMap::new();
    let mut incoming_by_source_damage: HashMap<EntityName, f32> = HashMap::new();
    let mut incoming_by_character_damage: HashMap<String, f32> = HashMap::new();
    let mut outgoing_by_character_damage: HashMap<String, f32> = HashMap::new();
    let mut outgoing_by_char_weapon_damage: HashMap<String, HashMap<WeaponName, f32>> =
        HashMap::new();
    let mut outgoing_by_char_target_damage: HashMap<String, HashMap<EntityName, f32>> =
        HashMap::new();
    let mut char_actions_map: HashMap<String, HashMap<(String, EventType, bool), f32>> =
        HashMap::new();

    for (i, sample) in samples.iter_mut().enumerate() {
        let center_millis = start_millis + i as u64 * step_millis;
        let window_start_millis = center_millis.saturating_sub(window_millis);

        // Add events entering the window (from the future relative to window start)
        while end_idx < events.len()
            && events[end_idx].timestamp.as_millis() as u64 <= center_millis
        {
            let event = &events[end_idx];
            if event.incoming {
                match event.event_type {
                    EventType::Damage => {
                        incoming_sum += event.amount;
                        *incoming_by_source_damage
                            .entry(event.source.clone())
                            .or_insert(0.0) += event.amount;
                        *incoming_by_character_damage
                            .entry(event.character.clone())
                            .or_insert(0.0) += event.amount;
                    },
                    EventType::Repair => incoming_hps_sum += event.amount,
                    EventType::Capacitor => incoming_cap_sum += event.amount,
                    EventType::Neut => incoming_neut_sum += event.amount,
                }
                *char_actions_map
                    .entry(event.character.clone())
                    .or_default()
                    .entry((event.weapon.clone(), event.event_type.clone(), true))
                    .or_insert(0.0) += event.amount;
            } else {
                // Outgoing logic
                *char_actions_map
                    .entry(event.character.clone())
                    .or_default()
                    .entry((event.weapon.clone(), event.event_type.clone(), false))
                    .or_insert(0.0) += event.amount;

                match event.event_type {
                    EventType::Damage => {
                        outgoing_sum += event.amount;
                        *outgoing_by_weapon_damage
                            .entry(event.weapon.clone())
                            .or_insert(0.0) += event.amount;
                        *outgoing_by_target_damage
                            .entry(event.target.clone())
                            .or_insert(0.0) += event.amount;
                        *outgoing_by_character_damage
                            .entry(event.character.clone())
                            .or_insert(0.0) += event.amount;
                        *outgoing_by_char_weapon_damage
                            .entry(event.character.clone())
                            .or_default()
                            .entry(event.weapon.clone())
                            .or_insert(0.0) += event.amount;
                        *outgoing_by_char_target_damage
                            .entry(event.character.clone())
                            .or_default()
                            .entry(event.target.clone())
                            .or_insert(0.0) += event.amount;
                    },
                    EventType::Repair => {
                        outgoing_hps_sum += event.amount;
                    },
                    EventType::Capacitor => {
                        outgoing_cap_sum += event.amount;
                    },
                    EventType::Neut => {
                        outgoing_neut_sum += event.amount;
                    }
                }
            }
            end_idx += 1;
        }

        // Remove events leaving the window (falling behind the start time)
        while start_idx < end_idx
            && (events[start_idx].timestamp.as_millis() as u64) < window_start_millis
        {
            let event = &events[start_idx];

            // Clean up combat actions map - runs for BOTH incoming and outgoing events
            if let Some(char_actions) = char_actions_map.get_mut(&event.character) {
                let key = (event.weapon.clone(), event.event_type.clone(), event.incoming);
                if let Some(val) = char_actions.get_mut(&key) {
                    *val -= event.amount;
                    if *val <= 0.0 {
                        char_actions.remove(&key);
                    }
                }
                if char_actions.is_empty() {
                    char_actions_map.remove(&event.character);
                }
            }

            if event.incoming {
                match event.event_type {
                    EventType::Damage => {
                        incoming_sum -= event.amount;
                        if let Some(value) = incoming_by_source_damage.get_mut(&event.source) {
                            *value -= event.amount;
                            if *value <= 0.0 {
                                incoming_by_source_damage.remove(&event.source);
                            }
                        }
                        if let Some(value) = incoming_by_character_damage.get_mut(&event.character) {
                            *value -= event.amount;
                            if *value <= 0.0 {
                                incoming_by_character_damage.remove(&event.character);
                            }
                        }
                    },
                    EventType::Repair => incoming_hps_sum -= event.amount,
                    EventType::Capacitor => incoming_cap_sum -= event.amount,
                    EventType::Neut => incoming_neut_sum -= event.amount,
                }
            } else {
                match event.event_type {
                    EventType::Damage => {
                        outgoing_sum -= event.amount;
                        if let Some(value) = outgoing_by_weapon_damage.get_mut(&event.weapon) {
                            *value -= event.amount;
                            if *value <= 0.0 {
                                outgoing_by_weapon_damage.remove(&event.weapon);
                            }
                        }
                        if let Some(value) = outgoing_by_target_damage.get_mut(&event.target) {
                            *value -= event.amount;
                            if *value <= 0.0 {
                                outgoing_by_target_damage.remove(&event.target);
                            }
                        }
                        if let Some(value) = outgoing_by_character_damage.get_mut(&event.character) {
                            *value -= event.amount;
                            if *value <= 0.0 {
                                outgoing_by_character_damage.remove(&event.character);
                            }
                        }
                        if let Some(char_weapons) = outgoing_by_char_weapon_damage.get_mut(&event.character)
                        {
                            if let Some(damage) = char_weapons.get_mut(&event.weapon) {
                                *damage -= event.amount;
                                if *damage <= 0.0 {
                                    char_weapons.remove(&event.weapon);
                                }
                            }
                            if char_weapons.is_empty() {
                                outgoing_by_char_weapon_damage.remove(&event.character);
                            }
                        }
                        if let Some(char_targets) = outgoing_by_char_target_damage.get_mut(&event.character)
                        {
                            if let Some(damage) = char_targets.get_mut(&event.target) {
                                *damage -= event.amount;
                                if *damage <= 0.0 {
                                    char_targets.remove(&event.target);
                                }
                            }
                            if char_targets.is_empty() {
                                outgoing_by_char_target_damage.remove(&event.character);
                            }
                        }
                    },
                    EventType::Repair => {
                        outgoing_hps_sum -= event.amount;
                    },
                    EventType::Capacitor => {
                        outgoing_cap_sum -= event.amount;
                    },
                    EventType::Neut => {
                        outgoing_neut_sum -= event.amount;
                    }
                }
            }
            start_idx += 1;
        }

        sample.outgoing_dps = outgoing_sum / window_seconds;
        sample.incoming_dps = incoming_sum / window_seconds;
        sample.outgoing_hps = outgoing_hps_sum / window_seconds;
        sample.incoming_hps = incoming_hps_sum / window_seconds;
        sample.outgoing_cap = outgoing_cap_sum / window_seconds;
        sample.incoming_cap = incoming_cap_sum / window_seconds;
        sample.outgoing_neut = outgoing_neut_sum / window_seconds;
        sample.incoming_neut = incoming_neut_sum / window_seconds;

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
        sample.outgoing_by_character = outgoing_by_character_damage
            .iter()
            .map(|(character, damage)| (character.clone(), damage / window_seconds))
            .collect();
        sample.incoming_by_character = incoming_by_character_damage
            .iter()
            .map(|(character, damage)| (character.clone(), damage / window_seconds))
            .collect();
        sample.outgoing_by_char_weapon = outgoing_by_char_weapon_damage
            .iter()
            .map(|(character, weapons)| {
                (
                    character.clone(),
                    weapons
                        .iter()
                        .map(|(weapon, damage)| (weapon.clone(), damage / window_seconds))
                        .collect(),
                )
            })
            .collect();
        sample.outgoing_by_char_target = outgoing_by_char_target_damage
            .iter()
            .map(|(character, targets)| {
                (
                    character.clone(),
                    targets
                        .iter()
                        .map(|(target, damage)| (target.clone(), damage / window_seconds))
                        .collect(),
                )
            })
            .collect();

        sample.combat_actions_by_character = char_actions_map
            .iter()
            .map(|(character, actions)| {
                (
                    character.clone(),
                    actions
                        .iter()
                        .map(|((name, action_type, incoming), value)| super::model::CombatAction {
                            name: name.clone(),
                            action_type: action_type.clone(),
                            value: value / window_seconds,
                            incoming: *incoming,
                        })
                        .collect(),
                )
            })
            .collect();
    }

    samples
}

#[cfg(test)]
mod tests {
    use super::super::model::CombatEvent;
    use super::*;
    use std::time::Duration;

    fn make_event(
        seconds: u64,
        amount: f32,
        incoming: bool,
        source: &str,
        target: &str,
    ) -> CombatEvent {
        CombatEvent {
            timestamp: Duration::from_secs(seconds),
            source: source.to_string(),
            target: target.to_string(),
            weapon: "Test".to_string(),
            amount,
            incoming,
            character: source.to_string(),
            event_type: EventType::Damage,
        }
    }

    #[test]
    fn keeps_slot_for_max_timestamp_even_if_unsorted() {
        let mut events = vec![
            make_event(3, 100.0, false, "Pilot", "Enemy"),
            make_event(1, 50.0, false, "Pilot", "Enemy"),
        ];
        events.sort_by_key(|event| event.timestamp.as_millis());

        let samples = compute_dps_series(&events, Duration::from_secs(1), Duration::from_secs(3));

        assert_eq!(samples.len(), 4);
        assert!(
            samples[3].outgoing_dps > 0.0,
            "latest timestamp should fill slot 3"
        );
        assert!(
            samples[1].outgoing_dps > 0.0,
            "middle slot should also exist"
        );
    }

    #[test]
    fn isolates_stats_per_character() {
        let mut events = vec![
            CombatEvent {
                timestamp: Duration::from_secs(1),
                source: "PilotA".to_string(),
                target: "Enemy1".to_string(),
                weapon: "Lasers".to_string(),
                amount: 100.0,
                incoming: false,
                character: "PilotA".to_string(),
                event_type: EventType::Damage,
            },
            CombatEvent {
                timestamp: Duration::from_secs(1),
                source: "PilotB".to_string(),
                target: "Enemy2".to_string(),
                weapon: "Missiles".to_string(),
                amount: 50.0,
                incoming: false,
                character: "PilotB".to_string(),
                event_type: EventType::Damage,
            },
        ];
        events.sort_by_key(|event| event.timestamp.as_millis());

        let samples = compute_dps_series(&events, Duration::from_secs(1), Duration::from_secs(1));
        let sample = &samples[1]; // slot at t=1s

        // Verify global totals
        assert_eq!(sample.outgoing_dps, 150.0);

        // Verify PilotA isolation
        let a_weapons = sample.outgoing_by_char_weapon.get("PilotA").unwrap();
        assert!(a_weapons.contains_key("Lasers"));
        assert!(!a_weapons.contains_key("Missiles"));

        let a_targets = sample.outgoing_by_char_target.get("PilotA").unwrap();
        assert!(a_targets.contains_key("Enemy1"));
        assert!(!a_targets.contains_key("Enemy2"));

        // Verify PilotB isolation
        let b_weapons = sample.outgoing_by_char_weapon.get("PilotB").unwrap();
        assert!(b_weapons.contains_key("Missiles"));
        assert!(!b_weapons.contains_key("Lasers"));

        let b_targets = sample.outgoing_by_char_target.get("PilotB").unwrap();
        assert!(b_targets.contains_key("Enemy2"));
        assert!(!b_targets.contains_key("Enemy1"));
    }

    #[test]
    fn calculates_hps_separately() {
        let mut events = vec![
            // Damage Event
            make_event(1, 100.0, false, "Pilot", "Enemy"),
            // Repair Event
            CombatEvent {
                timestamp: Duration::from_secs(1),
                source: "Pilot".to_string(),
                target: "Friend".to_string(),
                weapon: "Remote Rep".to_string(),
                amount: 50.0,
                incoming: false,
                character: "Pilot".to_string(),
                event_type: EventType::Repair,
            }
        ];
        
        let samples = compute_dps_series(&events, Duration::from_secs(1), Duration::from_secs(1));
        let sample = &samples[1];

        assert_eq!(sample.outgoing_dps, 100.0);
        assert_eq!(sample.outgoing_hps, 50.0);
    }

    #[test]
    fn aggregates_unified_combat_actions() {
        let events = vec![
            CombatEvent {
                timestamp: Duration::from_secs(1),
                source: "Pilot".to_string(),
                target: "Enemy".to_string(),
                weapon: "Laser".to_string(),
                amount: 100.0,
                incoming: false,
                character: "Pilot".to_string(),
                event_type: EventType::Damage,
            },
            CombatEvent {
                timestamp: Duration::from_secs(1),
                source: "Pilot".to_string(),
                target: "Friend".to_string(),
                weapon: "Rep".to_string(),
                amount: 50.0,
                incoming: false,
                character: "Pilot".to_string(),
                event_type: EventType::Repair,
            },
            CombatEvent {
                timestamp: Duration::from_secs(1),
                source: "Pilot".to_string(),
                target: "Enemy".to_string(),
                weapon: "Neut".to_string(),
                amount: 20.0,
                incoming: false,
                character: "Pilot".to_string(),
                event_type: EventType::Neut,
            },
        ];

        let samples = compute_dps_series(&events, Duration::from_secs(1), Duration::from_secs(1));
        let sample = &samples[1];

        let actions = sample.combat_actions_by_character.get("Pilot").unwrap();
        assert_eq!(actions.len(), 3);

        let laser = actions.iter().find(|a| a.name == "Laser").unwrap();
        assert_eq!(laser.action_type, EventType::Damage);
        assert_eq!(laser.value, 100.0);

        let rep = actions.iter().find(|a| a.name == "Rep").unwrap();
        assert_eq!(rep.action_type, EventType::Repair);
        assert_eq!(rep.value, 50.0);

        let neut = actions.iter().find(|a| a.name == "Neut").unwrap();
        assert_eq!(neut.action_type, EventType::Neut);
        assert_eq!(neut.value, 20.0);
    }

    #[test]
    fn expires_incoming_events_from_actions_map() {
        // Create an incoming damage event at t=1s with a 1s window
        let events = vec![
            CombatEvent {
                timestamp: Duration::from_secs(1),
                source: "Enemy".to_string(),
                target: "Pilot".to_string(),
                weapon: "NPC Attack".to_string(),
                amount: 100.0,
                incoming: true,
                character: "Pilot".to_string(),
                event_type: EventType::Damage,
            },
        ];

        // Sample at t=3s (2 seconds after event, window is 1s)
        let samples = compute_dps_series(&events, Duration::from_secs(1), Duration::from_secs(3));
        let sample_at_3s = &samples[3];

        // The incoming event should have expired from the window
        assert!(
            !sample_at_3s.combat_actions_by_character.contains_key("Pilot"),
            "Incoming events should be removed from actions map after window expires"
        );
        assert_eq!(
            sample_at_3s.incoming_dps, 0.0,
            "Incoming DPS should be 0 after window expires"
        );
    }
}
