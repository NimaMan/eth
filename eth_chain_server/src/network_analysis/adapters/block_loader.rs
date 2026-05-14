use std::collections::BTreeMap;

use eth_token::network::flow_context::block_loader::ProcessedBlockLoader;
use eyre::{bail, Result};
use tx_processor::{ProcessedBlock, ProcessedBlockProvider};

use crate::network_analysis::job::NetworkAnalysisJob;

#[derive(Clone, Debug)]
pub struct PreloadedProcessedBlockLoader {
    blocks_by_number: BTreeMap<u64, ProcessedBlock>,
}

impl PreloadedProcessedBlockLoader {
    pub fn new(blocks_by_number: BTreeMap<u64, ProcessedBlock>) -> Self {
        Self { blocks_by_number }
    }
}

impl ProcessedBlockLoader for PreloadedProcessedBlockLoader {
    fn load_blocks(&self, block_numbers: &[u64]) -> Result<Vec<ProcessedBlock>> {
        block_numbers
            .iter()
            .map(|block_number| {
                self.blocks_by_number
                    .get(block_number)
                    .cloned()
                    .ok_or_else(|| eyre::eyre!("context block {block_number} was not preloaded"))
            })
            .collect()
    }
}

pub async fn load_blocks(
    block_provider: &ProcessedBlockProvider,
    block_numbers: &[u64],
    job: &NetworkAnalysisJob,
    stage: &str,
) -> Result<BTreeMap<u64, ProcessedBlock>> {
    let mut blocks_by_number = BTreeMap::new();
    for block_number in block_numbers {
        if job.stop_requested() {
            bail!("network analysis canceled");
        }

        job.update_progress(|progress| {
            progress.stage = stage.to_string();
        })
        .await;

        let loaded = block_provider.load_block(*block_number).await?;
        blocks_by_number.insert(loaded.block.header.number, loaded.block);
        job.update_progress(|progress| {
            progress.blocks_loaded += 1;
        })
        .await;
    }
    Ok(blocks_by_number)
}
