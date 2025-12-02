use chrono::NaiveDateTime;
use lazy_static::lazy_static;
use regex::Regex;

use super::model::CombatEvent;

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
            if let Some(timestamp) = trimmed
                .strip_prefix(SESSION_PREFIX)
                .map(str::trim)
                .and_then(|value| NaiveDateTime::parse_from_str(value, &TIMESTAMP_FMT).ok())
            {
                self.base_time = Some(timestamp);
            }
            return None;
        }

        if !trimmed.contains("(combat)") {
            return None;
        }

        let timestamp = extract_timestamp(trimmed)?;
        let body = trimmed
            .split("(combat)")
            .nth(1)
            .map(str::trim)
            .unwrap_or_default();
        let cleaned_body = strip_tags(body);
        let lower = cleaned_body.to_ascii_lowercase();

        if lower.contains("remote armor repaired") {
            return None;
        }

        let direction = if lower.contains(" to ") {
            DamageDirection::Outgoing
        } else if lower.contains(" from ") {
            DamageDirection::Incoming
        } else {
            return None;
        };

        let (damage, remainder) = split_damage_body(&cleaned_body)?;
        let (source_entity, target_entity, weapon) =
            split_entities_and_weapon(remainder, direction, source)?;

        self.ensure_base_time(timestamp);

        let base = *self.base_time.as_ref()?;
        let duration = timestamp.signed_duration_since(base).to_std().ok()?;

        Some(CombatEvent {
            timestamp: duration,
            source: source_entity,
            target: target_entity,
            weapon,
            damage,
            incoming: matches!(direction, DamageDirection::Incoming),
        })
    }

    fn ensure_base_time(&mut self, timestamp: NaiveDateTime) {
        if self.base_time.is_none() {
            self.base_time = Some(timestamp);
        }
    }
}

fn extract_timestamp(line: &str) -> Option<NaiveDateTime> {
    let first_section = line.split(']').next()?;
    let timestamp_text = first_section.trim_start_matches('[').trim();
    NaiveDateTime::parse_from_str(timestamp_text, &TIMESTAMP_FMT).ok()
}

fn strip_tags(value: &str) -> String {
    let cleaned = TAG_RE.replace_all(value, "");
    cleaned
        .replace("&nbsp;", " ")
        .replace('\r', " ")
        .replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn split_damage_body(body: &str) -> Option<(f32, &str)> {
    let trimmed = body.trim();
    let split_index = trimmed.find(char::is_whitespace)?;
    let (damage_str, remainder) = trimmed.split_at(split_index);
    let remainder = remainder.trim();
    let damage = damage_str.parse::<f32>().ok()?;
    Some((damage, remainder))
}

#[derive(Clone, Copy)]
enum DamageDirection {
    Outgoing,
    Incoming,
}

fn split_entities_and_weapon(
    remainder: &str,
    direction: DamageDirection,
    listener: &str,
) -> Option<(String, String, String)> {
    let trimmed = remainder.trim();

    match direction {
        DamageDirection::Outgoing => {
            let mut text = trimmed;
            for prefix in ["to ", "against "] {
                if text.starts_with(prefix) {
                    text = text.strip_prefix(prefix)?.trim();
                    break;
                }
            }

            let parts: Vec<_> = text.split(" - ").collect();
            let target = parts.get(0)?.trim();
            let weapon = parts.get(1).map(|value| value.trim()).unwrap_or("");

            if target.is_empty() {
                return None;
            }

            Some((listener.to_string(), target.to_string(), weapon.to_string()))
        }
        DamageDirection::Incoming => {
            let mut text = trimmed;
            if text.starts_with("from ") {
                text = text.strip_prefix("from ")?.trim();
            }

            let parts: Vec<_> = text.split(" - ").collect();
            let source = parts.get(0)?.trim();
            let weapon = parts.get(1).map(|value| value.trim()).unwrap_or("");

            if source.is_empty() {
                return None;
            }

            Some((source.to_string(), listener.to_string(), weapon.to_string()))
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

        assert_eq!(event.damage, 523.0);
        assert!(!event.incoming);
        assert_eq!(event.source, "You");
        assert_eq!(event.target, "Starving Damavik");
        assert_eq!(event.weapon, "Small Focused Beam Laser II");
        assert!(event.timestamp.as_secs() > 0);
    }

    #[test]
    fn ignores_miss_lines_without_damage_number() {
        let mut parser = LineParser::new();
        let _ = parser.parse_line("Session Started: 2025.11.15 07:09:22", "You");

        let miss_line = "[ 2025.11.15 07:14:42 ] (combat) Your group of Small Focused Beam Laser II misses Starving Damavik completely - Small Focused Beam Laser II";
        let event = parser.parse_line(miss_line, "You");

        assert!(event.is_none());
    }

    #[test]
    fn ignores_remote_repair_lines() {
        let mut parser = LineParser::new();
        let _ = parser.parse_line("Session Started: 2025.11.15 07:09:22", "You");

        let rep_line = "[ 2025.11.15 07:14:52 ] (combat) <color=0xffccff66><b>96</b><color=0x77ffffff><font size=10> remote armor repaired to </font><b><color=0xffffffff><font size=12><color=0xFFFFB300> <u><b>Retribution</b></u></color></font> [<b>CARII</b>]  [Felix Allistar]<color=0xFFFFFFFF><b> -</b><color=0x77ffffff><font size=10> - Small Remote Armor Repairer II</font>";

        let event = parser.parse_line(rep_line, "You");

        assert!(event.is_none());
    }

    #[test]
    fn parses_incoming_hit_as_incoming() {
        let mut parser = LineParser::new();
        let _ = parser.parse_line("Session Started: 2025.11.17 17:51:40", "You");

        let line = "[ 2025.11.17 17:51:49 ] (combat) <color=0xffcc0000><b>44</b> <color=0x77ffffff><font size=10>from</font> <b><color=0xffffffff>Guristas Heavy Missile Battery</b><font size=10><color=0x77ffffff> - Inferno Heavy Missile - Hits";

        let event = parser.parse_line(line, "You").expect("should parse");

        assert!(event.incoming);
        assert_eq!(event.damage, 44.0);
        assert_eq!(event.source, "Guristas Heavy Missile Battery");
        assert_eq!(event.target, "You");
        assert_eq!(event.weapon, "Inferno Heavy Missile");
    }
}
