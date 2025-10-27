use alloy_primitives::{Address, U256};
/// Mempool Simulator that manages both single and pool buy/sell transaction simulations
///
/// This module solves the database lock conflict (error code 11) by creating a single
/// provider factory and sharing it between both simulators.
///
/// The mempool simulator provides a simple interface to:
/// - Simulate single transactions
/// - Simulate buy/sell sequences
/// - All using the same database connection
use eyre::{eyre, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task::spawn_blocking;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info};

use super::pool_buy_sell_simulator::{PoolBuySellSimulator, PoolSimulationResult};
use crate::canonical_head_cache::{CanonicalHeadCache, HeadSnapshot};
use crate::common::convert::ipc_to_unsigned_tx;
use crate::mempool_fetcher::MempoolTransaction;
use reth_primitives::SealedHeader;
use reth_provider::StateProviderBox;
use tx_processor::{PoolBuySellParameters, PoolType};
use tx_simulator::{SimulationResult as TxSimResult, TxSimulator, UnsignedTransaction};

/// Mempool simulator that manages both mempool transaction and pool buy/sell simulations
pub struct MempoolSimulator {
    /// The underlying TxSimulator for direct simulation
    tx_simulator: Arc<TxSimulator>,
    /// The pool buy/sell simulator
    pool_simulator: PoolBuySellSimulator,
    /// Shared canonical head cache populated by the live subscription.
    head_cache: Arc<CanonicalHeadCache>,
}

/// Simulation result for mempool transactions
#[derive(Debug, Clone)]
pub struct SimulationResult {
    pub success: bool,
    pub gas_used: u64,
    pub revert_reason: Option<String>,
    pub simulation_time_us: u64,
}

/// State change result (placeholder - use tx_processor for actual state changes)
#[derive(Debug)]
pub struct StateChangeResult {
    pub success: bool,
    pub gas_used: u64,
    pub revert_reason: Option<String>,
    pub state_changes: HashMap<Address, AddressStateChange>,
}

/// Placeholder for AddressStateChange - actual implementation in tx_processor
#[derive(Debug, Clone)]
pub struct AddressStateChange {
    pub eth_net: f64,
    pub token_net: HashMap<Address, f64>,
}

impl MempoolSimulator {
    /// Create a new mempool simulator
    pub fn new(datadir: &str, head_cache: Arc<CanonicalHeadCache>) -> Result<Self> {
        info!("Initializing mempool simulator...");

        // Create the TxSimulator directly
        let tx_simulator = Arc::new(TxSimulator::new(datadir)?);

        // Create pool simulator with the same TxSimulator
        let pool_simulator = PoolBuySellSimulator::with_tx_simulator(tx_simulator.clone())?;

        info!("✅ Mempool simulator initialized");

        Ok(Self {
            tx_simulator,
            pool_simulator,
            head_cache,
        })
    }

    /// Create a new mempool simulator from a shared TxSimulator instance
    /// This allows external components (like tx_processor) to share the same
    /// database connection and avoid writer lock conflicts.
    pub fn from_shared_simulator(
        tx_simulator: Arc<TxSimulator>,
        head_cache: Arc<CanonicalHeadCache>,
    ) -> Result<Self> {
        info!("Initializing mempool simulator from shared TxSimulator...");

        // Reuse the provided TxSimulator for both single and pool simulations
        let pool_simulator = PoolBuySellSimulator::with_tx_simulator(tx_simulator.clone())?;

        info!("✅ Mempool simulator initialized with shared TxSimulator");

        Ok(Self {
            tx_simulator,
            pool_simulator,
            head_cache,
        })
    }

    /// Create with custom buyer address
    pub fn with_custom_buyer(
        datadir: &str,
        buyer_address: Address,
        head_cache: Arc<CanonicalHeadCache>,
    ) -> Result<Self> {
        info!("Initializing mempool simulator with custom buyer...");

        // Create the TxSimulator directly
        let tx_simulator = Arc::new(TxSimulator::new(datadir)?);

        // Create pool simulator with custom buyer and default amount
        let default_amount = U256::from(1_000_000_000_000_000_000u128); // 1 ETH
        let pool_simulator = PoolBuySellSimulator::with_tx_simulator_and_config(
            tx_simulator.clone(),
            buyer_address,
            default_amount,
        )?;

        info!("✅ Mempool simulator initialized with custom buyer");

        Ok(Self {
            tx_simulator,
            pool_simulator,
            head_cache,
        })
    }

