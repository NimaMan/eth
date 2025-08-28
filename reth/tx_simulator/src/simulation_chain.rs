/// Stateful Simulation Chain
/// 
/// This module provides the SimulationChain struct which maintains blockchain state
/// between individual transaction simulations. This is essential for workflows where
/// each transaction depends on the results of previous ones (e.g., buy → approve → sell).
/// 
/// The chain maintains a forked state that persists between `step()` calls, allowing
/// complex multi-transaction workflows to be simulated accurately.

use crate::{
    simulator::TxSimulator,
    types::{SimulationResult, SequentialTransactionResult, FullSimulationResult},
    call_simulator::CallRequest,
    batch_sequence_simulation::ForkedState,
};
use std::sync::Arc;
use std::collections::HashMap;
use alloy_primitives::Address;
use eyre::Result;

/// Information about the current chain state
#[derive(Debug, Clone)]
pub struct ChainStateInfo {
    /// Current block number
    pub block_number: u64,
    /// Number of transactions executed
    pub transaction_count: usize,
    /// Total gas used across all transactions
    pub total_gas_used: u64,
    /// Current nonces for addresses that have sent transactions
    pub nonces: HashMap<Address, u64>,
}

/// A stateful simulation chain that maintains blockchain state between transactions
pub struct SimulationChain {
    /// Reference to the underlying simulator
    simulator: Arc<TxSimulator>,
    /// The forked blockchain state
    forked_state: ForkedState,
    /// Results from all executed transactions
    results: Vec<SimulationResult>,
    /// Total gas used
    total_gas_used: u64,
    /// Initial block number
    initial_block: u64,
}

impl SimulationChain {
    /// Create a new simulation chain
    pub(crate) fn new(
        simulator: Arc<TxSimulator>,
        forked_state: ForkedState,
        block_number: u64,
    ) -> Self {
        Self {
            simulator,
            forked_state,
            results: Vec::new(),
            total_gas_used: 0,
            initial_block: block_number,
        }
    }
    
    /// Execute a single transaction and advance the chain state
    /// 
    /// This simulates the transaction on the current state and commits the changes,
    /// making them visible to subsequent transactions.
    /// 
    /// # Example
    /// ```rust
    /// let mut chain = simulator.start_simulation_chain(None).await?;
    /// let result = chain.step(buy_transaction).await?;
    /// // State now includes the effects of buy_transaction
    /// ```
    pub async fn step(&mut self, call: CallRequest) -> Result<SimulationResult> {
        // Prepare the call with automatic nonce management
        let mut call = call;
        if let Some(from) = call.from {
            if call.nonce.is_none() {
                // Get nonce from our tracked state, or query from database if not tracked yet
                let nonce = if let Some(&tracked_nonce) = self.forked_state.nonces.get(&from) {
                    tracked_nonce
                } else {
                    // Query the actual nonce from the forked state database
                    let db_nonce = self.simulator.get_nonce_from_state(&mut self.forked_state, from)?;
                    // Store it in our tracking
                    self.forked_state.nonces.insert(from, db_nonce);
                    db_nonce
                };
                call.nonce = Some(nonce);
            }
        }
        
        // Use the existing simulate_on_fork method from TxSimulator
        let result = self.simulator.simulate_on_fork(&mut self.forked_state, call.clone())?;
        
        // Convert SequentialTransactionResult to SimulationResult
        let sim_result = SimulationResult {
            success: result.success,
            gas_used: result.gas_used,
            revert_reason: result.revert_reason,
        };
        
        // Update tracking
        if result.success {
            self.total_gas_used += result.gas_used;
            
            // Update nonce tracking
            if let Some(from) = call.from {
                let current = self.forked_state.nonces.entry(from).or_insert(0);
                *current += 1;
            }
        }
        
        self.results.push(sim_result.clone());
        Ok(sim_result)
    }
    
