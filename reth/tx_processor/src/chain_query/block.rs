/// Block-related queries
/// 
/// Provides methods for querying block information including timestamps, base fees, and headers

use alloy_primitives::{U256, B256};
use eyre::Result;
use reth_tx_simulator::RethTxSimulator;
use std::sync::Arc;

/// Block information structure
#[derive(Debug, Clone)]
pub struct BlockInfo {
    pub number: u64,
    pub timestamp: u64,
    pub base_fee_per_gas: Option<u128>,
    pub gas_limit: u64,
    pub gas_used: u64,
}

/// Block query module
pub struct BlockQuery {
    simulator: Arc<RethTxSimulator>,
}

impl BlockQuery {
    /// Create new BlockQuery instance
    pub fn new(simulator: Arc<RethTxSimulator>) -> Self {
        Self { simulator }
    }
    
    /// Get the latest block number
    pub fn get_latest_block(&self) -> Result<u64> {
        self.simulator.get_latest_block()
    }
    
    /// Get the base fee for the latest block
    pub fn get_latest_base_fee(&self) -> Result<u128> {
        let latest_block = self.simulator.get_latest_block()?;
        self.simulator.get_base_fee_at_block(latest_block)
    }
    
    /// Get the base fee for a specific block
    pub fn get_base_fee_at_block(&self, block_number: u64) -> Result<u128> {
        self.simulator.get_base_fee_at_block(block_number)
    }
    
    /// Get block header information
    /// Note: Currently returns basic info; full header access requires additional APIs
    pub async fn get_block_info(&self, block_number: Option<u64>) -> Result<BlockInfo> {
        // Get block number to query
        let block_number = if let Some(bn) = block_number {
            bn
        } else {
            self.simulator.get_latest_block()?
        };
        
        // Get base fee for the block
        let base_fee_per_gas = Some(self.simulator.get_base_fee_at_block(block_number)?);
        
        // For now, return basic info
        // Full header access would require additional provider methods
        Ok(BlockInfo {
            number: block_number,
            timestamp: 0, // Would need header access for actual timestamp
            base_fee_per_gas,
            gas_limit: 30_000_000, // Standard mainnet gas limit
            gas_used: 0, // Would need header access for actual gas used
        })
    }
    
    /// Get block timestamp
    pub async fn get_block_timestamp(&self, block_number: Option<u64>) -> Result<u64> {
        let info = self.get_block_info(block_number).await?;
        Ok(info.timestamp)
    }
    
    /// Get a range of block headers
    pub async fn get_block_range(&self, start: u64, end: u64) -> Result<Vec<BlockInfo>> {
        if start > end {
            return Err(eyre::eyre!("Invalid block range: start {} > end {}", start, end));
        }
        
        let mut blocks = Vec::with_capacity((end - start + 1) as usize);
        
        for block_num in start..=end {
            match self.get_block_info(Some(block_num)).await {
                Ok(info) => blocks.push(info),
                Err(e) => {
                    // Log error but continue with other blocks
                    eprintln!("Failed to get block {}: {}", block_num, e);
                }
            }
        }
        
        Ok(blocks)
    }
    
    /// Check if a block exists
    pub async fn block_exists(&self, block_number: u64) -> Result<bool> {
        // Try to get provider at block - if it succeeds, block exists
        match self.simulator.provider_factory()
            .history_by_block_number(block_number.into()) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}