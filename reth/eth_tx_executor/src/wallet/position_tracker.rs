//! Position Tracker
//!
//! Tracks token balances and positions to ensure we only try to sell tokens we own

use ethers::prelude::*;
use ethers::abi::Abi;
use ethers::utils::format_ether;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug};

/// Token position information
#[derive(Debug, Clone, Default)]
pub struct TokenPosition {
    /// Token contract address
    pub address: Address,
    /// Token symbol
    pub symbol: String,
    /// Number of decimals
    pub decimals: u8,
    /// Current balance (in token units)
    pub balance: U256,
    /// Balance in human-readable format
    pub balance_formatted: f64,
    /// Approximate value in ETH
    pub value_eth: f64,
    /// Last update timestamp
    pub last_updated: u64,
}

/// Position tracker for managing token balances
pub struct PositionTracker {
    /// Ethereum provider
    provider: Arc<Provider<Http>>,
    /// Wallet address to track
    wallet_address: Address,
    /// Token positions (token address -> position)
    positions: Arc<RwLock<HashMap<Address, TokenPosition>>>,
    /// ERC20 ABI for balance queries
    erc20_abi: BaseContract,
}

impl PositionTracker {
    /// Create new position tracker
    pub fn new(provider: Arc<Provider<Http>>, wallet_address: Address) -> Result<Self, Box<dyn std::error::Error>> {
        // Minimal ERC20 ABI for balance and decimals
        let erc20_abi_json = r#"[
            {
                "constant": true,
                "inputs": [{"name": "_owner", "type": "address"}],
                "name": "balanceOf",
                "outputs": [{"name": "balance", "type": "uint256"}],
                "type": "function"
            },
            {
                "constant": true,
                "inputs": [],
                "name": "decimals",
                "outputs": [{"name": "", "type": "uint8"}],
                "type": "function"
            },
            {
                "constant": true,
                "inputs": [],
                "name": "symbol",
                "outputs": [{"name": "", "type": "string"}],
                "type": "function"
            }
        ]"#;
        
        let erc20_abi = BaseContract::from(serde_json::from_str::<Abi>(erc20_abi_json)?);
        
        Ok(Self {
            provider,
            wallet_address,
            positions: Arc::new(RwLock::new(HashMap::new())),
            erc20_abi,
        })
    }
    
    /// Update balance for a specific token
    pub async fn update_token_balance(&self, token_address: Address) -> Result<TokenPosition, Box<dyn std::error::Error>> {
        debug!("Updating balance for token {}", token_address);
        
        // Get token contract
        let token = Contract::new(token_address, self.erc20_abi.clone(), self.provider.clone());
        
        // Query balance
        let balance: U256 = token
            .method::<_, U256>("balanceOf", self.wallet_address)?
            .call()
            .await?;
        
        // Get decimals and symbol (with fallbacks)
        let decimals: u8 = token
            .method::<_, u8>("decimals", ())?
            .call()
            .await
            .unwrap_or(18);
            
        let symbol: String = token
            .method::<_, String>("symbol", ())?
            .call()
            .await
            .unwrap_or_else(|_| "UNKNOWN".to_string());
        
        // Calculate formatted balance
        let divisor = U256::from(10).pow(U256::from(decimals));
        let balance_formatted = if balance > U256::zero() {
            balance.as_u128() as f64 / divisor.as_u128() as f64
        } else {
            0.0
        };
        
        let position = TokenPosition {
            address: token_address,
            symbol: symbol.clone(),
            decimals,
            balance,
            balance_formatted,
            value_eth: 0.0, // TODO: Calculate from price oracle
            last_updated: chrono::Utc::now().timestamp() as u64,
        };
        
        // Update cache
        {
            let mut positions = self.positions.write().await;
            positions.insert(token_address, position.clone());
        }
        
        info!("Updated {} balance: {} ({})", 
            symbol, balance_formatted, balance);
        
        Ok(position)
    }
    
    /// Get current position for a token (from cache or fresh)
    pub async fn get_position(&self, token_address: Address) -> Result<TokenPosition, Box<dyn std::error::Error>> {
        // Check cache first
        {
            let positions = self.positions.read().await;
            if let Some(position) = positions.get(&token_address) {
                // Return cached if less than 60 seconds old
                let now = chrono::Utc::now().timestamp() as u64;
                if now - position.last_updated < 60 {
                    return Ok(position.clone());
                }
            }
        }
        
        // Update and return fresh data
        self.update_token_balance(token_address).await
    }
    
    /// Get balance for a token
    pub async fn get_balance(&self, token_address: Address) -> Result<U256, Box<dyn std::error::Error>> {
        let position = self.get_position(token_address).await?;
        Ok(position.balance)
    }
    
    /// Check if we have sufficient balance for a trade
    pub async fn has_sufficient_balance(&self, token_address: Address, amount: U256) -> Result<bool, Box<dyn std::error::Error>> {
        let balance = self.get_balance(token_address).await?;
        Ok(balance >= amount)
    }
    
    /// Update ETH balance
    pub async fn update_eth_balance(&self) -> Result<U256, Box<dyn std::error::Error>> {
        let balance = self.provider.get_balance(self.wallet_address, None).await?;
        info!("ETH balance: {} ETH", format_ether(balance));
        Ok(balance)
    }
    
    /// Get all tracked positions
    pub async fn get_all_positions(&self) -> Vec<TokenPosition> {
        let positions = self.positions.read().await;
        positions.values().cloned().collect()
    }
    
    /// Clear position cache
    pub async fn clear_cache(&self) {
        let mut positions = self.positions.write().await;
        positions.clear();
        info!("Position cache cleared");
    }
    
    /// Remove a position from tracking
    pub async fn remove_position(&self, token_address: Address) {
        let mut positions = self.positions.write().await;
        if positions.remove(&token_address).is_some() {
            info!("Removed position for {}", token_address);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_position_tracker_creation() {
        let provider = Provider::<Http>::try_from("http://localhost:8545").unwrap();
        let wallet = "0x0000000000000000000000000000000000000001".parse::<Address>().unwrap();
        
        let tracker = PositionTracker::new(Arc::new(provider), wallet);
        assert!(tracker.is_ok());
    }
    
    #[test]
    fn test_balance_formatting() {
        let balance = U256::from_dec_str("1000000000000000000").unwrap(); // 1 token with 18 decimals
        let decimals = 18;
        let divisor = U256::from(10).pow(U256::from(decimals));
        let formatted = balance.as_u128() as f64 / divisor.as_u128() as f64;
        assert_eq!(formatted, 1.0);
    }
}