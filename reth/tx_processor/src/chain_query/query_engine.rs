/// Main ChainQuery implementation
/// 
/// Central hub for all blockchain data queries using direct database access

use alloy_primitives::{Address, U256, B256};
use eyre::Result;
use reth_tx_simulator::RethTxSimulator;
use std::sync::Arc;

use super::{
    account::AccountQuery,
    token::TokenQuery,
    storage::StorageQuery,
    block::BlockQuery,
};

/// ChainQuery provides direct database access for blockchain queries
/// 
/// This is the main entry point for querying blockchain state without RPC calls.
/// It uses Reth's database directly for maximum performance.
pub struct ChainQuery {
    /// The underlying simulator that provides database access
    simulator: Arc<RethTxSimulator>,
    
    /// Account query module
    pub account: AccountQuery,
    
    /// Token query module
    pub token: TokenQuery,
    
    /// Storage query module
    pub storage: StorageQuery,
    
    /// Block query module
    pub block: BlockQuery,
}

impl ChainQuery {
    /// Create a new ChainQuery instance
    /// 
    /// # Arguments
    /// * `reth_datadir` - Path to Reth data directory (e.g., "/home/user/.local/share/reth/mainnet")
    pub fn new(reth_datadir: &str) -> Result<Self> {
        let simulator = Arc::new(RethTxSimulator::new(reth_datadir)?);
        
        Ok(Self {
            account: AccountQuery::new(simulator.clone()),
            token: TokenQuery::new(simulator.clone()),
            storage: StorageQuery::new(simulator.clone()),
            block: BlockQuery::new(simulator.clone()),
            simulator,
        })
    }
    
    /// Create ChainQuery from existing simulator
    pub fn from_simulator(simulator: Arc<RethTxSimulator>) -> Self {
        Self {
            account: AccountQuery::new(simulator.clone()),
            token: TokenQuery::new(simulator.clone()),
            storage: StorageQuery::new(simulator.clone()),
            block: BlockQuery::new(simulator.clone()),
            simulator,
        }
    }
    
    /// Get the latest block number
    pub fn get_latest_block(&self) -> Result<u64> {
        self.simulator.get_latest_block()
    }
    
    /// Get provider factory for advanced queries
    pub(crate) fn provider_factory(&self) -> &reth_tx_simulator::RethTxSimulator {
        &self.simulator
    }
    
    /// Get shared simulator for trading simulation
    pub fn get_simulator(&self) -> Arc<RethTxSimulator> {
        self.simulator.clone()
    }
    
    // Convenience methods that delegate to specialized modules
    
    /// Get ETH balance for an address
    pub async fn get_balance(&self, address: Address, block_number: Option<u64>) -> Result<U256> {
        self.account.get_balance(address, block_number).await
    }
    
    /// Get nonce for an address
    pub async fn get_nonce(&self, address: Address, block_number: Option<u64>) -> Result<u64> {
        self.account.get_nonce(address, block_number).await
    }
    
    /// Check if address has code (is a contract)
    pub async fn has_code(&self, address: Address, block_number: Option<u64>) -> Result<bool> {
        self.account.has_code(address, block_number).await
    }
    
    /// Get ERC20 token balance
    pub async fn get_token_balance(&self, token: Address, holder: Address, block_number: Option<u64>) -> Result<U256> {
        self.token.get_erc20_balance(token, holder, block_number).await
    }
    
    /// Get ERC20 total supply
    pub async fn get_token_total_supply(&self, token: Address, block_number: Option<u64>) -> Result<U256> {
        self.token.get_erc20_total_supply(token, block_number).await
    }
    
    /// Get storage value at specific slot
    pub async fn get_storage_at(&self, address: Address, slot: B256, block_number: Option<u64>) -> Result<U256> {
        self.storage.get_storage_at(address, slot, block_number).await
    }
}