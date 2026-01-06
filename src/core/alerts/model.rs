// Alert model types for configuration and events.
//
// NOTE: TypeScript mirror types are in ui/src/types.ts
// Keep both files in sync when modifying data structures.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Unique identifier for hardcoded alert rules
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlertRuleId {
    /// Damage from environmental hazards like "Unstable Abyssal Depths"
    EnvironmentalDamage,
    /// Tracked character damaging another tracked character (excluding Vorton)
    FriendlyFire,
    /// Designated logi character receiving damage
    LogiTakingDamage,
    /// Designated neut-sensitive character being neuted
    NeutSensitiveNeuted,
    /// Module activation failed due to insufficient capacitor
    CapacitorFailure,
    /// Designated logi character being neuted
    LogiNeuted,
}

impl AlertRuleId {
    /// Get the display name for this alert
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::EnvironmentalDamage => "Environmental Damage",
            Self::FriendlyFire => "Friendly Fire",
            Self::LogiTakingDamage => "Logi Taking Damage",
            Self::NeutSensitiveNeuted => "Neut Pressure",
            Self::CapacitorFailure => "Capacitor Failure",
            Self::LogiNeuted => "Logi Neuted",
        }
    }

    /// Get a description of what this alert does
    pub fn description(&self) -> &'static str {
        match self {
            Self::EnvironmentalDamage => "Alert when taking damage from Unstable Abyssal Depths",
            Self::FriendlyFire => "Alert when a tracked character damages another tracked character (excludes Vorton weapons)",
            Self::LogiTakingDamage => "Alert when your designated logi character takes incoming damage",
            Self::NeutSensitiveNeuted => "Alert when a designated neut-sensitive character is neuted",
            Self::CapacitorFailure => "Alert when a module fails to activate due to insufficient capacitor",
            Self::LogiNeuted => "Alert when a designated logi character is neuted",
        }
    }

    /// Get all available alert rule IDs
    pub fn all() -> &'static [AlertRuleId] {
        &[
            Self::EnvironmentalDamage,
            Self::FriendlyFire,
            Self::LogiTakingDamage,
            Self::NeutSensitiveNeuted,
            Self::CapacitorFailure,
            Self::LogiNeuted,
        ]
    }
}

/// Sound options for alerts
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AlertSound {
    #[default]
    Default,
    Warning,
    Critical,
    None,
}

impl AlertSound {
    /// Get the filename for this sound based on the rule
    pub fn filename(&self, rule_id: AlertRuleId) -> Option<&'static str> {
        match self {
            Self::Default | Self::Warning | Self::Critical => Some(match rule_id {
                AlertRuleId::EnvironmentalDamage => "boundary",
                AlertRuleId::FriendlyFire => "friendly_fire",
                AlertRuleId::LogiTakingDamage => "logi_attacked",
                AlertRuleId::NeutSensitiveNeuted => "neut",
                AlertRuleId::CapacitorFailure => "capacitor_empty",
                AlertRuleId::LogiNeuted => "logi_neuted",
            }),
            Self::None => None,
        }
    }
}

/// Per-rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRuleConfig {
    pub enabled: bool,
    pub sound: AlertSound,
    /// Per-rule cooldown in seconds (default: 3)
    #[serde(default = "default_cooldown")]
    pub cooldown_seconds: u32,
    /// For FriendlyFire: ignore damage from Vorton weapons (chain lightning AOE)
    #[serde(default = "default_ignore_vorton")]
    pub ignore_vorton: bool,
}

fn default_cooldown() -> u32 {
    3
}

fn default_ignore_vorton() -> bool {
    true
}

impl Default for AlertRuleConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sound: AlertSound::Default,
            cooldown_seconds: 3,
            ignore_vorton: true, // Default to ignoring Vorton for FriendlyFire
        }
    }
}

/// Alert event fired when a trigger matches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertEvent {
    pub rule_id: AlertRuleId,
    pub timestamp: Duration,
    pub message: String,
    pub sound: AlertSound,
}

/// Character role designations for alert logic
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CharacterRoles {
    /// Characters designated as "logi" (squishy healers who shouldn't take damage)
    pub logi_characters: Vec<String>,
    /// Characters that are particularly vulnerable to neut pressure
    pub neut_sensitive_characters: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_rules_have_names() {
        for rule in AlertRuleId::all() {
            assert!(!rule.display_name().is_empty());
            assert!(!rule.description().is_empty());
        }
    }

    #[test]
    fn test_sound_filenames() {
        assert_eq!(AlertSound::Default.filename(AlertRuleId::EnvironmentalDamage), Some("boundary"));
        assert_eq!(AlertSound::Default.filename(AlertRuleId::LogiNeuted), Some("logi_neuted"));
        assert_eq!(AlertSound::None.filename(AlertRuleId::FriendlyFire), None);
    }
}
