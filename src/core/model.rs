use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

pub type EntityName = String;
pub type WeaponName = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventType {
    Damage,
    Repair,
    Capacitor,
    Neut,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatEvent {
    pub timestamp: Duration,
    pub source: EntityName,
    pub target: EntityName,
    pub weapon: WeaponName,
    pub amount: f32, // renamed from 'damage' to 'amount' to reflect generic nature
    pub incoming: bool,
    pub character: String, // The character whose log this event came from
    pub event_type: EventType,
}

impl CombatEvent {
    pub fn damage(&self) -> f32 {
        if self.event_type == EventType::Damage {
            self.amount
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DpsSample {
    pub time: Duration,
    pub outgoing_dps: f32,
    pub incoming_dps: f32,
    pub outgoing_hps: f32, // Healing Per Second
    pub outgoing_cap: f32, // Capacitor Transferred Per Second
    pub outgoing_neut: f32, // Energy Neutralized/Drained Per Second
    
    // Detailed breakdowns
    pub outgoing_by_weapon: HashMap<WeaponName, f32>,
    pub outgoing_by_target: HashMap<EntityName, f32>,
    pub incoming_by_source: HashMap<EntityName, f32>,
    
    pub outgoing_by_character: HashMap<String, f32>,
    pub incoming_by_character: HashMap<String, f32>,
    
    // Per-character detailed maps
    pub outgoing_by_char_weapon: HashMap<String, HashMap<WeaponName, f32>>,
    pub outgoing_by_char_target: HashMap<String, HashMap<EntityName, f32>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct FightSummary {
    pub start: Duration,
    pub end: Duration,
    pub total_damage: f32,
    pub samples: Vec<DpsSample>,
}
