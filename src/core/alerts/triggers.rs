// Trigger evaluation logic for alert rules.
//
// Each trigger evaluates combat/notify events and returns an optional message
// when the trigger condition is met.

use std::collections::HashSet;

use super::model::AlertRuleId;
use crate::core::model::{CombatEvent, EventType, NotifyEvent};

/// Context provided to triggers for evaluation
pub struct TriggerContext<'a> {
    /// Recent combat events (since last evaluation)
    pub combat_events: &'a [CombatEvent],
    /// Recent notify events (since last evaluation)
    pub notify_events: &'a [NotifyEvent],
    /// Set of currently tracked character names
    pub tracked_characters: &'a HashSet<String>,
    /// Characters designated as logi
    pub logi_characters: &'a HashSet<String>,
    /// Characters designated as neut-sensitive
    pub neut_sensitive_characters: &'a HashSet<String>,
    /// Whether to ignore Vorton weapons in FriendlyFire evaluation
    pub ignore_vorton: bool,
}

/// Evaluate a specific trigger against the current context.
/// Returns Some(message) if the trigger fired, None otherwise.
pub fn evaluate_trigger(rule_id: AlertRuleId, ctx: &TriggerContext) -> Option<String> {
    match rule_id {
        AlertRuleId::EnvironmentalDamage => evaluate_environmental_damage(ctx),
        AlertRuleId::FriendlyFire => evaluate_friendly_fire(ctx),
        AlertRuleId::LogiTakingDamage => evaluate_logi_taking_damage(ctx),
        AlertRuleId::NeutSensitiveNeuted => evaluate_neut_sensitive(ctx),
        AlertRuleId::CapacitorFailure => evaluate_capacitor_failure(ctx),
        AlertRuleId::LogiNeuted => evaluate_logi_neuted(ctx),
    }
}

/// Alert when taking damage from "Unstable Abyssal Depths"
fn evaluate_environmental_damage(ctx: &TriggerContext) -> Option<String> {
    const ENVIRONMENTAL_SOURCE: &str = "Unstable Abyssal Depths";
    
    for event in ctx.combat_events {
        if event.event_type == EventType::Damage 
            && event.incoming 
            && event.source.contains(ENVIRONMENTAL_SOURCE) 
        {
            return Some(format!(
                "{} taking damage from {}!",
                event.character,
                ENVIRONMENTAL_SOURCE
            ));
        }
    }
    None
}

/// Alert when a tracked character damages another tracked character.
/// Excludes Vorton weapons (chain lightning hits teammates).
fn evaluate_friendly_fire(ctx: &TriggerContext) -> Option<String> {
    for event in ctx.combat_events {
        if event.event_type != EventType::Damage || event.incoming {
            continue;
        }
        
        // Source must be a tracked character
        if !ctx.tracked_characters.contains(&event.character) {
            continue;
        }
        
        // Target must also be a tracked character (but not self)
        if !ctx.tracked_characters.contains(&event.target) 
            || event.target == event.character 
        {
            continue;
        }
        
        // Optionally exclude Vorton weapons (case-insensitive check)
        if ctx.ignore_vorton {
            let weapon_lower = event.weapon.to_lowercase();
            if weapon_lower.contains("vorton") {
                continue;
            }
        }
        
        return Some(format!(
            "Friendly fire! {} hit {} with {}",
            event.character,
            event.target,
            event.weapon
        ));
    }
    None
}

/// Alert when a logi-designated character receives incoming damage
fn evaluate_logi_taking_damage(ctx: &TriggerContext) -> Option<String> {
    println!("[DEBUG] Logi chars in context: {:?}", ctx.logi_characters);
    for event in ctx.combat_events {
        println!("[DEBUG] Checking event: type={:?}, incoming={}, character='{}'", 
            event.event_type, event.incoming, event.character);
        if event.event_type != EventType::Damage || !event.incoming {
            continue;
        }
        
        // Check if the character receiving damage is designated as logi
        let is_logi = ctx.logi_characters.contains(&event.character);
        println!("[DEBUG] Is '{}' in logi set? {}", event.character, is_logi);
        if is_logi {
            return Some(format!(
                "LOGI TAKING DAMAGE! {} hit by {} for {:.0}",
                event.character,
                event.source,
                event.amount
            ));
        }
    }
    None
}

