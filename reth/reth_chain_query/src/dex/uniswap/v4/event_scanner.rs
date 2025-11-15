use alloy_primitives::{keccak256, Address, B256};
use eyre::Result;

use crate::provider::RethQueryProvider;

impl RethQueryProvider {
    /// Scan recent blocks for Uniswap V4 pool initialization events and return the latest pool id.
    pub async fn uni_v4_find_recent_pool_id(
        &self,
        pool_manager: Address,
        blocks_back: u64,
    ) -> Result<Option<(B256, u64)>> {
        // Initialize(bytes32,address,address,uint24,int24,address,uint160,int24)
        let init_sig =
            keccak256(b"Initialize(bytes32,address,address,uint24,int24,address,uint160,int24)");

        let latest = self.get_latest_block()?;
        let start = latest.saturating_sub(blocks_back);

        for block in (start..=latest).rev() {
            let receipts = match self.fetch_block_receipts_only(block).await {
                Ok(r) => r,
                Err(_) => continue,
            };
            for rec in receipts {
                for log in rec.logs {
                    if log.address == pool_manager
                        && !log.topics.is_empty()
                        && log.topics[0] == init_sig
                        && log.topics.len() >= 2
                    {
                        return Ok(Some((log.topics[1], block)));
                    }
                }
            }
        }

        Ok(None)
    }
}
