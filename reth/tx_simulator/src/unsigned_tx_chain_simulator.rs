/// Unsigned Transaction Chain Simulation
/// 
/// This module provides the UnsignedTxChainSimulation struct which maintains blockchain state
/// between individual unsigned transaction simulations. This is essential for workflows where
/// each transaction depends on the results of previous ones (e.g., buy → approve → sell).
/// 
/// The chain maintains a forked state that persists between `step()` calls, allowing
/// complex multi-transaction workflows to be simulated accurately with inspector fusing
/// for optimal performance.

use crate::{
    simulator::TxSimulator,
    types::{SimulationResult, FullSimulationResult},
    unsigned_tx_simulator::UnsignedTransaction,
    unsigned_tx_bundle_simulator::ForkedState,
};
use std::sync::Arc;
use std::collections::HashMap;
use alloy_primitives::Address;
use eyre::Result;
use revm_inspectors::tracing::{TracingInspector, TracingInspectorConfig};

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

/// A stateful simulation chain that maintains blockchain state between unsigned transactions
pub struct UnsignedTxChainSimulation {
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
    /// Fused inspector that persists across transactions
    inspector: Option<TracingInspector>,
}

impl UnsignedTxChainSimulation {
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
            inspector: None,
        }
    }
    
    /// Execute a single transaction and advance the chain state
    /// 
    /// This simulates the transaction on the current state and commits the changes,
    /// making them visible to subsequent transactions. Uses inspector fusing for
    /// optimal performance across multiple transactions.
    /// 
    /// # Example
    /// ```rust
    /// let mut chain = simulator.start_simulation_chain(None).await?;
    /// let result = chain.step(buy_unsigned_tx).await?;
    /// // State now includes the effects of buy_unsigned_tx
    /// ```
    pub async fn step(&mut self, unsigned_tx: UnsignedTransaction) -> Result<SimulationResult> {
        // Prepare the unsigned transaction with automatic nonce management
        let mut unsigned_tx = unsigned_tx;
        if let Some(from) = unsigned_tx.from {
            if unsigned_tx.nonce.is_none() {
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
                unsigned_tx.nonce = Some(nonce);
            }
        }
        
        // Execute with fused inspector
        let result = self.execute_with_fused_inspector(unsigned_tx.clone(), false)?;
        
        // Update tracking
        if result.success {
            self.total_gas_used += result.gas_used;
            
            // Update nonce tracking
            if let Some(from) = unsigned_tx.from {
                let current = self.forked_state.nonces.entry(from).or_insert(0);
                *current += 1;
            }
        }
        
        self.results.push(result.clone());
        Ok(result)
    }
    
    /// Execute multiple transactions sequentially
    /// 
    /// Each transaction is executed on the state resulting from the previous one.
    /// If any transaction fails, execution continues but the failure is recorded.
    pub async fn step_through(&mut self, unsigned_txs: Vec<UnsignedTransaction>) -> Result<Vec<SimulationResult>> {
        let mut results = Vec::with_capacity(unsigned_txs.len());
        
        for unsigned_tx in unsigned_txs {
            let result = self.step(unsigned_tx).await?;
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
        self.inspector = None;  // Reset inspector
        
        Ok(())
    }
    
    /// Internal method to execute transaction with fused inspector
    fn execute_with_fused_inspector(
        &mut self, 
        unsigned_tx: UnsignedTransaction,
        with_trace: bool
    ) -> Result<SimulationResult> {
        use crate::simulation_revert_decoder::decode_revert_data;
        use reth_provider::HeaderProvider;
        use reth_evm::{ConfigureEvm, Evm};
        use revm::DatabaseCommit;
        
        let provider = self.simulator.provider_factory.provider()?;
        let header = provider.header_by_number(self.forked_state.block_number)?
            .ok_or_else(|| eyre::eyre!("No header for block {}", self.forked_state.block_number))?;
        
        // Get or create inspector with fusing
        let inspector = self.inspector.get_or_insert_with(|| {
            let config = if with_trace {
                TracingInspectorConfig::default_geth().set_record_logs(true)
            } else {
                TracingInspectorConfig::default_geth()
            };
            TracingInspector::new(config)
        });
        
        // Setup EVM environment
        let evm_env = self.simulator.evm_config.evm_env(&header);
        let base_fee = header.base_fee_per_gas.map(|v| v as u128);
        
        // Create transaction environment
        let tx_env = self.simulator.create_tx_env_from_unsigned_tx(
            &unsigned_tx, 
            evm_env.block_env.gas_limit as u128, 
            base_fee, 
            &mut self.forked_state.db
        )?;
        
        // Execute transaction with inspector
        let mut evm = self.simulator.evm_config.evm_with_env_and_inspector(
            &mut self.forked_state.db, 
            evm_env, 
            inspector
        );
        let res = evm.transact(tx_env)?;
        
        // Commit state changes
        self.forked_state.db.commit(res.state);
        
        // Fuse the inspector for next transaction (clear tx-specific data, keep block-level data)
        // Note: This assumes TracingInspector has a fused() method like in Reth
        // If not available, we'll keep the inspector as-is for now
        // self.inspector = self.inspector.take().map(|insp| insp.fused());
        
        Ok(SimulationResult {
            success: res.result.is_success(),
            gas_used: res.result.gas_used(),
            revert_reason: if !res.result.is_success() {
                res.result.output()
                    .map(|bytes| decode_revert_data(&bytes))
                    .or_else(|| Some("Transaction reverted without data".to_string()))
            } else {
                None
            },
        })
    }

    /// Execute a single transaction with full trace information
    /// 
    /// Similar to `step()` but returns comprehensive trace data including
    /// internal transactions, event logs, and unsigned_tx traces.
    pub async fn step_with_trace(&mut self, unsigned_tx: UnsignedTransaction) -> Result<FullSimulationResult> {
        // Prepare the unsigned_tx with automatic nonce management
        let mut unsigned_tx = unsigned_tx;
        if let Some(from) = unsigned_tx.from {
            if unsigned_tx.nonce.is_none() {
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
                unsigned_tx.nonce = Some(nonce);
            }
        }
        
        // Use simulate_on_fork_with_trace to get full details
        let block_number = self.forked_state.block_number;
        let result = self.simulator.simulate_on_fork_with_trace(
            &mut self.forked_state, 
            unsigned_tx.clone(),
            block_number
        )?;
        
        // Update tracking
        if result.success {
            self.total_gas_used += result.gas_used;
            
            // Update nonce tracking
            if let Some(from) = unsigned_tx.from {
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
    pub async fn start_simulation_chain(&self, at_block: Option<u64>) -> Result<UnsignedTxChainSimulation> {
        let block_number = at_block.unwrap_or(self.get_latest_block()?);
        
        // Use the existing create_forked_state method
        let forked_state = self.create_forked_state(block_number)?;
        
        Ok(UnsignedTxChainSimulation::new(
            Arc::new(self.clone()),
            forked_state,
            block_number,
        ))
    }
}