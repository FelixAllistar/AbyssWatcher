use std::time::Duration;

use super::model::{CombatEvent, EntityName, WeaponName};

pub fn parse_line(line: &str) -> Option<CombatEvent> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let parts: Vec<_> = trimmed.split(',').map(str::trim).collect();
    if parts.len() != 5 {
        return None;
    }

    let timestamp_millis: u64 = parts[0].parse().ok()?;
    let damage: f32 = parts[4].parse().ok()?;

    let source: EntityName = parts[1].to_string();
    let target: EntityName = parts[2].to_string();
    let weapon: WeaponName = parts[3].to_string();

    Some(CombatEvent {
        timestamp: Duration::from_millis(timestamp_millis),
        source,
        target,
        weapon,
        damage,
    })
}

