#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
// The ultimate strictness: catches things like missing documentation or overflow risks
#![warn(clippy::restriction)] 
pub mod core {
    pub mod analysis;
    pub mod log_io;
    pub mod model;
    pub mod parser;
    pub mod tracker;
    pub mod state;
    pub mod watcher;
    pub mod coordinator;
    pub mod config;
    pub mod replay_engine;
    #[cfg(test)]
    pub mod sim_test;
    #[cfg(test)]
    pub mod bench_analysis;
}

pub mod app;
pub use app::run;
