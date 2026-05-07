use eth_live_state::{EncodedChainStateSnapshot, ProcessedBlockSnapshot};

#[derive(Clone, Debug, PartialEq)]
pub struct LiveFeedBlockInput {
    pub block: ProcessedBlockSnapshot,
    pub chain_state: Option<EncodedChainStateSnapshot>,
}

impl LiveFeedBlockInput {
    pub const fn new(
        block: ProcessedBlockSnapshot,
        chain_state: Option<EncodedChainStateSnapshot>,
    ) -> Self {
        Self { block, chain_state }
    }
}
