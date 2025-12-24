use std::collections::HashMap;
use std::time::Duration;

pub type EntityName = String;
pub type WeaponName = String;

#[derive(Clone, Debug)]
pub struct CombatEvent {
    pub timestamp: Duration,
    pub source: EntityName,
    pub target: EntityName,
    pub weapon: WeaponName,
    pub damage: f32,
    pub incoming: bool,
    pub character: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DpsSample {
    pub time: Duration,
    pub outgoing_dps: f32,
    pub incoming_dps: f32,
    pub outgoing_by_weapon: HashMap<WeaponName, f32>,
    pub outgoing_by_target: HashMap<EntityName, f32>,
    pub incoming_by_source: HashMap<EntityName, f32>,
    pub outgoing_by_character: HashMap<String, f32>,
    pub incoming_by_character: HashMap<String, f32>,
    pub outgoing_by_char_weapon: HashMap<String, HashMap<WeaponName, f32>>,
    pub outgoing_by_char_target: HashMap<String, HashMap<EntityName, f32>>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct FightSummary {
    pub start: Duration,
    pub end: Duration,
    pub total_damage: f32,
    pub samples: Vec<DpsSample>,
}
