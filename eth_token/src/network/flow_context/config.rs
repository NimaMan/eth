//! Configuration for second-order fund-flow context construction.

use serde::{Deserialize, Serialize};

/// Defaults are intentionally conservative because this layer may fan out from
/// a small token graph into many historical blocks.
pub const DEFAULT_MAX_SEED_ADDRESSES: usize = 64;
pub const DEFAULT_LOOKBACK_BLOCKS: u64 = 300;
pub const DEFAULT_LOOKAHEAD_BLOCKS: u64 = 80;
pub const DEFAULT_MAX_BLOCKS_PER_ADDRESS: usize = 512;
pub const DEFAULT_MAX_PATH_DEPTH: usize = 3;
pub const DEFAULT_MIN_SCALED_AMOUNT: f64 = 0.000001;
pub const DEFAULT_HUB_DEGREE_THRESHOLD: usize = 24;
pub const DEFAULT_MAX_CONTEXT_EDGES: usize = 512;
pub const DEFAULT_MAX_EVIDENCE_PER_EDGE: usize = 8;
pub const DEFAULT_MIN_PROMOTION_CONFIDENCE: f64 = 0.65;

/// Tunables for token-scoped fund-flow context discovery.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlowContextConfig {
    pub max_seed_addresses: usize,
    pub lookback_blocks: u64,
    pub lookahead_blocks: u64,
    pub max_blocks_per_address: usize,
    pub max_path_depth: usize,
    pub min_scaled_amount: f64,
    pub hub_degree_threshold: usize,
    pub max_context_edges: usize,
    pub max_evidence_per_edge: usize,
    pub min_promotion_confidence: f64,
}

impl Default for FlowContextConfig {
    fn default() -> Self {
        Self {
            max_seed_addresses: DEFAULT_MAX_SEED_ADDRESSES,
            lookback_blocks: DEFAULT_LOOKBACK_BLOCKS,
            lookahead_blocks: DEFAULT_LOOKAHEAD_BLOCKS,
            max_blocks_per_address: DEFAULT_MAX_BLOCKS_PER_ADDRESS,
            max_path_depth: DEFAULT_MAX_PATH_DEPTH,
            min_scaled_amount: DEFAULT_MIN_SCALED_AMOUNT,
            hub_degree_threshold: DEFAULT_HUB_DEGREE_THRESHOLD,
            max_context_edges: DEFAULT_MAX_CONTEXT_EDGES,
            max_evidence_per_edge: DEFAULT_MAX_EVIDENCE_PER_EDGE,
            min_promotion_confidence: DEFAULT_MIN_PROMOTION_CONFIDENCE,
        }
    }
}
