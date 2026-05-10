//! Shared-intermediary discovery for holder clusters.

use alloy_primitives::Address;

/// An intermediary address that connects multiple holders.
#[derive(Clone, Debug)]
pub struct IntermediaryNode {
    pub address: Address,
    pub in_degree: usize,
    pub total_denom_received: f64,
    pub labels: Vec<String>,
}

/// Discovers shared intermediaries between token holders.
pub struct IntermediaryDetector;

impl IntermediaryDetector {
    pub fn new() -> Self {
        Self
    }

    /// Find addresses that receive funds from multiple holders but don't hold the token.
    pub fn find_shared_intermediaries(&self) -> Vec<IntermediaryNode> {
        // TODO: Implement shared-intermediary detection
        vec![]
    }
}
