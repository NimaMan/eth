//! Address overlap between token networks.

use alloy_primitives::Address;
use std::collections::BTreeSet;

/// Overlap metrics between two token networks.
#[derive(Clone, Debug)]
pub struct NetworkOverlap {
    pub token_a: Address,
    pub token_b: Address,
    pub shared_addresses: BTreeSet<Address>,
    pub shared_percentage: f64,
}

/// Computes address overlap between token networks.
pub struct OverlapCalculator;

impl OverlapCalculator {
    pub fn new() -> Self {
        Self
    }

    pub fn compute_overlap(
        &self,
        _token_a: Address,
        _token_b: Address,
    ) -> Option<NetworkOverlap> {
        // TODO: Implement overlap calculation
        None
    }
}
