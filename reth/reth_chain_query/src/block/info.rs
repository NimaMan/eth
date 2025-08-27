/// Block-related queries
/// 
/// Provides methods for querying block information including timestamps, base fees, and headers

// use alloy_primitives::{U256, B256}; // Commented out unused imports
use eyre::Result;
use tx_simulator::TxSimulator;
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
    simulator: Arc<TxSimulator>,
}

impl BlockQuery {
    /// Create new BlockQuery instance
    pub fn new(simulator: Arc<TxSimulator>) -> Self {
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
    pub async fn get_block_info(&self, block_number: Option<u64>) -> Result<BlockInfo> {
        // Get block number to query
        let block_number = if let Some(bn) = block_number {
            bn
        } else {
            self.simulator.get_latest_block()?
        };
        
        // Get complete header information from the database
        let (timestamp, gas_limit, gas_used, base_fee_per_gas) = 
            self.simulator.get_block_metadata(block_number)?;
        
        Ok(BlockInfo {
            number: block_number,
            timestamp,
            base_fee_per_gas,
            gas_limit,
            gas_used,
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
        match self.simulator.get_chain_state_at_block(block_number) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}