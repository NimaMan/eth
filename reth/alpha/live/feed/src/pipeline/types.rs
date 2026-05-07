use eth_live_state::EncodedChainStateSnapshot;
use tx_processor::ProcessedBlock;

#[derive(Clone, Debug)]
pub struct LiveFeedBlockInput {
    pub block: ProcessedBlock,
    pub chain_state: Option<EncodedChainStateSnapshot>,
}

impl LiveFeedBlockInput {
    pub const fn new(
        block: ProcessedBlock,
        chain_state: Option<EncodedChainStateSnapshot>,
    ) -> Self {
        Self { block, chain_state }
    }
}