    /// Get the latest block number
    pub fn get_latest_block(&self) -> Result<u64> {
        self.tx_simulator.get_latest_block()
    }

    // ========== Mempool Transaction Simulation Methods ==========

    /// Simulate a mempool transaction
    pub async fn simulate_mempool_tx(&self, tx: &MempoolTransaction) -> Result<SimulationResult> {
        let unsigned_tx = ipc_to_unsigned_tx(&tx.data)?;
        if let Some(snapshot) = self.head_cache.latest_snapshot().await {
            self.simulate_with_snapshot(unsigned_tx, snapshot).await
        } else {
            let header = self.head_cache.latest_header().await.ok_or_else(|| {
                error!("No canonical header from subscription; skipping simulation");
                eyre!("Canonical head unavailable from subscription")
            })?;

            self.simulate_with_nonce_retry(unsigned_tx, header).await
        }
    }

    /// Simulate mempool transaction and get state changes (placeholder)
    pub async fn simulate_mempool_tx_with_state_changes(
        &self,
        tx: &MempoolTransaction,
    ) -> Result<StateChangeResult> {
        // For now, just do a basic simulation
        // TODO: Use tx_processor for actual state changes
        let sim_result = self.simulate_mempool_tx(tx).await?;

        Ok(StateChangeResult {
            success: sim_result.success,
            gas_used: sim_result.gas_used,
            revert_reason: sim_result.revert_reason,
            state_changes: HashMap::new(), // Placeholder - use tx_processor for actual state changes
        })
    }

    async fn simulate_with_snapshot(
        &self,
        unsigned_tx: UnsignedTransaction,
        snapshot: HeadSnapshot,
    ) -> Result<SimulationResult> {
        self.simulate_with_nonce_retry(unsigned_tx, snapshot.header.clone())
            .await
    }

    /// Private helper: Simulate with nonce retry logic
    async fn simulate_with_nonce_retry(
        &self,
        mut unsigned_tx: UnsignedTransaction,
        header: SealedHeader,
    ) -> Result<SimulationResult> {
        let block_number = header.number;

        // First try with original nonce
        let mut result = self
            .simulate_single_attempt(unsigned_tx.clone(), header.clone(), block_number)
            .await;

        // If nonce too high, try with expected nonce
        if let Err(ref e) = result {
            let error_msg = e.to_string();
            if error_msg.contains("nonce") && error_msg.contains("too high, expected") {
                if let Some(expected_nonce_str) = error_msg.split("expected ").nth(1) {
                    if let Ok(expected_nonce) = expected_nonce_str.trim().parse::<u64>() {
                        unsigned_tx.nonce = Some(expected_nonce);
                        result = self
                            .simulate_single_attempt(unsigned_tx, header.clone(), block_number)
                            .await;
                    }
                }
            }
        }

        let sim_result = result?;

        Ok(SimulationResult {
            success: sim_result.success,
            gas_used: sim_result.gas_used,
            revert_reason: sim_result.revert_reason,
            simulation_time_us: 0, // Will be measured externally
        })
    }

    // ========== Pool Buy/Sell Simulation Methods ==========

    /// Simulate buy/sell for a specific pool using configuration
    pub async fn simulate_pool_buy_sell(
        &self,
        config: PoolBuySellParameters,
    ) -> Result<PoolSimulationResult> {
        let mut config = config;

        if config.block_header.is_none() {
            if let Some(snapshot) = self.head_cache.latest_snapshot().await {
                let target_block = config.block_number.unwrap_or(snapshot.number);
                if snapshot.number == target_block {
                    config.block_number = Some(target_block);
                    config.block_header = Some(snapshot.header.clone());
                }
            }
        }

        self.pool_simulator.simulate_pool_with_config(config).await
    }

