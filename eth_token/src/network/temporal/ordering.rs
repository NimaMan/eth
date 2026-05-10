//! Transaction ordering analysis within blocks.

use alloy_primitives::Address;

/// A detected temporal pattern between two addresses.
#[derive(Clone, Debug)]
pub struct TemporalPattern {
    pub pattern_kind: TemporalPatternKind,
    pub address_a: Address,
    pub address_b: Address,
    pub block_number: u64,
    pub confidence: f64,
}

/// Kinds of temporal patterns we can detect.
#[derive(Clone, Debug)]
pub enum TemporalPatternKind {
    FundThenBuy,
    BuyThenTransfer,
    RemoveLiquidityThenDump,
    CreatorFundsThenEnablesTrading,
}

/// Analyzes ordering patterns between addresses.
pub struct OrderingAnalyzer;

impl OrderingAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Detect patterns between two addresses in a block range.
    pub fn analyze_pair(
        &self,
        _a: Address,
        _b: Address,
        _start_block: u64,
        _end_block: u64,
    ) -> Vec<TemporalPattern> {
        // TODO: Load blocks from index, order events by log_index, match patterns
        vec![]
    }
}
