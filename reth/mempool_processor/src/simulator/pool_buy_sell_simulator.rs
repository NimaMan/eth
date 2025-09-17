use alloy_primitives::{Address, U256};
use eyre::Result;
/// Pool Buy/Sell Simulator - Wrapper around tx_processor's PoolBuySellSimulator
///
/// This module provides a wrapper around tx_processor's proven pool buy/sell simulation.
/// It uses the PoolBuySellSimulator from tx_processor which:
/// - Simulates buy, approve, and sell transactions
/// - Calculates tax percentages automatically
/// - Returns PoolViabilityResult with all trading information
///
/// The tax calculation is now built into the result, so we no longer need
/// separate tax_calculator modules.
use std::sync::Arc;

// Import from tx_processor
use tx_processor::tx_processor::TxProcessor;
use tx_processor::{check_can_buy_sell_pool, PoolType, PoolViabilityConfig, PoolViabilityResult};
use tx_simulator::TxSimulator;

// Re-export the result type for compatibility
pub type PoolSimulationResult = PoolViabilityResult;

/// Wrapper around tx_processor's pool buy/sell simulator
pub struct PoolBuySellSimulator {
    tx_simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
    default_buyer_address: Address,
    default_test_amount: U256,
}

impl PoolBuySellSimulator {
    /// Create a new pool buy/sell simulator
    pub fn new(reth_datadir: &str) -> Result<Self> {
        let tx_simulator = Arc::new(TxSimulator::new(reth_datadir)?);
        let tx_processor = Arc::new(TxProcessor::new());

        // Default configuration
        let default_buyer_address = Address::from([
            0x0C, 0x96, 0xc6, 0x02, 0xb1, 0xb3, 0x32, 0xB8, 0xAB, 0x20, 0x93, 0xE5, 0xd7, 0x2D,
            0x80, 0x4a, 0x24, 0xbd, 0x56, 0x89,
        ]);
        let default_test_amount = U256::from(1_000_000_000_000_000_000u128); // 1 ETH

        Ok(Self {
            tx_simulator,
            tx_processor,
            default_buyer_address,
            default_test_amount,
        })
    }

    /// Create with custom configuration
    pub fn with_config(
        reth_datadir: &str,
        buyer_address: Address,
        test_amount: U256,
    ) -> Result<Self> {
        let tx_simulator = Arc::new(TxSimulator::new(reth_datadir)?);
        let tx_processor = Arc::new(TxProcessor::new());

        Ok(Self {
            tx_simulator,
            tx_processor,
            default_buyer_address: buyer_address,
            default_test_amount: test_amount,
        })
    }

    /// Create with existing TxSimulator (for database sharing)
    pub fn with_tx_simulator(tx_simulator: Arc<TxSimulator>) -> Result<Self> {
        let tx_processor = Arc::new(TxProcessor::new());

        // Default configuration
        let default_buyer_address = Address::from([
            0x0C, 0x96, 0xc6, 0x02, 0xb1, 0xb3, 0x32, 0xB8, 0xAB, 0x20, 0x93, 0xE5, 0xd7, 0x2D,
            0x80, 0x4a, 0x24, 0xbd, 0x56, 0x89,
        ]);
        let default_test_amount = U256::from(1_000_000_000_000_000_000u128); // 1 ETH

        Ok(Self {
            tx_simulator,
            tx_processor,
            default_buyer_address,
            default_test_amount,
        })
    }

    /// Create with existing TxSimulator and custom configuration
    pub fn with_tx_simulator_and_config(
        tx_simulator: Arc<TxSimulator>,
        buyer_address: Address,
        test_amount: U256,
    ) -> Result<Self> {
        let tx_processor = Arc::new(TxProcessor::new());

        Ok(Self {
            tx_simulator,
            tx_processor,
            default_buyer_address: buyer_address,
            default_test_amount: test_amount,
        })
    }

    /// Simulate buy/sell for a specific pool
    pub async fn simulate_pool(
        &self,
        token_address: Address,
        pool_address: Address,
        pool_type: PoolType,
        block_number: Option<u64>,
    ) -> Result<PoolViabilityResult> {
        let config = PoolViabilityConfig {
            token_address,
            pool_address,
            pool_type,
            test_amount: self.default_test_amount,
            buyer_address: self.default_buyer_address,
            block_number,
            gas_limit: 500_000,
            gas_price: 30_000_000_000,
            prior_tx: None,
            block_delay: 0,
            slippage_tolerance: 0.5,
            weth_address: Address::from([
                0xC0, 0x2a, 0xaA, 0x39, 0xb2, 0x23, 0xFE, 0x8D, 0x0A, 0x0e, 0x5C, 0x4F, 0x27, 0xeA,
                0xD9, 0x08, 0x3C, 0x75, 0x6C, 0xc2,
            ]),
            token_decimals: 18,
        };

        check_can_buy_sell_pool(self.tx_simulator.clone(), self.tx_processor.clone(), config).await
    }

    /// Simulate buy/sell with custom configuration
    pub async fn simulate_with_config(
        &self,
        config: PoolViabilityConfig,
    ) -> Result<PoolViabilityResult> {
        check_can_buy_sell_pool(self.tx_simulator.clone(), self.tx_processor.clone(), config).await
    }

    /// Simulate buy/sell with custom configuration (alias for simulate_with_config)
    pub async fn simulate_pool_with_config(
        &self,
        config: PoolViabilityConfig,
    ) -> Result<PoolViabilityResult> {
        self.simulate_with_config(config).await
    }

    /// Get the default buyer address
    pub fn get_buyer_address(&self) -> Address {
        self.default_buyer_address
    }

    /// Get the default test amount
    pub fn get_test_amount(&self) -> U256 {
        self.default_test_amount
    }
}

/// Helper function to convert string pool type to enum
pub fn parse_pool_type(pool_type_str: &str) -> PoolType {
    match pool_type_str.to_lowercase().as_str() {
        "uniswapv2" | "v2" => PoolType::UniswapV2,
        "sushiswap" | "sushi" => PoolType::SushiSwap,
        _ => PoolType::UniswapV2, // Default to V2
    }
}
