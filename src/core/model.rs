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
}

#[derive(Clone, Debug, PartialEq)]
pub struct DpsSample {
    pub time: Duration,
    pub total_dps: f32,
    pub by_weapon: HashMap<WeaponName, f32>,
    pub by_target: HashMap<EntityName, f32>,
}

#[derive(Clone, Debug)]
pub struct FightSummary {
    pub start: Duration,
    pub end: Duration,
    pub total_damage: f32,
    pub samples: Vec<DpsSample>,
}

