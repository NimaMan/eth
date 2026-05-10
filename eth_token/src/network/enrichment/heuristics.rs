//! On-chain heuristic label assignment.

use alloy_primitives::Address;

/// Heuristic labeler based on graph structure.
pub struct HeuristicLabeler;

impl HeuristicLabeler {
    pub fn new() -> Self {
        Self
    }

    /// Detect router addresses: high out-degree, no token holdings, fee source to many.
    pub fn detect_routers(&self) -> Vec<Address> {
        // TODO: Implement router heuristic
        vec![]
    }

    /// Detect CEX deposit addresses: receive from many, send to few large balances.
    pub fn detect_cex_deposits(&self) -> Vec<Address> {
        // TODO: Implement CEX deposit heuristic
        vec![]
    }

    /// Detect fresh wallets: low activity count, first seen near pool creation.
    pub fn detect_fresh_wallets(&self) -> Vec<Address> {
        // TODO: Implement fresh wallet heuristic
        vec![]
    }
}
