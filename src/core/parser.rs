use chrono::NaiveDateTime;
use lazy_static::lazy_static;
use regex::Regex;

use super::model::{CombatEvent, EventType, NotifyEvent};

const SESSION_PREFIX: &str = "Session Started:";
const TIMESTAMP_FMT: &str = "%Y.%m.%d %H:%M:%S";

lazy_static! {
    static ref TAG_RE: Regex = Regex::new(r"<[^>]+>").unwrap();
    // Pattern: "ModuleName requires X.X units of charge. The capacitor has only Y.Y units."
    static ref CAP_FAIL_RE: Regex = Regex::new(
        r"^(.+?) requires ([\d.]+) units of charge\. The capacitor has only ([\d.]+) units\.$"
    ).unwrap();
}

pub struct LineParser {
    base_time: Option<NaiveDateTime>,
}

impl LineParser {
    pub fn new() -> Self {
        Self { base_time: None }
    }

    pub fn get_base_time(&self) -> Option<NaiveDateTime> {
        self.base_time
    }
}

impl Default for LineParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LineParser {

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

        // 2. Identify Direction (pass raw body for color-based neut detection)
        let direction = determine_direction(&lower, &body, &event_type)?;

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

    /// Parse a (notify) line for capacitor failure events.
    /// Example: [ 2025.12.22 02:38:08 ] (notify) Gistii A-Type Small Remote Shield Booster requires 39.0 units of charge. The capacitor has only 6.2 units.
    pub fn parse_notify_line(&mut self, line: &str, source: &str) -> Option<NotifyEvent> {
        let trimmed = line.trim();
        if trimmed.is_empty() || !trimmed.contains("(notify)") {
            return None;
        }

        // Handle session start if we haven't yet
        if trimmed.starts_with(SESSION_PREFIX) {
            self.parse_session_start(trimmed);
            return None;
        }

        let timestamp = extract_timestamp(trimmed)?;
        
        // Extract body after (notify)
        let body = trimmed.split("(notify)")
            .nth(1)
            .map(str::trim)?;
        
        // Clean HTML tags from body
        let cleaned_body = strip_tags(body);
        
        println!("[DEBUG] parse_notify_line: cleaned_body='{}'", cleaned_body);
        
        // Try to match capacitor failure pattern
        let caps = CAP_FAIL_RE.captures(&cleaned_body);
        println!("[DEBUG] CAP_FAIL_RE matched: {}", caps.is_some());
        let caps = caps?;
        
        let module_name = caps.get(1)?.as_str().to_string();
        let required_cap: f32 = caps.get(2)?.as_str().parse().ok()?;
        let available_cap: f32 = caps.get(3)?.as_str().parse().ok()?;
        
        self.ensure_base_time(timestamp);
        
        let base = *self.base_time.as_ref()?;
        let duration = timestamp.signed_duration_since(base).to_std().ok()?;
        
        Some(NotifyEvent {
            timestamp: duration,
            character: source.to_string(),
            module_name,
            required_cap,
            available_cap,
        })
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

fn determine_direction(lower_body: &str, raw_body: &str, event_type: &EventType) -> Option<Direction> {
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
            // EVE uses same text for incoming/outgoing neuts, but colors differ:
            // 0xffe57f7f = incoming (reddish) - being neuted
            // 0xff7fffff = outgoing (cyan) - doing the neuting
            let raw_lower = raw_body.to_ascii_lowercase();
            
            if raw_lower.contains("0xffe57f7f") {
                // Reddish color = incoming neut
                Some(Direction::Incoming)
            } else if raw_lower.contains("0xff7fffff") {
                // Cyan color = outgoing neut  
                Some(Direction::Outgoing)
            } else {
                // Fallback to text-based detection
                if lower_body.contains(" neutralized by ") || lower_body.contains(" drained by ") {
                    Some(Direction::Incoming)
                } else if lower_body.contains(" neutralized ") || lower_body.contains(" drained from ") || lower_body.contains(" drained to ") {
                    Some(Direction::Outgoing)
                } else {
                    None
                }
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
    // Quality suffixes for damage
    let qualities = ["hits", "misses", "grazes", "glances", "scratches", "penetrates", "smashes", "wrecks"];

    let mut parts: Vec<&str> = remainder.split(" - ").map(|s| s.trim()).filter(|s| !s.is_empty()).collect();

    // Right-to-left parsing:
    // 1. Pop Quality (if rightmost matches known quality word)
    // 2. Pop Weapon (next segment from right)
    // 3. Everything remaining is the Entity (joined back with " - ")

    let mut weapon = String::new();

    match event_type {
        EventType::Damage => {
            // Check if rightmost is a quality word
            if parts.last().map(|s| qualities.iter().any(|q| s.to_lowercase().contains(q))).unwrap_or(false) {
                parts.pop(); // Remove quality
            }
            // Weapon is now the rightmost (if more than 1 part remains)
            if parts.len() > 1 {
                weapon = parts.pop().unwrap_or("").trim_start_matches('-').trim().to_string();
            }
        }
        _ => {
            // Repair, Cap, Neut: Weapon is always the last part (if multiple parts exist)
            if parts.len() > 1 {
                weapon = parts.pop().unwrap_or("").trim_start_matches('-').trim().to_string();
            }
        }
    }

    // Remaining parts form the Entity text (joined back with " - " to preserve dashes in names)
    let text_part = parts.join(" - ");

    let entity_name: String;

    match direction {
        Direction::Outgoing => {
            let prefixes = match event_type {
                EventType::Damage => vec!["to ", "against "],
                EventType::Repair => vec![
                    "remote armor repaired to ", "remote shield repaired to ", "remote hull repaired to ",
                    "remote armor boosted to ", "remote shield boosted to ", "remote hull boosted to ",
                    "armor repaired to ", "shield repaired to ", "hull repaired to ",
                    "armor boosted to ", "shield boosted to ", "hull boosted to ",
                    "repaired to ", "boosted to "
                ],
                EventType::Capacitor => vec!["remote capacitor transmitted to "],
                EventType::Neut => vec!["energy neutralized ", "energy drained from ", "energy drained to "],
            };
            
            let text = text_part.trim();
            let lower_text = text.to_lowercase();
            let mut result = text.to_string();
            for prefix in prefixes {
                if lower_text.starts_with(&prefix.to_lowercase()) {
                    result = text[prefix.len()..].trim().to_string();
                    break;
                }
            }
            entity_name = result;
        }
        Direction::Incoming => {
            let prefixes = match event_type {
                EventType::Damage => vec!["from "],
                EventType::Repair => vec![
                    "remote armor repaired by ", "remote shield repaired by ", "remote hull repaired by ",
                    "remote armor boosted by ", "remote shield boosted by ", "remote hull boosted by ",
                    "armor repaired by ", "shield repaired by ", "hull repaired by ",
                    "armor boosted by ", "shield boosted by ", "hull boosted by ",
                    "repaired by ", "boosted by "
                ],
                EventType::Capacitor => vec!["remote capacitor transmitted by "],
                EventType::Neut => vec!["energy neutralized by ", "energy drained by "],
            };

            let text = text_part.trim();
            let lower_text = text.to_lowercase();
            let mut result = text.to_string();
            for prefix in prefixes {
                if lower_text.starts_with(&prefix.to_lowercase()) {
                    result = text[prefix.len()..].trim().to_string();
                    break;
                }
            }
            entity_name = result;
        }
    }

    let entity = entity_name.trim_end_matches(" -").trim().to_string();
    if entity.is_empty() { return None; }

    // If weapon is empty, use Source (Incoming) or Target (Outgoing Neut/Rep) if appropriate? 
    // Actually, for incoming damage, we definitely want the Source name if no weapon exists.
    if weapon.is_empty() {
         weapon = entity.clone();
    }

    match direction {
        Direction::Outgoing => Some((listener.to_string(), entity, weapon)),
        Direction::Incoming => Some((entity, listener.to_string(), weapon)),
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
        let line = "[ 2025.11.15 07:15:05 ] (combat) 120 remote shield boosted to Friendly Logistic - Small Remote Shield Transmitter II";
        
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
        assert_eq!(event.target, "Skybreaker [CARII] [I CherryPick Gneiss]"); // Regex cleanup of tags is imperfect but this confirms logic
        assert!(!event.incoming);
    }

    #[test]
    fn parses_outgoing_neut() {
        let mut parser = LineParser::new();
        let _ = parser.parse_line("Session Started: 2025.12.11 08:30:48", "Source");

        // Outgoing neut - note the 0xff7fffff color (cyan) indicates outgoing
        let line = "[ 2025.12.11 08:30:48 ] (combat) <color=0xff7fffff><b>61 GJ</b><color=0x77ffffff><font size=10> energy neutralized </font><b><color=0xffffffff>Starving Damavik</b><color=0x77ffffff><font size=10> - Starving Damavik</font>";

        let event = parser.parse_line(line, "Source").expect("should parse neut");

        assert_eq!(event.amount, 61.0);
        assert_eq!(event.event_type, EventType::Neut);
        assert_eq!(event.target, "Starving Damavik");
        assert!(!event.incoming);
    }

    #[test]
    fn parses_incoming_neut() {
        let mut parser = LineParser::new();
        let _ = parser.parse_line("Session Started: 2026.01.06 21:56:26", "I CherryPick Gneiss");

        // Incoming neut - note the 0xffe57f7f color (reddish) indicates incoming
        let line = "[ 2026.01.06 21:56:26 ] (combat) <color=0xffe57f7f><b>38 GJ</b><color=0x77ffffff><font size=10> energy neutralized </font><b><color=0xffffffff><font size=12><color=0xFFFFB300> <u><b>Hawk</b></u></color></font> [<b>CARII</b>]  [Felix Allistar]<color=0xFFFFFFFF><b> -</b><color=0x77ffffff><font size=10> - Small Energy Neutralizer II</font>";

        let event = parser.parse_line(line, "I CherryPick Gneiss").expect("should parse incoming neut");

        assert_eq!(event.amount, 38.0);
        assert_eq!(event.event_type, EventType::Neut);
        assert!(event.incoming, "Should be incoming neut based on color 0xffe57f7f");
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
    #[test]
    fn parses_incoming_damage_no_weapon() {
        let mut parser = LineParser::new();
        let _ = parser.parse_line("Session Started: 2026.01.02 10:23:31", "Felix");
        
        // Log line: 26 from Lucifer Echo - Hits
        let line = "[ 2026.01.02 10:23:35 ] (combat) <color=0xffcc0000><b>26</b> <color=0x77ffffff><font size=10>from</font> <b><color=0xffffffff>Lucifer Echo</b><font size=10><color=0x77ffffff> - Hits";
        let event = parser.parse_line(line, "Felix").expect("should parse");
        
        assert_eq!(event.amount, 26.0);
        assert!(event.incoming);
        assert_eq!(event.source, "Lucifer Echo");
        assert_eq!(event.weapon, "Lucifer Echo"); // Fallback to source
    }

    #[test]
    fn parses_double_dash_repair() {
        let mut parser = LineParser::new();
        let _ = parser.parse_line("Session Started: 2026.01.02 10:23:31", "Felix");
        
        // Log line: 160 remote shield boosted to Hawk [CARII] [Felix Allistar] - - Pithi C-Type Small Remote Shield Booster
        let line = "[ 2026.01.02 10:23:31 ] (combat) <color=0xffccff66><b>160</b><color=0x77ffffff><font size=10> remote shield boosted to </font><b><color=0xffffffff><font size=12><color=0xFFFFFFFF><b>Hawk</b></color></font><font size=11> [CARII]</font> <font size=11>[Felix Allistar] -</font></b><color=0x77ffffff><font size=10> - Pithi C-Type Small Remote Shield Booster</font>";
        let event = parser.parse_line(line, "Felix").expect("should parse double dash");
        
        assert_eq!(event.amount, 160.0);
        assert!(event.weapon.contains("Pithi C-Type"));
        assert!(event.target.contains("Felix Allistar"));
    }

    #[test]
    fn parses_target_with_dash_in_name() {
        let mut parser = LineParser::new();
        let _ = parser.parse_line("Session Started: 2026.01.03 19:20:00", "TestPilot");
        
        // Target: "Habitation Module - Breeding Facility" (contains dash)
        // Weapon: "Small Vorton Projector II"
        let line = "[ 2026.01.03 19:20:00 ] (combat) <color=0xff00ffff><b>265</b> <color=0x77ffffff><font size=10>to</font> <b><color=0xffffffff>Habitation Module - Breeding Facility</b><font size=10><color=0x77ffffff> - Small Vorton Projector II - Hits";
        let event = parser.parse_line(line, "TestPilot").expect("should parse target with dash");
        
        assert_eq!(event.amount, 265.0);
        assert_eq!(event.target, "Habitation Module - Breeding Facility");
        assert_eq!(event.weapon, "Small Vorton Projector II");
        assert!(!event.incoming);
    }

    #[test]
    fn parses_damage_without_quality() {
        let mut parser = LineParser::new();
        let _ = parser.parse_line("Session Started: 2026.01.03 19:20:00", "TestPilot");
        
        // No quality suffix (user has it disabled in EVE settings)
        let line = "[ 2026.01.03 19:20:00 ] (combat) <color=0xff00ffff><b>265</b> <color=0x77ffffff><font size=10>to</font> <b><color=0xffffffff>Habitation Module - Breeding Facility</b><font size=10><color=0x77ffffff> - Small Vorton Projector II";
        let event = parser.parse_line(line, "TestPilot").expect("should parse without quality");
        
        assert_eq!(event.amount, 265.0);
        assert_eq!(event.target, "Habitation Module - Breeding Facility");
        assert_eq!(event.weapon, "Small Vorton Projector II");
    }

    #[test]
    fn parses_capacitor_failure_notify() {
        let mut parser = LineParser::new();
        let _ = parser.parse_line("Session Started: 2025.12.22 02:38:00", "TestPilot");
        
        let line = "[ 2025.12.22 02:38:08 ] (notify) Gistii A-Type Small Remote Shield Booster requires 39.0 units of charge. The capacitor has only 6.2 units.";
        let event = parser.parse_notify_line(line, "TestPilot").expect("should parse capacitor failure");
        
        assert_eq!(event.module_name, "Gistii A-Type Small Remote Shield Booster");
        assert_eq!(event.required_cap, 39.0);
        assert_eq!(event.available_cap, 6.2);
        assert_eq!(event.character, "TestPilot");
    }

    #[test]
    fn parses_afterburner_cap_failure() {
        let mut parser = LineParser::new();
        let _ = parser.parse_line("Session Started: 2025.12.22 02:38:00", "TestPilot");
        
        let line = "[ 2025.12.22 02:38:35 ] (notify) 1MN Y-S8 Compact Afterburner requires 5.0 units of charge. The capacitor has only 0.7 units.";
        let event = parser.parse_notify_line(line, "TestPilot").expect("should parse afterburner cap failure");
        
        assert_eq!(event.module_name, "1MN Y-S8 Compact Afterburner");
        assert_eq!(event.required_cap, 5.0);
        assert_eq!(event.available_cap, 0.7);
    }

    #[test]
    fn ignores_non_notify_lines() {
        let mut parser = LineParser::new();
        
        // Combat line should return None
        let line = "[ 2025.11.15 07:14:31 ] (combat) <b>523</b> to Starving Damavik";
        assert!(parser.parse_notify_line(line, "Test").is_none());
        
        // Empty line should return None
        assert!(parser.parse_notify_line("", "Test").is_none());
    }
}
