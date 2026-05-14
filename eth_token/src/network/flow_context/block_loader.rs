//! Processed-block loading abstraction for flow-context discovery.

use eyre::Result;
use tx_processor::ProcessedBlock;

/// Loads processed blocks selected through the address participation index.
pub trait ProcessedBlockLoader {
    fn load_blocks(&self, block_numbers: &[u64]) -> Result<Vec<ProcessedBlock>>;
}

/// No-op implementation useful until a real processed-block cache/provider is
/// wired in by `eth_chain_server`.
#[derive(Clone, Copy, Debug, Default)]
pub struct EmptyProcessedBlockLoader;

impl ProcessedBlockLoader for EmptyProcessedBlockLoader {
    fn load_blocks(&self, _block_numbers: &[u64]) -> Result<Vec<ProcessedBlock>> {
        Ok(Vec::new())
    }
}
