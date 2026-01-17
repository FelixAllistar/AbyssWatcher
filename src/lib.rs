#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
// The ultimate strictness: catches things like missing documentation or overflow risks
#![warn(clippy::restriction)]
pub mod core;

pub mod app;
pub use app::run;
