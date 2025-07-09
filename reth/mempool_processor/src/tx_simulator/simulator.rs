/// Direct Transaction Simulator Integration
/// 
/// This module integrates reth_tx_simulator for ultra-fast transaction simulation
/// by bypassing RPC and using direct database access.

use reth_tx_simulator::{RethDirectTxSimulator, CallRequest, ipc_to_call_request, AddressStateChange};
use crate::mempool_fetcher::FullTransaction;
use eyre::Result;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, warn};
use alloy_primitives::Address;

/// Wrapper around RethDirectTxSimulator for mempool processor integration
pub struct TxSimulator {
    simulator: Arc<RethDirectTxSimulator>,
    latest_block: Arc<RwLock<u64>>,
}

impl TxSimulator {
    /// Create a new transaction simulator
    pub fn new(db_path: &str) -> Result<Self> {
        info!("Initializing transaction simulator...");
        let simulator = RethDirectTxSimulator::new(db_path)?;
        let latest_block = simulator.get_latest_block()?;
        
        info!("✅ Transaction simulator ready at block {}", latest_block);
        
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
    /// Automatically retries with correct nonce if simulation fails due to nonce mismatch
    pub async fn simulate_mempool_tx(&self, tx: &FullTransaction) -> Result<SimulationResult> {
        // Convert IPC transaction to CallRequest
        let mut call_request = ipc_to_call_request(&tx.tx_data)?;
        let latest_block = self.get_latest_block().await;
        
        // First try with original nonce
        let mut result = self.simulator
            .simulate_unsigned_transaction_at_block(call_request.clone(), latest_block)
            .await;
            
        // If nonce too high, try with expected nonce
        if let Err(ref e) = result {
            let error_msg = e.to_string();
            if error_msg.contains("nonce") && error_msg.contains("too high, expected") {
                if let Some(expected_nonce_str) = error_msg.split("expected ").nth(1) {
                    if let Ok(expected_nonce) = expected_nonce_str.trim().parse::<u64>() {
                        call_request.nonce = Some(expected_nonce);
                        result = self.simulator
                            .simulate_unsigned_transaction_at_block(call_request, latest_block)
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
    
    /// Simulate with state changes using call tracer
    /// Automatically retries with correct nonce if simulation fails due to nonce mismatch
    pub async fn simulate_with_state_changes(&self, tx: &FullTransaction) -> Result<StateChangeResult> {
        // Convert IPC transaction to CallRequest
        let mut call_request = ipc_to_call_request(&tx.tx_data)?;
        let latest_block = self.get_latest_block().await;
        
        // First try with original nonce
        let mut result = self.simulator
            .simulate_unsigned_transaction_with_call_trace_at_block(call_request.clone(), latest_block)
            .await;
            
        // If nonce too high, try with expected nonce
        if let Err(ref e) = result {
            let error_msg = e.to_string();
            if error_msg.contains("nonce") && error_msg.contains("too high, expected") {
                if let Some(expected_nonce_str) = error_msg.split("expected ").nth(1) {
                    if let Ok(expected_nonce) = expected_nonce_str.trim().parse::<u64>() {
                        call_request.nonce = Some(expected_nonce);
                        result = self.simulator
                            .simulate_unsigned_transaction_with_call_trace_at_block(call_request, latest_block)
                            .await;
                    }
                }
            }
        }
        
        let state_changes = result?;
            
        Ok(StateChangeResult {
            success: true, // TODO: get from actual simulation
            gas_used: 0, // TODO: get from actual simulation
            revert_reason: None,
            state_changes,
        })
    }
    
    /// Simulate with detailed state changes using call tracer (eth_net, token_net format)
    /// Automatically retries with correct nonce if simulation fails due to nonce mismatch
    pub async fn simulate_with_call_trace(&self, tx: &FullTransaction) -> Result<CallTraceResult> {
        // Convert IPC transaction to CallRequest
        let mut call_request = ipc_to_call_request(&tx.tx_data)?;
        let latest_block = self.get_latest_block().await;
        
        // First try with original nonce
        let mut result = self.simulator
            .simulate_unsigned_transaction_with_call_trace_at_block(call_request.clone(), latest_block)
            .await;
            
        // If nonce too high, try with expected nonce
        if let Err(ref e) = result {
            let error_msg = e.to_string();
            if error_msg.contains("nonce") && error_msg.contains("too high, expected") {
                if let Some(expected_nonce_str) = error_msg.split("expected ").nth(1) {
                    if let Ok(expected_nonce) = expected_nonce_str.trim().parse::<u64>() {
                        call_request.nonce = Some(expected_nonce);
                        result = self.simulator
                            .simulate_unsigned_transaction_with_call_trace_at_block(call_request, latest_block)
                            .await;
                    }
                }
            }
        }
        
        let detailed_changes = result?;
            
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
    pub success: bool,
    pub gas_used: u64,
    pub revert_reason: Option<String>,
    pub state_changes: HashMap<Address, AddressStateChange>,
}

/// Call trace result with detailed state changes
#[derive(Debug)]
pub struct CallTraceResult {
    pub detailed_changes: HashMap<Address, AddressStateChange>,
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