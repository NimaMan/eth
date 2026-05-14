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
use eyre::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

use super::pool_buy_sell_simulator::{PoolBuySellSimulator, PoolSimulationResult};
use crate::common::convert::ipc_to_unsigned_tx;
use crate::mempool_fetcher::MempoolTransaction;
use tx_processor::PoolBuySellParameters;
use tx_simulator::{
    LiveStateStatus, LiveTxSimulator, SimulationResult as TxSimResult, TxSimulator,
    UnsignedTransaction,
};

/// Mempool simulator that manages both mempool transaction and pool buy/sell simulations
pub struct MempoolSimulator {
    /// The underlying TxSimulator for direct simulation
    tx_simulator: Arc<TxSimulator>,
    /// Live-first selector for mempool simulations.
    live_tx_simulator: LiveTxSimulator,
    /// The pool buy/sell simulator
    pool_simulator: PoolBuySellSimulator,
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
    pub fn new(datadir: &str) -> Result<Self> {
        info!("Initializing mempool simulator...");

        let tx_simulator = Arc::new(TxSimulator::new(datadir)?);
        let live_tx_simulator = LiveTxSimulator::from_simulator(tx_simulator.clone());

        // Create pool simulator with the same TxSimulator
        let pool_simulator = PoolBuySellSimulator::with_tx_simulator(tx_simulator.clone())?;

        info!("✅ Mempool simulator initialized");

        Ok(Self {
            tx_simulator,
            live_tx_simulator,
            pool_simulator,
        })
    }

    /// Create a new mempool simulator from a shared TxSimulator instance
    /// This allows external components (like tx_processor) to share the same
    /// database connection and avoid writer lock conflicts.
    pub fn from_shared_simulator(tx_simulator: Arc<TxSimulator>) -> Result<Self> {
        info!("Initializing mempool simulator from shared TxSimulator...");

        // Reuse the provided TxSimulator for both single and pool simulations
        let live_tx_simulator = LiveTxSimulator::from_simulator(tx_simulator.clone());
        let pool_simulator = PoolBuySellSimulator::with_tx_simulator(tx_simulator.clone())?;

        info!("✅ Mempool simulator initialized with shared TxSimulator");

        Ok(Self {
            tx_simulator,
            live_tx_simulator,
            pool_simulator,
        })
    }

    /// Create with custom buyer address
    pub fn with_custom_buyer(datadir: &str, buyer_address: Address) -> Result<Self> {
        info!("Initializing mempool simulator with custom buyer...");

        let tx_simulator = Arc::new(TxSimulator::new(datadir)?);
        let live_tx_simulator = LiveTxSimulator::from_simulator(tx_simulator.clone());

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
            live_tx_simulator,
            pool_simulator,
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
        let block_number = self.resolve_simulation_block_number().await?;
        self.simulate_with_nonce_retry(unsigned_tx, block_number)
            .await
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

    /// Private helper: Simulate with nonce retry logic
    async fn simulate_with_nonce_retry(
        &self,
        mut unsigned_tx: UnsignedTransaction,
        block_number: u64,
    ) -> Result<SimulationResult> {
        // First try with original nonce
        let mut result = self
            .simulate_single_attempt(unsigned_tx.clone(), block_number)
            .await;

        // If nonce too high, try with expected nonce
        if let Err(ref e) = result {
            let error_msg = e.to_string();
            if error_msg.contains("nonce") && error_msg.contains("too high, expected") {
                if let Some(expected_nonce_str) = error_msg.split("expected ").nth(1) {
                    if let Ok(expected_nonce) = expected_nonce_str.trim().parse::<u64>() {
                        unsigned_tx.nonce = Some(expected_nonce);
                        result = self
                            .simulate_single_attempt(unsigned_tx, block_number)
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
        self.pool_simulator.simulate_pool_with_config(config).await
    }

    /// Get the buyer address used for simulations
    pub fn get_buyer_address(&self) -> Address {
        self.pool_simulator.get_buyer_address()
    }

    /// Get access to the TxSimulator
    pub fn get_tx_simulator(&self) -> Arc<TxSimulator> {
        Arc::clone(&self.tx_simulator)
    }

    async fn simulate_single_attempt(
        &self,
        unsigned_tx: UnsignedTransaction,
        block_number: u64,
    ) -> Result<TxSimResult> {
        let mut session = self
            .live_tx_simulator
            .start_session_at(block_number)
            .await?;
        session.step_unsigned(unsigned_tx)
    }

    async fn resolve_simulation_block_number(&self) -> Result<u64> {
        self.live_tx_simulator.latest_state_block_number().await
    }

    /// Expose the current block number the simulator will target.
    pub async fn latest_simulation_block(&self) -> Result<u64> {
        self.resolve_simulation_block_number().await
    }

    /// Expose full state-selection diagnostics for live simulation.
    pub async fn latest_simulation_status(&self) -> Result<LiveStateStatus> {
        self.live_tx_simulator.latest_state_status().await
    }
}

/// Convert mempool transaction to UnsignedTransaction (convenience function)
pub fn mempool_tx_to_unsigned_tx(tx: &MempoolTransaction) -> Result<UnsignedTransaction> {
    ipc_to_unsigned_tx(&tx.data)
}