    /// Execute multiple transactions sequentially
    /// 
    /// Each transaction is executed on the state resulting from the previous one.
    /// If any transaction fails, execution continues but the failure is recorded.
    pub async fn step_through(&mut self, calls: Vec<CallRequest>) -> Result<Vec<SimulationResult>> {
        let mut results = Vec::with_capacity(calls.len());
        
        for call in calls {
            let result = self.step(call).await?;
            results.push(result);
        }
        
        Ok(results)
    }
    
    /// Get information about the current chain state
    pub fn current_state(&self) -> ChainStateInfo {
        ChainStateInfo {
            block_number: self.forked_state.block_number,
            transaction_count: self.results.len(),
            total_gas_used: self.total_gas_used,
            nonces: self.forked_state.nonces.clone(),
        }
    }
    
    /// Get all simulation results so far
    pub fn results(&self) -> &[SimulationResult] {
        &self.results
    }
    
    /// Reset the chain to its initial state
    /// 
    /// This discards all executed transactions and returns the chain
    /// to the state it had when created.
    pub async fn reset(&mut self) -> Result<()> {
        // Create fresh forked state at initial block
        self.forked_state = self.simulator.create_forked_state(self.initial_block)?;
        self.results.clear();
        self.total_gas_used = 0;
        
        Ok(())
    }
    
    /// Execute a single transaction with full trace information
    /// 
    /// Similar to `step()` but returns comprehensive trace data including
    /// internal transactions, event logs, and call traces.
    pub async fn step_with_trace(&mut self, call: CallRequest) -> Result<FullSimulationResult> {
        // Prepare the call with automatic nonce management
        let mut call = call;
        if let Some(from) = call.from {
            if call.nonce.is_none() {
                // Get nonce from our tracked state, or query from database if not tracked yet
                let nonce = if let Some(&tracked_nonce) = self.forked_state.nonces.get(&from) {
                    tracked_nonce
                } else {
                    // Query the actual nonce from the forked state database
                    let db_nonce = self.simulator.get_nonce_from_state(&mut self.forked_state, from)?;
                    // Store it in our tracking
                    self.forked_state.nonces.insert(from, db_nonce);
                    db_nonce
                };
                call.nonce = Some(nonce);
            }
        }
        
        // Use simulate_on_fork_with_trace to get full details
        let block_number = self.forked_state.block_number;
        let result = self.simulator.simulate_on_fork_with_trace(
            &mut self.forked_state, 
            call.clone(),
            block_number
        )?;
        
        // Update tracking
        if result.success {
            self.total_gas_used += result.gas_used;
            
            // Update nonce tracking
            if let Some(from) = call.from {
                let current = self.forked_state.nonces.entry(from).or_insert(0);
                *current += 1;
            }
        }
        
        // Store basic result for results tracking
        self.results.push(SimulationResult {
            success: result.success,
            gas_used: result.gas_used,
            revert_reason: result.revert_reason.clone(),
        });
        
        Ok(result)
    }
}

impl TxSimulator {
    /// Start a new simulation chain at the specified block
    /// 
    /// Creates a stateful simulation environment where each transaction
    /// builds on the state changes from previous ones.
    /// 
    /// # Example
    /// ```rust
    /// let simulator = TxSimulator::new("/path/to/db")?;
    /// let mut chain = simulator.start_simulation_chain(None).await?;
    /// 
    /// // Execute transactions sequentially with state preservation
    /// let buy_result = chain.step(buy_tx).await?;
    /// let approve_result = chain.step(approve_tx).await?;
    /// let sell_result = chain.step(sell_tx).await?;
    /// ```
    pub async fn start_simulation_chain(&self, at_block: Option<u64>) -> Result<SimulationChain> {
        let block_number = at_block.unwrap_or(self.get_latest_block()?);
        
        // Use the existing create_forked_state method
        let forked_state = self.create_forked_state(block_number)?;
        
        Ok(SimulationChain::new(
            Arc::new(self.clone()),
            forked_state,
            block_number,
        ))
    }
}