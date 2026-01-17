// Alert engine - orchestrates trigger evaluation and manages cooldowns.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::model::{AlertEvent, AlertRuleConfig, AlertRuleId, AlertSound, CharacterRoles};
use super::triggers::{evaluate_trigger, TriggerContext};
use crate::core::model::{CombatEvent, NotifyEvent};

/// Alert engine configuration - persisted in settings.json
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AlertEngineConfig {
    /// Per-rule configuration (enabled, sound, etc.)
    pub rules: HashMap<AlertRuleId, AlertRuleConfig>,
    /// Character role designations
    pub roles: CharacterRoles,
}

impl AlertEngineConfig {
    /// Create config with all rules enabled at default settings
    pub fn default_enabled() -> Self {
        let mut rules = HashMap::new();
        for rule_id in AlertRuleId::all() {
            rules.insert(*rule_id, AlertRuleConfig::default());
        }
        Self {
            rules,
            roles: CharacterRoles::default(),
        }
    }

    /// Check if a specific rule is enabled
    pub fn is_enabled(&self, rule_id: AlertRuleId) -> bool {
        self.rules.get(&rule_id).map(|c| c.enabled).unwrap_or(false)
    }

    /// Get the sound for a specific rule
    pub fn get_sound(&self, rule_id: AlertRuleId) -> AlertSound {
        self.rules
            .get(&rule_id)
            .map(|c| c.sound.clone())
            .unwrap_or_default()
    }

    /// Get the cooldown for a specific rule in seconds
    pub fn get_cooldown(&self, rule_id: AlertRuleId) -> Duration {
        let secs = self
            .rules
            .get(&rule_id)
            .map(|c| c.cooldown_seconds)
            .unwrap_or(3);
        Duration::from_secs(secs as u64)
    }
}

/// Alert engine state
pub struct AlertEngine {
    /// Configuration
    config: AlertEngineConfig,
    /// Cooldown tracking: last fire time per rule
    cooldowns: HashMap<AlertRuleId, Instant>,
}

impl AlertEngine {
    pub fn new(config: AlertEngineConfig) -> Self {
        Self {
            config,
            cooldowns: HashMap::new(),
        }
    }

    /// Update the engine configuration (hot-reload friendly)
    pub fn update_config(&mut self, config: AlertEngineConfig) {
        self.config = config;
    }

    /// Evaluate all triggers against current events.
    /// Returns list of alert events that fired (deduplicated by rule_id per tick).
    /// Audio playback is handled by the frontend/audio thread sequentially.
    pub fn evaluate(
        &mut self,
        combat_events: &[CombatEvent],
        notify_events: &[NotifyEvent],
        tracked_characters: &HashSet<String>,
    ) -> Vec<AlertEvent> {
        let mut alerts = Vec::new();
        let now = Instant::now();

        // Build sets from config for trigger context
        let logi_set: HashSet<String> = self.config.roles.logi_characters.iter().cloned().collect();
        let neut_set: HashSet<String> = self
            .config
            .roles
            .neut_sensitive_characters
            .iter()
            .cloned()
            .collect();

        let ctx = TriggerContext {
            combat_events,
            notify_events,
            tracked_characters,
            logi_characters: &logi_set,
            neut_sensitive_characters: &neut_set,
        };

        for rule_id in AlertRuleId::all() {
            // Skip disabled rules
            if !self.config.is_enabled(*rule_id) {
                continue;
            }

            // Check per-rule cooldown to prevent spam
            let rule_cooldown = self.config.get_cooldown(*rule_id);
            if let Some(last_fire) = self.cooldowns.get(rule_id) {
                if now.duration_since(*last_fire) < rule_cooldown {
                    continue;
                }
            }

            // Get per-rule ignore_vorton setting (only used by FriendlyFire and LogiTakingDamage)
            let ignore_vorton = self
                .config
                .rules
                .get(rule_id)
                .map(|c| c.ignore_vorton)
                .unwrap_or(true);

            // Evaluate trigger
            if let Some(message) = evaluate_trigger(*rule_id, &ctx, ignore_vorton) {
                self.cooldowns.insert(*rule_id, now);

                // Get timestamp from the first relevant event
                let timestamp = combat_events
                    .first()
                    .map(|e| e.timestamp)
                    .or_else(|| notify_events.first().map(|e| e.timestamp))
                    .unwrap_or_default();

                alerts.push(AlertEvent {
                    rule_id: *rule_id,
                    timestamp,
                    message,
                    sound: self.config.get_sound(*rule_id),
                });
            }
        }

        alerts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::model::EventType;

    fn make_combat_event(
        event_type: EventType,
        incoming: bool,
        source: &str,
        target: &str,
        character: &str,
    ) -> CombatEvent {
        CombatEvent {
            timestamp: Duration::from_secs(0),
            source: source.to_string(),
            target: target.to_string(),
            weapon: "TestWeapon".to_string(),
            amount: 100.0,
            incoming,
            character: character.to_string(),
            event_type,
        }
    }

    #[test]
    fn test_engine_disabled_rule_skipped() {
        let mut config = AlertEngineConfig::default_enabled();
        config
            .rules
            .get_mut(&AlertRuleId::EnvironmentalDamage)
            .unwrap()
            .enabled = false;

        let mut engine = AlertEngine::new(config);

        let combat = vec![make_combat_event(
            EventType::Damage,
            true,
            "Unstable Abyssal Depths",
            "MyShip",
            "MyPilot",
        )];

        let alerts = engine.evaluate(&combat, &[], &HashSet::new());
        assert!(alerts.is_empty(), "Disabled rule should not fire");
    }

    #[test]
    fn test_engine_cooldown_respected() {
        let config = AlertEngineConfig::default_enabled();
        let mut engine = AlertEngine::new(config);

        let combat = vec![make_combat_event(
            EventType::Damage,
            true,
            "Unstable Abyssal Depths",
            "MyShip",
            "MyPilot",
        )];

        // First evaluation should fire
        let alerts1 = engine.evaluate(&combat, &[], &HashSet::new());
        assert_eq!(alerts1.len(), 1);

        // Second evaluation should be blocked by cooldown
        let alerts2 = engine.evaluate(&combat, &[], &HashSet::new());
        assert!(alerts2.is_empty(), "Cooldown should prevent repeated alert");
    }

    #[test]
    fn test_engine_fires_environmental_alert() {
        let config = AlertEngineConfig::default_enabled();
        let mut engine = AlertEngine::new(config);

        let combat = vec![make_combat_event(
            EventType::Damage,
            true,
            "Unstable Abyssal Depths",
            "MyShip",
            "MyPilot",
        )];

        let alerts = engine.evaluate(&combat, &[], &HashSet::new());
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].rule_id, AlertRuleId::EnvironmentalDamage);
    }
}
