//! Time-window coactivity clustering.

use alloy_primitives::Address;

/// A group of addresses that were active in the same time window.
#[derive(Clone, Debug)]
pub struct TimeWindowCluster {
    pub start_block: u64,
    pub end_block: u64,
    pub addresses: Vec<Address>,
    pub activity_kind: String,
    pub shared_count: usize,
}

/// Detects coactivity clusters in block windows.
pub struct TimeWindowClusterDetector;

impl TimeWindowClusterDetector {
    pub fn new() -> Self {
        Self
    }

    /// Find addresses that traded together in a window.
    pub fn detect_coactivity(
        &self,
        _window_size_blocks: u64,
    ) -> Vec<TimeWindowCluster> {
        // TODO: Implement time-window coactivity detection
        vec![]
    }
}
