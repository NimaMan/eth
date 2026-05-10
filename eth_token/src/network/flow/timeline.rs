//! Block-by-block fund flow timeline.
//!
//! Shows when funds moved between addresses in chronological order,
//! useful for detecting "funded then bought" patterns.

use alloy_primitives::Address;

/// A single event in a fund flow timeline.
#[derive(Clone, Debug)]
pub struct TimelineEvent {
    pub block_number: u64,
    pub timestamp: u64,
    pub from: Address,
    pub to: Address,
    pub amount: f64,
    pub asset: String,
    pub tx_hash: String,
}

/// Chronological timeline of fund flows for a set of addresses.
#[derive(Clone, Debug)]
pub struct FundFlowTimeline {
    pub events: Vec<TimelineEvent>,
}

impl FundFlowTimeline {
    pub fn new() -> Self {
        Self { events: vec![] }
    }

    /// Build a timeline for a group of related addresses.
    pub fn for_addresses(_addresses: &[Address]) -> Self {
        // TODO: Load blocks from address-block index, extract transfers
        Self::new()
    }

    /// Detect "funded then bought" patterns.
    pub fn find_fund_then_buy_patterns(&self) -> Vec<(TimelineEvent, TimelineEvent)> {
        // TODO: Find pairs of events where funding is followed by token purchase
        vec![]
    }
}
