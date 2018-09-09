use std::time::Duration;

#[derive(Clone)]
pub struct Config {
    pub max_active_view_size: usize,
    pub max_passive_view_size: usize,
    pub active_rwl: usize,
    pub passive_rwl: usize,
    pub shuffle_rwl: usize,
    pub shuffle_active: usize,
    pub shuffle_passive: usize,
    pub shuffle_interval: Duration,
}

impl Config {
    pub fn default() -> Config {
        Config {
            max_active_view_size: 4,
            max_passive_view_size: 4,
            active_rwl: 3,
            passive_rwl: 2,
            shuffle_rwl: 1,
            shuffle_active: 2,
            shuffle_passive: 2,
            shuffle_interval: Duration::from_secs(30),
        }
    }
}
