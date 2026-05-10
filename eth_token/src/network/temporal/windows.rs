//! Sliding window coactivity detection.

use alloy_primitives::Address;

/// Addresses that were active together in a time window.
#[derive(Clone, Debug)]
pub struct CoactivityWindow {
    pub start_block: u64,
    pub end_block: u64,
    pub addresses: Vec<Address>,
    pub activity_kind: String,
}

/// Detects coactivity in sliding windows.
pub struct WindowDetector;

impl WindowDetector {
    pub fn new() -> Self {
        Self
    }

    /// Find addresses that performed the same activity in a window.
    pub fn find_coactivity(
        &self,
        _addresses: &[Address],
        _window_size_blocks: u64,
    ) -> Vec<CoactivityWindow> {
        // TODO: Implement sliding window detection
        vec![]
    }
}
