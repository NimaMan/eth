/// Main ChainQuery implementation
/// 
/// Central hub for all blockchain data queries using direct database access

use alloy_primitives::{Address, U256, B256};
use eyre::Result;
use tx_simulator::TxSimulator;
use std::sync::Arc;

use super::{
    time_utils::BlockTimeConverter,
    provider::RethQueryProvider,
};

/// ChainQuery provides direct database access for blockchain queries
/// 
/// This is the main entry point for querying blockchain state without RPC calls.
/// It uses Reth's database directly for maximum performance.
pub struct ChainQuery {
    /// The underlying simulator that provides database access
    simulator: Arc<TxSimulator>,
    
    /// Provider for direct access
    provider: Arc<RethQueryProvider>,
    
    /// Block-time converter for timestamp operations
    pub time_converter: Arc<BlockTimeConverter>,
}

impl ChainQuery {
    /// Create a new ChainQuery instance
    /// 
    /// # Arguments
    /// * `reth_datadir` - Path to Reth data directory (e.g., "/home/user/.local/share/reth/mainnet")
    pub fn new(reth_datadir: &str) -> Result<Self> {
        let simulator = Arc::new(TxSimulator::new(reth_datadir)?);
        let time_converter = Arc::new(BlockTimeConverter::new(simulator.clone()));
        let provider = Arc::new(RethQueryProvider::with_simulator(simulator.clone())?);
        
        Ok(Self {
            provider,
            time_converter,
            simulator,
        })
    }
    
    /// Create ChainQuery from existing simulator
    pub fn from_simulator(simulator: Arc<TxSimulator>) -> Result<Self> {
        let time_converter = Arc::new(BlockTimeConverter::new(simulator.clone()));
        let provider = Arc::new(RethQueryProvider::with_simulator(simulator.clone())?);
        
        Ok(Self {
            provider,
            time_converter,
            simulator,
        })
    }
    
    /// Get the latest block number (uses best_block_number from database)
    /// 
    /// This returns the latest fully processed block from Reth's Finish stage checkpoint.
    /// This is the most recent block that has completed all pipeline stages.
    pub fn get_latest_block(&self) -> Result<u64> {
        self.simulator.get_latest_block()
    }
    
    /// Get provider factory for advanced queries
    pub(crate) fn provider_factory(&self) -> &tx_simulator::TxSimulator {
        &self.simulator
    }
    
    /// Get shared simulator for trading simulation
    pub fn get_simulator(&self) -> Arc<TxSimulator> {
        self.simulator.clone()
    }
    
    // Convenience methods that delegate to specialized modules
    
    /// Get ETH balance for an address
    pub async fn get_balance(&self, address: Address, block_number: Option<u64>) -> Result<U256> {
        self.provider.get_eth_balance(address, block_number).await
    }
    
    /// Get nonce for an address
    pub async fn get_nonce(&self, address: Address, block_number: Option<u64>) -> Result<u64> {
        self.provider.get_transaction_count_at_block(address, block_number).await
    }
    
    /// Check if address has code (is a contract)
    pub async fn has_code(&self, address: Address, block_number: Option<u64>) -> Result<bool> {
        self.provider.has_code(address, block_number).await
    }
    
    /// Get ERC20 token balance
    pub async fn get_token_balance(&self, token: Address, holder: Address, block_number: Option<u64>) -> Result<U256> {
        self.provider.get_token_balance(token, holder, block_number).await
    }
    
    /// Get ERC20 total supply
    pub async fn get_token_total_supply(&self, token: Address, block_number: Option<u64>) -> Result<U256> {
        self.provider.get_token_total_supply(token, block_number).await
    }
    
    /// Get storage value at specific slot
    pub async fn get_storage_at(&self, address: Address, slot: B256, block_number: Option<u64>) -> Result<U256> {
        self.provider.get_storage(address, slot, block_number).await
    }
}