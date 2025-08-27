/// Account-related blockchain queries
/// 
/// Provides methods for querying account state including balances, nonces, and code

use alloy_primitives::{Address, U256};
use eyre::Result;
use tx_simulator::TxSimulator;
use std::sync::Arc;

/// Account information structure
#[derive(Debug, Clone)]
pub struct AccountInfo {
    pub address: Address,
    pub balance: U256,
    pub nonce: u64,
    pub has_code: bool,
}

/// Account query module
pub struct AccountQuery {
    simulator: Arc<TxSimulator>,
}

impl AccountQuery {
    /// Create new AccountQuery instance
    pub fn new(simulator: Arc<TxSimulator>) -> Self {
        Self { simulator }
    }
    
    /// Get ETH balance for an address at a specific block
    pub async fn get_balance(&self, address: Address, block_number: Option<u64>) -> Result<U256> {
        // Get block number to query at
        let block_number = if let Some(bn) = block_number {
            bn
        } else {
            self.simulator.get_latest_block()?
        };
        
        // Get provider at block
        let provider = self.simulator.get_chain_state_at_block(block_number)
            .map_err(|e| eyre::eyre!("Failed to get provider at block {}: {}", block_number, e))?;
        
        // Get account info
        let account_info = provider.basic_account(&address)
            .map_err(|e| eyre::eyre!("Failed to get account info: {}", e))?;
        
        Ok(account_info.map(|a| a.balance).unwrap_or(U256::ZERO))
    }
    
    /// Get nonce for an address at a specific block
    pub async fn get_nonce(&self, address: Address, block_number: Option<u64>) -> Result<u64> {
        // Get block number to query at
        let block_number = if let Some(bn) = block_number {
            bn
        } else {
            self.simulator.get_latest_block()?
        };
        
        // Get provider at block
        let provider = self.simulator.get_chain_state_at_block(block_number)
            .map_err(|e| eyre::eyre!("Failed to get provider at block {}: {}", block_number, e))?;
        
        // Get account info
        let account_info = provider.basic_account(&address)
            .map_err(|e| eyre::eyre!("Failed to get account info: {}", e))?;
        
        Ok(account_info.map(|a| a.nonce).unwrap_or(0))
    }
    
    /// Check if an address has code (is a contract)
    pub async fn has_code(&self, address: Address, block_number: Option<u64>) -> Result<bool> {
        // Get block number to query at
        let block_number = if let Some(bn) = block_number {
            bn
        } else {
            self.simulator.get_latest_block()?
        };
        
        // Get provider at block
        let provider = self.simulator.get_chain_state_at_block(block_number)
            .map_err(|e| eyre::eyre!("Failed to get provider at block {}: {}", block_number, e))?;
        
        // Check if account has code
        let has_code = provider.account_code(&address)
            .map_err(|e| eyre::eyre!("Failed to get account code: {}", e))?
            .map(|code| !code.is_empty())
            .unwrap_or(false);
        
        Ok(has_code)
    }
    
    /// Get complete account information
    pub async fn get_account_info(&self, address: Address, block_number: Option<u64>) -> Result<AccountInfo> {
        // Get block number to query at
        let block_number = if let Some(bn) = block_number {
            bn
        } else {
            self.simulator.get_latest_block()?
        };
        
        // Get provider at block
        let provider = self.simulator.get_chain_state_at_block(block_number)
            .map_err(|e| eyre::eyre!("Failed to get provider at block {}: {}", block_number, e))?;
        
        // Get account info
        let account = provider.basic_account(&address)
            .map_err(|e| eyre::eyre!("Failed to get account info: {}", e))?;
        
        // Check for code
        let has_code = provider.account_code(&address)
            .map_err(|e| eyre::eyre!("Failed to get account code: {}", e))?
            .map(|code| !code.is_empty())
            .unwrap_or(false);
        
        Ok(AccountInfo {
            address,
            balance: account.as_ref().map(|a| a.balance).unwrap_or(U256::ZERO),
            nonce: account.map(|a| a.nonce).unwrap_or(0),
            has_code,
        })
    }
}