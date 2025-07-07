/// Direct Transaction Simulator Integration
/// 
/// This module integrates reth_tx_simulator for ultra-fast transaction simulation
/// by bypassing RPC and using direct database access.

use reth_tx_simulator::{RethDirectTxSimulator, CallRequest, ipc_to_call_request, DebugAddressStateChange};
use crate::mempool_fetcher::FullTransaction;
use eyre::Result;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, warn};
use alloy_primitives::Address;

/// Wrapper around RethDirectTxSimulator for mempool processor integration
pub struct DirectTxSimulator {
    simulator: Arc<RethDirectTxSimulator>,
    latest_block: Arc<RwLock<u64>>,
}

impl DirectTxSimulator {
    /// Create a new direct simulator
    pub fn new(db_path: &str) -> Result<Self> {
        info!("Initializing direct transaction simulator...");
        let simulator = RethDirectTxSimulator::new(db_path)?;
        let latest_block = simulator.get_latest_block()?;
        
        info!("✅ Direct simulator ready at block {}", latest_block);
        
        Ok(Self {
            simulator: Arc::new(simulator),
            latest_block: Arc::new(RwLock::new(latest_block)),
        })
    }
    
    /// Update the latest block number
    pub async fn update_latest_block(&self) -> Result<u64> {
        let new_block = self.simulator.get_latest_block()?;
        let mut latest = self.latest_block.write().await;
        *latest = new_block;
        Ok(new_block)
    }
    
    /// Get current latest block
    pub async fn get_latest_block(&self) -> u64 {
        *self.latest_block.read().await
    }
    
    /// Simulate a transaction from mempool
    pub async fn simulate_mempool_tx(&self, tx: &FullTransaction) -> Result<SimulationResult> {
        // Convert IPC transaction to CallRequest
        let call_request = ipc_to_call_request(&tx.tx_data)?;
        let latest_block = self.get_latest_block().await;
        
        // Run simulation
        let result = self.simulator
            .simulate_unsigned_transaction_at_block(call_request, latest_block)
            .await?;
            
        Ok(SimulationResult {
            success: result.success,
            gas_used: result.gas_used,
            revert_reason: result.revert_reason,
            simulation_time_us: 0, // Will be measured externally
        })
    }
    
    /// Simulate with state changes using prestate tracer (raw format)
    pub async fn simulate_with_state_changes(&self, tx: &FullTransaction) -> Result<StateChangeResult> {
        // Convert IPC transaction to CallRequest
        let call_request = ipc_to_call_request(&tx.tx_data)?;
        let latest_block = self.get_latest_block().await;
        
        // Get state changes
        let state_changes = self.simulator
            .simulate_unsigned_transaction_with_state_changes_at_block(call_request, latest_block)
            .await?;
            
        // Extract structured state changes
        let parsed_changes = self.simulator
            .get_transaction_state_changes_from_value(&state_changes, latest_block, 0)?;
            
        Ok(StateChangeResult {
            raw_changes: state_changes,
            parsed_changes,
        })
    }
    
    /// Simulate with detailed state changes using call tracer (eth_net, token_net format)
    pub async fn simulate_with_call_trace(&self, tx: &FullTransaction) -> Result<CallTraceResult> {
        // Convert IPC transaction to CallRequest
        let call_request = ipc_to_call_request(&tx.tx_data)?;
        let latest_block = self.get_latest_block().await;
        
        // Get detailed state changes with call tracer
        let detailed_changes = self.simulator
            .simulate_unsigned_transaction_with_call_trace_at_block(call_request, latest_block)
            .await?;
            
        Ok(CallTraceResult {
            detailed_changes,
            block_number: latest_block,
        })
    }
    
    /// Batch simulate multiple transactions
    pub async fn batch_simulate(&self, transactions: &[FullTransaction]) -> Vec<Result<SimulationResult>> {
        let latest_block = self.get_latest_block().await;
        let mut results = Vec::with_capacity(transactions.len());
        
        for tx in transactions {
            match ipc_to_call_request(&tx.tx_data) {
                Ok(call_request) => {
                    let result = self.simulator
                        .simulate_unsigned_transaction_at_block(call_request, latest_block)
                        .await
                        .map(|r| SimulationResult {
                            success: r.success,
                            gas_used: r.gas_used,
                            revert_reason: r.revert_reason,
                            simulation_time_us: 0,
                        });
                    results.push(result);
                }
                Err(e) => {
                    results.push(Err(e));
                }
            }
        }
        
        results
    }
}

/// Simulation result
#[derive(Debug, Clone)]
pub struct SimulationResult {
    pub success: bool,
    pub gas_used: u64,
    pub revert_reason: Option<String>,
    pub simulation_time_us: u64,
}

/// State change result
#[derive(Debug)]
pub struct StateChangeResult {
    pub raw_changes: serde_json::Value,
    pub parsed_changes: reth_tx_simulator::TransactionStateChanges,
}

/// Call trace result with detailed state changes
#[derive(Debug)]
pub struct CallTraceResult {
    pub detailed_changes: HashMap<Address, DebugAddressStateChange>,
    pub block_number: u64,
}

/// Convert mempool transaction to CallRequest (convenience function)
pub fn mempool_tx_to_call_request(tx: &FullTransaction) -> Result<CallRequest> {
    ipc_to_call_request(&tx.tx_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_direct_simulator_creation() {
        let result = DirectTxSimulator::new("/home/nima/.local/share/reth/mainnet");
        assert!(result.is_ok());
    }
}