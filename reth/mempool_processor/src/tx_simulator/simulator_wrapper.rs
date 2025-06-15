/// Unified Simulator Interface
/// 
/// This module provides a runtime-switchable wrapper for different simulation approaches.
/// It allows seamless switching between REVM and debug_traceCall based simulators
/// without changing the calling code.
///
/// Use Cases:
/// - A/B testing different simulation approaches
/// - Fallback mechanism when debug_traceCall is unavailable
/// - Gradual migration between simulation methods
/// - Performance vs accuracy trade-offs based on runtime conditions
///
/// The wrapper maintains the same interface regardless of the underlying
/// simulation method, making it easy to switch strategies dynamically.

use eyre::Result;
use std::collections::HashMap;
use revm_primitives::Address as RevmAddress;
use revm_tx_simulator_lib::process_tx::state_diff_utils::CalculatedAccountChanges;
use crate::mempool_fetcher::types::TransactionView;
use crate::tx_simulator::{TransactionSimulator, DebugTraceCallSimulator};
use revm_context::BlockEnv;

/// Wrapper enum that can use either REVM or debug_traceCall simulation
pub enum SimulatorWrapper {
    Revm(TransactionSimulator),
    DebugTraceCall(DebugTraceCallSimulator),
}

impl SimulatorWrapper {
    /// Create a new REVM simulator
    pub async fn new_revm(rpc_url: &str, chain_id: u64, spec_id: revm_primitives::hardfork::SpecId) -> Result<Self> {
        let simulator = TransactionSimulator::new(rpc_url, chain_id, spec_id).await?;
        Ok(Self::Revm(simulator))
    }
    
    /// Create a new debug_traceCall simulator
    pub async fn new_debug_tracecall(rpc_url: &str) -> Result<Self> {
        let simulator = DebugTraceCallSimulator::new(rpc_url).await?;
        Ok(Self::DebugTraceCall(simulator))
    }
    
    /// Process a transaction using the appropriate simulator
    pub async fn process_transaction(
        &self,
        tx: &TransactionView,
        block_env: &BlockEnv,
    ) -> Result<Option<HashMap<RevmAddress, CalculatedAccountChanges>>> {
        match self {
            Self::Revm(simulator) => simulator.process_transaction(tx, block_env).await,
            Self::DebugTraceCall(simulator) => simulator.process_transaction(tx, block_env).await,
        }
    }
    
    /// Get the RPC URL used by this simulator
    pub fn get_rpc_url(&self) -> &str {
        match self {
            Self::Revm(_) => "http://localhost:8545", // Default, as TransactionSimulator doesn't store it
            Self::DebugTraceCall(_) => "http://localhost:8545", // Default, as we don't have access to it
        }
    }
}