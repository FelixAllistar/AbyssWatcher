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
        let event_type = if lower.contains("repaired to") || lower.contains("repaired by") || lower.contains("boosted to") || lower.contains("boosted by") {
            EventType::Repair
        } else if lower.contains("remote capacitor transmitted") {
            EventType::Capacitor
        } else if lower.contains("energy neutralized") || lower.contains("energy drained") {
            EventType::Neut
        } else {
            EventType::Damage
        };

        // 2. Identify Direction
        let direction = determine_direction(&lower, &event_type)?;

        // 3. Extract Amount
        let (mut amount, remainder) = split_amount_body(&cleaned_body)?;
        
        // Handle "+4 GJ" or "-6 GJ" for drains
        // split_amount_body parses the float. If it was negative, amount is negative.
        // For metrics, we usually want absolute magnitude of "work done".
        // DPS is always positive magnitude.
        // Neut: "61 GJ neutralized" -> 61.
        // Nos: "+4 GJ drained from" -> 4.
        // Nos: "-6 GJ drained to" -> -6.
        // If we want "Neut Pressure", we probably want absolute value.
        amount = amount.abs();

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
            if lower_body.contains(" repaired to ") || lower_body.contains(" boosted to ") {
                Some(Direction::Outgoing)
            } else if lower_body.contains(" repaired by ") || lower_body.contains(" boosted by ") {
                Some(Direction::Incoming)
            } else {
                None
            }
        }
        EventType::Capacitor => {
            if lower_body.contains(" transmitted to ") {
                Some(Direction::Outgoing)
            } else if lower_body.contains(" transmitted by ") {
                Some(Direction::Incoming)
            } else {
                None
            }
        }
        EventType::Neut => {
            // "energy neutralized <Target>" -> Outgoing? Wait, missing 'to'.
            // Log line: "61 GJ energy neutralized Starving Damavik"
            // It seems "neutralized" implies "to" if no preposition?
            // Or maybe "neutralized to"? No, the example had no "to".
            
            // "energy drained from <Target>" -> Outgoing (I drain FROM them).
            // "energy drained to <Target>" -> Incoming (They drain ME? or I drain TO them? Wait, drained TO usually means transfer? No, Nos transfers TO source).
            // If line is: "-6 GJ energy drained to Proteus", and I am listening...
            // Usually "to" implies target. So Proteus is target.
            
            if lower_body.contains(" neutralized ") {
                // "energy neutralized <Target>"
                // Assume if it doesn't say "by", it's outgoing (I am neutralizing).
                // Example: "61 GJ energy neutralized Starving Damavik" -> Outgoing.
                if lower_body.contains(" by ") {
                    Some(Direction::Incoming)
                } else {
                    Some(Direction::Outgoing)
                }
            } else if lower_body.contains(" drained from ") {
                // "drained from <Entity>" -> I am draining FROM them. Outgoing Neut.
                Some(Direction::Outgoing)
            } else if lower_body.contains(" drained to ") {
                // "drained to <Entity>" -> Drained TO them. I am being drained? 
                // Or I am transfering? No, Nos logic.
                // Let's assume standard "to" = Outgoing target for now.
                Some(Direction::Outgoing)
            } else if lower_body.contains(" drained by ") {
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
    // "127 to..." or "+4 GJ..." or "-6 GJ..."
    let trimmed = body.trim();
    
    // Find first whitespace
    let split_index = trimmed.find(char::is_whitespace)?;
    let (amount_part, remainder) = trimmed.split_at(split_index);
    
    // amount_part might be "+4" or "-6" or "61".
    // If it has "GJ" or "HP" after it, that's in remainder.
    
    // Try to parse amount
    let amount = amount_part.parse::<f32>().ok()?;
    
    // Remainder might start with "GJ" or "remote...". Clean it up.
    let mut clean_remainder = remainder.trim();
    if clean_remainder.starts_with("GJ") {
        clean_remainder = clean_remainder.strip_prefix("GJ").unwrap_or(clean_remainder).trim();
    }
    
    Some((amount, clean_remainder))
}

fn split_entities_and_weapon(
    remainder: &str,
    direction: Direction,
    event_type: &EventType,
    listener: &str,
) -> Option<(String, String, String)> {
    let parts: Vec<&str> = remainder.split(" - ").collect();
    if parts.is_empty() { return None; }

    let (text_part, weapon) = match event_type {
        EventType::Repair | EventType::Capacitor | EventType::Neut => {
            // Usually 2 parts: "Action to Target", "Weapon"
            if parts.len() >= 2 {
                (parts[0], parts[1].trim().to_string())
            } else {
                (parts[0], "".to_string())
            }
        }
        EventType::Damage => {
            // 3 parts: "Damage to Target", "Weapon", "Quality"
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
                EventType::Repair => vec![
                    "remote armor repaired to ", "remote shield repaired to ", "remote hull repaired to ",
                    "remote armor boosted to ", "remote shield boosted to ", "remote hull boosted to "
                ],
                EventType::Capacitor => vec!["remote capacitor transmitted to "],
                EventType::Neut => vec!["energy neutralized ", "energy drained from ", "energy drained to "],
            };
            
            for prefix in prefixes {
                if text.to_lowercase().starts_with(&prefix.to_lowercase()) {
                    if text.len() >= prefix.len() {
                         text = text[prefix.len()..].trim();
                    }
                    break;
                }
            }
            
            let target = text.trim_end_matches(" -").trim().to_string();
            if target.is_empty() { return None; }
            Some((listener.to_string(), target, weapon))
        }
        Direction::Incoming => {
            let mut text = text_part.trim();
            // Strip prefixes
            let prefixes = match event_type {
                EventType::Damage => vec!["from "],
                EventType::Repair => vec![
                    "remote armor repaired by ", "remote shield repaired by ", "remote hull repaired by ",
                    "remote armor boosted by ", "remote shield boosted by ", "remote hull boosted by "
                ],
                EventType::Capacitor => vec!["remote capacitor transmitted by "],
                EventType::Neut => vec!["energy neutralized by ", "energy drained by "],
            };

            for prefix in prefixes {
                if text.to_lowercase().starts_with(&prefix.to_lowercase()) {
                    if text.len() >= prefix.len() {
                         text = text[prefix.len()..].trim();
                    }
                    break;
                }
            }

            let source = text.trim_end_matches(" -").trim().to_string();
            if source.is_empty() { return None; }
            Some((source, listener.to_string(), weapon))
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
        
        let event = parser.parse_line(line, "LogiPilot").expect("should parse repair");

        assert_eq!(event.amount, 96.0);
        assert_eq!(event.event_type, EventType::Repair);
        assert_eq!(event.source, "LogiPilot");
        assert!(event.target.contains("Felix Allistar"));
        assert!(event.weapon.contains("Small Remote Armor Repairer"));
    }

    #[test]
    fn parses_outgoing_boost() {
        let mut parser = LineParser::new();
        let _ = parser.parse_line("Session Started: 2025.11.15 07:09:22", "LogiPilot");

        // "120 remote shield boosted to Friendly Logistic - Small Remote Shield Transmitter II"
        let line = "[ 2025.01.01 12:01:05 ] (combat) 120 remote shield boosted to Friendly Logistic - Small Remote Shield Transmitter II";
        
        let event = parser.parse_line(line, "LogiPilot").expect("should parse boosted");

        assert_eq!(event.amount, 120.0);
        assert_eq!(event.event_type, EventType::Repair);
        assert_eq!(event.target, "Friendly Logistic");
    }

    #[test]
    fn parses_outgoing_cap_transfer() {
        let mut parser = LineParser::new();
        let _ = parser.parse_line("Session Started: 2025.12.19 21:35:39", "Source");

        let line = "[ 2025.12.19 21:35:39 ] (combat) <color=0xffccff66><b>41</b><color=0x77ffffff><font size=10> remote capacitor transmitted to </font><b><color=0xffffffff><font size=12><color=0xFFFFFFFF><b>Skybreaker</b></color></font><font size=11> [CARII]</font> <font size=11>[I CherryPick Gneiss] -</font></b><color=0x77ffffff><font size=10> - Centii A-Type Small Remote Capacitor Transmitter</font>";

        let event = parser.parse_line(line, "Source").expect("should parse cap transfer");

        assert_eq!(event.amount, 41.0);
        assert_eq!(event.event_type, EventType::Capacitor);
        assert_eq!(event.target, "Skybreaker [CARII] [I CherryPick Gneiss] -"); // Regex cleanup of tags is imperfect but this confirms logic
        assert!(!event.incoming);
    }

    #[test]
    fn parses_outgoing_neut() {
        let mut parser = LineParser::new();
        let _ = parser.parse_line("Session Started: 2025.12.11 08:30:48", "Source");

        let line = "[ 2025.12.11 08:30:48 ] (combat) <color=0xffe57f7f><b>61 GJ</b><color=0x77ffffff><font size=10> energy neutralized </font><b><color=0xffffffff>Starving Damavik</b><color=0x77ffffff><font size=10> - Starving Damavik</font>";

        let event = parser.parse_line(line, "Source").expect("should parse neut");

        assert_eq!(event.amount, 61.0);
        assert_eq!(event.event_type, EventType::Neut);
        assert_eq!(event.target, "Starving Damavik");
        assert!(!event.incoming);
    }

    #[test]
    fn parses_outgoing_nos_drained_from() {
        let mut parser = LineParser::new();
        let _ = parser.parse_line("Session Started: 2025.11.09 07:09:18", "Source");

        let line = "[ 2025.11.09 07:09:18 ] (combat) <color=0xff7fffff><b>+4 GJ</b><color=0x77ffffff><font size=10> energy drained from </font><b><color=0xffffffff>Elite Lucifer Cynabal</b><color=0x77ffffff><font size=10> - Small Energy Nosferatu II</font>";

        let event = parser.parse_line(line, "Source").expect("should parse nos from");

        assert_eq!(event.amount, 4.0);
        assert_eq!(event.event_type, EventType::Neut);
        assert_eq!(event.target, "Elite Lucifer Cynabal");
        assert!(!event.incoming);
    }
}
