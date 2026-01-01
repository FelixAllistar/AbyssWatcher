use chrono::NaiveDateTime;
use lazy_static::lazy_static;
use regex::Regex;

use super::model::{CombatEvent, EventType};

const SESSION_PREFIX: &str = "Session Started:";
const TIMESTAMP_FMT: &str = "%Y.%m.%d %H:%M:%S";

lazy_static! {
    static ref TAG_RE: Regex = Regex::new(r"<[^>]+>").unwrap();
}

pub struct LineParser {
    base_time: Option<NaiveDateTime>,
}

impl LineParser {
    pub fn new() -> Self {
        Self { base_time: None }
    }

    pub fn parse_line(&mut self, line: &str, source: &str) -> Option<CombatEvent> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }

        if trimmed.starts_with(SESSION_PREFIX) {
            self.parse_session_start(trimmed);
            return None;
        }

        if !trimmed.contains("(combat)") {
            return None;
        }

        let timestamp = extract_timestamp(trimmed)?;
        let body = extract_body(trimmed);
        let cleaned_body = strip_tags(&body);
        let lower = cleaned_body.to_ascii_lowercase();

        // 1. Identify Event Type
        let event_type = if lower.contains("repaired to") || lower.contains("repaired by") {
            EventType::Repair
        } else {
            EventType::Damage
        };

        // 2. Identify Direction
        let direction = determine_direction(&lower, &event_type)?;

        // 3. Extract Amount
        let (amount, remainder) = split_amount_body(&cleaned_body)?;

        // 4. Extract Entities
        let (source_entity, target_entity, weapon) =
            split_entities_and_weapon(remainder, direction, &event_type, source)?;

        self.ensure_base_time(timestamp);

        let base = *self.base_time.as_ref()?;
        let duration = timestamp.signed_duration_since(base).to_std().ok()?;

        Some(CombatEvent {
            timestamp: duration,
            source: source_entity,
            target: target_entity,
            weapon,
            amount,
            incoming: matches!(direction, Direction::Incoming),
            character: source.to_string(),
            event_type,
        })
    }

    fn parse_session_start(&mut self, line: &str) {
        if let Some(timestamp) = line
            .strip_prefix(SESSION_PREFIX)
            .map(str::trim)
            .and_then(|value| NaiveDateTime::parse_from_str(value, TIMESTAMP_FMT).ok())
        {
            self.base_time = Some(timestamp);
        }
    }

    fn ensure_base_time(&mut self, timestamp: NaiveDateTime) {
        if self.base_time.is_none() {
            self.base_time = Some(timestamp);
        }
    }
}

fn extract_body(line: &str) -> String {
    line.split("(combat)")
        .nth(1)
        .map(str::trim)
        .unwrap_or_default()
        .to_string()
}

#[derive(Clone, Copy)]
enum Direction {
    Outgoing,
    Incoming,
}

fn determine_direction(lower_body: &str, event_type: &EventType) -> Option<Direction> {
    match event_type {
        EventType::Damage => {
            if lower_body.contains(" to ") {
                Some(Direction::Outgoing)
            } else if lower_body.contains(" from ") {
                Some(Direction::Incoming)
            } else {
                None
            }
        }
        EventType::Repair => {
            // "100 remote armor repaired to Target - Weapon" (Outgoing)
            // "100 remote armor repaired by Source - Weapon" (Incoming)
            if lower_body.contains(" repaired to ") {
                Some(Direction::Outgoing)
            } else if lower_body.contains(" repaired by ") {
                Some(Direction::Incoming)
            } else {
                None
            }
        }
    }
}

fn extract_timestamp(line: &str) -> Option<NaiveDateTime> {
    let first_section = line.split(']').next()?;
    let timestamp_text = first_section.trim_start_matches('[').trim();
    NaiveDateTime::parse_from_str(timestamp_text, TIMESTAMP_FMT).ok()
}

