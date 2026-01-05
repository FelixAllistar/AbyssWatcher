// Alert system module for configurable event-triggered notifications.
//
// Architecture:
// - model.rs: Alert configuration and event types
// - triggers.rs: Trigger evaluation logic for combat/notify events
// - engine.rs: Orchestrates trigger evaluation and action dispatch

pub mod engine;
pub mod model;
pub mod triggers;