/// Alert when a neut-sensitive character is being neuted
fn evaluate_neut_sensitive(ctx: &TriggerContext) -> Option<String> {
    for event in ctx.combat_events {
        if event.event_type != EventType::Neut || !event.incoming {
            continue;
        }
        
        // Check if the character being neuted is designated as neut-sensitive
        if ctx.neut_sensitive_characters.contains(&event.character) {
            return Some(format!(
                "NEUT PRESSURE on {}! {:.0} GJ from {}",
                event.character,
                event.amount,
                event.source
            ));
        }
    }
    None
}

/// Alert when a designated logi character is being neuted
fn evaluate_logi_neuted(ctx: &TriggerContext) -> Option<String> {
    for event in ctx.combat_events {
        if event.event_type != EventType::Neut || !event.incoming {
            continue;
        }
        
        // Check if the character being neuted is designated as logi
        if ctx.logi_characters.contains(&event.character) {
            return Some(format!(
                "LOGI NEUTED! {} draining {:.0} GJ from {}",
                event.source,
                event.amount,
                event.character
            ));
        }
    }
    None
}

/// Alert when a module fails to activate due to insufficient capacitor
fn evaluate_capacitor_failure(ctx: &TriggerContext) -> Option<String> {
    for event in ctx.notify_events {
        return Some(format!(
            "CAP FAILURE! {} can't activate {} (need {:.1}, have {:.1})",
            event.character,
            event.module_name,
            event.required_cap,
            event.available_cap
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn empty_context() -> (Vec<CombatEvent>, Vec<NotifyEvent>, HashSet<String>, HashSet<String>, HashSet<String>) {
        (Vec::new(), Vec::new(), HashSet::new(), HashSet::new(), HashSet::new())
    }

    fn make_combat_event(
        event_type: EventType,
        incoming: bool,
        source: &str,
        target: &str,
        character: &str,
        weapon: &str,
        amount: f32,
    ) -> CombatEvent {
        CombatEvent {
            timestamp: Duration::from_secs(0),
            source: source.to_string(),
            target: target.to_string(),
            weapon: weapon.to_string(),
            amount,
            incoming,
            character: character.to_string(),
            event_type,
        }
    }

    #[test]
    fn test_environmental_damage_triggers() {
        let (mut combat, notify, tracked, logi, neut) = empty_context();
        
        combat.push(make_combat_event(
            EventType::Damage,
            true, // incoming
            "Unstable Abyssal Depths",
            "MyShip",
            "MyPilot",
            "Environmental",
            100.0,
        ));
        
        let ctx = TriggerContext {
            combat_events: &combat,
            notify_events: &notify,
            tracked_characters: &tracked,
            logi_characters: &logi,
            neut_sensitive_characters: &neut,
            ignore_vorton: true,
        };
        
        let result = evaluate_trigger(AlertRuleId::EnvironmentalDamage, &ctx);
        assert!(result.is_some());
        assert!(result.unwrap().contains("Unstable Abyssal Depths"));
    }

    #[test]
    fn test_friendly_fire_triggers() {
        let (mut combat, notify, mut tracked, logi, neut) = empty_context();
        
        tracked.insert("Pilot1".to_string());
        tracked.insert("Pilot2".to_string());
        
        combat.push(make_combat_event(
            EventType::Damage,
            false, // outgoing
            "Pilot1",
            "Pilot2",
            "Pilot1",
            "Light Missile Launcher II",
            50.0,
        ));
        
        let ctx = TriggerContext {
            combat_events: &combat,
            notify_events: &notify,
            tracked_characters: &tracked,
            logi_characters: &logi,
            neut_sensitive_characters: &neut,
            ignore_vorton: true,
        };
        
        let result = evaluate_trigger(AlertRuleId::FriendlyFire, &ctx);
        assert!(result.is_some());
        assert!(result.unwrap().contains("Friendly fire"));
    }

    #[test]
    fn test_friendly_fire_excludes_vorton() {
        let (mut combat, notify, mut tracked, logi, neut) = empty_context();
        
        tracked.insert("Pilot1".to_string());
        tracked.insert("Pilot2".to_string());
        
        combat.push(make_combat_event(
            EventType::Damage,
            false,
            "Pilot1",
            "Pilot2",
            "Pilot1",
            "Small Vorton Projector II", // Vorton - should be excluded
            50.0,
        ));
        
        let ctx = TriggerContext {
            combat_events: &combat,
            notify_events: &notify,
            tracked_characters: &tracked,
            logi_characters: &logi,
            neut_sensitive_characters: &neut,
            ignore_vorton: true, // Test that Vorton IS excluded when this is true
        };
        
        let result = evaluate_trigger(AlertRuleId::FriendlyFire, &ctx);
        assert!(result.is_none(), "Vorton damage should not trigger friendly fire");
    }

    #[test]
    fn test_logi_taking_damage_triggers() {
        let (mut combat, notify, tracked, mut logi, neut) = empty_context();
        
        logi.insert("LogiPilot".to_string());
        
        combat.push(make_combat_event(
            EventType::Damage,
            true, // incoming
            "Starving Damavik",
            "LogiPilot",
            "LogiPilot",
            "Light Missile",
            30.0,
        ));
        
        let ctx = TriggerContext {
            combat_events: &combat,
            notify_events: &notify,
            tracked_characters: &tracked,
            logi_characters: &logi,
            neut_sensitive_characters: &neut,
            ignore_vorton: true,
        };
        
        let result = evaluate_trigger(AlertRuleId::LogiTakingDamage, &ctx);
        assert!(result.is_some());
        assert!(result.unwrap().contains("LOGI TAKING DAMAGE"));
    }

    #[test]
    fn test_capacitor_failure_triggers() {
        let (combat, mut notify, tracked, logi, neut) = empty_context();
        
        notify.push(NotifyEvent {
            timestamp: Duration::from_secs(0),
            character: "TestPilot".to_string(),
            module_name: "Afterburner".to_string(),
            required_cap: 10.0,
            available_cap: 2.0,
        });
        
        let ctx = TriggerContext {
            combat_events: &combat,
            notify_events: &notify,
            tracked_characters: &tracked,
            logi_characters: &logi,
            neut_sensitive_characters: &neut,
            ignore_vorton: true,
        };
        
        let result = evaluate_trigger(AlertRuleId::CapacitorFailure, &ctx);
        assert!(result.is_some());
        assert!(result.unwrap().contains("CAP FAILURE"));
    }

    #[test]
    fn test_logi_neuted_triggers() {
        let (mut combat, notify, tracked, mut logi, neut) = empty_context();
        
        logi.insert("LogiPilot".to_string());
        
        combat.push(make_combat_event(
            EventType::Neut,
            true, // incoming
            "Starving Damavik",
            "LogiPilot",
            "LogiPilot",
            "Energy Neutralizer",
            50.0,
        ));
        
        let ctx = TriggerContext {
            combat_events: &combat,
            notify_events: &notify,
            tracked_characters: &tracked,
            logi_characters: &logi,
            neut_sensitive_characters: &neut,
            ignore_vorton: true,
        };
        
        let result = evaluate_trigger(AlertRuleId::LogiNeuted, &ctx);
        assert!(result.is_some());
        assert!(result.unwrap().contains("LOGI NEUTED"));
    }
}