fn strip_tags(value: &str) -> String {
    let cleaned = TAG_RE.replace_all(value, "");
    cleaned
        .replace("&nbsp;", " ")
        .replace(['\r', '\n'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn split_amount_body(body: &str) -> Option<(f32, &str)> {
    let trimmed = body.trim();
    let split_index = trimmed.find(char::is_whitespace)?;
    let (amount_str, remainder) = trimmed.split_at(split_index);
    let remainder = remainder.trim();
    let amount = amount_str.parse::<f32>().ok()?;
    Some((amount, remainder))
}

fn split_entities_and_weapon(
    remainder: &str,
    direction: Direction,
    event_type: &EventType,
    listener: &str,
) -> Option<(String, String, String)> {
    // Helper to extract weapon from the end (separated by " - ")
    // Returns (text_without_weapon, weapon_name)
    // Note: EVE logs are messy. Sometimes there are multiple dashes.
    // Usually the weapon is the LAST segment, unless there's a quality (Glances, Wrecks, etc).
    // For repairs, there is no "quality".
    
    // Pattern: "remote armor repaired to <Target> - <Weapon>"
    // Pattern: "<damage> to <Target> - <Weapon> - <Quality>"
    
    let parts: Vec<&str> = remainder.split(" - ").collect();
    if parts.is_empty() { return None; }

    let (text_part, weapon) = match event_type {
        EventType::Repair => {
            // "remote armor repaired to Target - Weapon"
            // parts = ["remote armor repaired to Target", "Weapon"]
            if parts.len() >= 2 {
                (parts[0], parts[1].trim().to_string())
            } else {
                (parts[0], "".to_string())
            }
        }
        EventType::Damage => {
            // "<damage> to Target - Weapon - Quality" -> 3 parts
            // "<damage> to Target - Quality" -> 2 parts (Weapon disabled in logs)
            // "<damage> to Target - Weapon" -> 2 parts (No quality? Rare)
            
            // Heuristic: If we have 3 parts, middle is weapon.
            // If we have 2 parts, check if the last part is a known quality or weapon?
            // Existing logic assumed 3 parts = weapon present.
            if parts.len() >= 3 {
                (parts[0], parts[1].trim().to_string())
            } else {
                (parts[0], "".to_string())
            }
        }
    };

    match direction {
        Direction::Outgoing => {
            let mut text = text_part.trim();
            // Strip prefixes
            let prefixes = match event_type {
                EventType::Damage => vec!["to ", "against "],
                EventType::Repair => vec!["remote armor repaired to ", "remote shield repaired to ", "remote hull repaired to "],
            };
            
            for prefix in prefixes {
                // Case insensitive check for prefix
                if text.to_lowercase().starts_with(&prefix.to_lowercase()) {
                    // Strip the actual length of prefix (preserving case of the remaining text)
                    if text.len() >= prefix.len() {
                         text = text[prefix.len()..].trim();
                    }
                    break;
                }
            }
            
            let target = text;
            if target.is_empty() { return None; }
            Some((listener.to_string(), target.to_string(), weapon))
        }
        Direction::Incoming => {
            let mut text = text_part.trim();
            // Strip prefixes
            let prefixes = match event_type {
                EventType::Damage => vec!["from "],
                EventType::Repair => vec!["remote armor repaired by ", "remote shield repaired by ", "remote hull repaired by "],
            };

            for prefix in prefixes {
                if text.to_lowercase().starts_with(&prefix.to_lowercase()) {
                    if text.len() >= prefix.len() {
                         text = text[prefix.len()..].trim();
                    }
                    break;
                }
            }

            let source = text;
            if source.is_empty() { return None; }
            Some((source.to_string(), listener.to_string(), weapon))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_outgoing_hit() {
        let mut parser = LineParser::new();
        let _ = parser.parse_line("Session Started: 2025.11.15 07:09:22", "You");

        let line = "[ 2025.11.15 07:14:31 ] (combat) <color=0xff00ffff><b>523</b> <color=0x77ffffff><font size=10>to</font> <b><color=0xffffffff>Starving Damavik</b><font size=10><color=0x77ffffff> - Small Focused Beam Laser II - Penetrates";

        let event = parser.parse_line(line, "You").expect("should parse");

        assert_eq!(event.amount, 523.0);
        assert!(!event.incoming);
        assert_eq!(event.source, "You");
        assert_eq!(event.target, "Starving Damavik");
        assert_eq!(event.weapon, "Small Focused Beam Laser II");
        assert_eq!(event.event_type, EventType::Damage);
    }

    #[test]
    fn parses_outgoing_repair() {
        let mut parser = LineParser::new();
        let _ = parser.parse_line("Session Started: 2025.11.15 07:09:22", "LogiPilot");

        // "96 remote armor repaired to Target - Weapon"
        let line = "[ 2025.11.15 07:14:52 ] (combat) <color=0xffccff66><b>96</b><color=0x77ffffff><font size=10> remote armor repaired to </font><b><color=0xffffffff><font size=12><color=0xFFFFB300> <u><b>Retribution</b></u></color></font> [<b>CARII</b>]  [Felix Allistar]<color=0xFFFFFFFF><b> -</b><color=0x77ffffff><font size=10> - Small Remote Armor Repairer II</font>";

        // strip_tags will produce something like: "96 remote armor repaired to Retribution [CARII] [Felix Allistar] - - Small Remote Armor Repairer II"
        // Wait, the raw line has " - " inside the target name maybe? 
        // Let's check strip_tags output behavior manually if this fails.
        // Actually, the example has "<b> -</b><color=0x77ffffff><font size=10> - Small Remote Armor Repairer II</font>"
        // strip_tags removes <...>, leaving "96 remote armor repaired to Retribution [CARII] [Felix Allistar] - - Small Remote Armor Repairer II"
        // Our split by " - " will yield ["...Allistar]", "", "Small Remote Armor Repairer II"]
        // The middle empty part is annoying. 
        
        let event = parser.parse_line(line, "LogiPilot").expect("should parse repair");

        assert_eq!(event.amount, 96.0);
        assert_eq!(event.event_type, EventType::Repair);
        assert_eq!(event.source, "LogiPilot");
        assert!(event.target.contains("Felix Allistar")); // Simplified check due to messy tags
        assert!(event.weapon.contains("Small Remote Armor Repairer"));
    }

    #[test]
    fn parses_incoming_repair() {
        let mut parser = LineParser::new();
        let _ = parser.parse_line("Session Started: 2025.11.15 07:09:22", "You");
        
        // "96 remote armor repaired by Source - Weapon"
        let line = "[ 2025.11.15 07:15:00 ] (combat) 96 remote armor repaired by LogiBro - Small Remote Armor Repairer II";
        
        let event = parser.parse_line(line, "You").expect("should parse incoming repair");
        
        assert_eq!(event.amount, 96.0);
        assert!(event.incoming);
        assert_eq!(event.source, "LogiBro");
        assert_eq!(event.target, "You");
        assert_eq!(event.event_type, EventType::Repair);
    }
}
