//! Session configuration.

/// Limits and memory settings for a session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Config {
    /// Maximum evaluator steps for one top level evaluation.
    pub max_steps: u64,
    /// Maximum recursive evaluation depth.
    pub max_depth: u32,
    /// Initial expression arena capacity in bytes.
    pub initial_arena_bytes: usize,
    /// Garbage collection threshold in bytes.
    pub gc_threshold_bytes: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            max_steps: 1_000_000,
            max_depth: 2048,
            initial_arena_bytes: 1 << 20,
            gc_threshold_bytes: 8 << 20,
        }
    }
}