    /// Simulate buy/sell for a specific pool with simple parameters
    pub async fn simulate_pool_buy_sell_simple(
        &self,
        token_address: Address,
        pool_address: Address,
        token_decimals: u8,
        block_number: Option<u64>,
    ) -> Result<PoolSimulationResult> {
        let snapshot = self.head_cache.latest_snapshot().await;
        let (resolved_block, block_header) = match (block_number, snapshot) {
            (Some(number), Some(snapshot)) if snapshot.number == number => {
                (Some(number), Some(snapshot.header.clone()))
            }
            (Some(number), _) => (Some(number), None),
            (None, Some(snapshot)) => (Some(snapshot.number), Some(snapshot.header.clone())),
            (None, None) => {
                error!("No canonical snapshot from subscription; cannot simulate pool");
                return Err(eyre!("Canonical snapshot unavailable from subscription"));
            }
        };

        let config = PoolBuySellParameters {
            token_address,
            pool_address,
            pool_type: PoolType::UniswapV2,
            test_amount: self.pool_simulator.get_test_amount(),
            buyer_address: self.pool_simulator.get_buyer_address(),
            block_number: resolved_block,
            gas_price: None,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
            buy_gas_limit: 500_000,
            approve_gas_limit: 200_000,
            sell_gas_limit: 500_000,
            prior_txs: Vec::new(),
            block_delay: 0,
            slippage_tolerance: 5.0,
            weth_address: Address::from([
                0xC0, 0x2a, 0xaA, 0x39, 0xb2, 0x23, 0xFE, 0x8D, 0x0A, 0x0e, 0x5C, 0x4F, 0x27, 0xeA,
                0xD9, 0x08, 0x3C, 0x75, 0x6C, 0xc2,
            ]),
            token_decimals,
            block_header,
            uniswap_v4_config: None,
        };

        self.simulate_pool_buy_sell(config).await
    }

    /// Get the buyer address used for simulations
    pub fn get_buyer_address(&self) -> Address {
        self.pool_simulator.get_buyer_address()
    }

    /// Get access to the TxSimulator
    pub fn get_tx_simulator(&self) -> Arc<TxSimulator> {
        Arc::clone(&self.tx_simulator)
    }

    /// Get access to the canonical head cache
    pub fn head_cache(&self) -> Arc<CanonicalHeadCache> {
        self.head_cache.clone()
    }

    async fn simulate_single_attempt(
        &self,
        unsigned_tx: UnsignedTransaction,
        header: SealedHeader,
        block_number: u64,
    ) -> Result<TxSimResult> {
        let state = self.fetch_state_with_retry(block_number).await?;

        self.tx_simulator
            .simulate_unsigned_transaction_on_state(unsigned_tx, header, state)
            .await
    }

    async fn fetch_state_with_retry(&self, block_number: u64) -> Result<StateProviderBox> {
        const MAX_ATTEMPTS: usize = 12;
        const RETRY_DELAY: Duration = Duration::from_millis(100);

        for attempt in 1..=MAX_ATTEMPTS {
            match spawn_blocking({
                let simulator = self.tx_simulator.clone();
                move || {
                    simulator
                        .provider_factory()
                        .history_by_block_number(block_number)
                }
            })
            .await
            {
                Ok(Ok(state)) => {
                    if attempt > 1 {
                        debug!(
                            "Fetched state provider for block {} after {} attempts",
                            block_number, attempt
                        );
                    }
                    return Ok(state);
                }
                Ok(Err(err)) => {
                    if attempt == MAX_ATTEMPTS {
                        return Err(eyre!(
                            "Failed to fetch state for block {}: {}",
                            block_number,
                            err
                        ));
                    }
                }
                Err(join_err) => {
                    if attempt == MAX_ATTEMPTS {
                        return Err(eyre!(
                            "State fetch task panicked for block {}: {}",
                            block_number,
                            join_err
                        ));
                    }
                }
            }

            sleep(RETRY_DELAY).await;
        }

        Err(eyre!(
            "Failed to obtain state provider for block {}",
            block_number
        ))
    }
}

/// Convert mempool transaction to UnsignedTransaction (convenience function)
pub fn mempool_tx_to_unsigned_tx(tx: &MempoolTransaction) -> Result<UnsignedTransaction> {
    ipc_to_unsigned_tx(&tx.data)
}
