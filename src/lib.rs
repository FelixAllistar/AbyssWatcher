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
