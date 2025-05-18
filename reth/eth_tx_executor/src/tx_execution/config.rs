/*
* Transaction Execution Configuration
*
* Configuration for transaction execution.
*/

use ethers::types::U256;
use serde::{Serialize, Deserialize};
use std::time::Duration;

/// Transaction execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxExecutionConfig {
    /// Chain ID
    pub chain_id: u64,
    
    /// Base gas price for legacy transactions
    pub base_gas_price: U256,
    
    /// Base fee for EIP-1559 transactions
    pub base_fee: U256,
    
    /// Priority fee for EIP-1559 transactions
    pub priority_fee: U256,
    
    /// Gas price boost percentage for urgent transactions
    pub urgent_gas_boost_percent: u64,
    
    /// Default gas limit
    pub default_gas_limit: U256,
    
    /// Default confirmation blocks
    pub default_confirmation_blocks: u64,
    
    /// Maximum confirmation blocks
    pub max_confirmation_blocks: u64,
    
    /// Maximum number of retries for transaction submission
    pub max_retries: u32,
    
    /// Retry interval
    pub retry_interval: Duration,
    
    /// Transaction timeout
    pub transaction_timeout: Duration,
    
    /// Gas estimation buffer percentage
    pub gas_buffer_percent: u64,
    
    /// RPC endpoints
    pub rpc_endpoints: Vec<String>,
    
    /// Whether to use Flashbots
    pub use_flashbots: bool,
    
    /// Flashbots RPC endpoint
    pub flashbots_endpoint: Option<String>,
}

impl Default for TxExecutionConfig {
    fn default() -> Self {
        Self {
            chain_id: 1, // Ethereum Mainnet
            base_gas_price: U256::from(20_000_000_000u64), // 20 Gwei
            base_fee: U256::from(20_000_000_000u64), // 20 Gwei
            priority_fee: U256::from(1_500_000_000u64), // 1.5 Gwei
            urgent_gas_boost_percent: 50, // 50% boost for urgent transactions
            default_gas_limit: U256::from(200_000), // 200k gas units
            default_confirmation_blocks: 3,
            max_confirmation_blocks: 12,
            max_retries: 5,
            retry_interval: Duration::from_secs(5),
            transaction_timeout: Duration::from_secs(180), // 3 minutes
            gas_buffer_percent: 20, // 20% buffer on gas estimation
            rpc_endpoints: vec![
                "https://eth-mainnet.alchemyapi.io/v2/your-api-key".to_string(),
                "https://mainnet.infura.io/v3/your-api-key".to_string(),
            ],
            use_flashbots: false,
            flashbots_endpoint: Some("https://relay.flashbots.net".to_string()),
        }
    }
}
