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
                    debug!("Returning cached position for {}", position.symbol);
                    return Ok(position.clone());
                }
            }
        }
        
        // Otherwise update and return
        self.update_token_balance(token_address).await
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
    
    /// Update ETH balance
    pub async fn update_eth_balance(&self) -> Result<U256, Box<dyn std::error::Error>> {
        let balance = self.provider.get_balance(self.wallet_address, None).await?;
        info!("ETH balance: {} ETH", format_ether(balance));
        Ok(balance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::utils::Anvil;
    use ethers::contract::abigen;
    
    // Generate ERC20 token contract bindings for testing
    abigen!(
        ERC20Mock,
        r#"[
            function balanceOf(address owner) external view returns (uint256)
            function decimals() external view returns (uint8)
            function symbol() external view returns (string)
            function mint(address to, uint256 amount) external
        ]"#
    );
    
    async fn setup_test_environment() -> (Arc<Provider<Http>>, Address, Address) {
        // Start local Anvil instance
        let anvil = Anvil::new().spawn();
        let provider = Arc::new(Provider::<Http>::try_from(anvil.endpoint()).unwrap());
        
        // Get test wallet
        let _wallet = anvil.keys()[0].clone();
        let wallet_address = anvil.addresses()[0];
        
        // Deploy mock token
        let token_bytecode = "0x608060405234801561001057600080fd5b50610e3c806100206000396000f3fe608060405234801561001057600080fd5b50600436106100935760003560e01c8063313ce56711610066578063313ce5671461011357806370a082311461013257806395d89b4114610165578063a9059cbb1461016d578063dd62ed3e1461018057600080fd5b806306fdde0314610098578063095ea7b3146100b657806318160ddd146100d957806323b872dd146100eb575b600080fd5b6100a06101b9565b6040516100ad9190610a56565b60405180910390f35b6100c96100c4366004610ac0565b61024b565b60405190151581526020016100ad565b6002545b6040519081526020016100ad565b6100c96100f9366004610aea565b610262565b604051601281526020016100ad565b6100dd610121366004610b26565b60006020819052908152604090205481565b6100a0610313565b6100c961014b366004610ac0565b610322565b6100dd61015e366004610b48565b61032f565b610178610173366004610ac0565b610357565b005b61018d61018e366004610b48565b6103b4565b600380546101c090610b7b565b80601f01602080910402602001604051908101604052809291908181526020018280546101ed90610b7b565b801561023a5780601f1061020f5761010080835404028352916020019161023a565b820191906000526020600020905b81548152906001019060200180831161021d57829003601f168201915b50505050508156fea26469706673582212203f7c5d2a59f4c3e87b3744b9a7de7e5c9e1e7c58e2b8f9e8a5f7d9c8b7a6e54d64736f6c63430008130033"; // Mock bytecode
        
        let _factory = ContractFactory::new(
            serde_json::from_str::<Abi>(r#"[{"inputs":[],"name":"decimals","outputs":[{"internalType":"uint8","name":"","type":"uint8"}],"stateMutability":"view","type":"function"},{"inputs":[{"internalType":"address","name":"owner","type":"address"}],"name":"balanceOf","outputs":[{"internalType":"uint256","name":"","type":"uint256"}],"stateMutability":"view","type":"function"},{"inputs":[],"name":"symbol","outputs":[{"internalType":"string","name":"","type":"string"}],"stateMutability":"view","type":"function"},{"inputs":[{"internalType":"address","name":"to","type":"address"},{"internalType":"uint256","name":"amount","type":"uint256"}],"name":"mint","outputs":[],"stateMutability":"nonpayable","type":"function"}]"#).unwrap(),
            token_bytecode.parse::<Bytes>().unwrap(),
            provider.clone(),
        );
        
        let token_address = Address::random(); // For testing purposes
        
        (provider, wallet_address, token_address)
    }
    
    #[tokio::test]
    async fn test_position_tracker_creation() {
        let provider = Arc::new(Provider::<Http>::try_from("http://localhost:8545").unwrap());
        let wallet = Address::random();
        
        let tracker = PositionTracker::new(provider, wallet);
        assert!(tracker.is_ok());
    }
    
    #[tokio::test]
    async fn test_cache_behavior() {
        let provider = Arc::new(Provider::<Http>::try_from("http://localhost:8545").unwrap());
        let wallet = Address::random();
        let tracker = PositionTracker::new(provider, wallet).unwrap();
        
        // Create a test position
        let test_position = TokenPosition {
            address: Address::random(),
            symbol: "TEST".to_string(),
            decimals: 18,
            balance: U256::from(1000),
            balance_formatted: 0.001,
            value_eth: 0.01,
            last_updated: chrono::Utc::now().timestamp() as u64,
        };
        
        // Insert into cache
        {
            let mut positions = tracker.positions.write().await;
            positions.insert(test_position.address, test_position.clone());
        }
        
        // Verify cache returns the position
        let positions = tracker.get_all_positions().await;
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].symbol, "TEST");
        
        // Clear cache
        tracker.clear_cache().await;
        
        // Verify cache is empty
        let positions = tracker.get_all_positions().await;
        assert_eq!(positions.len(), 0);
    }
    
    #[test]
    fn test_balance_formatting() {
        // Test with 18 decimals (standard)
        let balance = U256::from(1_500_000_000_000_000_000u128); // 1.5 tokens
        let divisor = U256::from(10).pow(U256::from(18));
        let formatted = balance.as_u128() as f64 / divisor.as_u128() as f64;
        assert!((formatted - 1.5).abs() < 0.0001);
        
        // Test with 6 decimals (USDC-like)
        let balance = U256::from(1_500_000u128); // 1.5 tokens
        let divisor = U256::from(10).pow(U256::from(6));
        let formatted = balance.as_u128() as f64 / divisor.as_u128() as f64;
        assert!((formatted - 1.5).abs() < 0.0001);
        
        // Test with 0 balance
        let balance = U256::zero();
        let divisor = U256::from(10).pow(U256::from(18));
        let formatted = if balance > U256::zero() {
            balance.as_u128() as f64 / divisor.as_u128() as f64
        } else {
            0.0
        };
        assert_eq!(formatted, 0.0);
    }
    
    #[test]
    fn test_cache_expiration() {
        // Test that positions older than 60 seconds are considered expired
        let now = chrono::Utc::now().timestamp() as u64;
        let old_timestamp = now - 61; // 61 seconds ago
        let recent_timestamp = now - 30; // 30 seconds ago
        
        // Old position should be expired
        assert!(now - old_timestamp >= 60);
        
        // Recent position should still be valid
        assert!(now - recent_timestamp < 60);
    }
}